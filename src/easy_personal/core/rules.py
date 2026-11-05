"""Declared rules over core metrics (FR-006, FR-026). Rules are data; no rule reads raw events.

A rule fires when `measured <op> threshold`. A rule that cannot be measured (too little
history, no data in the window) abstains: it is listed with a reason, never counted as a pass.
"""

import statistics
from dataclasses import dataclass, replace
from datetime import UTC, datetime, timedelta
from pathlib import Path

from easy_personal.core import config, kit, metrics, owners
from easy_personal.sources import context

PACKAGED = Path(__file__).resolve().parent.parent / "data" / "rules.yaml"
LABELS = ("improve", "aware", "warning")
LIVE_DEFAULTS = {"live_failing_streak": 3, "live_open_call_minutes": 10}  # live attention states (FR-048/049)
KIT_ERROR_RATE_DEFAULT = 0.04  # median session tool-error rate on the owner store after the v8 re-read (HUN-193)
THRESHOLD_KEYS = (*LIVE_DEFAULTS, "kit_error_rate")  # config keys that are not rule ids
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
    unknown = set(overrides) - {r.id for r in pack} - set(THRESHOLD_KEYS)
    if unknown:
        raise ValueError(
            f"config {config_file}: thresholds for unknown rules: {', '.join(sorted(unknown))}"
        )
    try:
        for key in set(THRESHOLD_KEYS) & overrides.keys():
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


def kit_error_rate(config_file: Path | None = None) -> float:
    """Highest tool-error rate of a good session (FR-061): default, or `kit_error_rate` under `thresholds:`."""
    if config_file is not None and config_file.exists():
        load_rules(config_file)  # refuses unknown keys and non-numbers with the one message
        overrides = _read_flat(config_file, "thresholds")
        if "kit_error_rate" in overrides:
            return _number(overrides["kit_error_rate"])
    return KIT_ERROR_RATE_DEFAULT


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


# Metric functions: (conn, since, until, win=None, **scope) -> (measured, detail); `scope` is the optional
# `workspace` and `account` of `metrics._scope`. Raise Abstain when unmeasurable. The token metrics read the
# token windows `win` (one scan per window, shared with the causes of a hit); the formulas are those of
# `metrics.cache_hit_ratio`, `token_total` and `tokens_by_project`.
def _cache_hit_ratio(conn, since, until, win=None, **scope):
    window = (win or _Windows(conn, scope))(since, until)
    ratio = _reuse(window["cache_read"], window["prompt"])
    if ratio is None:
        raise Abstain("no prompt tokens in the window")
    return ratio, None


def _median_session_context(conn, since, until, win=None, **scope):
    sessions = metrics.session_contexts(conn, since, until, **scope)
    if not sessions:
        raise Abstain("no sessions in the window")
    open_sessions = [s["max_context_tokens"] for s in sessions if not s["compacted"]]
    return (statistics.median(open_sessions) if open_sessions else 0), None


def _unreadable_share(conn, since, until, win=None, **scope):
    shares = {
        version: round(bad / total, 4)
        for version, (bad, total) in metrics.record_readability(conn, since, until, **scope).items()
        if total
    }
    if not shares:
        raise Abstain("no records in the window")
    worst = max(sorted(shares), key=shares.get)  # ties: first version name
    return shares[worst], {"version": worst}


def _top_project_share(conn, since, until, win=None, **scope):
    window = (win or _Windows(conn, scope))(since, until)
    totals = {name: project["tokens"] for name, project in window["projects"].items()}
    whole = sum(totals.values())
    if len(totals) < 2 or not whole:
        raise Abstain("fewer than two projects with tokens in the window")
    top = min(totals, key=lambda k: (-totals[k], k))  # ties: first folder name
    return round(totals[top] / whole, 4), {"project": top}


def _week_over_week_tokens(conn, since, until, win=None, **scope):
    """Tokens of [since, until) against the equally long window before it: +0.5 means +50%."""
    start, end = datetime.fromisoformat(since), datetime.fromisoformat(until)
    win = win or _Windows(conn, scope)

    def tokens(lo, hi):
        return win(_iso(lo), _iso(hi))["tokens"]

    previous = tokens(start - (end - start), start)
    if not previous:
        raise Abstain("no tokens in the previous window")
    return round(tokens(start, end) / previous - 1, 4), None


def _kit_error_rate(conn, since, until, win=None, **scope):
    rate, calls = kit.kernel_error_rate(conn, since, until, **scope)
    if not calls:
        raise Abstain("no kernel-trace tool calls in the window")
    return round(rate, 4), None


def _context_stale_indexes(conn, since, until, win=None, **scope):
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


# --- Causes and diagnosis (FR-073) -------------------------------------------------------------
# For a hit, `_causes_<metric>(conn, since, until, measured, detail, win, **scope) -> (diagnosis, causes, facts)`
# looks at the same window and scope as the rule. `causes` are at most CAUSE_LIMIT items (name, kind, numbers, a
# fixed text built from the numbers); `facts` fill the fix lines of FIXES[(rule id, diagnosis)]. Ids, folder names,
# tool names and numbers only: never message or tool content. `win(since, until)` is the memoized token window.
CAUSE_LIMIT = 3
MIN_PROJECT_SHARE = 0.05  # a project needs this share of tokens to be named for low cache reuse
BIGGER_SESSIONS_FROM = 0.10  # tokens per session up by more than this = bigger sessions, not more sessions
TOKEN_METRICS = ("week_over_week_tokens", "cache_hit_ratio", "median_session_context", "top_project_share")


def _short(n: float) -> str:
    for unit, size in (("B", 1e9), ("M", 1e6), ("k", 1e3)):
        if n >= size:
            return f"{n / size:,.1f}{unit}"
    return f"{n:,.0f}"


def _fill(lines: tuple[str, ...], facts: dict) -> list[str]:
    """The fix lines whose placeholders all have a fact; a line about something the data lacks is left out."""
    steps = []
    for line in lines:
        try:
            steps.append(line.format_map(facts))
        except KeyError:
            continue
    return steps


FIXES = {
    ("tokens_jump", "more_sessions"): (
        "The rise comes from more sessions ({sessions_before} → {sessions_now}), not bigger ones: tokens per session went from {tps_before} to {tps_now}.",
        "New since the period before: {new}. Check that it should run this often.",
        "Biggest rise: {top} (+{top_rise} tokens). Decide if that pace is intended before it becomes your baseline.",
    ),
    ("tokens_jump", "bigger_sessions"): (
        "Each session got bigger: tokens per session went from {tps_before} to {tps_now} ({sessions_before} → {sessions_now} sessions).",
        "The largest session, {top_session} in {top_session_project}, used {top_session_tokens} tokens. Run /compact in long sessions or start a new one.",
        "Decide if the rise is intended before it becomes your baseline.",
    ),
    ("cache_hit_low", "model_switching"): (
        "{multi_count} sessions used 2 or more models; their cache reuse is {multi_reuse} against {single_reuse} in the others.",
        "Use one model for a whole session: switching model in the middle restarts the cache. Biggest case: {top_session} in {top_session_project}.",
        "Lowest reuse by project: {low_project} ({low_reuse}).",
    ),
    ("cache_hit_low", "unstable_prefix"): (
        "Lowest cache reuse by project: {low_project} ({low_reuse} of its prompt tokens came from cache).",
        "Keep the start of your prompts the same from one turn to the next.",
        "Avoid editing CLAUDE.md or switching models in the middle of a session.",
    ),
    ("session_huge_no_compact", "huge_sessions"): (
        "The largest open session is {top_session} in {top_session_project}: {top_context} tokens of context.",
        "Run /compact when a session grows long.",
        "Start a new session before the context passes {threshold} tokens.",
    ),
    ("project_dominates", "dominant_project"): (
        "{project} used {share} of tokens; its largest session, {top_session}, alone used {top_session_share} of all tokens.",
        "Confirm that this project should take most of your effort.",
        "Split long-running work into shorter sessions.",
    ),
    ("tool_errors_high", "one_tool_fails"): (
        "{top} caused {top_errors} of the {total_errors} failed calls ({top_share}).",
        "Read the failing {top} calls in memory/event-trace/ and fix the command or permission that keeps failing.",
    ),
    ("tool_errors_high", "errors_spread"): (
        "Failures are spread over many tools; {top} has the most failed calls: {top_errors} of {total_errors}.",
        "Read the failing tool calls of {top} in memory/event-trace/.",
        "Fix the command or permission that keeps failing.",
    ),
    ("records_unreadable", "unreadable_records"): (
        "Most unreadable records are {top} records from Claude Code {top_version} ({top_count} records).",
        "Update easy-personal to the latest version.",
        "If it stays high, tell the easy-personal maintainer about {top} records from Claude Code {top_version}.",
    ),
    ("context_index_stale", "stale_index"): (
        "The index is older than the latest commit in: {names} ({count} in all).",
        "Call `index_path` in easy-context for the listed projects.",
        "You run it yourself; easy-personal never does.",
    ),
}


class _Windows:
    """Tokens of a window per session and per project folder (FR-057/FR-069 grouping), built once per window per `evaluate`."""

    def __init__(self, conn, scope: dict):
        self.conn, self.scope, self._built, self._pmap = conn, scope, {}, None

    @property
    def pmap(self) -> dict[str, str]:
        if self._pmap is None:
            self._pmap = owners.project_map(self.conn)
        return self._pmap

    def folder(self, path: str | None) -> str:
        return metrics._folder(self.pmap.get(path, path))

    def __call__(self, since: str, until: str) -> dict:
        if (since, until) in self._built:
            return self._built[(since, until)]
        projects: dict[str, dict] = {}
        sessions: dict[str, dict] = {}
        total = prompt_total = cache_read = 0
        for row in metrics.session_tokens(self.conn, since, until, **self.scope):
            tokens = row["input"] + row["output"] + row["cache_creation"] + row["cache_read"]
            prompt = row["input"] + row["cache_creation"] + row["cache_read"]
            folder = self.folder(row["project_path"])
            total += tokens
            prompt_total += prompt
            cache_read += row["cache_read"]
            project = projects.setdefault(folder, {"tokens": 0, "prompt": 0, "cache_read": 0, "sessions": set()})
            project["tokens"] += tokens
            project["prompt"] += prompt
            project["cache_read"] += row["cache_read"]
            if row["session_id"] is None:
                continue
            project["sessions"].add(row["session_id"])
            found = sessions.setdefault(
                row["session_id"],
                {"id": row["session_id"], "tokens": 0, "prompt": 0, "cache_read": 0, "models": 0, "top": (-1, "")},
            )
            found["tokens"] += tokens
            found["prompt"] += prompt
            found["cache_read"] += row["cache_read"]
            found["models"] = max(found["models"], row["models"])
            found["top"] = max(found["top"], (tokens, folder))
        for found in sessions.values():
            found["project"] = found.pop("top")[1]  # the project that holds most of the session's tokens
        built = {"projects": projects, "sessions": sessions, "tokens": total, "prompt": prompt_total, "cache_read": cache_read}
        self._built[(since, until)] = built
        return built


def _reuse(cache_read: int, prompt: int) -> float | None:
    return round(cache_read / prompt, 4) if prompt else None


def _biggest_sessions(window, within: str | None = None) -> list[dict]:
    found = [s for s in window["sessions"].values() if within is None or s["project"] == within]
    return sorted(found, key=lambda s: (-s["tokens"], s["id"]))[:CAUSE_LIMIT]


def _session_cause(session: dict, window) -> dict:
    return {
        "name": session["id"],
        "kind": "session",
        "project": session["project"],
        "tokens": session["tokens"],
        "token_share": round(session["tokens"] / window["tokens"], 4) if window["tokens"] else 0.0,
        "text": f"{session['id']} in {session['project']} used {_short(session['tokens'])} tokens",
    }


def _causes_week_over_week_tokens(conn, since, until, measured, detail, win, **scope):
    start, end = datetime.fromisoformat(since), datetime.fromisoformat(until)
    before, now = win(_iso(start - (end - start)), _iso(start)), win(since, until)
    n_before, n_now = len(before["sessions"]), len(now["sessions"])
    tps_before = before["tokens"] / n_before if n_before else 0
    tps_now = now["tokens"] / n_now if n_now else 0
    facts = {
        "sessions_before": n_before,
        "sessions_now": n_now,
        "tps_before": _short(tps_before),
        "tps_now": _short(tps_now),
    }
    if not tps_before or tps_now > tps_before * (1 + BIGGER_SESSIONS_FROM):
        biggest = _biggest_sessions(now)
        if biggest:
            facts.update(
                top_session=biggest[0]["id"],
                top_session_project=biggest[0]["project"],
                top_session_tokens=_short(biggest[0]["tokens"]),
            )
        return "bigger_sessions", [_session_cause(s, now) for s in biggest], facts
    rises = []
    for name in before["projects"].keys() | now["projects"].keys():
        was, is_ = before["projects"].get(name), now["projects"].get(name)
        rise = (is_["tokens"] if is_ else 0) - (was["tokens"] if was else 0)
        if rise > 0:  # a project that fell, or left, is never a cause of the rise
            rises.append((rise, name, was, is_))
    rises.sort(key=lambda r: (-r[0], r[1]))
    causes = []
    for _, name, was, is_ in rises[:CAUSE_LIMIT]:
        tokens_before = was["tokens"] if was else 0
        sessions_before, sessions_now = len(was["sessions"]) if was else 0, len(is_["sessions"])
        causes.append(
            {
                "name": name,
                "kind": "project",
                "tokens_before": tokens_before,
                "tokens_now": is_["tokens"],
                "sessions_before": sessions_before,
                "sessions_now": sessions_now,
                "new": tokens_before == 0,
                "text": f"{name}: {_short(tokens_before)} → {_short(is_['tokens'])} tokens, "
                f"{sessions_before} → {sessions_now} sessions" + (" (new)" if tokens_before == 0 else ""),
            }
        )
    if causes:
        facts.update(top=causes[0]["name"], top_rise=_short(rises[0][0]))
    if any(c["new"] for c in causes):
        facts["new"] = ", ".join(c["name"] for c in causes if c["new"])
    return "more_sessions", causes, facts


def _causes_cache_hit_ratio(conn, since, until, measured, detail, win, **scope):
    now = win(since, until)
    low = sorted(
        (
            (_reuse(p["cache_read"], p["prompt"]), name, p)
            for name, p in now["projects"].items()
            if p["prompt"] and p["tokens"] >= now["tokens"] * MIN_PROJECT_SHARE
        ),
        key=lambda r: (r[0], r[1]),
    )
    project_causes = [
        {
            "name": name,
            "kind": "project",
            "cache_reuse": reuse,
            "tokens": p["tokens"],
            "token_share": round(p["tokens"] / now["tokens"], 4),
            "text": f"{name} read {_percent(reuse)} of its prompt tokens from cache ({_percent(p['tokens'] / now['tokens'])} of all tokens)",
        }
        for reuse, name, p in low
    ]
    facts = {}
    if low:
        facts.update(low_project=low[0][1], low_reuse=_percent(low[0][0]))
    sessions = [s for s in now["sessions"].values() if s["prompt"]]
    multi = [s for s in sessions if s["models"] >= 2]
    single = [s for s in sessions if s["models"] < 2]
    multi_reuse = _reuse(sum(s["cache_read"] for s in multi), sum(s["prompt"] for s in multi))
    single_reuse = _reuse(sum(s["cache_read"] for s in single), sum(s["prompt"] for s in single))
    if multi and single and multi_reuse < single_reuse:
        # the sessions with the most prompt tokens that did not come from cache
        worst = sorted(multi, key=lambda s: (s["cache_read"] - s["prompt"], s["id"]))[:2]
        session_causes = [
            {
                "name": s["id"],
                "kind": "session",
                "project": s["project"],
                "models": s["models"],
                "cache_reuse": _reuse(s["cache_read"], s["prompt"]),
                "tokens": s["tokens"],
                "text": f"{s['id']} in {s['project']} used {s['models']} models and read "
                f"{_percent(_reuse(s['cache_read'], s['prompt']))} of its prompt tokens from cache",
            }
            for s in worst
        ]
        facts.update(
            multi_count=len(multi),
            multi_reuse=_percent(multi_reuse),
            single_reuse=_percent(single_reuse),
            top_session=worst[0]["id"],
            top_session_project=worst[0]["project"],
        )
        return "model_switching", (session_causes + project_causes)[:CAUSE_LIMIT], facts
    return "unstable_prefix", project_causes[:CAUSE_LIMIT], facts


def _causes_median_session_context(conn, since, until, measured, detail, win, **scope):
    now = win(since, until)
    open_sessions = sorted(
        (s for s in metrics.session_contexts(conn, since, until, **scope) if not s["compacted"]),
        key=lambda s: (-s["max_context_tokens"], s["session_id"]),
    )[:CAUSE_LIMIT]
    causes = []
    for s in open_sessions:
        project = now["sessions"].get(s["session_id"], {}).get("project", "(unknown)")
        causes.append(
            {
                "name": s["session_id"],
                "kind": "session",
                "project": project,
                "context_tokens": s["max_context_tokens"],
                "text": f"{s['session_id']} in {project} reached {_short(s['max_context_tokens'])} tokens of context without compacting",
            }
        )
    facts = {}
    if causes:
        facts.update(
            top_session=causes[0]["name"],
            top_session_project=causes[0]["project"],
            top_context=_short(causes[0]["context_tokens"]),
        )
    return "huge_sessions", causes, facts


def _causes_top_project_share(conn, since, until, measured, detail, win, **scope):
    now = win(since, until)
    biggest = _biggest_sessions(now, within=detail["project"])
    facts = {"project": detail["project"], "share": _percent(measured)}
    if biggest:
        facts.update(top_session=biggest[0]["id"], top_session_share=_percent(biggest[0]["tokens"] / now["tokens"]))
    return "dominant_project", [_session_cause(s, now) for s in biggest], facts


def _causes_kit_error_rate(conn, since, until, measured, detail, win, **scope):
    by_tool: dict[str, list[int]] = {}
    by_project: dict[str, list[int]] = {}
    for tool, path, calls, errors in kit.kernel_errors_by_tool(conn, since, until, **scope):
        for table, key in ((by_tool, tool or "(unknown)"), (by_project, win.folder(path))):
            totals = table.setdefault(key, [0, 0])
            totals[0] += calls
            totals[1] += errors
    total_errors = sum(errors for _, errors in by_tool.values())
    tools = sorted(((n, c, e) for n, (c, e) in by_tool.items() if e), key=lambda r: (-r[2], r[0]))
    one_tool = bool(tools) and tools[0][2] * 2 > total_errors
    projects = sorted(((n, c, e) for n, (c, e) in by_project.items() if e), key=lambda r: (-r[2], r[0]))
    kind, rows = ("tool", tools) if one_tool else ("project", projects)
    causes = [
        {
            "name": name,
            "kind": kind,
            "calls": calls,
            "errors": errors,
            "error_rate": round(errors / calls, 4),
            "text": f"{name}: {errors} of {calls} calls failed ({_percent(errors / calls)})",
        }
        for name, calls, errors in rows[:CAUSE_LIMIT]
    ]
    facts = {"total_errors": total_errors}
    if causes:
        facts.update(
            top=causes[0]["name"], top_errors=causes[0]["errors"], top_share=_percent(causes[0]["errors"] / total_errors)
        )
    return ("one_tool_fails" if one_tool else "errors_spread"), causes, facts


def _causes_unreadable_share(conn, since, until, measured, detail, win, **scope):
    causes = [
        {
            "name": record_type or "(unknown)",
            "kind": "record_type",
            "version": version,
            "unreadable": count,
            "text": f"{record_type or '(unknown)'} records from Claude Code {version}: {count} could not be read",
        }
        for record_type, version, count in metrics.unreadable_records(conn, since, until, **scope)[:CAUSE_LIMIT]
    ]
    facts = {}
    if causes:
        facts.update(top=causes[0]["name"], top_version=causes[0]["version"], top_count=causes[0]["unreadable"])
    return "unreadable_records", causes, facts


def _causes_context_stale_indexes(conn, since, until, measured, detail, win, **scope):
    stale = set(detail["projects"])
    causes = []
    for (path,) in conn.execute("SELECT project_path FROM context_index ORDER BY 1"):
        if len(causes) == CAUSE_LIMIT:
            break
        name = Path(path).name or path
        if name not in stale:
            continue
        found = context.freshness(conn, path) or {}
        if not found.get("stale"):
            continue
        causes.append(
            {
                "name": name,
                "kind": "project",
                "last_index_at": found["last_index_at"],
                "latest_commit_utc": found["latest_commit_utc"],
                "text": f"{name}: indexed {found['last_index_at']}, latest commit {found['latest_commit_utc']}",
            }
        )
    return "stale_index", causes, {"names": ", ".join(c["name"] for c in causes), "count": int(measured)}


CAUSES = {
    "cache_hit_ratio": _causes_cache_hit_ratio,
    "median_session_context": _causes_median_session_context,
    "unreadable_share": _causes_unreadable_share,
    "top_project_share": _causes_top_project_share,
    "week_over_week_tokens": _causes_week_over_week_tokens,
    "kit_error_rate": _causes_kit_error_rate,
    "context_stale_indexes": _causes_context_stale_indexes,
}


def _explain(rule: Rule, conn, now, measured, detail, win, scope: dict) -> dict:
    """`causes`, `diagnosis`, the fix lines (`steps`, `fix`) and, for a rule that counts tokens, `cache_read_share`."""
    since, until = _iso(now - timedelta(days=rule.window_days)), _iso(now)
    diagnosis, causes, facts = CAUSES[rule.metric](conn, since, until, measured, detail, win, **scope)
    facts["threshold"] = f"{rule.threshold:,g}"
    steps = _fill(FIXES.get((rule.id, diagnosis), ()), facts) or list(rule.steps)
    found = {"causes": causes, "diagnosis": diagnosis, "steps": steps, "fix": " ".join(steps)}
    if rule.metric in TOKEN_METRICS:
        window = win(since, until)
        found["cache_read_share"] = round(window["cache_read"] / window["tokens"], 4) if window["tokens"] else None
    return found


def left_out(rule: Rule, conn, now: datetime, scope: dict) -> bool:
    """True when the rule is left out because the kit part it checks is not in use in this scope (FR-080).

    Only `context_stale_indexes` can be: easy-context counts as in use with a tool call in the last 7 days.
    """
    if rule.metric != "context_stale_indexes":
        return False
    since = _iso(now - timedelta(days=context.IN_USE_DAYS))
    return not context.in_use(conn, since, _iso(now), **scope)


def _measure(rule: Rule, conn, now, win, scope: dict):
    start = metrics.history_start(conn)
    needed = now - timedelta(days=rule.min_history_days)
    if start is None:
        raise Abstain("no stored history")
    if datetime.fromisoformat(start) > needed:
        raise Abstain(f"less than {rule.min_history_days} days of stored history")
    since = _iso(now - timedelta(days=rule.window_days))
    return METRICS[rule.metric](conn, since, _iso(now), win, **scope)


def evaluate(
    conn,
    now: datetime | None = None,
    rules: list[Rule] | None = None,
    workspace: str | None = None,
    account: str | None = None,
    checked: bool = False,
) -> dict:
    """Run every rule; `hits` are the fired rules, `abstained` the ones that could not be measured.

    Hits are ordered by priority, then rule id, so the same store gives the same output.
    A rule about a kit part that is not in use is in none of the lists (`left_out`).
    `workspace` (a name) and `account` (id or label) limit every metric to that scope; an unknown
    one raises ValueError, even when the store has too little history to measure anything.
    `checked=True` adds a `checked` list (by rule id): every measured rule with its `title`, `measured`,
    `threshold` and `fired`, so a rule that did not fire still has its value (FR-076). Off by default:
    the output is then exactly what it was before.
    """
    now = now or datetime.now(UTC)
    scope = {"workspace": workspace, "account": account}
    if workspace is not None or account is not None:
        metrics._scope(conn, workspace, None, None, account)
    hits, abstained, measured_rules = [], [], []
    win = _Windows(conn, scope)
    for rule in rules if rules is not None else load_rules(config.config_path()):
        if left_out(rule, conn, now, scope):  # not in use: not a hit, not abstained, not checked (FR-080)
            continue
        try:
            measured, detail = _measure(rule, conn, now, win, scope)
        except Abstain as why:
            abstained.append({"rule_id": rule.id, "title": rule.title, "reason": str(why)})
            continue
        fired = OPS[rule.op](measured, rule.threshold)
        measured_rules.append(
            {"rule_id": rule.id, "title": rule.title, "measured": measured, "threshold": rule.threshold, "fired": fired}
        )
        if fired:
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
            hit.update(_explain(rule, conn, now, measured, detail, win, scope))
            hits.append(hit)
    hits.sort(key=lambda h: (h["priority"], h["rule_id"]))
    abstained.sort(key=lambda a: a["rule_id"])
    result = {"as_of_utc": _iso(now), "hits": hits, "abstained": abstained}
    if checked:
        result["checked"] = sorted(measured_rules, key=lambda r: r["rule_id"])
    return result
