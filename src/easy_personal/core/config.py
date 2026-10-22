import os
from pathlib import Path

HOME_ENV = "EASY_PERSONAL_HOME"
CLAUDE_PROJECTS_ENV = "EASY_PERSONAL_CLAUDE_PROJECTS"
UI_REPORTS_ENV = "EASY_PERSONAL_UI_REPORTS"


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


def config_path() -> Path:
    """Owner settings file; optional. It holds rule `thresholds:` and `ui: reports_dir:`."""
    return data_dir() / "config.yaml"


def ui_reports_dir() -> Path | None:
    """easy-ui report folder (`session-*.json`); None when not configured (UI checks: no data).

    `EASY_PERSONAL_UI_REPORTS` wins; else `reports_dir` under `ui:` in config.yaml (the scheduled
    ingest has no shell env, so the file is the persistent place).
    """
    value = os.environ.get(UI_REPORTS_ENV)
    if not value and config_path().exists():
        from easy_personal.core.rules import _read_flat  # same reader as `thresholds:`; late: cycle

        try:
            value = _read_flat(config_path(), "ui").get("reports_dir")
        except (OSError, UnicodeDecodeError):
            value = None
    return Path(value).expanduser() if value else None
