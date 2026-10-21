"""Project assignment (FR-009): git top-level of a cwd, worktrees under their main repo."""

import subprocess
from pathlib import Path


def _git(cwd: str, *args: str) -> str | None:
    try:
        proc = subprocess.run(
            ["git", "-C", cwd, *args], capture_output=True, text=True, timeout=5, check=False
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    out = proc.stdout.strip()
    return out if proc.returncode == 0 and out else None


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
