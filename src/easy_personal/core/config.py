import os
from pathlib import Path

HOME_ENV = "EASY_PERSONAL_HOME"


def data_dir() -> Path:
    """Runtime data dir (~/.easy-personal by default), created with mode 0700."""
    path = Path(os.environ.get(HOME_ENV) or Path.home() / ".easy-personal")
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    path.chmod(0o700)
    return path
