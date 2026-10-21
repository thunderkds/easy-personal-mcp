import os
from pathlib import Path

SRC = str(Path(__file__).resolve().parents[1] / "src")


def child_env(tmp_path: Path) -> dict[str, str]:
    env = dict(os.environ)
    env.update(PYTHONPATH=SRC, EASY_PERSONAL_HOME=str(tmp_path / "home"))
    return env
