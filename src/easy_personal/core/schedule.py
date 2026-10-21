"""systemd user timer for scheduled ingest (Linux). Writes only under ~/.config/systemd/user/."""

import os
import shlex
import shutil
import subprocess
import sys
from importlib import resources
from pathlib import Path
from string import Template

SERVICE = "easy-personal-ingest.service"
TIMER = "easy-personal-ingest.timer"


def unit_dir() -> Path:
    base = os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config"
    return Path(base) / "systemd" / "user"


def command() -> str:
    """Absolute command that runs the installed CLI."""
    found = shutil.which("easy-personal")
    if found:
        return shlex.quote(str(Path(found).absolute()))
    return f"{shlex.quote(sys.executable)} -m easy_personal.surfaces.cli"


def ingest_command(cmd: str, retention_days: int | None) -> str:
    extra = f" --retention-days {retention_days}" if retention_days is not None else ""
    return f"{cmd} ingest{extra}"


def render_units(cmd: str, retention_days: int | None) -> dict[str, str]:
    data = resources.files("easy_personal") / "data" / "systemd"
    service = Template((data / SERVICE).read_text()).substitute(
        exec_start=ingest_command(cmd, retention_days)
    )
    return {SERVICE: service, TIMER: (data / TIMER).read_text()}


def cron_snippet(cmd: str, retention_days: int | None) -> str:
    return f"# add with: crontab -e\n17 3 * * * {ingest_command(cmd, retention_days)}"


def systemd_available() -> bool:
    if not shutil.which("systemctl"):
        return False
    done = subprocess.run(
        ["systemctl", "--user", "show-environment"], capture_output=True, check=False
    )
    return done.returncode == 0


def systemctl(*args: str) -> None:
    subprocess.run(["systemctl", "--user", *args], capture_output=True, check=True)


def install(cmd: str, retention_days: int | None) -> Path:
    directory = unit_dir()
    directory.mkdir(parents=True, exist_ok=True)
    for name, text in render_units(cmd, retention_days).items():
        (directory / name).write_text(text)
    systemctl("daemon-reload")
    systemctl("enable", "--now", TIMER)
    return directory


def remove() -> Path:
    directory = unit_dir()
    installed = (directory / TIMER).exists()
    if installed:
        systemctl("disable", "--now", TIMER)
    for name in (TIMER, SERVICE):
        (directory / name).unlink(missing_ok=True)
    if installed:
        systemctl("daemon-reload")
    return directory
