"""The `entrypoint` field of a transcript record (HUN-161): metadata only, one value per event."""

import pytest

from easy_personal.core import store
from easy_personal.sources import claude_code
from tests_helpers import TOKENS, rec, write


def _parse(record):
    return claude_code.parse_record(record, lambda cwd: cwd)


def test_entrypoint_is_read_from_user_assistant_and_system_records():
    for kind in ("user", "assistant", "system"):
        assert _parse(rec(1, "s", kind, entry="sdk-cli"))["entrypoint"] == "sdk-cli", kind


def test_record_without_entrypoint_stays_null():
    assert _parse(rec(1, "s", entry=None))["entrypoint"] is None


@pytest.mark.parametrize("value", ["has spaces and text", "x" * 300, 5, ["cli"], {"a": 1}, ""])
def test_a_value_that_is_not_an_identifier_is_dropped(value):
    assert _parse(rec(1, "s", entry=value))["entrypoint"] is None


def test_interrupted_backfill_resumes_without_duplicates(conn, tmp_path, monkeypatch):
    root = tmp_path / "p"
    for name, session, entry in (("a.jsonl", "sa", "cli"), ("b.jsonl", "sb", "sdk-cli"), ("c.jsonl", "sc", "cli")):
        write(root, name, [rec(1, session, entry=entry), rec(2, session, entry=entry)])
    claude_code.ingest(conn, root)
    rows = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    conn.execute("UPDATE events SET entrypoint = NULL")  # as if ingested before migration 7
    conn.execute("UPDATE source_files SET byte_offset = 0")
    conn.commit()

    real, calls = store.save_file, []

    def die_after_one(*args, **kwargs):
        calls.append(1)
        if len(calls) == 2:
            raise RuntimeError("killed")
        return real(*args, **kwargs)

    monkeypatch.setattr(store, "save_file", die_after_one)
    with pytest.raises(RuntimeError):
        claude_code.ingest(conn, root)
    monkeypatch.setattr(store, "save_file", real)
    assert claude_code.ingest(conn, root)["new_events"] == 0
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == rows
    assert conn.execute("SELECT COUNT(*) FROM events WHERE entrypoint IS NULL AND record_type = 'assistant'").fetchone()[0] == 0
    assert conn.execute("SELECT SUM(output_tokens + input_tokens) FROM events").fetchone()[0] == 6 * TOKENS


def test_stored_entrypoint_is_never_free_text(conn, tmp_path):
    claude_code.ingest(conn, write(tmp_path / "p", "a.jsonl", [rec(1, "s", entry="planted secret sk-FAKE")]))
    assert conn.execute("SELECT COUNT(*) FROM events WHERE entrypoint IS NOT NULL").fetchone()[0] == 0
