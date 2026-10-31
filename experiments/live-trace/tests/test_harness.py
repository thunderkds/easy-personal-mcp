"""Harness tests on synthetic stream-json and transcript fixtures (HUN-165). No live session."""
import json
import os
import stat
import sys
from pathlib import Path

import pytest

HARNESS = Path(__file__).resolve().parents[1] / "harness"
sys.path.insert(0, str(HARNESS))

import make_scratch
import reference
import run_session

SECRET = "sk-ant-FAKE-SECRET-123"
SENTENCE = "Please summarise my quarterly tax plan for the Zebra account"


def ev(**kw):
    return json.dumps(kw)


def stream_lines():
    """A full standard session: 3 Read, 2 Bash (one fails), 1 Edit, 1 Agent (+1 inner Read)."""
    def tu(i, name, parent=None):
        return ev(type="assistant", parent_tool_use_id=parent, session_id="S1", message={
            "content": [{"type": "text", "text": SENTENCE + SECRET},
                        {"type": "tool_use", "id": i, "name": name, "input": {"command": SECRET}}]})

    def tr(i, err=False, parent=None):
        return ev(type="user", parent_tool_use_id=parent, message={"content": [
            {"type": "tool_result", "tool_use_id": i, "is_error": err, "content": SECRET}]})

    lines = [ev(type="system", subtype="init", session_id="S1", cwd="/x")]
    for n, name in enumerate(["Read", "Read", "Read", "Bash", "Bash", "Edit", "Agent"]):
        lines.append(tu(f"t{n}", name))
        lines.append(tr(f"t{n}", err=(n == 4)))
    lines += [tu("t7", "Read", parent="t6"), tr("t7", parent="t6"), "not json " + SECRET,
              ev(type="result", subtype="success", is_error=False, num_turns=9, duration_ms=1234,
                 total_cost_usd=0.01, result=SENTENCE + SECRET, session_id="S1")]
    return lines


def test_harness_parse_counts_tool_use_by_name():
    meta = run_session.parse_stream(stream_lines())
    assert meta["session_id"] == "S1"  # taken from system/init
    assert meta["tool_use_by_name"] == {"Read": 3, "Bash": 2, "Edit": 1, "Agent": 1}
    assert meta["subagent_tool_use_by_name"] == {"Read": 1}
    assert meta["bash_total"] == 2 and meta["bash_failed"] == 1
    assert meta["unparsed_lines"] == 1
    assert meta["result_subtype"] == "success" and meta["num_turns"] == 9
    steps = run_session.steps_done(meta, 0)
    assert all(steps.values()), steps


def test_harness_parse_keeps_no_text():
    dumped = json.dumps(run_session.parse_stream(stream_lines()))
    assert SECRET not in dumped and SENTENCE not in dumped and "Zebra" not in dumped


def test_harness_skipped_step_is_recorded_not_faked():
    lines = [l for l in stream_lines() if '"Edit"' not in l]
    meta = run_session.parse_stream(lines)
    steps = run_session.steps_done(meta, 0)
    assert steps["edit_1"] is False and steps["read_3"] is True
    assert run_session.steps_done(meta, 1)["normal_end"] is False


def test_harness_no_failing_bash_is_not_step_done():
    lines = [l.replace('"is_error": true', '"is_error": false') for l in stream_lines()]
    steps = run_session.steps_done(run_session.parse_stream(lines), 0)
    assert steps["bash_2_one_failing"] is False


def write_transcript(path: Path, session_id: str, n: int, prefix: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as fh:
        fh.writelines(json.dumps({
                "type": "assistant", "sessionId": session_id,
                "timestamp": f"2026-10-31T10:00:0{i}.500Z",
                "message": {"content": [
                    {"type": "text", "text": SENTENCE + SECRET},
                    {"type": "tool_use", "id": f"{prefix}{i}", "name": "Bash",
                     "input": {"command": SECRET}}]}}) + "\n" for i in range(n))
        fh.write(json.dumps({"type": "user", "timestamp": "2026-10-31T10:00:09.000Z",
                             "message": {"content": [{"type": "tool_result", "tool_use_id": "x",
                                                      "content": SECRET}]}}) + "\n")
        fh.write("{partial " + SECRET + "\n")


def test_harness_reference_reads_main_and_subagents(tmp_path):
    cfg = tmp_path / "cfg"
    proj = cfg / "projects" / "-tmp-scratch"
    write_transcript(proj / "SID.jsonl", "SID", 2, "m")
    write_transcript(proj / "SID" / "subagents" / "agent-1.jsonl", "SID", 1, "s")
    write_transcript(proj / "OTHER.jsonl", "OTHER", 1, "o")
    ref = reference.read_reference("SID", cfg)
    assert ref == {"m0": ("Bash", "2026-10-31T10:00:00.500Z"),
                   "m1": ("Bash", "2026-10-31T10:00:01.500Z"),
                   "s0": ("Bash", "2026-10-31T10:00:00.500Z")}
    assert SECRET not in json.dumps(ref) and SENTENCE not in json.dumps(ref)
    assert reference.to_epoch_ms("2026-10-31T10:00:00.500Z") == pytest.approx(1793440800500.0)


def test_harness_reference_unknown_session_is_empty(tmp_path):
    assert reference.read_reference("NOPE", tmp_path) == {}


def test_harness_make_scratch_is_outside_repo_and_a_git_repo(tmp_path):
    path = make_scratch.make_scratch(tmp_path)
    assert make_scratch.REPO_ROOT not in (path, *path.parents)
    assert (path / ".git").is_dir()
    txt = sorted(p.name for p in path.glob("*.txt"))
    assert txt == ["notes_a.txt", "notes_b.txt", "notes_c.txt"]
    assert os.access(path / "check.sh", os.X_OK)


def test_harness_make_scratch_refuses_parent_inside_repo():
    with pytest.raises(ValueError):
        make_scratch.make_scratch(make_scratch.REPO_ROOT / "experiments")


def fake_claude(tmp_path: Path) -> Path:
    """A stand-in `claude` that prints the synthetic stream, planted secret included."""
    body = tmp_path / "stream.jsonl"
    body.write_text("\n".join(stream_lines()) + "\n")
    script = tmp_path / "claude"
    script.write_text(f"#!/bin/sh\ncat {body}\necho '{SECRET}' >&2\nexit 0\n")
    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    return script


def test_harness_run_session_writes_metadata_only_json(tmp_path):
    scratch = make_scratch.make_scratch(tmp_path)
    runs = tmp_path / "runs"
    rec = run_session.run_session(scratch, {"MY_FLAG": "1"}, None, "unit", runs_dir=runs,
                                  claude_bin=str(fake_claude(tmp_path)))
    files = list(runs.glob("*.json"))
    assert len(files) == 1
    stored = json.loads(files[0].read_text())
    assert stored["session_id"] == "S1" and stored["exit_code"] == 0
    assert stored["extra_env_names"] == ["MY_FLAG"]
    assert all(stored["steps"].values())
    assert {"start_utc", "end_utc", "wall_ms", "tool_use_by_name"} <= set(stored)
    raw = files[0].read_text()
    assert SECRET not in raw and SENTENCE not in raw
    assert rec["run_file"] == str(files[0])


def test_harness_run_session_refuses_content_logging_env(tmp_path):
    with pytest.raises(ValueError):
        run_session.run_session(tmp_path, {"OTEL_LOG_USER_PROMPTS": "1"}, runs_dir=tmp_path)


def test_harness_summarize_median_and_p95():
    s = run_session.summarize([{"wall_ms": v} for v in (10, 50, 20, 40, 30)])
    assert s["median_ms"] == 30 and s["p95_ms"] == 50 and s["n"] == 5


def test_harness_planted_secret_absent_from_stored_files():
    """Grep check over everything this task stores: runs/, RESULT.md, NOTES.md."""
    root = HARNESS.parent
    paths = list((HARNESS / "runs").glob("*")) + [HARNESS / "RESULT.md", root / "NOTES.md"]
    for p in paths:
        if p.exists():
            text = p.read_text()
            assert SECRET not in text and SENTENCE not in text, p
