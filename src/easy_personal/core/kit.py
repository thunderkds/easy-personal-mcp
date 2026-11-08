"""kit_summary (FR-015): what the Easy Kit did, per project, one entry per kit part.

Parts: `kernel` (kernel traces), `git` (local commits per day, read from git at query time) and
`context` (easy-context index freshness, stored at ingest, stale judged at query time) and
`verifier` (newest easy-verifier score JSON per project);
later parts add keys next to them. A part with nothing to read is `{"status": "no data"}`, never zero-filled.
`ui` (easy-ui checks, FR-013) is overall, not per project: a top-level key next to `projects`.
`workspace_rollup` (FR-087, FR-088) groups the verifier scores and the easy-ui calls by workspace.
`report_folders` (FR-089) lists the report folders of each workspace's repos, by name and counts only.
`report_files` and `report_file` (FR-090) list and read the files of one report folder, read-only.
"""

import os
import sqlite3
import stat
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
REPORT_LIST_CAP = 200  # files in one `report_files` answer
REPORT_READ_CAP = 10 * 1024 * 1024  # a file over this is listed, not openable, and not served
REPORT_PATH_MAX, REPORT_PATH_PARTS = 512, 8
REPORT_TYPES = {  # the content type comes from the extension, never from the content
    "html": "text/html; charset=utf-8", "htm": "text/html; charset=utf-8", "md": "text/markdown; charset=utf-8",
    "txt": "text/plain; charset=utf-8", "json": "application/json", "png": "image/png", "jpg": "image/jpeg",
    "jpeg": "image/jpeg", "pdf": "application/pdf",
}  # fmt: skip
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


def _file_utc(mtime: float) -> str | None:
    """A file time as `%Y-%m-%dT%H:%M:%SZ`, or None when `datetime` cannot represent it.

    A report folder is written by other tools, so a file can carry a time far outside the range
    `datetime` holds: an archive extracted with absurd timestamps, a corrupt inode, a plain
    `touch -d`. Formatting such a value raises `OSError`, `OverflowError` or `ValueError`, and one
    odd file must never break a whole folder's answer (FR-089, FR-090).
    """
    try:
        return datetime.fromtimestamp(mtime, UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    except (OSError, OverflowError, ValueError):
        return None


def _report_stats(folder: str) -> dict | None:
    """Count the regular files of a report folder to `REPORT_DEPTH` levels, `REPORT_FILE_CAP` files at most.

    Names and `stat` only: no file is opened and no symlink is followed. A sub-folder that cannot be
    listed is skipped; a top folder that cannot be listed gives None.
    """
    files, sessions, newest, newest_utc, capped = 0, 0, None, None, False
    try:
        os.scandir(folder).close()
    except OSError:
        return None
    pending = [(folder, 1)]  # paths, not open handles: a folder with thousands of sub-folders holds one handle
    while pending and not capped:
        path, depth = pending.pop()
        try:
            entries = os.scandir(path)
        except OSError:
            continue
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
                        stamp = _file_utc(mtime)
                        if stamp is not None and (newest is None or mtime > newest):
                            newest, newest_utc = mtime, stamp  # an unreadable time is counted, never the newest
                    elif depth < REPORT_DEPTH and entry.is_dir(follow_symlinks=False):
                        pending.append((entry.path, depth + 1))
                except OSError:
                    continue
    return {
        "files": files,
        "files_capped": capped,
        "easy_ui_sessions": sessions,
        "newest_utc": newest_utc,
    }


def _report_names(repo: str) -> list[str]:
    """The report folder names of a repo (FR-089 rule): depth 1, `report` in the lowercase name, a real directory; sorted, 8 at most."""
    try:
        with os.scandir(repo) as entries:
            names = sorted(e.name for e in entries if REPORT_WORD in e.name.lower() and _is_real_dir(e))
    except OSError:
        return []  # a repo folder that is gone or unreadable
    return names[:REPORT_FOLDERS_PER_REPO]


def _repo_report_folders(repo: str, repo_no: int) -> list[dict]:
    found = []
    for name in _report_names(repo):
        stats = _report_stats(os.path.join(repo, name))
        if stats is not None:
            found.append({"repo": os.path.basename(repo), "repo_no": repo_no, "folder": name, **stats})
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
    `repo_no` is the place of the repo in the workspace's repo list. Never a path, a file name or a sub-folder name. Nothing is opened, written or stored.
    """
    return {
        "workspaces": [
            {"name": workspace["name"], "folders": [f for no, repo in enumerate(workspace["repos"]) for f in _repo_report_folders(repo, no)]}
            for workspace in workspaces.list_all(conn)
            if not workspace["is_orphan"]
        ]
    }


class ReportNotFound(Exception):
    """Any miss of `report_files` / `report_file`. The text is fixed: it never echoes the input."""

    def __init__(self) -> None:
        super().__init__("not found")


class ReportTooLarge(Exception):
    def __init__(self) -> None:
        super().__init__("too large")


def _report_dir(conn: sqlite3.Connection, workspace, repo_no, folder) -> str:
    """The path of a report folder found by the server: non-orphan workspace, repo number, FR-089 folder name."""
    if not isinstance(workspace, str) or not isinstance(folder, str) or type(repo_no) is not int or repo_no < 0:
        raise ReportNotFound
    for entry in workspaces.list_all(conn):
        if entry["name"] == workspace and not entry["is_orphan"]:
            repos = entry["repos"]
            if repo_no < len(repos) and folder in _report_names(repos[repo_no]):
                return os.path.join(repos[repo_no], folder)
    raise ReportNotFound


def _extension(name: str) -> str:
    return os.path.splitext(name)[1][1:].lower()


def report_files(conn: sqlite3.Connection, workspace, repo_no, folder) -> dict:
    """The regular files of one report folder (FR-090), to `REPORT_DEPTH` levels, newest first, 200 at most.

    `path` is relative to the report folder, `/` separated. No symlink is followed; nothing is opened.
    """
    top = _report_dir(conn, workspace, repo_no, folder)
    found, pending, capped = [], [(top, "", 1)], False
    while pending and not capped:
        directory, prefix, depth = pending.pop()
        try:
            entries = os.scandir(directory)
        except OSError:
            if depth == 1:
                raise ReportNotFound from None
            continue
        with entries:
            for entry in entries:
                try:
                    if entry.is_file(follow_symlinks=False):
                        if len(found) == REPORT_FILE_CAP:
                            capped = True
                            break
                        info = entry.stat(follow_symlinks=False)
                        found.append((info.st_mtime, prefix + entry.name, info.st_size))
                    elif depth < REPORT_DEPTH and entry.is_dir(follow_symlinks=False):
                        pending.append((entry.path, f"{prefix}{entry.name}/", depth + 1))
                except OSError:
                    continue
    found.sort(key=lambda f: (-f[0], f[1]))
    return {
        "files": [
            {
                "path": path, "size": size, "openable": _extension(path) in REPORT_TYPES and size <= REPORT_READ_CAP,
                "mtime_utc": _file_utc(mtime),
            }
            for mtime, path, size in found[:REPORT_LIST_CAP]
        ],
        "truncated": capped or len(found) > REPORT_LIST_CAP,
    }  # fmt: skip


def _check_report_path(path) -> list[str]:
    if not isinstance(path, str) or not path or len(path) > REPORT_PATH_MAX or path[0] == "/" or "\\" in path or "\0" in path:
        raise ReportNotFound
    parts = path.split("/")
    if len(parts) > REPORT_PATH_PARTS or len(parts) > REPORT_DEPTH or any(p in ("", ".", "..") for p in parts):
        raise ReportNotFound
    if _extension(parts[-1]) not in REPORT_TYPES:
        raise ReportNotFound
    return parts


def report_file(conn: sqlite3.Connection, workspace, repo_no, folder, path) -> tuple[str, bytes]:
    """`(content type, bytes)` of one file of a report folder (FR-090).

    Each level is opened relative to its parent with `O_NOFOLLOW`: a symlink at any level, a file that is not
    regular, an extension outside `REPORT_TYPES` or a depth over `REPORT_DEPTH` is `ReportNotFound`; a file
    over `REPORT_READ_CAP` is `ReportTooLarge`. Read errors are `ReportNotFound`. Nothing is written.
    """
    parts = _check_report_path(path)
    top = _report_dir(conn, workspace, repo_no, folder)
    dir_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    fds: list[int] = []
    try:
        fds.append(os.open(top, dir_flags))
        for part in parts[:-1]:
            fds.append(os.open(part, dir_flags, dir_fd=fds[-1]))
        fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=fds[-1])
        fds.append(fd)
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode):
            raise ReportNotFound
        if info.st_size > REPORT_READ_CAP:
            raise ReportTooLarge
        chunks, left = [], REPORT_READ_CAP + 1
        while left > 0 and (chunk := os.read(fd, min(left, 1 << 20))):
            chunks.append(chunk)
            left -= len(chunk)
    except OSError:
        raise ReportNotFound from None
    finally:
        for fd in fds:
            os.close(fd)
    data = b"".join(chunks)
    if len(data) > REPORT_READ_CAP:
        raise ReportTooLarge  # grew after fstat
    return REPORT_TYPES[_extension(parts[-1])], data
