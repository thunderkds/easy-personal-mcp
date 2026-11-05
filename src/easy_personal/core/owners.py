"""Owner project (FR-057): the stored repo a gone worktree or subfolder belongs to, worked out at read time.

For a stored path whose folder is gone: (1) the git repo of its nearest existing parent folder; else
(2) the one stored repo that has one of the path's event branches as a ref or in a merge commit
subject (on a tie, the one outside hidden folders); else (3) no owner. An existing folder has none.
Nothing is written to the store, and a branch text is compared in Python, never given to git.

`project_map` (FR-069) widens this to every stored path: its project key is the owner repo when there is
one, else `SCRATCH` for a scratch folder (system temp folder, gone, not in a git repo, or inside a hidden
folder), else the path itself.
"""

import os
import re
import sqlite3
import subprocess
import tempfile
import time
from collections.abc import Mapping
from pathlib import Path
from types import MappingProxyType

from easy_personal.core import discovery, memo

SCRATCH = "Scratch"
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


def merge_branch(subject: str) -> str | None:
    """The branch a merge commit subject names (any FR-057 form), else None."""
    for pattern in MERGE_SUBJECTS:
        found = pattern.match(subject)
        if found:
            return found.group(1)
    return None


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
            branch = merge_branch(subject)
            if branch:
                names.add(branch)
        return frozenset(names)

    return _cached("branches", repo, read)


def _repo_of_parent(path: str) -> str | None:
    """Rule 1: the repo of the nearest existing parent folder of a gone path."""
    if not Path(path).is_absolute():
        return None
    parent = next(p for p in Path(path).parents if p.is_dir())  # "/" always exists
    owner = _cached("repo", str(parent), lambda: discovery.repo_of(str(parent)))
    return owner if owner and Path(owner).is_dir() else None


@memo.memoized
def _stored_branches(conn: sqlite3.Connection) -> MappingProxyType:
    """Stored project path -> the git branches of its events (`""` for none).

    Read-only (mapping and frozensets): the result is shared between calls while the store is unchanged.
    """
    branches: dict[str, set[str]] = {}
    for path, branch in conn.execute(
        "SELECT DISTINCT project_path, git_branch FROM events WHERE project_path IS NOT NULL"
    ):
        branches.setdefault(path, set()).add(branch or "")
    return MappingProxyType({path: frozenset(names) for path, names in branches.items()})


def owner_map(conn: sqlite3.Connection) -> dict[str, str]:
    """Stored gone project path -> its owner project path; only the paths that have an owner."""
    return _owners(_stored_branches(conn))


def _owners(branches: Mapping[str, frozenset[str]]) -> dict[str, str]:
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


def _tmp_roots() -> tuple[str, ...]:
    """Real paths of the system temp folder and `/tmp` (a symlinked `/tmp` still matches)."""
    return tuple({os.path.realpath(tempfile.gettempdir()), os.path.realpath("/tmp")})


def _is_scratch(path: str) -> bool:
    """FR-069 (a)-(d) for a path with no owner project; a path under a temp folder is decided before any git call."""
    if not path or not os.path.isabs(path):
        return True
    real = os.path.realpath(path)
    if any(real == root or real.startswith(root.rstrip("/") + "/") for root in _tmp_roots()):
        return True  # (a)
    if any(part.startswith(".") for part in Path(path).parts):
        return True  # (d)
    if not Path(path).is_dir():
        return True  # (b)
    return _cached("repo", path, lambda: discovery.repo_of(path)) is None  # (c)


def project_map(conn: sqlite3.Connection) -> dict[str, str]:
    """Every stored project path -> its project key (FR-069): the owner repo (FR-057), else `SCRATCH`, else the path.

    Owner first, then the scratch rule. Git checks share the 300 s cache with the owner rules.
    """
    branches = _stored_branches(conn)
    owner = _owners(branches)
    return {p: owner.get(p) or (SCRATCH if _is_scratch(p) else p) for p in branches}
