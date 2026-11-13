"""HUN-244 (FR-091): the read-only store health check, `core/store_health.py` and `easy-personal store-check`."""

import hashlib
import json
import os
import shutil
import sqlite3
import threading
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
    assert result["healthy"] is None  # a cut-off check is never "healthy"
    text = "\n".join(store_health.lines(result))
    assert "not finished in time" in text and "verdict: not fully checked" in text
    assert "verdict: healthy" not in text and "NOT healthy" not in text


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


# ---- fix round (Stage 4) ------------------------------------------------------------------------------------


def _big(path, monkeypatch=None):
    with store.opened(path) as conn:
        for n in range(5, 4000):
            add_event(conn, n, "/work/x")
        conn.commit()


def test_a_cut_off_check_is_unknown_in_json_and_exits_3(path, monkeypatch, capsys):
    _big(path)
    monkeypatch.setattr(store_health, "BUDGET_S", 0.0)
    assert cli.main(["store-check", "--json"]) == 3
    data = json.loads(capsys.readouterr().out)
    assert data["healthy"] is None and "integrity_check" in data["incomplete"]
    assert cli.main(["store-check"]) == 3
    assert "verdict: not fully checked" in capsys.readouterr().out
    assert store_health.exit_code({"healthy": True}) == 0 and store_health.exit_code({"healthy": False}) == 1


def test_a_cut_off_table_count_is_named_not_hidden(path, monkeypatch):
    _big(path)

    def tables(conn):
        conn.execute("SELECT 1")
        raise sqlite3.OperationalError("interrupted")  # what the progress handler raises mid-count

    monkeypatch.setattr(store_health, "_tables", tables)
    result = store_health.check(path)
    assert result["tables"] is None and "tables" in result["incomplete"] and result["healthy"] is None
    assert "tables: not finished in time" in "\n".join(store_health.lines(result))


def test_a_found_problem_wins_over_an_unfinished_check(path, monkeypatch):
    _big(path)
    with sqlite3.connect(path) as conn:
        conn.execute(f"PRAGMA user_version = {LATEST - 1}")
    monkeypatch.setattr(store_health, "BUDGET_S", 0.0)
    result = store_health.check(path)
    assert result["incomplete"] and result["healthy"] is False


def test_full_check_on_a_clean_store_prints_progress_and_exits_0(path, monkeypatch, capsys):
    monkeypatch.setattr(store_health, "FULL_CHECK_MAX_BYTES", 1)  # the default run would use quick_check
    assert cli.main(["store-check", "--full", "--json"]) == 0
    captured = capsys.readouterr()
    assert captured.err.startswith("full check: this can take minutes")
    data = json.loads(captured.out)  # stdout stays clean JSON
    assert data["integrity_check"] == {"ok": True, "problems": 0, "mode": "full"} and data["healthy"] is True


def test_full_check_has_no_time_budget(path, monkeypatch):
    _big(path)
    monkeypatch.setattr(store_health, "BUDGET_S", 0.0)
    result = store_health.check(path, full=True)
    assert result["incomplete"] == [] and result["healthy"] is True
    assert result["integrity_check"]["mode"] == "full"


def test_full_check_on_a_small_corrupt_store_finds_the_problem(home, capsys):
    target = config.store_path()
    with store.opened(target) as conn:
        for n in range(3000):
            add_event(conn, n, f"/work/{CANARY}")
        conn.commit()
    with sqlite3.connect(target) as conn:
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    pages = os.path.getsize(target) // sqlite3.connect(target).execute("PRAGMA page_size").fetchone()[0]
    _corrupt_page(target, pages // 2)
    assert cli.main(["store-check", "--full"]) == 1
    out = capsys.readouterr()
    assert "problem(s)" in out.out and "verdict: NOT healthy" in out.out and CANARY not in out.out + out.err


def test_ctrl_c_stops_the_full_check_with_130_and_no_traceback(path, monkeypatch, capsys):
    def interrupted(*args, **kwargs):
        raise KeyboardInterrupt

    monkeypatch.setattr(store_health, "check", interrupted)
    assert cli.main(["store-check", "--full"]) == 130
    captured = capsys.readouterr()
    assert "Traceback" not in captured.err + captured.out and captured.out == ""


def test_cancel_stops_every_running_read(path):
    _big(path)
    cancel = threading.Event()
    cancel.set()
    result = store_health.check(path, full=True, cancel=cancel)
    assert "integrity_check" in result["incomplete"] and result["healthy"] is None


def test_a_wal_without_shm_is_read_from_a_private_copy_and_creates_no_file(path, tmp_path):
    writer = sqlite3.connect(path)  # a live writer keeps a row only in the WAL
    writer.execute("PRAGMA journal_mode=WAL")
    writer.execute("INSERT INTO meta (key, value) VALUES ('k', 'v')")
    writer.commit()
    beside = tmp_path / "copy"
    beside.mkdir()
    shutil.copyfile(path, beside / "store.sqlite")
    shutil.copyfile(f"{path}-wal", beside / "store.sqlite-wal")  # no -shm: a crashed or copied store
    names, before = sorted(p.name for p in beside.iterdir()), _fingerprint(beside / "store.sqlite")
    result = store_health.check(beside / "store.sqlite")
    assert result["readable"] is True and result["tables"]["meta"] == 2  # the WAL row is seen
    assert sorted(p.name for p in beside.iterdir()) == names == ["store.sqlite", "store.sqlite-wal"]
    assert _fingerprint(beside / "store.sqlite") == before
    writer.close()


def test_the_way_to_open_is_decided_once_before_the_threads_start(path, monkeypatch):
    calls = []
    real = store_health._target
    monkeypatch.setattr(store_health, "_target", lambda p: (calls.append(p), real(p))[1])
    store_health.check(path)
    assert len(calls) == 1


def test_a_symlinked_store_reads_the_wal_beside_its_target(path, tmp_path):
    writer = sqlite3.connect(path)
    writer.execute("INSERT INTO meta (key, value) VALUES ('k', 'v')")
    writer.commit()
    link_dir = tmp_path / "links"
    link_dir.mkdir()
    (link_dir / "store.sqlite").symlink_to(path)
    result = store_health.check(link_dir / "store.sqlite")
    assert result["wal_bytes"] > 0 and result["tables"]["meta"] == 2
    assert sorted(p.name for p in link_dir.iterdir()) == ["store.sqlite"]
    writer.close()


def test_text_from_the_store_is_capped_and_cleaned(path, capsys):
    with sqlite3.connect(path) as conn:
        conn.execute('CREATE TABLE "evil\x1b[31mtable\nline' + "x" * 200 + '" (a)')
        conn.execute("UPDATE events SET source = ?, ts_utc = ? WHERE byte_offset = 1",
                     ("bad\x1b]0;title\x07src\n" + "y" * 200, "2030\x1b[2J" + "z" * 200))
    result = store_health.check(path)
    keys = [*result["tables"], *result["sources"]]
    assert all(len(k) <= store_health.TEXT_LIMIT for k in keys)
    assert all(k.isprintable() for k in keys)
    assert all(len(s["last_ingest_utc"] or "") <= store_health.TEXT_LIMIT for s in result["sources"].values())
    assert cli.main(["store-check"]) in (0, 1)
    out = capsys.readouterr().out
    assert "\x1b" not in out and "\x07" not in out and len(out.splitlines()) == 8


def test_a_blob_time_is_unreadable_for_that_part_never_a_traceback(path, capsys):
    with sqlite3.connect(path) as conn:
        conn.execute("UPDATE events SET ts_utc = X'0102ff' WHERE byte_offset = 1")
    result = store_health.check(path)
    assert result["sources"] is None and "sources" in result["unreadable"] and result["healthy"] is False
    assert result["tables"]["events"] == 5  # the other parts are unaffected
    assert cli.main(["store-check", "--json"]) == 1
    assert json.loads(capsys.readouterr().out)["unreadable"] == ["sources"]
    assert cli.main(["store-check"]) == 1
    assert "sources: unreadable" in capsys.readouterr().out


def test_a_table_name_with_invalid_utf8_is_unreadable_never_a_traceback(path, capsys):
    with sqlite3.connect(path) as conn:
        conn.execute("CREATE TABLE badname (a)")
        conn.execute("PRAGMA writable_schema=ON")
        conn.execute("UPDATE sqlite_master SET name = CAST(X'c328ff' AS TEXT) WHERE name = 'badname'")
        conn.execute("PRAGMA writable_schema=OFF")
    result = store_health.check(path)
    assert result["tables"] is None and result["healthy"] is False  # SQLite itself refuses this schema
    assert cli.main(["store-check", "--json"]) == 1
    json.loads(capsys.readouterr().out)
    assert cli.main(["store-check"]) == 1
    assert "verdict: NOT healthy" in capsys.readouterr().out


def test_a_part_that_raises_a_decode_error_is_unreadable(path, monkeypatch):
    def tables(conn):
        b"\xc3\x28".decode("utf-8")  # what the sqlite3 module raises for a name that is not UTF-8

    monkeypatch.setattr(store_health, "_tables", tables)
    result = store_health.check(path)
    assert result["tables"] is None and result["unreadable"] == ["tables"] and result["healthy"] is False
    assert "tables: unreadable" in "\n".join(store_health.lines(result))


class _FakeConn:
    def __init__(self, rows):
        self.rows = rows

    def execute(self, sql):
        return self

    def fetchall(self):
        return self.rows


def test_the_problem_count_is_the_number_of_rows_sqlite_returned():
    two = [("row 1 missing from index i",), ("wrong # of entries in index i",)]
    assert store_health._verdict("integrity_check")(_FakeConn(two)) == {"ok": False, "problems": 2, "mode": "full"}
    assert store_health._verdict("quick_check")(_FakeConn(two + two))["problems"] == 4
    assert store_health._verdict("integrity_check")(_FakeConn([("ok",)]))["ok"] is True
    assert store_health._foreign_keys(_FakeConn([(1,), (2,), (3,)])) == {"ok": False, "problems": 3}


def test_a_locked_store_is_unreadable_locked_not_corruption(path, monkeypatch, capsys):
    def locked(*args, **kwargs):
        raise sqlite3.OperationalError("database is locked")

    monkeypatch.setattr(sqlite3, "connect", locked)
    result = store_health.check(path)
    assert result["locked"] is True and result["healthy"] is None and result["integrity_check"] is None
    assert cli.main(["store-check"]) == 3
    out = capsys.readouterr().out
    assert "unreadable (locked)" in out and "problem" not in out


def test_the_wait_for_a_lock_is_the_rest_of_the_budget(path):
    conn = store_health._open(f"{path.resolve().as_uri()}?mode=ro", time.monotonic() + 0.3, threading.Event())
    assert 0 <= conn.execute("PRAGMA busy_timeout").fetchone()[0] <= 300
    conn.close()
