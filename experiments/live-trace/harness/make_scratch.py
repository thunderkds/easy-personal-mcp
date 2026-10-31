"""Make a scratch git repo for the standard test session (HUN-165).

Metadata only: the files hold short harmless text; nothing here reads credentials.
Usage: python make_scratch.py [--parent DIR]   (prints the new folder path)
"""
import argparse
import subprocess
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

FILES = {
    "notes_a.txt": "alpha one\nalpha two\nalpha three\n",
    "notes_b.txt": "bravo one\nbravo two\n",
    "notes_c.txt": "charlie one\ncharlie two\ncharlie three\ncharlie four\n",
    "check.sh": "#!/bin/sh\necho check ok\nexit 0\n",
}


def make_scratch(parent=None) -> Path:
    """Create a git repo with 3 text files and a script, outside the product repo."""
    path = Path(tempfile.mkdtemp(prefix="live-trace-", dir=parent)).resolve()
    if REPO_ROOT in (path, *path.parents):
        raise ValueError(f"scratch folder must be outside the repo: {path}")
    for name, body in FILES.items():
        (path / name).write_text(body)
    (path / "check.sh").chmod(0o755)
    git = ["git", "-C", str(path)]
    subprocess.run([*git, "init", "-q"], check=True)
    subprocess.run([*git, "config", "user.email", "scratch@example.invalid"], check=True)
    subprocess.run([*git, "config", "user.name", "scratch"], check=True)
    subprocess.run([*git, "add", "."], check=True)
    subprocess.run([*git, "commit", "-q", "-m", "scratch"], check=True)
    return path


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--parent", default=None, help="parent folder (default: system temp dir)")
    print(make_scratch(ap.parse_args().parent))


if __name__ == "__main__":
    main()
