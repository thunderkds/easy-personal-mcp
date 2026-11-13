"""HUN-244 (FR-091): the read-only store health check, `core/store_health.py` and `easy-personal store-check`."""

import hashlib
import json
import os
import sqlite3
import time

import pytest

from easy_personal.core import config, memo, store, store_health
from easy_personal.core.migrations import MIGRATIONS
from easy_personal.surfaces import cli
from tests_helpers import add_event

LATEST = max(n for n, _ in MIGRATIONS)
CANARY = "CANARY-SECRET-244"


@pytest.fixture
def home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path))
    return tmp_path


@pytest.fixture
def path(home):
    """A migrated store: a few events (one abstained), a kit trace row, a ui row; the canary sits in a row."""
    target = config.store_path()
    with store.opened(target) as conn:
        for n in range(5):
            add_event(conn, n, f"/work/{CANARY}")
        conn.execute("UPDATE events SET abstained = 1 WHERE byte_offset = 0")
        conn.execute(
            "INSERT INTO kit_trace_events (machine_id, source_path, inode, byte_offset, project_path, abstained, ts_utc)"
            " VALUES ('m', '/t', 1, 0, ?, 1, '2026-02-01T00:00:00.000000Z')", (f"/work/{CANARY}",))
        conn.execute("INSERT INTO ui_sessions VALUES ('u1', 'ok', '2026-03-01T00:00:00Z', NULL, 2)")
        conn.commit()
    return target


def _fingerprint(target):
    wal = target.parent / (target.name + "-wal")
    return (
        hashlib.sha256(target.read_bytes()).hexdigest(), target.stat().st_mtime_ns,
        wal.read_bytes() if wal.exists() else None,
    )


def test_every_field_is_there_with_the_right_type(path):
    result = store_health.check(path)
    assert result["store_file"] == "store.sqlite" and result["exists"] is True and result["readable"] is True
    assert isinstance(result["size_bytes"], int) and result["size_bytes"] > 0
    assert isinstance(result["wal_bytes"], int)
    assert result["user_version"] == LATEST == result["latest_migration"]
    assert result["migration_pending"] is False and result["newer_than_code"] is False
    assert result["integrity_check"] == {"ok": True, "problems": 0, "mode": "full"}
    assert result["foreign_key_check"] == {"ok": True, "problems": 0}
    assert isinstance(result["tables"], dict) and all(isinstance(c, int) for c in result["tables"].values())
    assert set(result["sources"]) >= {"claude_code", "kit_trace", "context", "ui", "verifier"}
    assert result["backups"] == [] and result["incomplete"] == [] and result["healthy"] is True


def test_cli_json_and_human_forms(path, capsys):
    assert cli.main(["store-check", "--json"]) == 0
    data = json.loads(capsys.readouterr().out)
    assert data["user_version"] == LATEST and data["healthy"] is True
    assert cli.main(["store-check"]) == 0
    out = capsys.readouterr().out.splitlines()
    prefixes = ["file:", "schema:", "integrity_check:", "foreign_key_check:", "tables:", "sources:", "backups:", "verdict:"]
    assert [line.split(" ")[0] for line in out] == prefixes
    assert out[-1] == "verdict: healthy"


def test_a_store_one_migration_behind_is_pending_and_never_migrated(path, capsys):
    with sqlite3.connect(path) as conn:
        conn.execute(f"PRAGMA user_version = {LATEST - 1}")
    before = _fingerprint(path)
    result = store_health.check(path)
    assert result["user_version"] == LATEST - 1 and result["migration_pending"] is True and result["healthy"] is False
    assert cli.main(["store-check"]) == 1
    assert "migration pending: yes" in capsys.readouterr().out
    assert _fingerprint(path) == before
    assert not list(path.parent.glob("store.sqlite.bak-v*"))  # a migration would have left a backup


def test_a_store_newer_than_the_code_is_reported_not_migrated(path):
    with sqlite3.connect(path) as conn:
        conn.execute(f"PRAGMA user_version = {LATEST + 1}")
    result = store_health.check(path)
    assert result["newer_than_code"] is True and result["migration_pending"] is False and result["healthy"] is False
    assert "newer than this program" in "\n".join(store_health.lines(result))


def _corrupt_page(target, page: int) -> None:
    size = sqlite3.connect(target).execute("PRAGMA page_size").fetchone()[0]
    with open(target, "r+b") as handle:
        handle.seek(size * (page - 1))
        handle.write(b"\xff" * size)


def test_a_corrupted_store_reports_problem_counts_and_no_row_content(home, capsys):
    target = config.store_path()
    with store.opened(target) as conn:
        for n in range(3000):
            add_event(conn, n, f"/work/{CANARY}")
        conn.commit()
    with sqlite3.connect(target) as conn:
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    pages = os.path.getsize(target) // sqlite3.connect(target).execute("PRAGMA page_size").fetchone()[0]
    _corrupt_page(target, pages // 2)
    result = store_health.check(target)
    assert result["integrity_check"]["ok"] is False and result["integrity_check"]["problems"] >= 1
    assert result["healthy"] is False
    assert cli.main(["store-check", "--json"]) == 1
    out = capsys.readouterr().out
    assert cli.main(["store-check"]) == 1
    out += capsys.readouterr().out + json.dumps(result)
    assert CANARY not in out and "/work/" not in out


def test_foreign_key_violations_are_counted_not_shown(path):
    with sqlite3.connect(path) as conn:  # foreign keys are off on a plain connection: plant an orphan row
        conn.execute("INSERT INTO workspace_repos (workspace_id, repo_path) VALUES (999, ?)", (f"/{CANARY}",))
    result = store_health.check(path)
    assert result["foreign_key_check"] == {"ok": False, "problems": 1}
    assert result["healthy"] is False and CANARY not in json.dumps(result)


def test_row_counts_equal_count_star_and_a_later_table_is_counted(path):
    with sqlite3.connect(path) as conn:
        conn.execute("CREATE TABLE added_by_a_later_migration (x)")
        conn.executemany("INSERT INTO added_by_a_later_migration VALUES (?)", [(1,), (2,)])
    result = store_health.check(path)
    with sqlite3.connect(path) as conn:
        names = [r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")]
        for name in names:
            assert result["tables"][name] == conn.execute(f'SELECT COUNT(*) FROM "{name}"').fetchone()[0]
    assert set(result["tables"]) == set(names) and result["tables"]["added_by_a_later_migration"] == 2
    assert result["tables"]["events"] == 5 and result["tables"]["workspace_repos"] == 0


def test_sources_report_last_time_and_abstained_or_no_data(path, capsys):
    sources = store_health.check(path)["sources"]
    assert sources["claude_code"] == {"last_ingest_utc": "2026-01-01T00:00:04.000000Z", "abstained": 1}
    assert sources["kit_trace"] == {"last_ingest_utc": "2026-02-01T00:00:00.000000Z", "abstained": 1}
    assert sources["ui"] == {"last_ingest_utc": "2026-03-01T00:00:00Z", "abstained": None}
    for never in ("context", "verifier"):
        assert sources[never] == {"last_ingest_utc": None, "abstained": None}
    assert cli.main(["store-check"]) == 0  # a source that never ran does not fail the command
    assert "context no data" in capsys.readouterr().out


def test_a_fresh_empty_store_is_healthy_with_every_source_no_data(home, capsys):
    with store.opened(config.store_path()):
        pass
    assert cli.main(["store-check"]) == 0
    assert capsys.readouterr().out.count("no data") == 5


def test_backups_are_listed_by_name_oldest_version_first(path):
    for name in ("store.sqlite.bak-v10", "store.sqlite.bak-v3", "store.sqlite.bak-v4",
                 f"store.sqlite.bak-v5-{CANARY}", "store.sqlite.bak-v3-wal", f"{CANARY}.txt"):
        (path.parent / name).write_bytes(b"x")
    result = store_health.check(path)
    assert result["backups"] == ["store.sqlite.bak-v3", "store.sqlite.bak-v4", "store.sqlite.bak-v10"]
    assert str(path.parent) not in json.dumps(result) and CANARY not in json.dumps(result)


def test_no_backups_is_an_empty_list(path):
    assert store_health.check(path)["backups"] == []


def test_nothing_is_written_not_the_file_its_mtime_or_the_wal(path):
    writer = sqlite3.connect(path)  # a live writer keeps rows in the WAL
    writer.execute("PRAGMA journal_mode=WAL")
    writer.execute("INSERT INTO meta (key, value) VALUES ('k', 'v')")
    writer.commit()
    assert (path.parent / "store.sqlite-wal").stat().st_size > 0
    before = _fingerprint(path)
    wal_name = sorted(p.name for p in path.parent.iterdir())
    store_health.check(path)
    assert cli.main(["store-check", "--json"]) == 0
    assert _fingerprint(path) == before and sorted(p.name for p in path.parent.iterdir()) == wal_name
    writer.close()


def test_a_clean_store_with_no_wal_gets_no_side_files(path):
    names = sorted(p.name for p in path.parent.iterdir())
    assert "store.sqlite-wal" not in names  # closed cleanly: SQLite removed it
    store_health.check(path)
    assert sorted(p.name for p in path.parent.iterdir()) == names


def test_the_check_sees_rows_a_running_writer_has_only_in_the_wal(path):
    writer = sqlite3.connect(path)
    writer.execute("INSERT INTO meta (key, value) VALUES ('k', 'v')")
    writer.commit()
    assert store_health.check(path)["tables"]["meta"] == 2
    writer.close()


def test_the_store_is_opened_read_only(path, monkeypatch):
    seen = []
    real = sqlite3.connect

    def spy(database, *args, **kwargs):
        seen.append(str(database))
        return real(database, *args, **kwargs)

    monkeypatch.setattr(sqlite3, "connect", spy)
    store_health.check(path)
    assert seen and all(uri.startswith("file:") and "mode=ro" in uri for uri in seen)
    with real(seen[0], uri=True) as conn, pytest.raises(sqlite3.OperationalError, match="readonly"):
        conn.execute("CREATE TABLE t (x)")


def test_a_missing_store_is_a_clear_answer_not_a_traceback(home, capsys):
    result = store_health.check(config.store_path())
    assert result["exists"] is False and result["readable"] is False and result["healthy"] is False
    assert cli.main(["store-check"]) == 1
    out = capsys.readouterr().out
    assert "does not exist" in out and "verdict: NOT healthy" in out
    assert not (home / "store.sqlite").exists()  # the check did not create it


def test_a_wal_with_no_store_and_a_store_with_no_wal(home, path):
    lone = home / "other" / "store.sqlite"
    lone.parent.mkdir()
    (home / "other" / "store.sqlite-wal").write_bytes(b"w" * 10)
    result = store_health.check(lone)
    assert result["exists"] is False and result["wal_bytes"] == 10
    assert not lone.exists()


@pytest.mark.parametrize("content", [b"", b"this is not a database " * 200])
def test_an_empty_or_foreign_file_is_a_clear_answer(home, content, capsys):
    target = home / "store.sqlite"
    target.write_bytes(content)
    result = store_health.check(target)
    assert result["healthy"] is False
    assert cli.main(["store-check"]) == 1
    assert "verdict: NOT healthy" in capsys.readouterr().out
    assert target.read_bytes() == content
    if content:
        assert result["readable"] is False


def test_a_store_locked_by_a_writer_still_answers(path):
    writer = sqlite3.connect(path, isolation_level=None)
    writer.execute("BEGIN IMMEDIATE")
    writer.execute("INSERT INTO meta (key, value) VALUES ('k', 'v')")
    start = time.monotonic()
    result = store_health.check(path)
    assert result["readable"] is True and time.monotonic() - start < store_health.BUDGET_S + 1
    writer.execute("ROLLBACK")
    writer.close()


def test_the_check_stops_at_its_budget_and_names_what_did_not_finish(path, monkeypatch):
    with store.opened(path) as conn:
        for n in range(5, 4000):
            add_event(conn, n, "/work/x")
        conn.commit()
    monkeypatch.setattr(store_health, "BUDGET_S", 0.0)
    result = store_health.check(path)
    assert result["readable"] is True
    assert "integrity_check" in result["incomplete"] and result["integrity_check"] is None
    assert result["healthy"] is True  # unfinished is not a problem
    assert "not finished in time" in "\n".join(store_health.lines(result))


def test_a_large_store_gets_the_quick_check(path, monkeypatch):
    monkeypatch.setattr(store_health, "FULL_CHECK_MAX_BYTES", 1)
    assert store_health.check(path)["integrity_check"] == {"ok": True, "problems": 0, "mode": "quick"}


def test_the_memo_still_works_after_the_check(path):
    calls = []

    @memo.memoized
    def costly(conn, n):
        calls.append(n)
        return (n,)

    memo.reset()
    with store.opened(path) as conn:
        assert costly(conn, 1) == (1,) and costly(conn, 1) == (1,)
        store_health.check(path)
        assert costly(conn, 1) == (1,)
        assert calls == [1]  # the read-only check changed no data_version
    memo.reset()


def _seed_events(target, count: int) -> None:
    with store.opened(target) as conn:
        conn.executemany(
            "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,"
            " project_path, model, message_id, input_tokens, output_tokens)"
            " VALUES ('m', 'claude_code', '/x', 1, ?, 0, ?, ?, '/w/p', 'model-0', ?, 10, 5)",
            ((i, f"2026-01-01T00:{i // 60 % 60:02d}:{i % 60:02d}.000000Z", f"s{i % 500}", f"m{i}") for i in range(count)),
        )
        conn.commit()


def test_answers_within_two_seconds_on_a_larger_store(home):
    target = config.store_path()
    _seed_events(target, 100_000)
    start = time.monotonic()
    result = store_health.check(target)
    elapsed = time.monotonic() - start
    assert result["tables"]["events"] == 100_000 and elapsed < 2.0, (elapsed, result["incomplete"])


@pytest.mark.perf
def test_answers_within_two_seconds_on_a_store_of_the_owners_size(home):
    """1M events (about 360 MB): the quick check cannot always finish in the budget; the answer still arrives in < 2 s."""
    target = config.store_path()
    _seed_events(target, 1_000_000)
    timings = []
    for _ in range(3):
        start = time.monotonic()
        result = store_health.check(target)
        timings.append(round(time.monotonic() - start, 2))
    print(f"perf store-check over 1M events: {timings} s, incomplete {result['incomplete']}")
    assert result["tables"]["events"] == 1_000_000 and max(timings) < 2.0, timings
