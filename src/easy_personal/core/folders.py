"""Folder names for the dashboard's folder browser (HUN-203, FR-068). Read-only.

Sub-folder names and a `repo` flag only: never a file name, size, content or symlink target.
"""

import os
from pathlib import Path

MAX_DIRS = 500


def list_dirs(path: str | None) -> dict:
    """Sub-folders of `path` (default: the process working directory). ValueError = bad path; PermissionError = unreadable."""
    if not path:
        try:
            path = os.getcwd()
        except OSError:  # the launch folder was deleted
            path = str(Path.home())
    if not os.path.isabs(path):
        raise ValueError("Path must be absolute")
    here = Path(path).resolve()
    if not here.is_dir():  # also False for a NUL byte in the path
        raise ValueError("Not an existing folder")
    names = []
    with os.scandir(here) as entries:  # PermissionError / other OSError: the route answers 403
        for entry in entries:
            if entry.name.startswith("."):
                continue
            try:
                if entry.is_dir(follow_symlinks=True):  # a broken link or a loop is not a folder
                    names.append(entry.name)
            except OSError:
                continue
    names.sort(key=lambda n: (n.casefold(), n))
    cut = len(names) > MAX_DIRS
    body = {
        "path": str(here),
        "parent": None if here == here.parent else str(here.parent),
        "home": str(Path.home()),
        "dirs": [{"name": n, "repo": os.path.lexists(here / n / ".git")} for n in names[:MAX_DIRS]],
    }
    if cut:
        body["truncated"] = True
    return body
