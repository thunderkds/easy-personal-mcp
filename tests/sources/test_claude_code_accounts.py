"""Ingest over every confirmed Claude agent account, events tagged by account (HUN-63)."""

import json

import pytest

from easy_personal.core import accounts, config, metrics, store
from easy_personal.sources import claude_code


def _line(msg: str, minute: int, session: str = "s-1") -> bytes:
    record = {
        "type": "assistant", "version": "2.1.295", "cwd": "/work/demo", "sessionId": session,
        "timestamp": f"2026-01-16T10:{minute:02d}:00.000Z",
        "message": {"id": msg, "model": "m-1", "content": [], "usage": {
            "input_tokens": 1, "output_tokens": 1, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0}},
    }  # fmt: skip
    return (json.dumps(record) + "\n").encode()


def _write(folder, name: str, *messages: str, start: int = 0) -> None:
    projects = folder / "projects" / "p"
    projects.mkdir(parents=True, exist_ok=True)
    (projects / name).write_bytes(b"".join(_line(m, start + i) for i, m in enumerate(messages)))


@pytest.fixture
def root():
    return config.agent_root().resolve()


@pytest.fixture
def fleet(conn, root):
    """Default account (`.claude`) and a confirmed `Claude work` account, one file each."""
    _write(root / ".claude", "a.jsonl", "d1", "d2")
    _write(root / ".claude-work", "b.jsonl", "w1", "w2", "w3")
    accounts.detect(conn)
    accounts.confirm(conn, str(root / ".claude-work"), "Claude work")
    return conn


def tags(conn) -> dict[str, int]:
    return dict(conn.execute(
        "SELECT a.label, COUNT(*) FROM events e JOIN agent_accounts a ON a.id = e.agent_account_id GROUP BY 1"
    ))  # fmt: skip


def test_every_confirmed_account_is_ingested_and_tagged(fleet):
    assert claude_code.ingest_accounts(fleet)["new_events"] == 5
    assert tags(fleet) == {"Claude default": 2, "Claude work": 3}
    assert metrics.usage_summary(fleet)["messages"] == 5
    assert metrics.usage_summary(fleet, account="Claude work")["messages"] == 3
    assert metrics.usage_summary(fleet, account="Claude default")["messages"] == 2
    assert claude_code.ingest_accounts(fleet)["new_events"] == 0


def test_detected_ignored_and_codex_accounts_are_skipped(conn, root):
    _write(root / ".claude", "a.jsonl", "d1")
    _write(root / ".claude-new", "n.jsonl", "n1")  # detected, never confirmed
    _write(root / ".claude-off", "o.jsonl", "o1")
    (root / ".codex").mkdir()
    (root / ".codex" / "config.toml").write_text("")
    accounts.detect(conn)
    accounts.ignore(conn, str(root / ".claude-off"))
    claude_code.ingest_accounts(conn)
    assert tags(conn) == {"Claude default": 1}


def test_account_confirmed_later_is_read_from_offset_zero(fleet):
    _write(config.agent_root().resolve() / ".claude-late", "l.jsonl", "l1", "l2")
    accounts.detect(fleet)
    claude_code.ingest_accounts(fleet)
    accounts.confirm(fleet, str(config.agent_root().resolve() / ".claude-late"), "Late")
    assert claude_code.ingest_accounts(fleet)["new_events"] == 2
    assert tags(fleet)["Late"] == 2


def test_env_override_replaces_the_default_accounts_projects_folder(fleet, tmp_path, monkeypatch):
    _write(tmp_path, "x.jsonl", "o1")  # tmp_path/projects/p/x.jsonl
    monkeypatch.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(tmp_path / "projects"))
    claude_code.ingest_accounts(fleet)
    assert tags(fleet) == {"Claude default": 1, "Claude work": 3}  # default's own folder is not read


def test_override_without_a_default_row_stores_untagged_rows(conn, tmp_path):
    _write(tmp_path, "x.jsonl", "o1")
    claude_code.ingest_accounts(conn, override=tmp_path / "projects")
    assert conn.execute("SELECT COUNT(*), COUNT(agent_account_id) FROM events").fetchone() == (1, 0)


def test_shared_message_counts_once_in_totals_and_in_each_account(conn, root):
    _write(root / ".claude", "a.jsonl", "d1", "shared")
    _write(root / ".claude-work", "b.jsonl", "w1", "shared")  # same message id in two accounts
    accounts.detect(conn)
    accounts.confirm(conn, str(root / ".claude-work"), "Claude work")
    claude_code.ingest_accounts(conn)
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == 4  # both rows kept
    assert metrics.usage_summary(conn)["messages"] == 3
    assert metrics.usage_summary(conn)["input_tokens"] == 3
    assert metrics.usage_breakdown(conn, "model")["total"]["messages"] == 3
    assert metrics.usage_summary(conn, account="Claude default")["messages"] == 2
    assert metrics.usage_summary(conn, account="Claude work")["messages"] == 2


def test_missing_account_is_skipped_and_keeps_rows_and_offsets(fleet, root):
    claude_code.ingest_accounts(fleet)
    offsets = fleet.execute("SELECT COUNT(*) FROM source_files").fetchone()[0]
    import shutil

    shutil.rmtree(root / ".claude-work")
    assert claude_code.ingest_accounts(fleet)["new_events"] == 0
    assert tags(fleet) == {"Claude default": 2, "Claude work": 3}
    assert fleet.execute("SELECT COUNT(*) FROM source_files").fetchone()[0] == offsets


def test_deleted_transcript_still_retires_its_offset_only(fleet, root):
    claude_code.ingest_accounts(fleet)
    (root / ".claude-work" / "projects" / "p" / "b.jsonl").unlink()
    claude_code.ingest_accounts(fleet)
    paths = [p for (p,) in fleet.execute("SELECT path FROM source_files")]
    assert len(paths) == 1 and ".claude/" in paths[0]
    assert tags(fleet)["Claude work"] == 3


def test_legacy_untagged_rows_are_tagged_with_the_default_account(fleet):
    claude_code.ingest_accounts(fleet)
    fleet.execute("UPDATE events SET agent_account_id = NULL")
    fleet.commit()
    claude_code.ingest_accounts(fleet)
    assert fleet.execute("SELECT COUNT(*) FROM events WHERE agent_account_id IS NULL").fetchone()[0] == 0
    assert tags(fleet) == {"Claude default": 5}  # NULL meant default; the next ingest says so


def test_killed_ingest_resumes_to_the_same_totals(fleet, root, monkeypatch, tmp_path):
    for i in range(6):
        _write(root / ".claude-work", f"more{i}.jsonl", *(f"k{i}-{j}" for j in range(4)), start=10)
    clean = store.connect(tmp_path / "clean.sqlite")
    accounts.detect(clean)
    accounts.confirm(clean, str(root / ".claude-work"), "Claude work")
    claude_code.ingest_accounts(clean)

    real, calls = store.save_file, []

    def die_after_three(*args, **kwargs):
        if len(calls) == 3:
            raise KeyboardInterrupt
        calls.append(args[2])
        return real(*args, **kwargs)

    monkeypatch.setattr(store, "save_file", die_after_three)
    with pytest.raises(KeyboardInterrupt):
        claude_code.ingest_accounts(fleet)
    monkeypatch.setattr(store, "save_file", real)
    assert fleet.execute("SELECT COUNT(*) FROM source_files").fetchone()[0] == 3  # committed per file
    claude_code.ingest_accounts(fleet)

    def snapshot(c):
        return (
            metrics.usage_summary(c), tags(c),
            c.execute("SELECT COUNT(*) FROM events").fetchone()[0],
            sorted(c.execute("SELECT path, byte_offset FROM source_files")),
        )  # fmt: skip

    resumed, expected = snapshot(fleet), snapshot(clean)
    assert resumed == expected
