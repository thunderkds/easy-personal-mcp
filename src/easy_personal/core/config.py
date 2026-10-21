import os
from pathlib import Path

HOME_ENV = "EASY_PERSONAL_HOME"
CLAUDE_PROJECTS_ENV = "EASY_PERSONAL_CLAUDE_PROJECTS"


def data_dir() -> Path:
    """Runtime data dir (~/.easy-personal by default), created with mode 0700.

    A directory that already exists is used as is: its mode is not ours to change.
    """
    path = Path(os.environ.get(HOME_ENV) or Path.home() / ".easy-personal")
    try:
        path.mkdir(mode=0o700, parents=True)
    except FileExistsError:
        return path
    path.chmod(0o700)  # mkdir's mode is masked by the umask
    return path


def store_path() -> Path:
    return data_dir() / "store.sqlite"


def claude_projects_dir() -> Path:
    """Claude Code transcript root; overridable so tests never read the real folder."""
    return Path(os.environ.get(CLAUDE_PROJECTS_ENV) or Path.home() / ".claude" / "projects")
