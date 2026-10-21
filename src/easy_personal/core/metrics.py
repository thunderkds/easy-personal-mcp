"""Usage metrics over the store. Every surface (MCP, CLI) calls these, so numbers match."""

import sqlite3
from datetime import UTC, datetime


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


def usage_summary(conn: sqlite3.Connection, since: str | None = None, until: str | None = None):
    """Sessions, tokens and cache-hit ratio for [since, until); no bound = all history.

    Usage is counted once per `message.id` (one API message spans several records), at the
    message's earliest record time. Abstained records never feed a metric; they are counted.
    """
    since_utc, until_utc = _utc_bound(since), _utc_bound(until)

    where, params = _period("ts_utc", since_utc, until_utc)
    sessions = conn.execute(
        "SELECT COUNT(DISTINCT session_id) FROM events "
        f"WHERE abstained = 0 AND session_id IS NOT NULL AND {where}",
        params,
    ).fetchone()[0]

    where, params = _period("ts", since_utc, until_utc)
    messages, inp, out, cache_create, cache_read = conn.execute(
        "SELECT COUNT(*), COALESCE(SUM(i), 0), COALESCE(SUM(o), 0), COALESCE(SUM(cc), 0), "
        "COALESCE(SUM(cr), 0) FROM ("
        "  SELECT MIN(ts_utc) AS ts, MAX(input_tokens) AS i, MAX(output_tokens) AS o,"
        "         MAX(cache_creation_input_tokens) AS cc, MAX(cache_read_input_tokens) AS cr"
        "  FROM events WHERE abstained = 0 AND message_id IS NOT NULL"
        "  AND input_tokens IS NOT NULL GROUP BY message_id"
        f") WHERE {where}",
        params,
    ).fetchone()

    where, params = _period("ts_utc", since_utc, until_utc)
    by_version = dict(
        conn.execute(
            "SELECT COALESCE(version, 'unknown'), COUNT(*) FROM events "
            f"WHERE abstained = 1 AND {where} GROUP BY 1 ORDER BY 1",
            params,
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
