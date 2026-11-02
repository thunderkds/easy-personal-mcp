"""Declared rules over core metrics (FR-006, FR-026). Rules are data; no rule reads raw events.

A rule fires when `measured <op> threshold`. A rule that cannot be measured (too little
history, no data in the window) abstains: it is listed with a reason, never counted as a pass.
"""

import statistics
from dataclasses import dataclass, replace
from datetime import UTC, datetime, timedelta
from pathlib import Path

from easy_personal.core import config, kit, metrics
from easy_personal.sources import context

PACKAGED = Path(__file__).resolve().parent.parent / "data" / "rules.yaml"
LABELS = ("improve", "aware", "warning")
LIVE_DEFAULTS = {"live_failing_streak": 3, "live_open_call_minutes": 10}  # live attention states (FR-048/049)
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
    title: str
    why: str
    steps: tuple[str, ...]

    @property
    def fix(self) -> str:
        """The steps as one line, for readers that predate `steps`."""
        return " ".join(self.steps)

    def with_threshold(self, threshold: float) -> "Rule":
        return replace(self, threshold=threshold)


class Abstain(Exception):
    """The metric cannot be measured; the message is the reason shown to the owner."""


def _number(text: str) -> float:
    value = float(text)
    return int(value) if value.is_integer() else value


def _parse_rule(rule_id: str, text: str) -> Rule:
    """`key=value` pairs, then ` title=`, ` why=` and ` steps=` (pipe-separated) whose text may hold `=`."""
    head, _, plain = text.partition(" title=")
    title, _, plain = plain.partition(" why=")
    why, _, steps = plain.partition(" steps=")
    steps = tuple(s.strip() for s in steps.split("|") if s.strip())
    for name, value in (("title", title.strip()), ("why", why.strip()), ("steps", steps)):
        if not value:
            raise ValueError(f"rule {rule_id}: missing {name}")
    if len(steps) > 3:
        raise ValueError(f"rule {rule_id}: steps must be 1 to 3")
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
        title=title.strip(),
        why=why.strip(),
        steps=steps,
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
    unknown = set(overrides) - {r.id for r in pack} - set(LIVE_DEFAULTS)
    if unknown:
        raise ValueError(
            f"config {config_file}: thresholds for unknown rules: {', '.join(sorted(unknown))}"
        )
    try:
        for key in LIVE_DEFAULTS.keys() & overrides.keys():
            _number(overrides[key])
        return [
            r.with_threshold(_number(overrides[r.id])) if r.id in overrides else r for r in pack
        ]
    except ValueError:
        raise ValueError(f"config thresholds must be numbers: {config_file}") from None


def live_thresholds(config_file: Path | None = None) -> dict[str, float]:
    """The two live attention bounds: defaults, with `thresholds:` of `config_file` applied (same checks as rules)."""
    found = dict(LIVE_DEFAULTS)
    if config_file is not None and config_file.exists():
        load_rules(config_file)  # refuses unknown keys and non-numbers with the one message
        overrides = _read_flat(config_file, "thresholds")
        found.update({k: _number(overrides[k]) for k in LIVE_DEFAULTS if k in overrides})
    return found


def _iso(moment: datetime) -> str:
    return moment.astimezone(UTC).isoformat()


def _percent(value: float) -> str:
    return f"{value * 100:,.1f}".rstrip("0").rstrip(".") + "%"


def _count(n: int, one: str, many: str) -> str:
    return f"{n} {one if n == 1 else many}"


def summary(rule: Rule, measured, detail: dict | None = None) -> str:
    """One human sentence: what was measured, the target and the look-back window."""
    detail = detail or {}
    days = rule.window_days
    target = f"{_percent(rule.threshold)} {'or more' if rule.op == 'lt' else 'or less'}"
    if rule.metric == "cache_hit_ratio":
        sentence = f"{_percent(measured)} of prompt tokens came from cache"
    elif rule.metric == "median_session_context":
        sentence = f"The median open session reached {measured:,.0f} tokens of context"
        target = f"{rule.threshold:,.0f} or fewer"
    elif rule.metric == "unreadable_share":
        version = f" {detail['version']}" if "version" in detail else ""
        sentence = f"{_percent(measured)} of records from Claude Code{version} could not be read"
    elif rule.metric == "week_over_week_tokens":
        way = "rose" if measured >= 0 else "fell"
        return (
            f"Token use {way} {_percent(abs(measured))} in the last {days} days against the {days} days "
            f"before; target {target}"
        )
    elif rule.metric == "top_project_share":
        sentence = f"One project ({detail.get('project', '?')}) used {_percent(measured)} of tokens"
    elif rule.metric == "kit_error_rate":
        sentence = f"{_percent(measured)} of kernel-trace tool calls failed"
    else:  # context_stale_indexes: a count of projects
        n = int(measured)
        sentence = f"{n} project{' has' if n == 1 else 's have'} a stale easy-context index"
        target = "none" if rule.threshold == 0 else f"{rule.threshold:,.0f} or fewer"
    return f"{sentence}; target {target}, last {days} days"


# Metric functions: (conn, since, until, **scope) -> (measured, detail); `scope` is the optional
# `workspace` and `account` of `metrics._scope`. Raise Abstain when unmeasurable.
def _cache_hit_ratio(conn, since, until, **scope):
    ratio = metrics.usage_summary(conn, since, until, **scope)["cache_hit_ratio"]
    if ratio is None:
        raise Abstain("no prompt tokens in the window")
    return ratio, None


def _median_session_context(conn, since, until, **scope):
    sessions = metrics.session_contexts(conn, since, until, **scope)
    if not sessions:
        raise Abstain("no sessions in the window")
    open_sessions = [s["max_context_tokens"] for s in sessions if not s["compacted"]]
    return (statistics.median(open_sessions) if open_sessions else 0), None


def _unreadable_share(conn, since, until, **scope):
    shares = {
        version: round(bad / total, 4)
        for version, (bad, total) in metrics.record_readability(conn, since, until, **scope).items()
        if total
    }
    if not shares:
        raise Abstain("no records in the window")
    worst = max(sorted(shares), key=shares.get)  # ties: first version name
    return shares[worst], {"version": worst}


def _top_project_share(conn, since, until, **scope):
    groups = metrics.usage_breakdown(conn, "project", since, until, **scope)["groups"]
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


def _week_over_week_tokens(conn, since, until, **scope):
    """Tokens of [since, until) against the equally long window before it: +0.5 means +50%."""
    start, end = datetime.fromisoformat(since), datetime.fromisoformat(until)

    def tokens(lo, hi):
        total = metrics.usage_breakdown(conn, "day", _iso(lo), _iso(hi), **scope)["total"]
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


def _kit_error_rate(conn, since, until, **scope):
    rate, calls = kit.kernel_error_rate(conn, since, until, **scope)
    if not calls:
        raise Abstain("no kernel-trace tool calls in the window")
    return round(rate, 4), None


def _context_stale_indexes(conn, since, until, **scope):
    within = set(kit.scoped_projects(conn, **scope)) if any(v is not None for v in scope.values()) else None
    found = context.stale_projects(conn, within)
    if found is None:
        raise Abstain("no easy-context data (service down or no index for a known project)")
    count, names = found
    return count, {"projects": names}


METRICS = {
    "cache_hit_ratio": _cache_hit_ratio,
    "median_session_context": _median_session_context,
    "unreadable_share": _unreadable_share,
    "top_project_share": _top_project_share,
    "week_over_week_tokens": _week_over_week_tokens,
    "kit_error_rate": _kit_error_rate,
    "context_stale_indexes": _context_stale_indexes,
}


def _measure(rule: Rule, conn, now, scope: dict):
    start = metrics.history_start(conn)
    needed = now - timedelta(days=rule.min_history_days)
    if start is None:
        raise Abstain("no stored history")
    if datetime.fromisoformat(start) > needed:
        raise Abstain(f"less than {rule.min_history_days} days of stored history")
    since = _iso(now - timedelta(days=rule.window_days))
    return METRICS[rule.metric](conn, since, _iso(now), **scope)


def evaluate(
    conn,
    now: datetime | None = None,
    rules: list[Rule] | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> dict:
    """Run every rule; `hits` are the fired rules, `abstained` the ones that could not be measured.

    Hits are ordered by priority, then rule id, so the same store gives the same output.
    `workspace` (a name) and `account` (id or label) limit every metric to that scope; an unknown
    one raises ValueError, even when the store has too little history to measure anything.
    """
    now = now or datetime.now(UTC)
    scope = {"workspace": workspace, "account": account}
    if workspace is not None or account is not None:
        metrics._scope(conn, workspace, None, None, account)
    hits, abstained = [], []
    for rule in rules if rules is not None else load_rules(config.config_path()):
        try:
            measured, detail = _measure(rule, conn, now, scope)
        except Abstain as why:
            abstained.append({"rule_id": rule.id, "title": rule.title, "reason": str(why)})
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
                "title": rule.title,
                "why": rule.why,
                "steps": list(rule.steps),
                "summary": summary(rule, measured, detail),
            }
            if detail:
                hit["detail"] = detail
            hits.append(hit)
    hits.sort(key=lambda h: (h["priority"], h["rule_id"]))
    abstained.sort(key=lambda a: a["rule_id"])
    return {"as_of_utc": _iso(now), "hits": hits, "abstained": abstained}
