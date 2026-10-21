"""Declared rules over core metrics (FR-006, FR-026). Rules are data; no rule reads raw events.

A rule fires when `measured <op> threshold`. A rule that cannot be measured (too little
history, no data in the window) abstains: it is listed with a reason, never counted as a pass.
"""

import statistics
from dataclasses import dataclass, replace
from datetime import UTC, datetime, timedelta
from pathlib import Path

from easy_personal.core import config, metrics

PACKAGED = Path(__file__).resolve().parent.parent / "data" / "rules.yaml"
LABELS = ("improve", "aware", "warning")
OPS = {"lt": lambda m, t: m < t, "gt": lambda m, t: m > t}


@dataclass(frozen=True)
class Rule:
    id: str
    metric: str
    op: str
    threshold: float
    label: str
    priority: int
    window_days: int
    min_history_days: int
    fix: str

    def with_threshold(self, threshold: float) -> "Rule":
        return replace(self, threshold=threshold)


class Abstain(Exception):
    """The metric cannot be measured; the message is the reason shown to the owner."""


def _number(text: str) -> float:
    value = float(text)
    return int(value) if value.is_integer() else value


def _parse_rule(rule_id: str, text: str) -> Rule:
    head, _, fix = text.partition(" fix=")
    fields = dict(item.split("=", 1) for item in head.split())
    if fields["op"] not in OPS or fields["label"] not in LABELS or fields["metric"] not in METRICS:
        raise ValueError(f"rule {rule_id}: bad op, label or metric")
    return Rule(
        id=rule_id,
        metric=fields["metric"],
        op=fields["op"],
        threshold=_number(fields["threshold"]),
        label=fields["label"],
        priority=int(fields["priority"]),
        window_days=int(fields["window_days"]),
        min_history_days=int(fields["min_history_days"]),
        fix=fix.strip(),
    )


def _read_flat(path: Path, section: str) -> dict[str, str]:
    """`section:` followed by indented `name: value` lines -> {name: value}."""
    found, inside = {}, False
    for line in path.read_text().splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line.startswith(" "):
            inside = line.partition(":")[0] == section
        elif inside:
            name, _, value = line.strip().partition(":")
            found[name] = value.strip()
    return found


def load_rules(config_file: Path | None = None) -> list[Rule]:
    """Packaged pack, with `thresholds:` from `config_file` applied (a missing file = defaults)."""
    pack = [_parse_rule(k, v) for k, v in _read_flat(PACKAGED, "rules").items()]
    if config_file is None or not config_file.exists():
        return pack
    overrides = _read_flat(config_file, "thresholds")
    unknown = set(overrides) - {r.id for r in pack}
    if unknown:
        raise ValueError(
            f"config {config_file}: thresholds for unknown rules: {', '.join(sorted(unknown))}"
        )
    try:
        return [
            r.with_threshold(_number(overrides[r.id])) if r.id in overrides else r for r in pack
        ]
    except ValueError:
        raise ValueError(f"config thresholds must be numbers: {config_file}") from None


def _iso(moment: datetime) -> str:
    return moment.astimezone(UTC).isoformat()


# Metric functions: (conn, since, until) -> (measured, detail). Raise Abstain when unmeasurable.
def _cache_hit_ratio(conn, since, until):
    ratio = metrics.usage_summary(conn, since, until)["cache_hit_ratio"]
    if ratio is None:
        raise Abstain("no prompt tokens in the window")
    return ratio, None


def _median_session_context(conn, since, until):
    sessions = metrics.session_contexts(conn, since, until)
    if not sessions:
        raise Abstain("no sessions in the window")
    open_sessions = [s["max_context_tokens"] for s in sessions if not s["compacted"]]
    return (statistics.median(open_sessions) if open_sessions else 0), None


def _unreadable_share(conn, since, until):
    shares = {
        version: round(bad / total, 4)
        for version, (bad, total) in metrics.record_readability(conn, since, until).items()
        if total
    }
    if not shares:
        raise Abstain("no records in the window")
    worst = max(sorted(shares), key=shares.get)  # ties: first version name
    return shares[worst], {"version": worst}


def _top_project_share(conn, since, until):
    groups = metrics.usage_breakdown(conn, "project", since, until)["groups"]
    totals = {
        g["key"]: g["input_tokens"]
        + g["output_tokens"]
        + g["cache_creation_input_tokens"]
        + g["cache_read_input_tokens"]
        for g in groups
    }
    whole = sum(totals.values())
    if len(totals) < 2 or not whole:
        raise Abstain("fewer than two projects with tokens in the window")
    top = min(totals, key=lambda k: (-totals[k], k))  # ties: first folder name
    return round(totals[top] / whole, 4), {"project": top}


def _week_over_week_tokens(conn, since, until):
    """Tokens of [since, until) against the equally long window before it: +0.5 means +50%."""
    start, end = datetime.fromisoformat(since), datetime.fromisoformat(until)

    def tokens(lo, hi):
        total = metrics.usage_breakdown(conn, "day", _iso(lo), _iso(hi))["total"]
        return (
            total["input_tokens"]
            + total["output_tokens"]
            + total["cache_creation_input_tokens"]
            + total["cache_read_input_tokens"]
        )

    previous = tokens(start - (end - start), start)
    if not previous:
        raise Abstain("no tokens in the previous window")
    return round(tokens(start, end) / previous - 1, 4), None


METRICS = {
    "cache_hit_ratio": _cache_hit_ratio,
    "median_session_context": _median_session_context,
    "unreadable_share": _unreadable_share,
    "top_project_share": _top_project_share,
    "week_over_week_tokens": _week_over_week_tokens,
}


def _measure(rule: Rule, conn, now):
    start = metrics.history_start(conn)
    needed = now - timedelta(days=rule.min_history_days)
    if start is None:
        raise Abstain("no stored history")
    if datetime.fromisoformat(start) > needed:
        raise Abstain(f"less than {rule.min_history_days} days of stored history")
    since = _iso(now - timedelta(days=rule.window_days))
    return METRICS[rule.metric](conn, since, _iso(now))


def evaluate(conn, now: datetime | None = None, rules: list[Rule] | None = None) -> dict:
    """Run every rule; `hits` are the fired rules, `abstained` the ones that could not be measured.

    Hits are ordered by priority, then rule id, so the same store gives the same output.
    """
    now = now or datetime.now(UTC)
    hits, abstained = [], []
    for rule in rules if rules is not None else load_rules(config.config_path()):
        try:
            measured, detail = _measure(rule, conn, now)
        except Abstain as why:
            abstained.append({"rule_id": rule.id, "reason": str(why)})
            continue
        if OPS[rule.op](measured, rule.threshold):
            hit = {
                "rule_id": rule.id,
                "label": rule.label,
                "priority": rule.priority,
                "measured": measured,
                "threshold": rule.threshold,
                "window_days": rule.window_days,
                "fix": rule.fix,
            }
            if detail:
                hit["detail"] = detail
            hits.append(hit)
    hits.sort(key=lambda h: (h["priority"], h["rule_id"]))
    abstained.sort(key=lambda a: a["rule_id"])
    return {"as_of_utc": _iso(now), "hits": hits, "abstained": abstained}
