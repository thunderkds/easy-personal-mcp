"""kit_summary (FR-015): what the Easy Kit did, per project, one entry per kit part.

Parts: `kernel` (kernel traces), `git` (local commits per day, read from git at query time) and
`context` (easy-context index freshness, stored at ingest, stale judged at query time) and
`verifier` (newest easy-verifier score JSON per project);
later parts add keys next to them. A part with nothing to read is `{"status": "no data"}`, never zero-filled.
`ui` (easy-ui checks, FR-013) is overall, not per project: a top-level key next to `projects`.
"""

import sqlite3
from pathlib import Path

from easy_personal.sources.context import context_part
from easy_personal.sources.git_local import git_part
from easy_personal.sources.ui import ui_part
from easy_personal.sources.verifier import verifier_part

NO_DATA = {"status": "no data"}


def kernel_error_rate(conn: sqlite3.Connection, since: str, until: str) -> tuple[float, int]:
    """(share of tool calls with is_error, number of calls) for [since, until), all projects."""
    calls, errors = conn.execute(
        "SELECT COUNT(*), COALESCE(SUM(is_error), 0) FROM kit_trace_events "
        "WHERE abstained = 0 AND ts_utc >= ? AND ts_utc < ?",
        (since, until),
    ).fetchone()
    return (errors / calls if calls else 0.0), calls


def _kernel(conn, project: str) -> dict:
    calls, errors, tasks, first, last = conn.execute(
        "SELECT COUNT(*), COALESCE(SUM(is_error), 0), COUNT(DISTINCT kit_task_id), "
        "MIN(ts_utc), MAX(ts_utc) FROM kit_trace_events WHERE project_path = ? AND abstained = 0",
        (project,),
    ).fetchone()
    abstained = conn.execute(
        "SELECT COUNT(*) FROM kit_trace_events WHERE project_path = ? AND abstained = 1", (project,)
    ).fetchone()[0]
    if not calls and not abstained:
        return dict(NO_DATA)
    return {
        "status": "ok",
        "tool_calls": calls,
        "tool_errors": errors,
        "error_rate": round(errors / calls, 4) if calls else None,
        "kit_tasks": tasks,
        "first_utc": first,
        "last_utc": last,
        "abstained": abstained,
    }


def kit_summary(conn: sqlite3.Connection) -> dict:
    """Every project seen in usage, by folder name, with its kit parts."""
    projects = [
        p
        for (p,) in conn.execute(
            "SELECT DISTINCT project_path FROM events WHERE project_path IS NOT NULL "
            "AND source = 'claude_code' ORDER BY project_path"
        )
    ]
    return {
        "projects": [
            {
                "project": Path(p).name or p,
                "missing_on_disk": not Path(p).is_dir(),
                "parts": {
                    "kernel": _kernel(conn, p),
                    "git": git_part(p),
                    "context": context_part(conn, p),
                    "verifier": verifier_part(conn, p),
                },
            }
            for p in projects
        ],
        "ui": ui_part(conn),
    }
