"""kit_summary (FR-015): what the Easy Kit did, per project, one entry per kit part.

Parts: `kernel` (kernel traces), `git` (local commits per day, read from git at query time) and
`context` (easy-context index freshness, stored at ingest, stale judged at query time) and
`verifier` (newest easy-verifier score JSON per project);
later parts add keys next to them. A part with nothing to read is `{"status": "no data"}`, never zero-filled.
`ui` (easy-ui checks, FR-013) is overall, not per project: a top-level key next to `projects`.
"""

import sqlite3
from pathlib import Path

from easy_personal.core import metrics
from easy_personal.sources.context import context_part
from easy_personal.sources.git_local import git_part
from easy_personal.sources.ui import ui_part
from easy_personal.sources.verifier import verifier_part

NO_DATA = {"status": "no data"}


def scoped_projects(
    conn: sqlite3.Connection, workspace: str | None = None, account: str | None = None
) -> list[str]:
    """Stored project paths seen in usage, limited to a workspace's repos and/or an account's events.

    Kit traces carry no account, so the kit parts follow this project list. An unknown workspace
    or account raises ValueError (see `metrics._scope`).
    """
    scope, params = metrics._scope(conn, workspace, None, None, account)
    return [
        p
        for (p,) in conn.execute(
            "SELECT DISTINCT project_path FROM events WHERE project_path IS NOT NULL "
            f"AND source = 'claude_code' AND {scope} ORDER BY project_path",
            params,
        )
    ]


def kernel_error_rate(
    conn: sqlite3.Connection,
    since: str,
    until: str,
    workspace: str | None = None,
    account: str | None = None,
) -> tuple[float, int]:
    """(share of tool calls with is_error, number of calls) for [since, until).

    All projects, or only those of the `workspace` / `account` scope.
    """
    where, params = "abstained = 0 AND ts_utc >= ? AND ts_utc < ?", [since, until]
    if workspace is not None or account is not None:
        paths = scoped_projects(conn, workspace, account)
        if not paths:
            return 0.0, 0
        where += f" AND project_path IN ({', '.join('?' * len(paths))})"
        params += paths
    calls, errors = conn.execute(
        f"SELECT COUNT(*), COALESCE(SUM(is_error), 0) FROM kit_trace_events WHERE {where}", params
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


def kit_summary(
    conn: sqlite3.Connection, workspace: str | None = None, account: str | None = None
) -> dict:
    """Every project seen in usage, by folder name, with its kit parts.

    `workspace` / `account` keep only the projects of that workspace or with events of that account.
    """
    projects = scoped_projects(conn, workspace, account)
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
