import json

import pytest

from easy_personal.core import config, kit
from easy_personal.sources import ui

CANARY = "LEAKCANARY-ui-0123456789"


def _session(folder, name, sid, status="passed", started="2026-10-01T10:00:00Z",
             ended="2026-10-01T10:05:00Z", actions=3, **extra):  # fmt: skip
    data = {"id": sid, "status": status, "startedAt": started, "endedAt": ended,
            "target": CANARY, "issues": [{"note": CANARY}],
            "actions": [{"step": CANARY, "n": i} for i in range(actions)], **extra}  # fmt: skip
    (folder / name).write_text(json.dumps(data))


@pytest.fixture
def folder(tmp_path):
    path = tmp_path / "reports"
    path.mkdir()
    return path


def _rows(conn):
    return conn.execute("SELECT * FROM ui_sessions ORDER BY id").fetchall()


# AC1: only the five allowed fields are stored
def test_stores_only_allowed_fields(conn, folder):
    _session(folder, "session-a.json", "a1", actions=3)
    ui.ingest(conn, folder)
    assert _rows(conn) == [
        ("a1", "passed", "2026-10-01T10:00:00.000000Z", "2026-10-01T10:05:00.000000Z", 3)
    ]
    assert CANARY not in "\n".join(conn.iterdump())


def test_summary_counts_and_times(conn, folder):
    _session(folder, "session-a.json", "a1", "passed", "2026-10-01T10:00:00Z", None, 2)
    _session(folder, "session-b.json", "b1", "failed", "2026-10-03T10:00:00Z", None, 4)
    _session(folder, "session-c.json", "c1", "running", "2026-10-02T10:00:00Z", None, 1)
    ui.ingest(conn, folder)
    assert kit.kit_summary(conn)["ui"] == {
        "status": "ok", "sessions": 3, "passed": 1, "failed": 1, "other": 1, "actions": 7,
        "first_utc": "2026-10-01T10:00:00.000000Z", "last_utc": "2026-10-03T10:00:00.000000Z",
    }  # fmt: skip


def test_ignores_other_files_and_bad_files(conn, folder):
    _session(folder, "session-a.json", "a1")
    _session(folder, "other.json", "x1")
    (folder / "session-bad.json").write_text("{not json " + CANARY)
    (folder / "session-list.json").write_text("[1]")
    _session(folder, "session-nokey.json", "n1", status=None)
    _session(folder, "session-badtime.json", "t1", started="yesterday")
    _session(folder, "session-noactions.json", "m1")
    data = json.loads((folder / "session-noactions.json").read_text())
    del data["actions"]
    (folder / "session-noactions.json").write_text(json.dumps(data))
    result = ui.ingest(conn, folder)
    assert [r[0] for r in _rows(conn)] == ["a1"]
    assert result["skipped"] == 5


def test_huge_file_is_skipped(conn, folder, monkeypatch):
    monkeypatch.setattr(ui, "MAX_BYTES", 100)
    _session(folder, "session-a.json", "a1", actions=50)
    ui.ingest(conn, folder)
    assert _rows(conn) == []


# edge: the same session copied under two names -> one row; re-ingest is idempotent
def test_dedupes_by_id(conn, folder):
    _session(folder, "session-a.json", "a1")
    _session(folder, "session-a-copy.json", "a1")
    ui.ingest(conn, folder)
    ui.ingest(conn, folder)
    assert len(_rows(conn)) == 1


def test_changed_session_is_updated(conn, folder):
    _session(folder, "session-a.json", "a1", "running", ended=None, actions=1)
    ui.ingest(conn, folder)
    _session(folder, "session-a.json", "a1", "passed", actions=5)
    ui.ingest(conn, folder)
    assert [(r[1], r[4]) for r in _rows(conn)] == [("passed", 5)]


# AC2: folder missing / unset / empty -> no data
@pytest.mark.parametrize("which", ["unset", "missing", "empty"])
def test_no_folder_is_no_data(conn, folder, which):
    ui.ingest(conn, {"unset": None, "missing": folder / "nope", "empty": folder}[which])
    assert kit.kit_summary(conn)["ui"] == {"status": "no data"}


def test_folder_removed_after_data_is_no_data(conn, folder):
    _session(folder, "session-a.json", "a1")
    ui.ingest(conn, folder)
    ui.ingest(conn, folder / "gone")
    assert kit.kit_summary(conn)["ui"] == {"status": "no data"}


# AC3: folder path configurable
def test_folder_from_env(conn, folder, monkeypatch):
    monkeypatch.delenv(config.UI_REPORTS_ENV, raising=False)
    assert config.ui_reports_dir() is None
    monkeypatch.setenv(config.UI_REPORTS_ENV, str(folder))
    assert config.ui_reports_dir() == folder
    _session(folder, "session-a.json", "a1")
    ui.ingest(conn)
    assert kit.kit_summary(conn)["ui"]["sessions"] == 1


def test_ui_is_top_level_not_per_project(conn):
    assert set(kit.kit_summary(conn)) == {"projects", "ui"}


def test_table_holds_only_the_five_fields(conn):
    cols = [r[1] for r in conn.execute("PRAGMA table_info(ui_sessions)")]
    assert cols == ["id", "status", "started_at_utc", "ended_at_utc", "action_count"]
