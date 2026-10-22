import json
import os

import pytest

from easy_personal.core import config, kit
from easy_personal.sources import verifier

CANARY = "LEAKCANARY-verifier-0123456789"
NAME = "evidence-report-project-20261021T124041-825600Z"


def _report(overall=None, **extra):
    data = {
        "schema_version": 1, "scope": "project", "generated_at": "2026-10-21T12:40:41Z",
        "overall": overall or {"value": 48, "abstained": False, "reason_code": None},
        "dimensions": [{"dimension": "api", "rating": 0, "abstained": False, "reason_code": CANARY}],
        "evidence": CANARY,
    }  # fmt: skip
    data.update(extra)
    return data


def _write(folder, data, name=NAME + ".json"):
    folder.mkdir(parents=True, exist_ok=True)
    (folder / name).write_text(data if isinstance(data, str) else json.dumps(data))


@pytest.fixture
def repo(conn, tmp_path):
    path = tmp_path / "work" / "proj"
    path.mkdir(parents=True)
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, "
        "project_path) VALUES ('m', 'claude_code', 's', 1, 0, 0, ?)", (str(path),)
    )  # fmt: skip
    return path


@pytest.fixture
def shared(tmp_path):
    path = tmp_path / "shared"
    path.mkdir()
    return path


def _part(conn, repo):
    return kit.kit_summary(conn)["projects"][0]["parts"]["verifier"]


# AC1
def test_repo_local_score_is_stored_and_shown(conn, repo, shared):
    _write(repo / "reports", _report())
    assert verifier.ingest(conn, shared) == {"status": "ok", "projects": 1, "skipped": 0}
    assert _part(conn, repo) == {
        "status": "ok", "scope": "project", "generated_at": "2026-10-21T12:40:41.000000Z",
        "overall": {"value": 48, "abstained": False, "reason_code": None},
    }  # fmt: skip
    assert CANARY not in "\n".join(conn.iterdump())


def test_shared_container_by_folder_name_and_newest_wins(conn, repo, shared):
    _write(shared / "proj", _report({"value": 10, "abstained": False, "reason_code": None}))
    newer = _report({"value": 70, "abstained": False, "reason_code": None},
                    generated_at="2026-10-22T00:00:00Z")  # fmt: skip
    _write(shared / "proj", newer, "evidence-report-project-20261022T000000-000000Z.json")
    _write(repo / "reports", _report())  # same report location twice: still one row
    verifier.ingest(conn, shared)
    assert _part(conn, repo)["overall"]["value"] == 70
    assert conn.execute("SELECT COUNT(*) FROM verifier_scores").fetchone()[0] == 1


def test_abstained_overall_has_null_value(conn, repo, shared):
    _write(repo / "reports", _report({"value": None, "abstained": True, "reason_code": "no_data"}))
    verifier.ingest(conn, shared)
    assert _part(conn, repo)["overall"] == {"value": None, "abstained": True, "reason_code": "no_data"}


# AC2
def test_html_only_is_no_data(conn, repo, shared):
    (repo / "reports").mkdir()
    (repo / "reports" / (NAME + ".html")).write_text(f"<html>{CANARY}</html>")
    assert verifier.ingest(conn, shared)["status"] == "no data"
    assert _part(conn, repo) == {"status": "no data"}


def test_worktree_named_folder_is_ignored(conn, repo, shared):
    _write(shared / "proj-HUN-42", _report())
    verifier.ingest(conn, shared)
    assert _part(conn, repo) == {"status": "no data"}


# AC3
@pytest.mark.parametrize(
    "data",
    [
        _report(schema_version=2),
        _report(schema_version=True),
        _report(schema_version="1"),
        {k: v for k, v in _report().items() if k != "overall"},
        _report(scope=5),
        _report(generated_at="not a date"),
        _report({"value": 48, "abstained": True, "reason_code": None}),
        _report({"value": None, "abstained": False, "reason_code": None}),
        _report({"value": True, "abstained": False, "reason_code": None}),
        _report({"value": 400, "abstained": False, "reason_code": None}),
        _report({"value": 5, "abstained": "no", "reason_code": None}),
        "{not json",
        "[1, 2]",
        "[" * 100000,
    ],
)
def test_bad_or_unknown_schema_abstains(conn, repo, shared, data):
    _write(repo / "reports", data)
    result = verifier.ingest(conn, shared)
    assert result["skipped"] == 1
    assert _part(conn, repo) == {"status": "no data"}


def test_bad_file_does_not_hide_a_good_one(conn, repo, shared):
    _write(repo / "reports", _report())
    _write(repo / "reports", "{oops", "evidence-report-project-99.json")
    assert verifier.ingest(conn, shared)["skipped"] == 1
    assert _part(conn, repo)["status"] == "ok"


def test_oversized_file_is_skipped(conn, repo, shared):
    _write(repo / "reports", _report(pad="x" * verifier.MAX_BYTES))
    assert verifier.ingest(conn, shared)["skipped"] == 1
    assert _part(conn, repo) == {"status": "no data"}


def test_symlink_and_fifo_are_skipped(conn, repo, shared, tmp_path):
    outside = tmp_path / "outside.json"
    outside.write_text(json.dumps(_report()))
    reports = repo / "reports"
    reports.mkdir()
    (reports / "evidence-report-project-1.json").symlink_to(outside)
    os.mkfifo(reports / "evidence-report-project-2.json")
    assert verifier.ingest(conn, shared)["skipped"] == 2
    assert _part(conn, repo) == {"status": "no data"}


def test_symlinked_project_dir_is_not_followed(conn, repo, shared, tmp_path):
    _write(tmp_path / "elsewhere", _report())
    (shared / "proj").symlink_to(tmp_path / "elsewhere")
    verifier.ingest(conn, shared)
    assert _part(conn, repo) == {"status": "no data"}


def test_name_collision_ignores_shared_dir(conn, repo, shared, tmp_path):
    other = tmp_path / "other" / "proj"
    other.mkdir(parents=True)
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, "
        "project_path) VALUES ('m', 'claude_code', 's2', 2, 0, 0, ?)", (str(other),)
    )  # fmt: skip
    _write(shared / "proj", _report())
    assert verifier.ingest(conn, shared)["projects"] == 0
    _write(other / "reports", _report())  # repo-local is unambiguous
    assert verifier.ingest(conn, shared)["projects"] == 1


def test_missing_container_and_report_removed_is_no_data(conn, repo, tmp_path):
    verifier.ingest(conn, tmp_path / "gone")
    assert _part(conn, repo) == {"status": "no data"}
    _write(repo / "reports", _report())
    verifier.ingest(conn, tmp_path / "gone")
    assert _part(conn, repo)["status"] == "ok"
    (repo / "reports" / (NAME + ".json")).unlink()
    verifier.ingest(conn, tmp_path / "gone")
    assert _part(conn, repo) == {"status": "no data"}


def test_source_files_are_not_modified(conn, repo, shared):
    _write(repo / "reports", _report())
    path = repo / "reports" / (NAME + ".json")
    before = (path.read_bytes(), path.stat().st_mtime_ns)
    verifier.ingest(conn, shared)
    assert (path.read_bytes(), path.stat().st_mtime_ns) == before
    assert sorted(p.name for p in (repo / "reports").iterdir()) == [NAME + ".json"]


def test_table_holds_only_allowed_columns(conn):
    cols = [r[1] for r in conn.execute("PRAGMA table_info(verifier_scores)")]
    assert cols == ["project_path", "scope", "generated_at_utc", "overall_value",
                    "overall_abstained", "reason_code"]  # fmt: skip


# config: env, then config.yaml key, then default
@pytest.fixture
def _home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    monkeypatch.delenv(config.VERIFIER_REPORTS_ENV, raising=False)
    (tmp_path / "home").mkdir()


def test_default_dir(tmp_path, _home, monkeypatch):
    monkeypatch.setenv("HOME", str(tmp_path))
    assert config.verifier_reports_dir() == tmp_path / ".easy-verifier-reports"


def test_config_key_and_env_override(tmp_path, _home, monkeypatch):
    (tmp_path / "home" / "config.yaml").write_text("verifier:\n  reports_dir: ~/vr\n")
    monkeypatch.setenv("HOME", str(tmp_path))
    assert config.verifier_reports_dir() == tmp_path / "vr"
    monkeypatch.setenv(config.VERIFIER_REPORTS_ENV, str(tmp_path / "env"))
    assert config.verifier_reports_dir() == tmp_path / "env"


def test_ingest_uses_configured_dir(conn, repo, shared, monkeypatch):
    _write(shared / "proj", _report())
    monkeypatch.setenv(config.VERIFIER_REPORTS_ENV, str(shared))
    verifier.ingest(conn)
    assert _part(conn, repo)["status"] == "ok"
