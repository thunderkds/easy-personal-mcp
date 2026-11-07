"""kit_summary (FR-015): what the Easy Kit did, per project, one entry per kit part.

Parts: `kernel` (kernel traces), `git` (local commits per day, read from git at query time) and
`context` (easy-context index freshness, stored at ingest, stale judged at query time) and
`verifier` (newest easy-verifier score JSON per project);
later parts add keys next to them. A part with nothing to read is `{"status": "no data"}`, never zero-filled.
`ui` (easy-ui checks, FR-013) is overall, not per project: a top-level key next to `projects`.
`workspace_rollup` (FR-087, FR-088) groups the verifier scores and the easy-ui calls by workspace.
`report_folders` (FR-089) lists the report folders of each workspace's repos, by name and counts only.
"""

import os
import sqlite3
from concurrent.futures import ThreadPoolExecutor
from datetime import UTC, datetime, timedelta
from pathlib import Path

from easy_personal.core import discovery, metrics, owners, workspaces
from easy_personal.sources.context import IN_USE_DAYS, context_part, in_use
from easy_personal.sources.git_local import git_part
from easy_personal.sources.ui import ui_part
from easy_personal.sources.verifier import verifier_part

NO_DATA = {"status": "no data"}
NOT_IN_USE = {"status": "not in use"}
REPORT_WORD = "report"  # a report folder name contains this (lowercase compare)
REPORT_FOLDERS_PER_REPO = 8
REPORT_FILE_CAP = 2000  # files counted per report folder; past it `files_capped` is true
REPORT_DEPTH = 3  # a file directly in the folder is depth 1
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


UI_PREFIX = "mcp__easy-ui__"  # a call to this server's tools shows easy-ui use (FR-088, FR-060)
NO_UI = {"status": "not used", "sessions": 0, "calls": 0, "last_utc": None}


def _ui_calls(conn: sqlite3.Connection, since: str | None, until: str | None, account: str | None) -> dict[str, tuple]:
    """Stored project path -> (session ids, easy-ui call count, newest call time) over the events with such a call."""
    since_utc, until_utc = metrics._utc_bound(since), metrics._utc_bound(until)
    scope, scope_params = metrics._scope(conn, None, since_utc, until_utc, account)  # raises on an unknown account
    period, period_params = metrics._period("ts_utc", since_utc, until_utc)
    found: dict[str, tuple] = {}
    for path, session, tool_names, ts in conn.execute(
        "SELECT project_path, session_id, tool_names, ts_utc FROM events WHERE abstained = 0 "
        f"AND source = 'claude_code' AND project_path IS NOT NULL AND {period} AND {scope} "
        f"AND (tool_names GLOB '{UI_PREFIX}*' OR tool_names GLOB '*,{UI_PREFIX}*')",
        period_params + scope_params,
    ):
        calls = sum(name.startswith(UI_PREFIX) for name in tool_names.split(","))
        sessions, total, last = found.get(path, (set(), 0, ""))
        sessions.add(session)
        found[path] = (sessions, total + calls, max(last, ts))
    return found


def _verifier_part(roots_by_repo: dict[str, set[str]], covers, scored: list[tuple]) -> dict:
    """Verifier entry of one workspace: each existing repo takes its newest scored row; the others are no score."""
    total = sum(Path(repo).is_dir() for repo in roots_by_repo)
    per_repo = []  # (value, folder name, generated time) of the scored repos
    for repo, roots in roots_by_repo.items():
        if not Path(repo).is_dir():
            continue
        matched = [row for row in scored if covers(row[0], roots)]
        if matched:
            _, value, generated = max(matched, key=lambda row: (row[2], row[0]))
            per_repo.append((value, Path(repo).name or "/", generated))
    if not per_repo:
        return {"status": "no data", "repos_scored": 0, "repos_total": total, "mean": None, "lowest": None,
                "newest_utc": None}  # fmt: skip
    value, name, _ = min(per_repo, key=lambda item: (item[0], item[1]))
    return {
        "status": "ok",
        "repos_scored": len(per_repo),
        "repos_total": total,
        "mean": round(sum(item[0] for item in per_repo) / len(per_repo), 1),
        "lowest": {"project": name, "value": value},
        "newest_utc": max(item[2] for item in per_repo),
    }


def workspace_rollup(
    conn: sqlite3.Connection, since: str | None = None, until: str | None = None, account: str | None = None
) -> dict:
    """The Kit view's by-workspace block (FR-087, FR-088): one entry per non-orphan workspace, `list_all` order.

    `verifier`: from the stored `verifier_scores` rows only; a row with a number is a score, an abstained,
    unknown-schema or missing one is no score and never 0. It ignores `since`, `until` and `account`.
    `ui`: easy-ui use from stored Claude events in [since, until) of the account: sessions with a call,
    the calls, the newest call. A project counts for a repo when its path (or its FR-057 owner project) equals
    or sits under the repo path, as in `workspaces.project_paths`. The easy-ui check results stay overall
    in `ui_checks`. An unknown `account` raises `AccountError`.
    """
    ui_found = _ui_calls(conn, since, until, account)
    scored = conn.execute(
        "SELECT project_path, overall_value, generated_at_utc FROM verifier_scores "
        "WHERE status = 'ok' AND overall_abstained = 0 AND overall_value IS NOT NULL"
    ).fetchall()
    owner = owners.owner_map(conn)

    def covers(path: str, roots: set[str]) -> bool:
        return any(workspaces._under(owner.get(path, path), root) or workspaces._under(path, root) for root in roots)

    entries = []
    for workspace in workspaces.list_all(conn):
        if workspace["is_orphan"]:
            continue
        roots_by_repo = {repo: {repo, discovery.resolve_project(repo)} for repo in workspace["repos"]}
        every_root = set().union(*roots_by_repo.values())
        sessions, calls, last = set(), 0, ""
        for path, (path_sessions, path_calls, path_last) in ui_found.items():
            if covers(path, every_root):
                sessions |= path_sessions
                calls += path_calls
                last = max(last, path_last)
        entries.append(
            {
                "name": workspace["name"],
                "color": workspace["color"],
                "repos": len(workspace["repos"]),
                "verifier": _verifier_part(roots_by_repo, covers, scored),
                "ui": {"status": "ok", "sessions": len(sessions), "calls": calls, "last_utc": last} if calls else dict(NO_UI),
            }
        )
    return {"workspaces": entries, "ui_checks": ui_part(conn)}


def _report_stats(folder: str) -> dict | None:
    """Count the regular files of a report folder to `REPORT_DEPTH` levels, `REPORT_FILE_CAP` files at most.

    Names and `stat` only: no file is opened and no symlink is followed. A sub-folder that cannot be
    listed is skipped; a top folder that cannot be listed gives None.
    """
    files, sessions, newest, capped = 0, 0, None, False
    try:
        top = os.scandir(folder)
    except OSError:
        return None
    pending = [(top, 1)]
    while pending and not capped:
        entries, depth = pending.pop()
        with entries:
            for entry in entries:
                try:
                    if entry.is_file(follow_symlinks=False):
                        if files == REPORT_FILE_CAP:
                            capped = True
                            break
                        mtime = entry.stat(follow_symlinks=False).st_mtime
                        files += 1
                        sessions += entry.name.startswith("session-") and entry.name.endswith(".json")
                        newest = mtime if newest is None else max(newest, mtime)
                    elif depth < REPORT_DEPTH and entry.is_dir(follow_symlinks=False):
                        pending.append((os.scandir(entry.path), depth + 1))
                except OSError:
                    continue
    for entries, _ in pending:
        entries.close()
    return {
        "files": files,
        "files_capped": capped,
        "easy_ui_sessions": sessions,
        "newest_utc": None if newest is None else datetime.fromtimestamp(newest, UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
    }


def _repo_report_folders(repo: str) -> list[dict]:
    try:
        with os.scandir(repo) as entries:
            names = sorted(e.name for e in entries if REPORT_WORD in e.name.lower() and _is_real_dir(e))
    except OSError:
        return []  # a repo folder that is gone or unreadable
    found = []
    for name in names[:REPORT_FOLDERS_PER_REPO]:
        stats = _report_stats(os.path.join(repo, name))
        if stats is not None:
            found.append({"repo": os.path.basename(repo), "folder": name, **stats})
    return found


def _is_real_dir(entry: os.DirEntry) -> bool:
    try:
        return entry.is_dir(follow_symlinks=False)
    except OSError:
        return False


def report_folders(conn: sqlite3.Connection) -> dict:
    """The Kit view's report folders (FR-089): per non-orphan workspace, `list_all` order, for reference only.

    A report folder sits directly inside a workspace repo and has `report` in its lowercase name; at most
    8 per repo, by name. Only the repo folder name, the report folder name, counts and a time are returned:
    never a path, a file name or a sub-folder name. Nothing is opened, written or stored.
    """
    return {
        "workspaces": [
            {"name": workspace["name"], "folders": [f for repo in workspace["repos"] for f in _repo_report_folders(repo)]}
            for workspace in workspaces.list_all(conn)
            if not workspace["is_orphan"]
        ]
    }
