"""Owner project (FR-057): the stored repo a gone worktree or subfolder belongs to, worked out at read time.

For a stored path whose folder is gone: (1) the git repo of its nearest existing parent folder; else
(2) the one stored repo that has one of the path's event branches as a ref or in a merge commit
subject (on a tie, the one outside hidden folders); else (3) no owner. An existing folder has none.
Nothing is written to the store, and a branch text is compared in Python, never given to git.
"""

import re
import sqlite3
import subprocess
import time
from pathlib import Path

from easy_personal.core import discovery

IGNORED_BRANCHES = frozenset({"", "main", "master", "HEAD"})
CACHE_SECONDS = 300
MERGE_SUBJECTS = (
    re.compile(r"^Merge branch '([^']+)'"),
    re.compile(r"^Merge pull request #\d+ from [^/\s]+/(\S+)"),
    re.compile(r"^Merge (?!branch |pull request |remote-tracking )([^\s:']+)"),  # `Merge <b>: <what it does>`
)

_cache: dict[tuple[str, str], tuple[float, object]] = {}


def clear_cache() -> None:
    _cache.clear()


def _cached(kind: str, folder: str, compute):
    """`compute()` once per (kind, folder) and CACHE_SECONDS, so a repo costs at most two git calls per process."""
    hit = _cache.get((kind, folder))
    if hit and time.monotonic() - hit[0] < CACHE_SECONDS:
        return hit[1]
    value = compute()
    _cache[(kind, folder)] = (time.monotonic(), value)
    return value


def _lines(repo: str, *args: str) -> list[str]:
    try:
        return (discovery.run_git(repo, *args) or "").splitlines()
    except subprocess.TimeoutExpired:
        return []


def _repo_branches(repo: str) -> frozenset[str]:
    """Branch names of a repo: local and remote refs, plus branches named in merge commit subjects."""

    def read():
        names = set()
        for ref in _lines(repo, "for-each-ref", "--format=%(refname)", "refs/heads", "refs/remotes"):
            parts = ref.split("/", 3)  # refs/heads/<name> or refs/remotes/<remote>/<name>
            if parts[1] == "heads" and len(parts) > 2:
                names.add(ref.split("/", 2)[2])
            elif len(parts) == 4:
                names.add(parts[3])
        for subject in _lines(repo, "log", "--all", "--merges", "--format=%s"):
            for pattern in MERGE_SUBJECTS:
                found = pattern.match(subject)
                if found:
                    names.add(found.group(1))
        return frozenset(names)

    return _cached("branches", repo, read)


def _repo_of_parent(path: str) -> str | None:
    """Rule 1: the repo of the nearest existing parent folder of a gone path."""
    if not Path(path).is_absolute():
        return None
    parent = next(p for p in Path(path).parents if p.is_dir())  # "/" always exists
    owner = _cached("repo", str(parent), lambda: discovery.repo_of(str(parent)))
    return owner if owner and Path(owner).is_dir() else None


def owner_map(conn: sqlite3.Connection) -> dict[str, str]:
    """Stored gone project path -> its owner project path; only the paths that have an owner."""
    branches: dict[str, set[str]] = {}
    for path, branch in conn.execute(
        "SELECT DISTINCT project_path, git_branch FROM events WHERE project_path IS NOT NULL"
    ):
        branches.setdefault(path, set()).add(branch or "")
    mapping: dict[str, str] = {}
    candidates = None
    for path in sorted(p for p in branches if not Path(p).is_dir()):
        owner = _repo_of_parent(path)
        wanted = branches[path] - IGNORED_BRANCHES
        if owner is None and wanted:
            if candidates is None:
                candidates = [p for p in sorted(branches) if (Path(p) / ".git").exists()]
            matches = [repo for repo in candidates if wanted & _repo_branches(repo)]
            if len(matches) > 1:  # a tie: a repo inside a hidden folder (e.g. `.spawn/…/repo`, a clone) loses to a visible one
                matches = [r for r in matches if not any(part.startswith(".") for part in Path(r).parts)]
            owner = matches[0] if len(matches) == 1 else None
        if owner:
            mapping[path] = owner
    return mapping
