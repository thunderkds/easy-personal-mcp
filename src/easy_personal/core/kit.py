"""kit_summary (FR-015): what the Easy Kit did, per project, one entry per kit part.

Parts: `kernel` (kernel traces), `git` (local commits per day, read from git at query time) and
`context` (easy-context index freshness, stored at ingest, stale judged at query time) and
`verifier` (newest easy-verifier score JSON per project);
later parts add keys next to them. A part with nothing to read is `{"status": "no data"}`, never zero-filled.
`ui` (easy-ui checks, FR-013) is overall, not per project: a top-level key next to `projects`.
"""

import sqlite3
from concurrent.futures import ThreadPoolExecutor
from datetime import UTC, datetime, timedelta
from pathlib import Path

from easy_personal.core import metrics, owners
from easy_personal.sources.context import IN_USE_DAYS, context_part, in_use
from easy_personal.sources.git_local import git_part
from easy_personal.sources.ui import ui_part
from easy_personal.sources.verifier import verifier_part

NO_DATA = {"status": "no data"}
NOT_IN_USE = {"status": "not in use"}
GIT_WORKERS = 8  # `git rev-list` runs in parallel: it is a subprocess wait, and a big repo takes 100+ ms


def _stored_projects(
    conn: sqlite3.Connection, workspace: str | None = None, account: str | None = None
) -> list[str]:
    scope, params = metrics._scope(conn, workspace, None, None, account)
    return [
        p
        for (p,) in conn.execute(
            "SELECT DISTINCT project_path FROM events WHERE project_path IS NOT NULL "
            f"AND source = 'claude_code' AND {scope} ORDER BY project_path",
            params,
        )
    ]


def scoped_projects(
    conn: sqlite3.Connection, workspace: str | None = None, account: str | None = None
) -> list[str]:
    """Project paths seen in usage, limited to a workspace's repos and/or an account's events.

    A gone worktree is listed as its owner project, once (FR-057). Kit traces carry no account, so the
    kit parts follow this project list. An unknown workspace or account raises ValueError (see `metrics._scope`).
    """
    paths = _stored_projects(conn, workspace, account)
    owner = owners.owner_map(conn)
    return sorted({owner.get(p, p) for p in paths})


def _trace_window(
    conn: sqlite3.Connection, since: str, until: str, workspace: str | None, account: str | None
) -> tuple[str, list] | None:
    """WHERE clause + params for the trace rows of [since, until) in scope; None = the scope has no project."""
    where, params = "abstained = 0 AND ts_utc >= ? AND ts_utc < ?", [since, until]
    if workspace is not None or account is not None:
        paths = _stored_projects(conn, workspace, account)  # trace rows keep their own stored path
        if not paths:
            return None
        where += f" AND project_path IN ({', '.join('?' * len(paths))})"
        params += paths
    return where, params


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
    window = _trace_window(conn, since, until, workspace, account)
    if window is None:
        return 0.0, 0
    where, params = window
    calls, errors = conn.execute(
        f"SELECT COUNT(*), COALESCE(SUM(is_error), 0) FROM kit_trace_events WHERE {where}", params
    ).fetchone()
    return (errors / calls if calls else 0.0), calls


def kernel_errors_by_tool(
    conn: sqlite3.Connection,
    since: str,
    until: str,
    workspace: str | None = None,
    account: str | None = None,
) -> list[tuple[str | None, str, int, int]]:
    """(tool name, stored project path, calls, failed calls) for [since, until) in scope, for rule causes (FR-073)."""
    window = _trace_window(conn, since, until, workspace, account)
    if window is None:
        return []
    where, params = window
    return conn.execute(
        "SELECT tool_name, project_path, COUNT(*), COALESCE(SUM(is_error), 0) "
        f"FROM kit_trace_events WHERE {where} GROUP BY 1, 2",
        params,
    ).fetchall()


def _kernel_by_project(conn) -> dict[str, dict]:
    """The `kernel` part of every project that has trace rows, from one pass over the table."""
    rows = conn.execute(
        "SELECT project_path, SUM(abstained = 0), COALESCE(SUM(CASE WHEN abstained = 0 THEN is_error END), 0), "
        "COUNT(DISTINCT CASE WHEN abstained = 0 THEN kit_task_id END), "
        "MIN(CASE WHEN abstained = 0 THEN ts_utc END), MAX(CASE WHEN abstained = 0 THEN ts_utc END), "
        "SUM(abstained = 1) FROM kit_trace_events GROUP BY project_path"
    )
    return {
        project: {
            "status": "ok",
            "tool_calls": calls,
            "tool_errors": errors,
            "error_rate": round(errors / calls, 4) if calls else None,
            "kit_tasks": tasks,
            "first_utc": first,
            "last_utc": last,
            "abstained": abstained,
        }
        for project, calls, errors, tasks, first, last, abstained in rows
        if calls or abstained
    }


def split_gone(paths: list[str]) -> tuple[list[str], int]:
    """(the paths whose folder exists, how many do not): a *gone project* is a stored path with no folder (FR-054)."""
    alive = [p for p in paths if Path(p).is_dir()]
    return alive, len(paths) - len(alive)


def kit_summary(
    conn: sqlite3.Connection,
    workspace: str | None = None,
    account: str | None = None,
    hide_gone: bool = False,
    now: datetime | None = None,
) -> dict:
    """Every project seen in usage, by folder name, with its kit parts.

    Every `context` part is `{"status": "not in use"}` when the scope holds no easy-context tool call in the
    7 days ending at `now` (FR-080).

    `workspace` / `account` keep only the projects of that workspace or with events of that account.
    `hide_gone` (the dashboard) drops projects whose folder is missing and adds a top-level `gone` count;
    MCP and CLI leave it off and keep those rows, flagged `missing_on_disk`.
    """
    projects = scoped_projects(conn, workspace, account)
    gone = None
    if hide_gone:
        projects, gone = split_gone(projects)
    now = now or datetime.now(UTC)
    since = (now - timedelta(days=IN_USE_DAYS)).isoformat()
    context_used = in_use(conn, since, now.isoformat(), workspace=workspace, account=account)
    kernel = _kernel_by_project(conn)
    present = [p for p in projects if Path(p).is_dir()]
    with ThreadPoolExecutor(max_workers=GIT_WORKERS) as pool:
        git = dict(zip(present, pool.map(git_part, present), strict=True))
    summary = {
        "projects": [
            {
                "project": Path(p).name or p,
                "missing_on_disk": p not in git,
                "parts": {
                    "kernel": kernel.get(p) or dict(NO_DATA),
                    "git": git.get(p) or dict(NO_DATA),  # git on a missing folder says "no data" too
                    "context": context_part(conn, p) if context_used else dict(NOT_IN_USE),
                    "verifier": verifier_part(conn, p),
                },
            }
            for p in projects
        ],
        "ui": ui_part(conn),
    }
    if gone is not None:
        summary["gone"] = gone
    return summary
