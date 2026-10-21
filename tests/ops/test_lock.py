import os
import sqlite3
import subprocess
import sys
import time

import pytest

from easy_personal.core import lock
from tests_helpers import child_env

HOLDER = """
import sys, time
from pathlib import Path
from easy_personal.core import lock
with lock.ingest_lock(Path(sys.argv[1]), timeout=5):
    print("held", flush=True)
    time.sleep(float(sys.argv[2]))
"""


def test_second_holder_gets_lock_busy(tmp_path):
    path = tmp_path / "ingest.lock"
    holder = subprocess.Popen(
        [sys.executable, "-c", HOLDER, str(path), "3"], stdout=subprocess.PIPE, text=True,
        env=child_env(tmp_path),
    )  # fmt: skip
    try:
        assert holder.stdout.readline().strip() == "held"
        with pytest.raises(lock.LockBusy), lock.ingest_lock(path, timeout=0.2):
            pass
    finally:
        holder.kill()
        holder.wait()


def test_waiter_gets_lock_after_release(tmp_path):
    path = tmp_path / "ingest.lock"
    holder = subprocess.Popen(
        [sys.executable, "-c", HOLDER, str(path), "0.6"], stdout=subprocess.PIPE, text=True,
        env=child_env(tmp_path),
    )  # fmt: skip
    assert holder.stdout.readline().strip() == "held"
    start = time.monotonic()
    with lock.ingest_lock(path, timeout=10):
        waited = time.monotonic() - start
    holder.wait()
    assert waited >= 0.2


def test_lock_file_left_by_crashed_run_does_not_block(tmp_path):
    path = tmp_path / "ingest.lock"
    holder = subprocess.Popen(
        [sys.executable, "-c", HOLDER, str(path), "30"], stdout=subprocess.PIPE, text=True,
        env=child_env(tmp_path),
    )  # fmt: skip
    assert holder.stdout.readline().strip() == "held"
    holder.kill()  # crash: the file stays, the OS drops the flock
    holder.wait()
    assert path.exists()
    with lock.ingest_lock(path, timeout=1):
        pass


def test_two_concurrent_ingests_keep_store_intact(tmp_path, fixtures_root):
    home = tmp_path / "home"
    env = child_env(tmp_path)
    env.update(EASY_PERSONAL_HOME=str(home), EASY_PERSONAL_CLAUDE_PROJECTS=str(fixtures_root))
    cmd = [sys.executable, "-m", "easy_personal.surfaces.cli", "ingest"]
    procs = [subprocess.Popen(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
             for _ in range(2)]  # fmt: skip
    results = [p.communicate(timeout=120) + (p.returncode,) for p in procs]
    assert [r[2] for r in results] == [0, 0], results
    conn = sqlite3.connect(home / "store.sqlite")
    assert conn.execute("PRAGMA integrity_check").fetchone() == ("ok",)
    total, distinct = conn.execute(
        "SELECT COUNT(*), COUNT(DISTINCT machine_id || source_path || inode || byte_offset) "
        "FROM events"
    ).fetchone()
    assert total > 0 and total == distinct


def test_ingest_exits_with_message_when_lock_stays_busy(tmp_path, fixtures_root):
    home = tmp_path / "home"
    home.mkdir()
    env = child_env(tmp_path)
    env.update(
        EASY_PERSONAL_HOME=str(home),
        EASY_PERSONAL_CLAUDE_PROJECTS=str(fixtures_root),
        EASY_PERSONAL_LOCK_TIMEOUT="0.2",
    )
    holder = subprocess.Popen(
        [sys.executable, "-c", HOLDER, str(home / "ingest.lock"), "5"], stdout=subprocess.PIPE,
        text=True, env=env,
    )  # fmt: skip
    try:
        assert holder.stdout.readline().strip() == "held"
        done = subprocess.run(
            [sys.executable, "-m", "easy_personal.surfaces.cli", "ingest"],
            env=env, capture_output=True, text=True, timeout=60, check=False,
        )  # fmt: skip
    finally:
        holder.kill()
        holder.wait()
    assert done.returncode != 0
    assert "another ingest is running" in done.stderr
    assert os.path.exists(home / "ingest.lock")
