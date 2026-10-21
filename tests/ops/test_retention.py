import json
import sqlite3
import subprocess
import sys
from datetime import UTC, datetime, timedelta
from pathlib import Path

from easy_personal.core import store
from tests_helpers import child_env

NOW = datetime(2026, 10, 21, 12, 0, tzinfo=UTC)


def _fmt(dt):
    return dt.strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def _add(conn, offset, ts):
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, "
        "ts_utc) VALUES ('m', 's', 'p', 1, ?, ?, ?)",
        (offset, 0 if ts else 1, ts),
    )


def _seed(conn):
    _add(conn, 1, _fmt(NOW - timedelta(days=45)))
    _add(conn, 2, _fmt(NOW - timedelta(days=31)))
    _add(conn, 3, _fmt(NOW - timedelta(days=29)))
    _add(conn, 4, _fmt(NOW))
    _add(conn, 5, None)  # abstained record without a timestamp: age unknown, kept
    conn.commit()


def _offsets(conn):
    return [r[0] for r in conn.execute("SELECT byte_offset FROM events ORDER BY byte_offset")]


def test_unset_prunes_nothing(conn):
    _seed(conn)
    assert store.prune(conn, None, now=NOW) == 0
    assert _offsets(conn) == [1, 2, 3, 4, 5]


def test_prune_removes_only_rows_older_than_n_days(conn):
    _seed(conn)
    assert store.prune(conn, 30, now=NOW) == 2
    assert _offsets(conn) == [3, 4, 5]


def test_cli_ingest_retention_days_prunes_and_reports(tmp_path, fixtures_root):
    env = child_env(tmp_path)
    env["EASY_PERSONAL_CLAUDE_PROJECTS"] = str(fixtures_root)
    cmd = [sys.executable, "-m", "easy_personal.surfaces.cli", "ingest"]
    first = json.loads(subprocess.run(cmd, env=env, capture_output=True, check=True).stdout)
    assert first["new_events"] > 0 and "pruned" not in first
    out = subprocess.run(
        [*cmd, "--retention-days", "1"], env=env, capture_output=True, check=True
    ).stdout
    result = json.loads(out)
    # fixtures are older than 1 day; rows without a timestamp (abstained) stay
    with_ts = (
        sqlite3.connect(Path(env["EASY_PERSONAL_HOME"]) / "store.sqlite")
        .execute("SELECT COUNT(*) FROM events WHERE ts_utc IS NOT NULL")
        .fetchone()[0]
    )
    assert with_ts == 0 and result["pruned"] > 0
