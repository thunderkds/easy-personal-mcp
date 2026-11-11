"""T042: Python's answers over the store and folders that `kit_parity.rs` builds (`expected.json`).

Run from the repo root with the parity venv (HOME and EASY_PERSONAL_HOME are scratch folders, never the real ones):

    TZ=UTC rust/parity/.venv/bin/python -B -I rust/crates/core/tests/kit_extra/make_expected.py > rust/crates/core/tests/kit_extra/expected.json

It builds the same tree as the Rust test: a copy of `fixtures-rich/kit`, the rich store with `/work/rich/kit` moved to the
work folder, `extra.sql`, four git repos, report folders. The answers hold names and counts only, so the work folder
does not show. `commits_by_day` is OS-local-day text: the Rust test checks it apart, under each `TZ`.
"""

import json
import os
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from datetime import UTC, datetime
from pathlib import Path

REPO = Path(__file__).resolve().parents[5]
PARITY = REPO / "rust" / "parity"
HERE = Path(__file__).resolve().parent
FROZEN_NOW = datetime(2026, 1, 20, tzinfo=UTC)
RICH_MTIME = 1768694400
COMMITS = {  # repo -> committer dates (UTC)
    "repo-a": ["2026-01-05T10:00:00+0000", "2026-01-05T23:30:00+0000", "2026-01-12T01:00:00+0000"],
    "repo-b": ["2026-01-03T12:00:00+0000"],
    "repo-c": [],
}


def git(cwd, *args, date=None):
    env = {"PATH": os.environ["PATH"], "HOME": str(cwd), "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1"}
    if date:
        env |= {"GIT_AUTHOR_DATE": date, "GIT_COMMITTER_DATE": date}
    subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid", "-C", str(cwd), *args],
                   check=True, capture_output=True, env=env)


def touch(path, ns=RICH_MTIME * 10**9):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("x")
    os.utime(path, ns=(ns, ns))


def build(work: Path) -> Path:
    shutil.copytree(PARITY / "fixtures-rich" / "kit", work / "kit")
    for path in (work / "kit").rglob("*"):
        os.utime(path, (RICH_MTIME, RICH_MTIME))
    for name, dates in COMMITS.items():
        (work / name).mkdir()
        git(work / name, "init", "-q")
        for date in dates:
            git(work / name, "commit", "-q", "--allow-empty", "-m", "c", date=date)
    (work / "plain").mkdir()
    reports = work / "repo-a"
    touch(reports / "test-reports" / "session-001.json", RICH_MTIME * 10**9 + 999_999_700)
    touch(reports / "test-reports" / "notes.txt")
    touch(reports / "test-reports" / "sub" / "deep" / "x.json", (RICH_MTIME - 86400) * 10**9)
    touch(reports / "test-reports" / "sub" / "deep" / "y" / "too-deep.json")
    touch(reports / "Reports-Archive" / "a.json", (RICH_MTIME - 5) * 10**9)
    touch(reports / "reports.txt")
    os.symlink("test-reports", reports / "ref-reports")
    for name in ["reports"] + [f"report-{n}" for n in range(1, 9)]:
        (work / "repo-b" / name).mkdir()
    store = work / "store.sqlite"
    shutil.copy(PARITY / "store-rich" / "store-v11.sqlite", store)
    conn = sqlite3.connect(store)
    tables = [r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")]
    for table in tables:
        for _, column, ctype, *_ in conn.execute(f'PRAGMA table_info("{table}")').fetchall():
            if ctype.upper() == "TEXT":
                conn.execute(
                    f'UPDATE "{table}" SET "{column}" = replace("{column}", ?, ?) WHERE instr("{column}", ?) > 0',
                    ("/work/rich/kit", f"{work}/kit", "/work/rich/kit"),
                )
    conn.executescript((HERE / "extra.sql").read_text().replace("@T@", str(work)))
    conn.commit()
    conn.close()
    return store


def main() -> None:
    scratch = Path(tempfile.mkdtemp(prefix="easy-personal-t042-"))
    os.environ["HOME"] = str(scratch)
    os.environ["EASY_PERSONAL_HOME"] = str(scratch / "home")
    work = scratch / "w"
    work.mkdir()
    sys.path.insert(0, str(REPO / "src"))
    from easy_personal.core import kit

    conn = sqlite3.connect(build(work))
    answers = {}
    for name, who in {"all": {}, "acme": {"workspace": "acme"}, "personal": {"workspace": "personal"},
                      "work": {"account": "Claude work"}, "default": {"account": "1"},
                      "acme_work": {"workspace": "acme", "account": "Claude work"}}.items():
        answers[f"mcp_{name}"] = kit.kit_summary(conn, **who, now=FROZEN_NOW)
        answers[f"api_{name}"] = kit.kit_summary(conn, **who, hide_gone=True, now=FROZEN_NOW)
    answers["rollup_all"] = kit.workspace_rollup(conn)
    answers["rollup_work"] = kit.workspace_rollup(conn, account="Claude work")
    answers["rollup_window"] = kit.workspace_rollup(conn, "2026-01-16T00:00:00Z", "2026-01-20T00:00:00Z")
    answers["reports"] = kit.report_folders(conn)
    print(json.dumps(answers, sort_keys=True, indent=1).replace(str(work), "<T>"))
    shutil.rmtree(scratch)


main()
