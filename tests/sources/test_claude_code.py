import json
import shutil
import subprocess
from pathlib import Path

from easy_personal.core import metrics
from easy_personal.sources import claude_code

SID = "00000000-0000-4000-8000-0000000000aa"


def _record(n: int, **extra) -> dict:
    record = {
        "type": "assistant",
        "version": "2.1.295",
        "cwd": "/work/demo-app",
        "sessionId": SID,
        "gitBranch": "main",
        "isSidechain": False,
        "timestamp": f"2026-01-16T10:{n:02d}:00.000Z",
        "message": {
            "id": f"msg_inc_{n}",
            "model": "claude-fixture-model-1",
            "content": [{"type": "text", "text": "INVENTED_TEXT"}],
            "usage": {
                "input_tokens": 1,
                "output_tokens": 1,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
            },
        },
    }
    record.update(extra)
    return record


def _line(n: int, **extra) -> bytes:
    return (json.dumps(_record(n, **extra)) + "\n").encode()


def _offset(conn, path: Path) -> int:
    return conn.execute(
        "SELECT byte_offset FROM source_files WHERE path = ?", (str(path),)
    ).fetchone()[0]


def _count(conn) -> int:
    return conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]


def test_appended_complete_lines_ingest_only_new(conn, tmp_path):
    root = tmp_path / "projects"
    root.mkdir()
    f = root / "s.jsonl"
    f.write_bytes(_line(1) + _line(2))
    assert claude_code.ingest(conn, root)["new_events"] == 2
    with f.open("ab") as h:
        h.write(_line(3))
    assert claude_code.ingest(conn, root)["new_events"] == 1
    assert metrics.usage_summary(conn)["messages"] == 3


def test_truncated_last_line_not_consumed_until_complete(conn, tmp_path):
    root = tmp_path / "projects"
    root.mkdir()
    f = root / "s.jsonl"
    whole = _line(1)
    partial = _line(2)
    f.write_bytes(whole + partial[:20])
    assert claude_code.ingest(conn, root)["new_events"] == 1
    assert _offset(conn, f) == len(whole)  # never mid-line
    with f.open("ab") as h:
        h.write(partial[20:])
    assert claude_code.ingest(conn, root)["new_events"] == 1
    assert _offset(conn, f) == len(whole) + len(partial)
    assert metrics.usage_summary(conn)["abstained"]["total"] == 0


def test_fixture_truncated_lines_never_consumed(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root)
    assert (
        conn.execute(
            "SELECT COUNT(*) FROM events WHERE message_id = 'msg_fixture_TRUNC'"
        ).fetchone()[0]
        == 0
    )
    for path, offset in conn.execute("SELECT path, byte_offset FROM source_files"):
        data = Path(path).read_bytes()
        assert offset < len(data) and data[offset - 1 : offset] == b"\n"


def test_deleted_file_keeps_rows_and_retires_offset(conn, tmp_path):
    root = tmp_path / "projects"
    root.mkdir()
    f = root / "s.jsonl"
    f.write_bytes(_line(1))
    claude_code.ingest(conn, root)
    f.unlink()
    claude_code.ingest(conn, root)
    assert _count(conn) == 1
    assert conn.execute("SELECT COUNT(*) FROM source_files").fetchone()[0] == 0
    assert metrics.usage_summary(conn)["messages"] == 1


def test_replaced_file_new_inode_restarts_offset(conn, tmp_path):
    root = tmp_path / "projects"
    root.mkdir()
    f = root / "s.jsonl"
    f.write_bytes(_line(1) + _line(2))
    claude_code.ingest(conn, root)
    replacement = tmp_path / "new.jsonl"
    replacement.write_bytes(_line(3))
    replacement.replace(f)  # same path, new inode, shorter than the old offset
    assert claude_code.ingest(conn, root)["new_events"] == 1
    assert metrics.usage_summary(conn)["messages"] == 3


def test_source_files_unchanged_after_ingest(conn, fixtures_root):
    def snapshot():
        return {p: (p.stat().st_mtime_ns, p.stat().st_size) for p in fixtures_root.rglob("*.jsonl")}

    before = snapshot()
    claude_code.ingest(conn, fixtures_root)
    assert snapshot() == before


def test_missing_source_root_is_no_data(conn, tmp_path):
    assert claude_code.ingest(conn, tmp_path / "absent") == {"files": 0, "new_events": 0}


def test_rerun_resumes_from_offset_without_reparsing(conn, fixtures_root, monkeypatch):
    claude_code.ingest(conn, fixtures_root)
    calls = []
    real = claude_code.parse_record
    monkeypatch.setattr(claude_code, "parse_record", lambda *a: calls.append(1) or real(*a))
    claude_code.ingest(conn, fixtures_root)
    assert calls == []


def _types(conn, abstained):
    return {
        r[0]
        for r in conn.execute(
            "SELECT DISTINCT record_type FROM events WHERE abstained = ?", (abstained,)
        )
    }


def test_unversioned_metadata_types_are_known_not_abstained(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root / "unversioned")
    assert _types(conn, 1) == set()
    assert _types(conn, 0) == {
        "agent-name",
        "custom-title",
        "last-prompt",
        "atis-latch",
        "queue-operation",
        "cost-state",
        "file-history-snapshot",
        "artifact-comment-monitor",
        "frame-link",
    }


def test_unknown_types_abstained_per_version(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root)
    assert _types(conn, 1) == {"future-record-type-xyz", "another-unknown-type"}
    assert metrics.usage_summary(conn)["abstained"]["by_version"] == {
        "2.1.293": 1,
        "2.1.295": 1,
        "2.1.302": 1,
    }


def test_unknown_shape_and_bad_json_abstain_and_feed_no_metric(conn, tmp_path):
    root = tmp_path / "projects"
    root.mkdir()
    bad_usage = _record(1)
    bad_usage["message"]["usage"]["input_tokens"] = "10"
    lines = [json.dumps(bad_usage), "not json at all", json.dumps(["a", "list"])]
    (root / "s.jsonl").write_text("\n".join(lines) + "\n")
    claude_code.ingest(conn, root)
    summary = metrics.usage_summary(conn)
    assert summary["abstained"] == {"total": 3, "by_version": {"2.1.295": 1, "unknown": 2}}
    assert summary["messages"] == 0 and summary["sessions"] == 0


def test_api_error_counts_session_adds_zero_tokens(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root / "2.1.302")
    summary = metrics.usage_summary(conn)
    assert summary["sessions"] == 1
    assert summary["input_tokens"] == summary["output_tokens"] == 0
    row = conn.execute("SELECT is_api_error FROM events WHERE message_id = 'msg_fixture_ERR'")
    assert row.fetchone()[0] == 1


def test_split_message_counted_once(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root / "2.1.293")
    assert (
        conn.execute("SELECT COUNT(*) FROM events WHERE message_id = 'msg_fixture_A'").fetchone()[0]
        == 3
    )
    summary = metrics.usage_summary(conn)
    assert (summary["messages"], summary["input_tokens"], summary["output_tokens"]) == (1, 10, 5)


def test_tool_names_and_sidechain_kept_as_metadata(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root / "2.1.295")
    assert conn.execute(
        "SELECT tool_names FROM events WHERE tool_names IS NOT NULL"
    ).fetchall() == [("Read",)]
    assert (
        conn.execute(
            "SELECT COUNT(*) FROM events WHERE is_sidechain = 1 AND message_id = 'msg_fixture_SUB'"
        ).fetchone()[0]
        == 1
    )


def test_free_text_in_label_fields_is_dropped(conn, tmp_path):
    root = tmp_path / "projects"
    root.mkdir()
    record = _record(1, gitBranch="INVENTED secret text with spaces")
    record["message"]["content"] = [{"type": "tool_use", "name": "INVENTED tool text", "input": {}}]
    (root / "s.jsonl").write_text(json.dumps(record) + "\n")
    claude_code.ingest(conn, root)
    assert conn.execute("SELECT git_branch, tool_names FROM events").fetchone() == (None, None)


def _git(*args):
    subprocess.run(
        ["git", "-c", "user.email=t@example.invalid", "-c", "user.name=t", *args],
        check=True,
        capture_output=True,
    )


def test_project_assignment_per_fr009(conn, tmp_path):
    repo = (tmp_path / "repo").resolve()
    (repo / "sub" / "deep").mkdir(parents=True)
    _git("init", "-q", str(repo))
    _git("-C", str(repo), "commit", "-q", "--allow-empty", "-m", "init")
    worktree = tmp_path.resolve() / "repo-wt"
    _git("-C", str(repo), "worktree", "add", "-q", str(worktree))
    plain = tmp_path.resolve() / "plain"
    plain.mkdir()
    gone = tmp_path.resolve() / "deleted-folder"
    expected = {
        str(repo): str(repo),
        str(repo / "sub" / "deep"): str(repo),
        str(worktree): str(repo),
        str(plain): str(plain),
        str(gone): str(gone),
    }
    root = tmp_path / "projects"
    root.mkdir()
    lines = [json.dumps(_record(n, cwd=cwd, sessionId=f"s-{n}")) for n, cwd in enumerate(expected)]
    (root / "s.jsonl").write_text("\n".join(lines) + "\n")
    claude_code.ingest(conn, root)
    stored = dict(conn.execute("SELECT session_id, project_path FROM events"))
    assert stored == {f"s-{n}": project for n, project in enumerate(expected.values())}

    shutil.rmtree(repo)  # history survives a deleted folder: stored at ingest, not recomputed
    claude_code.ingest(conn, root)
    assert dict(conn.execute("SELECT session_id, project_path FROM events")) == stored


def test_usage_without_message_id_abstains_not_silently_dropped(conn, tmp_path):
    root = tmp_path / "projects"
    root.mkdir()
    record = _record(1)
    del record["message"]["id"]
    (root / "s.jsonl").write_text(json.dumps(record) + "\n")
    claude_code.ingest(conn, root)
    assert metrics.usage_summary(conn)["abstained"] == {
        "total": 1,
        "by_version": {"2.1.295": 1},
    }


def test_file_vanishing_mid_ingest_is_skipped(conn, tmp_path, monkeypatch):
    root = tmp_path / "projects"
    root.mkdir()
    (root / "a.jsonl").write_bytes(_line(1))
    (root / "b.jsonl").write_bytes(_line(2))
    real = claude_code.read_complete_lines

    def flaky(path, offset):
        if path.name == "a.jsonl":
            raise FileNotFoundError(path)
        return real(path, offset)

    monkeypatch.setattr(claude_code, "read_complete_lines", flaky)
    assert claude_code.ingest(conn, root) == {"files": 1, "new_events": 1}
