"""git_local source (FR-014): local commits per day for a project, read from local refs at query time.

Only `git log` runs, on local refs; never fetch, pull or ls-remote. Reads commit times only: no
messages, authors or diffs.
"""

import subprocess
from datetime import datetime

from easy_personal.core.discovery import run_git

TIMEOUT_S = 10


def commits_by_day(project: str) -> dict[str, int] | None:
    """{OS-local day: commit count} over all local branches, tags and remote-tracking refs.

    None when the folder is not a git repo, is missing, or has no commits. A worktree path or the
    main repo gives the same answer because refs are shared. Raises `subprocess.TimeoutExpired`.
    """
    out = run_git(
        project, "log", "--branches", "--tags", "--remotes", "--format=%ct", timeout=TIMEOUT_S
    )
    if not out:
        return None
    days: dict[str, int] = {}
    for stamp in out.split():
        day = datetime.fromtimestamp(int(stamp)).astimezone().strftime("%Y-%m-%d")
        days[day] = days.get(day, 0) + 1
    return dict(sorted(days.items()))


def git_part(project: str) -> dict:
    """The `git` part of kit_summary: ok, no data, or abstained (git too slow)."""
    try:
        days = commits_by_day(project)
    except subprocess.TimeoutExpired:
        return {"status": "abstained"}
    if days is None:
        return {"status": "no data"}
    return {"status": "ok", "commits": sum(days.values()), "commits_by_day": days}
