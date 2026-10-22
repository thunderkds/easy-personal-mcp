"""Project assignment (FR-009): git top-level of a cwd, worktrees under their main repo."""

import os
import subprocess
from pathlib import Path


def run_git(cwd: str, *args: str, timeout: float = 5) -> str | None:
    """Run a read-only git command in `cwd`; stdout, or None on failure or empty output.

    Hardened: no fsmonitor daemon, no optional index lock. Raises `subprocess.TimeoutExpired`.
    """
    try:
        proc = subprocess.run(
            ["git", "-c", "core.fsmonitor=false", "-C", cwd, *args],
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
            env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"},
        )
    except OSError:
        return None
    out = proc.stdout.strip()
    return out if proc.returncode == 0 and out else None


def _git(cwd: str, *args: str) -> str | None:
    try:
        return run_git(cwd, *args)
    except subprocess.TimeoutExpired:
        return None


def resolve_project(cwd: str) -> str:
    """Return the project path for a session cwd.

    A folder missing on disk, or not inside a git repo, is its own project. A worktree belongs to
    the repo that owns its common git dir; a subfolder belongs to its repo's top level.
    """
    if not Path(cwd).is_dir():
        return cwd
    common = _git(cwd, "rev-parse", "--path-format=absolute", "--git-common-dir")
    if common and Path(common).name == ".git":
        return str(Path(common).parent)
    return _git(cwd, "rev-parse", "--show-toplevel") or cwd
