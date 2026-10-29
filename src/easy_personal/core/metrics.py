"""Usage metrics over the store. Every surface (MCP, CLI) calls these, so numbers match."""

import sqlite3
from datetime import UTC, datetime, timedelta
from itertools import pairwise
from pathlib import Path

from easy_personal.core import accounts, workspaces
from easy_personal.core import prices as price_table

DIMENSIONS = ("day", "model", "project", "tool")
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
    messages, inp, out, cache_create, cache_read = conn.execute(
        "SELECT COUNT(*), COALESCE(SUM(i), 0), COALESCE(SUM(o), 0), COALESCE(SUM(cc), 0), "
        "COALESCE(SUM(cr), 0) FROM ("
        "  SELECT MIN(ts_utc) AS ts, MAX(input_tokens) AS i, MAX(output_tokens) AS o,"
        "         MAX(cache_creation_input_tokens) AS cc, MAX(cache_read_input_tokens) AS cr"
        "  FROM events WHERE abstained = 0 AND message_id IS NOT NULL"
        f"  AND input_tokens IS NOT NULL AND {inner} AND {scope} GROUP BY message_id"
        f") WHERE {where}",
        inner_params + scope_params + params,
    ).fetchone()

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
        "abstained": {"total": sum(by_version.values()), "by_version": by_version},
    }


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
        "SELECT ts, model, project_path, tools, i, o, cc, cr FROM ("
        "  SELECT MIN(ts_utc) AS ts, MAX(model) AS model, MAX(project_path) AS project_path,"
        "         GROUP_CONCAT(tool_names) AS tools, MAX(input_tokens) AS i,"
        "         MAX(output_tokens) AS o, MAX(cache_creation_input_tokens) AS cc,"
        "         MAX(cache_read_input_tokens) AS cr"
        "  FROM events WHERE abstained = 0 AND message_id IS NOT NULL"
        f"  AND input_tokens IS NOT NULL AND {inner} AND {scope} GROUP BY message_id"
        f") WHERE {where} ORDER BY ts",
        inner_params + scope_params + params,
    ).fetchall()


def _keys(by: str, ts: str, model, project_path, tools) -> list[str]:
    if by == "day":  # OS local timezone, DST-aware
        utc = datetime.strptime(ts, "%Y-%m-%dT%H:%M:%S.%fZ").replace(tzinfo=UTC)
        return [utc.astimezone().strftime("%Y-%m-%d")]
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
        "_usd": 0.0,
        "_priced": 0,
        "_abstained": set(),
    }


def _add(group: dict, model, i, o, cc, cr, usd) -> None:
    group["messages"] += 1
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


def usage_breakdown(
    conn: sqlite3.Connection,
    by: str,
    since: str | None = None,
    until: str | None = None,
    prices: price_table.PriceTable | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> dict:
    """Tokens per day (OS local) / model / project (folder name) / tool for [since, until).

    `total` counts each message once, so it equals `usage_summary`. Tool groups overlap: a
    message that used several tools appears in each. Cost is API-equivalent, never a bill; a
    model missing from the price table abstains.
    """
    if by not in DIMENSIONS:
        raise ValueError(f"by must be one of {', '.join(DIMENSIONS)}")
    prices = prices or price_table.load()
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)
    groups: dict[str, dict] = {}
    total = _new_group()
    for ts, model, project_path, tools, i, o, cc, cr in _messages(conn, since_utc, until_utc, workspace, account):
        usd = prices.cost(model, i, o, cc, cr)
        _add(total, model, i, o, cc, cr, usd)
        for key in _keys(by, ts, model, project_path, tools):
            _add(groups.setdefault(key, _new_group()), model, i, o, cc, cr, usd)
    rows = [{"key": key, **_finish(group, prices.as_of)} for key, group in groups.items()]
    if by == "day":
        rows.sort(key=lambda r: r["key"])
    else:
        rows.sort(
            key=lambda r: (
                -(
                    r["input_tokens"]
                    + r["output_tokens"]
                    + r["cache_creation_input_tokens"]
                    + r["cache_read_input_tokens"]
                ),
                r["key"],
            )
        )
    return {
        "by": by,
        "period": {"since_utc": since_utc, "until_utc": until_utc},
        "groups": rows,
        "total": _finish(total, prices.as_of),
    }


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


def _parse_utc(value: str) -> datetime:
    return datetime.strptime(value, "%Y-%m-%dT%H:%M:%S.%fZ").replace(tzinfo=UTC)


def session_timeline(
    conn: sqlite3.Connection,
    since: str | None = None,
    until: str | None = None,
    workspace: str | None = None,
    account: str | None = None,
) -> dict:
    """Per session in [since, until): start/end, models, token points per message and idle gaps.

    Start/end/gaps come from every non-abstained event of the session; points and totals count
    each message once (same rule as `usage_summary`). Tokens and times only, no text.
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

    result = []
    for session, state in sessions.items():
        stamps, pts = times[session], points.get(session, [])
        gaps = []
        for before, after in pairwise(stamps):
            minutes = (_parse_utc(after) - _parse_utc(before)).total_seconds() / 60
            if minutes >= TIMELINE_GAP_MINUTES:
                gaps.append({"start_utc": before, "end_utc": after, "minutes": round(minutes, 1)})
        result.append(
            {
                "session_id": session,
                "project": _folder(state["path"]),
                "start_utc": stamps[0],
                "end_utc": stamps[-1],
                "models": sorted({p["model"] for p in pts if p["model"]}),
                "messages": len(pts),
                "input_tokens": sum(p["input_tokens"] for p in pts),
                "output_tokens": sum(p["output_tokens"] for p in pts),
                "cache_creation_input_tokens": sum(p["cache_creation_input_tokens"] for p in pts),
                "cache_read_input_tokens": sum(p["cache_read_input_tokens"] for p in pts),
                "points": pts,
                "gaps": gaps,
            }
        )
    result.sort(key=lambda s: s["session_id"])
    result.sort(key=lambda s: s["start_utc"], reverse=True)
    return {
        "period": {"since_utc": since_utc, "until_utc": until_utc},
        "gap_minutes": TIMELINE_GAP_MINUTES,
        "sessions": result,
    }


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
