import json
from datetime import UTC, datetime, timedelta

import pytest

from easy_personal.core import kit, rules, store
from easy_personal.sources import claude_code, kit_trace

NOW = datetime(2026, 10, 21, 12, 0, tzinfo=UTC)
CANARY = "LEAKCANARY-summary-0123456789"


def _ts(delta=timedelta(hours=1)):
    return (NOW - delta).isoformat()


def _line(tool="Bash", error=False, ts=None, **extra):
    return json.dumps({"timestamp": ts or _ts(), "tool_name": tool, "summary": CANARY,
                       "is_error": error, **extra}) + "\n"  # fmt: skip


@pytest.fixture
def repo(tmp_path, conn):
    """A project folder known to the store (via one usage event) with an event-trace folder."""
    path = tmp_path / "demo"
    (path / "memory" / "event-trace").mkdir(parents=True)
    root = tmp_path / "projects"
    root.mkdir()
    (root / "s.jsonl").write_text(
        json.dumps({"type": "user", "version": "1", "cwd": str(path), "sessionId": "s1",
                    "timestamp": _ts(timedelta(days=15))}) + "\n"
    )  # fmt: skip
    claude_code.ingest(conn, root)
    return path


def _trace(repo, name, *lines):
    (repo / "memory" / "event-trace" / f"{name}.jsonl").write_text("".join(lines))


def _rows(conn):
    return conn.execute(
        "SELECT kit_task_id, abstained, ts_utc, tool_name, is_error FROM kit_trace_events "
        "ORDER BY id"
    ).fetchall()


def test_only_four_fields_are_stored_and_canary_is_absent(conn, repo, tmp_path):
    _trace(repo, "T001", _line("Edit", True), _line("Bash"))
    kit_trace.ingest(conn)
    assert [c[1] for c in conn.execute("PRAGMA table_info(kit_trace_events)")] == [
        "id", "machine_id", "source_path", "inode", "byte_offset", "project_path",
        "kit_task_id", "abstained", "ts_utc", "tool_name", "is_error",
    ]  # fmt: skip
    assert CANARY not in "\n".join(conn.iterdump())
    assert [r[0] for r in _rows(conn)] == ["T001", "T001"]


def test_untagged_rows_have_no_kit_task_id(conn, repo):
    _trace(repo, "_untagged", _line())
    kit_trace.ingest(conn)
    assert _rows(conn)[0][0] is None


def test_malformed_and_unreadable_lines_abstain(conn, repo):
    _trace(repo, "T002", "{not json\n", json.dumps({"tool_name": "Bash"}) + "\n", _line(), "[]\n")
    kit_trace.ingest(conn)
    assert [r[1] for r in _rows(conn)] == [1, 1, 0, 1]
    assert _rows(conn)[0][3] is None


def test_incremental_per_file_and_partial_last_line(conn, repo):
    path = repo / "memory" / "event-trace" / "T001.jsonl"
    path.write_text(_line() + _line()[:20])  # second line still being written
    assert kit_trace.ingest(conn)["new_events"] == 1
    with open(path, "w") as handle:
        handle.write(_line() + _line("Edit"))
    assert kit_trace.ingest(conn)["new_events"] == 1
    assert kit_trace.ingest(conn)["new_events"] == 0
    assert len(_rows(conn)) == 2
    with open(path, "a") as handle:
        handle.write(_line("Read"))
    assert kit_trace.ingest(conn)["new_events"] == 1


def test_huge_summary_line_is_read(conn, repo):
    big = json.dumps({"timestamp": _ts(), "tool_name": "Bash", "is_error": False,
                      "summary": "x" * 5_000_000}) + "\n"  # fmt: skip
    _trace(repo, "T001", big)
    assert kit_trace.ingest(conn)["new_events"] == 1


def test_deleted_project_is_skipped_but_history_stays(conn, repo):
    _trace(repo, "T001", _line())
    kit_trace.ingest(conn)
    for f in (repo / "memory" / "event-trace").iterdir():
        f.unlink()
    (repo / "memory" / "event-trace").rmdir()
    (repo / "memory").rmdir()
    repo.rmdir()
    assert kit_trace.ingest(conn) == {"files": 0, "new_events": 0}
    summary = kit.kit_summary(conn)["projects"][0]
    assert summary["missing_on_disk"] is True
    assert summary["parts"]["kernel"]["tool_calls"] == 1


def test_kit_summary_counts_and_no_data(conn, repo, tmp_path):
    assert kit.kit_summary(conn)["projects"] == [
        {
            "project": "demo",
            "missing_on_disk": False,
            "parts": {
                "kernel": {"status": "no data"},
                "git": {"status": "no data"},
                "context": {"status": "no data"},
            },
        }
    ]
    _trace(repo, "T001", _line(error=True), _line(), _line())
    _trace(repo, "T002", _line())
    kit_trace.ingest(conn)
    part = kit.kit_summary(conn)["projects"][0]["parts"]["kernel"]
    assert (part["status"], part["tool_calls"], part["tool_errors"], part["kit_tasks"]) == (
        "ok", 4, 1, 2,
    )  # fmt: skip
    assert part["error_rate"] == 0.25
    assert CANARY not in json.dumps(kit.kit_summary(conn))


def _rule_result(conn):
    out = rules.evaluate(conn, now=NOW)
    for hit in out["hits"]:
        if hit["rule_id"] == "tool_errors_high":
            return "fired", hit
    for item in out["abstained"]:
        if item["rule_id"] == "tool_errors_high":
            return "abstained", item
    return "ok", None


@pytest.mark.parametrize(("errors", "expected"), [(11, "fired"), (10, "ok"), (9, "ok")])
def test_tool_errors_high_boundary(conn, repo, errors, expected):
    _trace(repo, "T001", *[_line(error=i < errors) for i in range(100)])
    kit_trace.ingest(conn)
    status, hit = _rule_result(conn)
    assert status == expected
    if hit:
        assert hit["measured"] == 0.11
        assert hit["label"] == "warning"
        assert hit["fix"]


def test_tool_errors_high_ignores_calls_older_than_7_days_and_abstains_without_data(conn, repo):
    assert _rule_result(conn)[0] == "abstained"
    _trace(repo, "T001", *[_line(error=True, ts=_ts(timedelta(days=8))) for _ in range(5)])
    kit_trace.ingest(conn)
    assert _rule_result(conn)[0] == "abstained"


def test_prune_covers_kit_rows(conn, repo):
    _trace(repo, "T001", _line(ts=_ts(timedelta(days=40))), _line())
    kit_trace.ingest(conn)
    assert store.prune(conn, 30, now=NOW) == 1
    assert len(_rows(conn)) == 1
