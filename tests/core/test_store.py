import json
import re

from easy_personal.core import store
from easy_personal.sources import claude_code

# FR-002 metadata plus bookkeeping; no column may hold a raw record, message or tool content.
ALLOWED_EVENT_COLUMNS = {
    "id",
    "machine_id",
    "source",
    "source_path",
    "inode",
    "byte_offset",
    "abstained",
    "record_type",
    "version",
    "ts_utc",
    "session_id",
    "project_path",
    "git_branch",
    "is_sidechain",
    "is_api_error",
    "model",
    "message_id",
    "tool_names",
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
}
FORBIDDEN_MARKERS = ("sk-FAKE-0000000000", "INVENTED_", "planted-secret", "totalCostUSD", "0.01")


def _columns(conn, table):
    return {row[1] for row in conn.execute(f"PRAGMA table_info({table})")}


def test_events_schema_is_explicit_metadata_only(conn):
    assert _columns(conn, "events") == ALLOWED_EVENT_COLUMNS
    assert {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")} == {
        "meta",
        "source_files",
        "events",
        "kit_trace_events",
        "context_index",
        "ui_sessions",
    }


def test_store_uses_wal(conn):
    assert conn.execute("PRAGMA journal_mode").fetchone()[0] == "wal"


def test_planted_secret_and_text_absent_from_dump(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root)
    dump = "\n".join(conn.iterdump())
    for marker in FORBIDDEN_MARKERS:
        assert marker not in dump, marker


def test_every_row_has_machine_id_and_utc_time(conn, fixtures_root, tmp_path):
    claude_code.ingest(conn, fixtures_root)
    mid = store.machine_id(conn)
    assert re.fullmatch(r"[0-9a-f-]{36}", mid)
    for table in ("events", "source_files"):
        assert (
            conn.execute(f"SELECT COUNT(*) FROM {table} WHERE machine_id != ?", (mid,)).fetchone()[
                0
            ]
            == 0
        )
    times = [r[0] for r in conn.execute("SELECT ts_utc FROM events WHERE ts_utc IS NOT NULL")]
    assert times and all(re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{6}Z", t) for t in times)


def test_machine_id_is_stable_across_reopen(tmp_path):
    path = tmp_path / "store.sqlite"
    first = store.connect(path)
    mid = store.machine_id(first)
    first.close()
    again = store.connect(path)
    assert store.machine_id(again) == mid
    again.close()


def test_non_utc_offset_is_stored_as_utc(conn, tmp_path):
    root = tmp_path / "projects"
    root.mkdir()
    record = {
        "type": "user",
        "version": "2.1.295",
        "cwd": "/work/demo-app",
        "sessionId": "s-1",
        "timestamp": "2026-01-16T01:30:00+07:00",
        "message": {"content": "INVENTED_TEXT"},
    }
    (root / "s.jsonl").write_text(json.dumps(record) + "\n")
    claude_code.ingest(conn, root)
    assert conn.execute("SELECT ts_utc FROM events").fetchone()[0] == "2026-01-15T18:30:00.000000Z"
