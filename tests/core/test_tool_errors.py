"""Migration 8 (events.tool_error_count) and its fill on the next ingest (HUN-192, FR-059)."""

import sqlite3

import pytest

from easy_personal.core import store
from easy_personal.core.migrations import MIGRATIONS
from easy_personal.sources import claude_code
from tests_helpers import rec, write


def _user(blocks, n, session="s"):
    record = rec(n, session, "user")
    record["message"] = {"role": "user", "content": blocks}
    return record


def _result(failed):
    return {"type": "tool_result", "tool_use_id": "toolu_x", "content": "INVENTED_text", "is_error": failed}


def _records():
    return [
        rec(1, "s"),
        _user([_result(True), _result(True), _result(False)], 2),
        _user([_result(False)], 3),
        _user("a typed prompt", 4),
        rec(5, "s"),
    ]


def _v7_store(tmp_path, root):
    """A store as m0007 left it: transcripts ingested, `tool_error_count` does not exist yet."""
    path = tmp_path / "store.sqlite"
    conn = store.connect(path)
    claude_code.ingest(conn, root)
    conn.execute("INSERT INTO source_files VALUES ('m', 'kit_trace', '/kt.jsonl', 1, 77)")
    conn.commit()
    rows = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    conn.close()
    raw = sqlite3.connect(path)
    raw.execute("ALTER TABLE events DROP COLUMN tool_error_count")
    raw.execute("PRAGMA user_version = 7")
    raw.commit()
    raw.close()
    return path, rows


def _counts(conn):
    return conn.execute("SELECT tool_error_count FROM events WHERE record_type = 'user' ORDER BY byte_offset").fetchall()


def test_migration_8_is_registered():
    assert 8 in [n for n, _ in MIGRATIONS]


def test_v7_store_upgrades_resets_claude_offsets_and_keeps_the_backup(tmp_path):
    root = write(tmp_path / "projects", "a.jsonl", _records())
    path, rows = _v7_store(tmp_path, root)
    conn = store.connect(path)
    columns = {r[1]: r for r in conn.execute("PRAGMA table_info(events)")}
    assert "tool_error_count" in columns and columns["tool_error_count"][3] == 0  # nullable
    assert conn.execute("PRAGMA user_version").fetchone()[0] == max(n for n, _ in MIGRATIONS)
    assert list(tmp_path.glob("*.bak-v7"))
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == rows > 0
    assert conn.execute("SELECT COUNT(*) FROM events WHERE tool_error_count IS NOT NULL").fetchone()[0] == 0
    assert conn.execute("SELECT byte_offset FROM source_files WHERE source = 'claude_code'").fetchall() == [(0,)]
    assert conn.execute("SELECT byte_offset FROM source_files WHERE source = 'kit_trace'").fetchone() == (77,)


def test_next_ingest_fills_the_existing_rows_without_adding_any(tmp_path):
    root = write(tmp_path / "projects", "a.jsonl", _records())
    path, rows = _v7_store(tmp_path, root)
    conn = store.connect(path)
    assert claude_code.ingest(conn, root)["new_events"] == 0
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == rows
    assert _counts(conn) == [(2,), (0,), (None,)]
    # assistant rows stay unknown
    assert conn.execute("SELECT COUNT(*) FROM events WHERE record_type = 'assistant' AND tool_error_count IS NOT NULL").fetchone()[0] == 0


def test_second_ingest_and_a_repeated_offset_reset_change_nothing(tmp_path):
    root = write(tmp_path / "projects", "a.jsonl", _records())
    path, rows = _v7_store(tmp_path, root)
    conn = store.connect(path)
    claude_code.ingest(conn, root)
    first = conn.execute("SELECT id, tool_error_count, entrypoint FROM events ORDER BY id").fetchall()
    conn.execute("UPDATE source_files SET byte_offset = 0")
    conn.commit()
    assert claude_code.ingest(conn, root)["new_events"] == 0
    assert conn.execute("SELECT id, tool_error_count, entrypoint FROM events ORDER BY id").fetchall() == first
    assert len(first) == rows


def test_a_file_deleted_before_the_re_read_keeps_unknown_never_zero(tmp_path):
    root = write(tmp_path / "projects", "a.jsonl", _records())
    write(root, "b.jsonl", [_user([_result(True)], 1, "t")])
    path, rows = _v7_store(tmp_path, root)
    (root / "b.jsonl").unlink()
    conn = store.connect(path)
    claude_code.ingest(conn, root)
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == rows
    assert conn.execute("SELECT tool_error_count FROM events WHERE session_id = 't'").fetchall() == [(None,)]
    assert conn.execute(
        "SELECT tool_error_count FROM events WHERE session_id = 's' AND record_type = 'user' ORDER BY byte_offset"
    ).fetchall() == [(2,), (0,), (None,)]


def test_a_re_read_never_turns_a_known_count_back_to_null(conn, tmp_path):
    root = write(tmp_path / "projects", "a.jsonl", _records())
    claude_code.ingest(conn, root)
    before = _counts(conn)
    conn.execute("UPDATE source_files SET byte_offset = 0")
    conn.commit()
    claude_code.ingest(conn, root)
    assert _counts(conn) == before == [(2,), (0,), (None,)]


def test_migration_8_is_idempotent(conn):
    from easy_personal.core.migrations import m0008_tool_errors as m

    conn.execute("BEGIN")
    m.apply(conn)
    m.apply(conn)
    names = [r[1] for r in conn.execute("PRAGMA table_info(events)")]
    conn.execute("ROLLBACK")
    assert names.count("tool_error_count") == 1


def test_store_already_at_the_latest_version_is_left_alone(tmp_path):
    path = tmp_path / "store.sqlite"
    store.connect(path).close()
    store.connect(path).close()
    assert not list(tmp_path.glob("*.bak-v*"))
    assert sqlite3.connect(path).execute("PRAGMA user_version").fetchone()[0] == max(n for n, _ in MIGRATIONS)


def test_an_older_program_refuses_a_v8_store(tmp_path):
    path = tmp_path / "store.sqlite"
    store.connect(path).close()
    older = [m for m in MIGRATIONS if m[0] <= 7]
    conn = sqlite3.connect(path)
    with pytest.raises(RuntimeError, match="newer than this program"):
        store.migrate(conn, path, older)
