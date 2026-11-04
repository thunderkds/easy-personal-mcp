"""Usage metrics over the store. Every surface (MCP, CLI) calls these, so numbers match."""

import sqlite3
from datetime import UTC, datetime, timedelta
from functools import lru_cache
from itertools import pairwise
from pathlib import Path

from easy_personal.core import accounts, owners, workspaces
from easy_personal.core import prices as price_table

DIMENSIONS = ("day", "model", "project", "tool")
WORK_KINDS = ("typed", "agent", "unknown")
OUTLIER_TOOL_CALLS = 90
OUTLIER_CONTEXT_TOKENS = 200_000
TIMELINE_GAP_MINUTES = 30


def _utc_bound(value: str | None) -> str | None:
    """ISO date or datetime -> stored UTC form. A value without offset is OS local time."""
    if value is None:
        return None
    parsed = datetime.fromisoformat(value)
    if parsed.tzinfo is None:
        parsed = parsed.astimezone()
    return parsed.astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def _period(column: str, since: str | None, until: str | None) -> tuple[str, list[str]]:
    clauses, params = [], []
    if since is not None:
        clauses.append(f"{column} >= ?")
        params.append(since)
    if until is not None:
        clauses.append(f"{column} < ?")
        params.append(until)
    return " AND ".join(clauses) or "1", params


_STORED_TS = "%Y-%m-%dT%H:%M:%S.%fZ"
MESSAGE_PAD = timedelta(days=1)


def _inner_window(since: str | None, until: str | None) -> tuple[str, list[str]]:
    """Index-friendly `ts_utc` bounds for the inner per-message GROUP BY, padded by one day.

    Filtering events before grouping lets SQLite use `events_ts_message`. The pad keeps every event
    of a message that starts inside the period, so `MIN(ts_utc)` stays exact while a message's
    events span less than a day; the exact period test still runs on the grouped `MIN(ts_utc)`.
    """
    return _period("ts_utc", *_padded(since, until))


def _padded(since: str | None, until: str | None) -> tuple[str | None, str | None]:
    padded_since = padded_until = None
    if since is not None:
        padded_since = (datetime.strptime(since, _STORED_TS) - MESSAGE_PAD).strftime(_STORED_TS)
    if until is not None:
        padded_until = (datetime.strptime(until, _STORED_TS) + MESSAGE_PAD).strftime(_STORED_TS)
    return padded_since, padded_until


def _scope(conn, workspace: str | None, since_utc: str | None, until_utc: str | None, account: str | None = None):
    """AND-ed filter clause + params: workspace (FR-035) and agent account (FR-036); `1` when neither is set.

    The workspace's repo paths are resolved to distinct stored project paths first (see
    `workspaces.project_paths`), so the big `events` table is never matched with a pattern. An
    unknown workspace raises ValueError; a workspace without matching projects selects nothing (`0`).
    The account (id or label) becomes `agent_account_id = ?`; an unknown one raises `AccountError`.
    """
    clauses, params = [], []
    if workspace is not None:
        paths = workspaces.project_paths(conn, workspace, *_padded(since_utc, until_utc))
        clauses.append(f"project_path IN ({', '.join('?' * len(paths))})" if paths else "0")
        params += paths
    if account is not None:
        clause, account_params = accounts.resolve(conn, account)
        clauses.append(clause)
        params += account_params
    return " AND ".join(clauses) or "1", params


def usage_summary(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
):
    """Sessions, tokens and cache-hit ratio for [since, until); no bound = all history.

    `workspace` (a name) keeps only events under that workspace's repo paths; `account` (id or
    label) only events of that agent account. Both given: both must match.

    Usage is counted once per `message.id` (one API message spans several records), at the
    message's earliest record time. Abstained records never feed a metric; they are counted.

    `split` divides all tokens (input + output + cache) into typed / agent / unknown work
    (`work_kind`); the three sum to the total. `entrypoints` lists the tokens per stored entrypoint
    value, so a wrong typed-or-agent call for a new value is visible.
    """
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    scope, scope_params = _scope(conn, workspace, since_utc, until_utc, account)

    where, params = _period("ts_utc", since_utc, until_utc)
    sessions = conn.execute(
        "SELECT COUNT(DISTINCT session_id) FROM events "
        f"WHERE abstained = 0 AND session_id IS NOT NULL AND {where} AND {scope}",
        params + scope_params,
    ).fetchone()[0]

    inner, inner_params = _inner_window(since_utc, until_utc)
    where, params = _period("ts", since_utc, until_utc)
    messages = inp = out = cache_create = cache_read = 0
    split = dict.fromkeys(WORK_KINDS, 0)
    entrypoints: dict[str, int] = {}
    for sidechain, entry, n, i, o, cc, cr in conn.execute(
        "SELECT sc, ep, COUNT(*), COALESCE(SUM(i), 0), COALESCE(SUM(o), 0), COALESCE(SUM(cc), 0), "
        "COALESCE(SUM(cr), 0) FROM ("
        "  SELECT MIN(ts_utc) AS ts, MAX(input_tokens) AS i, MAX(output_tokens) AS o,"
        "         MAX(cache_creation_input_tokens) AS cc, MAX(cache_read_input_tokens) AS cr,"
        "         MAX(is_sidechain) AS sc, MAX(entrypoint) AS ep"
        "  FROM events WHERE abstained = 0 AND message_id IS NOT NULL"
        f"  AND input_tokens IS NOT NULL AND {inner} AND {scope} GROUP BY message_id"
        f") WHERE {where} GROUP BY sc, ep",
        inner_params + scope_params + params,
    ):
        messages, inp, out = messages + n, inp + i, out + o
        cache_create, cache_read = cache_create + cc, cache_read + cr
        split[work_kind(sidechain, entry)] += i + o + cc + cr
        if entry is not None:
            entrypoints[entry] = entrypoints.get(entry, 0) + i + o + cc + cr

    where, params = _period("ts_utc", since_utc, until_utc)
    by_version = dict(
        conn.execute(
            "SELECT COALESCE(version, 'unknown'), COUNT(*) FROM events "
            f"WHERE abstained = 1 AND {where} AND {scope} GROUP BY 1 ORDER BY 1",
            params + scope_params,
        ).fetchall()
    )

    prompt_tokens = inp + cache_create + cache_read
    return {
        "period": {"since_utc": since_utc, "until_utc": until_utc},
        "sessions": sessions,
        "messages": messages,
        "input_tokens": inp,
        "output_tokens": out,
        "cache_creation_input_tokens": cache_create,
        "cache_read_input_tokens": cache_read,
        "cache_hit_ratio": round(cache_read / prompt_tokens, 4) if prompt_tokens else None,
        "split": split,
        "entrypoints": dict(sorted(entrypoints.items())),
        "abstained": {"total": sum(by_version.values()), "by_version": by_version},
    }


def work_kind(is_sidechain, entrypoint) -> str:
    """`typed` / `agent` / `unknown` work of one message (FR-039).

    A sidechain record is agent work. Otherwise the session entrypoint decides: a missing one is
    unknown (abstained, counted, never guessed), one starting with `sdk` (`sdk-cli`, `sdk-py`, ...)
    is agent work, and any other value (`cli`, `claude-desktop`, `ide`, ...) is typed.
    """
    if is_sidechain == 1:
        return "agent"
    if entrypoint is None:
        return "unknown"
    return "agent" if entrypoint.startswith("sdk") else "typed"


def _parse_utc(value: str) -> datetime:
    """A stored UTC time as an aware datetime (`fromisoformat` is ~20x faster than `strptime`; it ran 200k times)."""
    return datetime.fromisoformat(value)


@lru_cache(maxsize=4096)
def _folder(project_path: str | None) -> str:
    return Path(project_path).name or project_path if project_path else "(unknown)"


def _messages(
    conn: sqlite3.Connection,
    since_utc: str | None,
    until_utc: str | None,
    workspace: str | None = None,
    account: str | None = None,
):
    """One row per API message in the period, same selection as `usage_summary`."""
    inner, inner_params = _inner_window(since_utc, until_utc)
    scope, scope_params = _scope(conn, workspace, since_utc, until_utc, account)
    where, params = _period("ts", since_utc, until_utc)
    return conn.execute(
        "SELECT ts, model, project_path, tools, i, o, cc, cr, sc, ep FROM ("
        "  SELECT MIN(ts_utc) AS ts, MAX(model) AS model, MAX(project_path) AS project_path,"
        "         GROUP_CONCAT(tool_names) AS tools, MAX(input_tokens) AS i,"
        "         MAX(output_tokens) AS o, MAX(cache_creation_input_tokens) AS cc,"
        "         MAX(cache_read_input_tokens) AS cr, MAX(is_sidechain) AS sc,"
        "         MAX(entrypoint) AS ep"
        "  FROM events WHERE abstained = 0 AND message_id IS NOT NULL"
        f"  AND input_tokens IS NOT NULL AND {inner} AND {scope} GROUP BY message_id"
        f") WHERE {where} ORDER BY ts",
        inner_params + scope_params + params,
    ).fetchall()


def _keys(by: str, ts: str, model, project_path, tools) -> list[str]:
    if by == "day":  # OS local timezone, DST-aware
        return [_parse_utc(ts).astimezone().strftime("%Y-%m-%d")]
    if by == "model":
        return [model or "(unknown)"]
    if by == "project":
        return [_folder(project_path)]
    return sorted(set(tools.split(","))) if tools else ["(none)"]


def _new_group() -> dict:
    return {
        "messages": 0,
        "input_tokens": 0,
        "output_tokens": 0,
        "cache_creation_input_tokens": 0,
        "cache_read_input_tokens": 0,
        "split": dict.fromkeys(WORK_KINDS, 0),
        "_usd": 0.0,
        "_priced": 0,
        "_abstained": set(),
    }


def _add(group: dict, model, i, o, cc, cr, usd, kind) -> None:
    group["messages"] += 1
    group["split"][kind] += i + o + cc + cr
    group["input_tokens"] += i
    group["output_tokens"] += o
    group["cache_creation_input_tokens"] += cc
    group["cache_read_input_tokens"] += cr
    if usd is None:
        group["_abstained"].add(model or "(unknown)")
    else:
        group["_usd"] += usd
        group["_priced"] += 1


def _finish(group: dict, as_of: str) -> dict:
    usd, priced = group.pop("_usd"), group.pop("_priced")
    abstained = sorted(group.pop("_abstained"))
    group["api_equivalent_cost"] = {
        "usd": round(usd, 6) if priced or not abstained else None,
        "price_table_date": as_of,
        "abstained_models": abstained,
    }
    return group


def _by_tokens(rows: list[dict]) -> list[dict]:
    """Most tokens first, ties by key."""
    return sorted(
        rows,
        key=lambda r: (
            -(r["input_tokens"] + r["output_tokens"] + r["cache_creation_input_tokens"] + r["cache_read_input_tokens"]),
            r["key"],
        ),
    )


def usage_breakdown(
    conn: sqlite3.Connection,
    by: str,
    since: str | None = None,
    until: str | None = None,
    prices: price_table.PriceTable | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> dict:
    """Tokens per day (OS local) / model / project (folder name of the owner project, FR-057) / tool for [since, until).

    Scratch folders (FR-069) are one `project` group, key `Scratch`, `scratch: true`, with `folders`: the
    same fields per scratch folder name.

    `total` counts each message once, so it equals `usage_summary`. Tool groups overlap: a
    message that used several tools appears in each. Cost is API-equivalent, never a bill; a
    model missing from the price table abstains. Each group's `split` divides its tokens into
    typed / agent / unknown work, as in `usage_summary`.
    """
    if by not in DIMENSIONS:
        raise ValueError(f"by must be one of {', '.join(DIMENSIONS)}")
    prices = prices or price_table.load()
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    groups: dict[str, dict] = {}
    total = _new_group()
    pmap = owners.project_map(conn) if by == "project" else {}  # owner project (FR-057) or Scratch (FR-069)
    folders: dict[str, dict] = {}
    for ts, model, project_path, tools, i, o, cc, cr, sc, ep in _messages(
        conn, since_utc, until_utc, workspace, account
    ):
        usd, kind = prices.cost(model, i, o, cc, cr), work_kind(sc, ep)
        _add(total, model, i, o, cc, cr, usd, kind)
        if pmap.get(project_path) == owners.SCRATCH:
            _add(folders.setdefault(_folder(project_path), _new_group()), model, i, o, cc, cr, usd, kind)
            project_path = owners.SCRATCH
        else:
            project_path = pmap.get(project_path, project_path)
        for key in _keys(by, ts, model, project_path, tools):
            _add(groups.setdefault(key, _new_group()), model, i, o, cc, cr, usd, kind)
    rows = [{"key": key, **_finish(group, prices.as_of)} for key, group in groups.items()]
    if folders:
        scratch = next(r for r in rows if r["key"] == owners.SCRATCH)
        scratch["scratch"] = True
        scratch["folders"] = _by_tokens([{"key": k, **_finish(g, prices.as_of)} for k, g in folders.items()])
    rows = sorted(rows, key=lambda r: r["key"]) if by == "day" else _by_tokens(rows)
    return {
        "by": by,
        "period": {"since_utc": since_utc, "until_utc": until_utc},
        "groups": rows,
        "total": _finish(total, prices.as_of),
    }


def _message_sums(conn, since, until, workspace, account, by_project: bool = False):
    """Token sums over the messages in [since, until): `(input, output, cache creation, cache read)`, or per stored project path."""
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    inner, inner_params = _inner_window(since_utc, until_utc)
    scope, scope_params = _scope(conn, workspace, since_utc, until_utc, account)
    where, params = _period("ts", since_utc, until_utc)
    sums = "COALESCE(SUM(i), 0), COALESCE(SUM(o), 0), COALESCE(SUM(cc), 0), COALESCE(SUM(cr), 0)"
    rows = conn.execute(
        f"SELECT {'project_path, ' if by_project else ''}{sums} FROM ("
        "  SELECT MIN(ts_utc) AS ts, MAX(input_tokens) AS i, MAX(output_tokens) AS o,"
        "         MAX(cache_creation_input_tokens) AS cc, MAX(cache_read_input_tokens) AS cr,"
        "         MAX(project_path) AS project_path"
        "  FROM events WHERE abstained = 0 AND message_id IS NOT NULL"
        f"  AND input_tokens IS NOT NULL AND {inner} AND {scope} GROUP BY message_id"
        f") WHERE {where}{' GROUP BY project_path' if by_project else ''}",
        inner_params + scope_params + params,
    ).fetchall()
    return rows if by_project else rows[0]


def token_total(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> int:
    """All tokens (input + output + cache) of the messages in [since, until); the `total` of `usage_breakdown`, in one query."""
    return sum(_message_sums(conn, since, until, workspace, account))


def cache_hit_ratio(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> float | None:
    """Cache-read share of prompt tokens in [since, until), as `usage_summary` reports it; None without prompt tokens."""
    inp, _, cache_create, cache_read = _message_sums(conn, since, until, workspace, account)
    prompt_tokens = inp + cache_create + cache_read
    return round(cache_read / prompt_tokens, 4) if prompt_tokens else None


def tokens_by_project(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> dict[str, int]:
    """All tokens per project folder name in [since, until); the `project` groups of `usage_breakdown`, in one query."""
    totals: dict[str, int] = {}
    pmap = owners.project_map(conn)
    for path, *sums in _message_sums(conn, since, until, workspace, account, by_project=True):
        key = _folder(pmap.get(path, path))
        totals[key] = totals.get(key, 0) + sum(sums)
    return totals


def session_tokens(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> list[dict]:
    """Token sums of the messages in [since, until) per (session, stored project path), for rule causes (FR-073).

    Same message selection as `token_total`, so the sums add up to it (messages with no session id come back with
    `session_id` None). `models` = distinct models the session used in the period.
    """
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    inner, inner_params = _inner_window(since_utc, until_utc)
    scope, scope_params = _scope(conn, workspace, since_utc, until_utc, account)
    where, params = _period("ts", since_utc, until_utc)
    rows = conn.execute(
        "SELECT sid, project_path, SUM(i), SUM(o), SUM(cc), SUM(cr), COUNT(DISTINCT model) FROM ("
        "  SELECT MIN(ts_utc) AS ts, MAX(session_id) AS sid, MAX(project_path) AS project_path,"
        "         MAX(model) AS model, MAX(input_tokens) AS i, MAX(output_tokens) AS o,"
        "         MAX(cache_creation_input_tokens) AS cc, MAX(cache_read_input_tokens) AS cr"
        "  FROM events WHERE abstained = 0 AND message_id IS NOT NULL"
        f"  AND input_tokens IS NOT NULL AND {inner} AND {scope} GROUP BY message_id"
        f") WHERE {where} GROUP BY sid, project_path",
        inner_params + scope_params + params,
    ).fetchall()
    return [
        {
            "session_id": sid,
            "project_path": path,
            "input": i or 0,
            "output": o or 0,
            "cache_creation": cc or 0,
            "cache_read": cr or 0,
            "models": models,
        }
        for sid, path, i, o, cc, cr, models in rows
    ]


def session_outliers(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> dict:
    """Sessions with >= OUTLIER_TOOL_CALLS tool calls or a context >= OUTLIER_CONTEXT_TOKENS.

    Context = input + cache creation + cache read tokens of one message (its prompt size).
    """
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    where, params = _period("ts_utc", since_utc, until_utc)
    scope, scope_params = _scope(conn, workspace, since_utc, until_utc, account)
    where, params = f"{where} AND {scope}", params + scope_params
    calls: dict[str, tuple[int, str | None]] = {}
    for session, tools, project_path in conn.execute(
        "SELECT session_id, tool_names, project_path FROM events "
        f"WHERE abstained = 0 AND session_id IS NOT NULL AND tool_names IS NOT NULL AND {where}",
        params,
    ):
        count, project = calls.get(session, (0, None))
        calls[session] = (count + len(tools.split(",")), project or project_path)
    contexts = dict(
        conn.execute(
            "SELECT session_id, MAX(COALESCE(input_tokens, 0) + "
            "COALESCE(cache_creation_input_tokens, 0) + COALESCE(cache_read_input_tokens, 0)) "
            "FROM events WHERE abstained = 0 AND session_id IS NOT NULL "
            f"AND message_id IS NOT NULL AND {where} GROUP BY session_id",
            params,
        )
    )
    projects = dict(
        conn.execute(
            "SELECT session_id, MAX(project_path) FROM events "
            f"WHERE abstained = 0 AND session_id IS NOT NULL AND {where} GROUP BY session_id",
            params,
        )
    )
    sessions = []
    for session in sorted(set(calls) | set(contexts)):
        tool_calls, context = calls.get(session, (0, None))[0], contexts.get(session, 0)
        reasons = []
        if tool_calls >= OUTLIER_TOOL_CALLS:
            reasons.append("tool_calls")
        if context >= OUTLIER_CONTEXT_TOKENS:
            reasons.append("context_tokens")
        if reasons:
            sessions.append(
                {
                    "session_id": session,
                    "project": _folder(projects.get(session)),
                    "tool_calls": tool_calls,
                    "max_context_tokens": context,
                    "reasons": reasons,
                }
            )
    sessions.sort(key=lambda s: (-s["tool_calls"], s["session_id"]))
    return {
        "period": {"since_utc": since_utc, "until_utc": until_utc},
        "thresholds": {"tool_calls": OUTLIER_TOOL_CALLS, "context_tokens": OUTLIER_CONTEXT_TOKENS},
        "sessions": sessions,
    }


def session_timeline(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
    detail_sessions: int | None = None,
) -> dict:
    """Per session in [since, until): start/end, models, token points per message and idle gaps.

    Start/end/gaps come from every non-abstained event of the session; points and totals count
    each message once (same rule as `usage_summary`). Tokens and times only, no text.

    `detail_sessions` (the dashboard sends 50): only the newest N sessions carry `points` and
    `gaps`; the others keep every total but have both lists empty, and the answer says so in a
    top-level `detail_sessions`. The page lists every session ("Show N more") and draws the chart of
    the newest `detail_sessions` only; the rest show their totals without a chart. None = every
    session in full.
    """
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    scope, scope_params = _scope(conn, workspace, since_utc, until_utc, account)
    where, params = _period("ts_utc", since_utc, until_utc)
    where, params = f"{where} AND {scope}", params + scope_params
    sessions: dict[str, dict] = {}
    times: dict[str, list[str]] = {}
    for session, ts, project_path in conn.execute(
        "SELECT session_id, ts_utc, project_path FROM events WHERE abstained = 0 "
        f"AND session_id IS NOT NULL AND ts_utc IS NOT NULL AND {where} ORDER BY ts_utc",
        params,
    ):
        times.setdefault(session, []).append(ts)
        state = sessions.setdefault(session, {"path": None})
        state["path"] = state["path"] or project_path

    points: dict[str, list[dict]] = {}
    inner, inner_params = _inner_window(since_utc, until_utc)
    where, params = _period("ts", since_utc, until_utc)
    for session, ts, model, i, o, cc, cr in conn.execute(
        "SELECT session_id, ts, model, i, o, cc, cr FROM ("
        "  SELECT MAX(session_id) AS session_id, MIN(ts_utc) AS ts, MAX(model) AS model,"
        "         message_id, MAX(input_tokens) AS i,"
        "         COALESCE(MAX(output_tokens), 0) AS o,"
        "         COALESCE(MAX(cache_creation_input_tokens), 0) AS cc,"
        "         COALESCE(MAX(cache_read_input_tokens), 0) AS cr"
        "  FROM events WHERE abstained = 0 AND message_id IS NOT NULL"
        f"  AND input_tokens IS NOT NULL AND session_id IS NOT NULL AND {inner} AND {scope}"
        "  GROUP BY message_id"
        f") WHERE ts IS NOT NULL AND {where} ORDER BY ts, message_id",
        inner_params + scope_params + params,
    ):
        points.setdefault(session, []).append(
            {
                "ts_utc": ts,
                "model": model,
                "input_tokens": i,
                "output_tokens": o,
                "cache_creation_input_tokens": cc,
                "cache_read_input_tokens": cr,
            }
        )

    order = sorted(sessions)
    order.sort(key=lambda session: times[session][0], reverse=True)  # newest first, ties by session id
    result = []
    for rank, session in enumerate(order):
        stamps, pts = times[session], points.get(session, [])
        detailed = detail_sessions is None or rank < detail_sessions
        gaps = []
        for before, after in pairwise(stamps) if detailed else ():
            minutes = (_parse_utc(after) - _parse_utc(before)).total_seconds() / 60
            if minutes >= TIMELINE_GAP_MINUTES:
                gaps.append({"start_utc": before, "end_utc": after, "minutes": round(minutes, 1)})
        result.append(
            {
                "session_id": session,
                "project": _folder(sessions[session]["path"]),
                "start_utc": stamps[0],
                "end_utc": stamps[-1],
                "models": sorted({p["model"] for p in pts if p["model"]}),
                "messages": len(pts),
                "input_tokens": sum(p["input_tokens"] for p in pts),
                "output_tokens": sum(p["output_tokens"] for p in pts),
                "cache_creation_input_tokens": sum(p["cache_creation_input_tokens"] for p in pts),
                "cache_read_input_tokens": sum(p["cache_read_input_tokens"] for p in pts),
                "points": pts if detailed else [],
                "gaps": gaps,
            }
        )
    answer = {
        "period": {"since_utc": since_utc, "until_utc": until_utc},
        "gap_minutes": TIMELINE_GAP_MINUTES,
        "sessions": result,
    }
    if detail_sessions is not None:
        answer["detail_sessions"] = detail_sessions
    return answer


def history_start(conn: sqlite3.Connection) -> str | None:
    """UTC time of the earliest stored event; None for an empty store."""
    return conn.execute("SELECT MIN(ts_utc) FROM events").fetchone()[0]


def session_contexts(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> list[dict]:
    """Per session: peak context (prompt tokens of one message) and whether it was compacted.

    Compaction is inferred from token counts only: a later message whose context is below half
    of the session's earlier peak. No record text is read.
    """
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    sessions: dict[str, dict] = {}
    inner, inner_params = _inner_window(since_utc, until_utc)
    scope, scope_params = _scope(conn, workspace, since_utc, until_utc, account)
    rows = conn.execute(
        "SELECT session_id, ts, ctx FROM ("
        "  SELECT MAX(session_id) AS session_id, MIN(ts_utc) AS ts,"
        "         MAX(COALESCE(input_tokens, 0) + COALESCE(cache_creation_input_tokens, 0)"
        "             + COALESCE(cache_read_input_tokens, 0)) AS ctx"
        "  FROM events WHERE abstained = 0 AND session_id IS NOT NULL"
        f"  AND message_id IS NOT NULL AND input_tokens IS NOT NULL AND {inner} AND {scope}"
        "  GROUP BY message_id"
        f") WHERE {_period('ts', since_utc, until_utc)[0]} ORDER BY session_id, ts",
        inner_params + scope_params + _period("ts", since_utc, until_utc)[1],
    )
    for session, _, context in rows:
        state = sessions.setdefault(session, {"session_id": session, "peak": 0, "compacted": False})
        if context * 2 < state["peak"]:
            state["compacted"] = True
        state["peak"] = max(state["peak"], context)
    return [
        {
            "session_id": s["session_id"],
            "max_context_tokens": s["peak"],
            "compacted": s["compacted"],
        }
        for s in sessions.values()
    ]


def record_readability(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> dict[str, tuple[int, int]]:
    """Source version -> (abstained records, all records) with a timestamp in [since, until)."""
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    where, params = _period("ts_utc", since_utc, until_utc)
    scope, scope_params = _scope(conn, workspace, since_utc, until_utc, account)
    return {
        version: (abstained, total)
        for version, abstained, total in conn.execute(
            "SELECT COALESCE(version, 'unknown'), SUM(abstained), COUNT(*) FROM events "
            f"WHERE {where} AND {scope} GROUP BY 1 ORDER BY 1",
            params + scope_params,
        )
    }


def unreadable_records(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> list[tuple[str | None, str, int]]:
    """(record type, source version, abstained records) with a timestamp in [since, until), most records first."""
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    where, params = _period("ts_utc", since_utc, until_utc)
    scope, scope_params = _scope(conn, workspace, since_utc, until_utc, account)
    return conn.execute(
        "SELECT record_type, COALESCE(version, 'unknown'), COUNT(*) FROM events "
        f"WHERE abstained = 1 AND {where} AND {scope} GROUP BY 1, 2 ORDER BY 3 DESC, 1, 2",
        params + scope_params,
    ).fetchall()
