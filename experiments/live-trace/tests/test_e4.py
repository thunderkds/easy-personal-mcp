"""E4 capture tests on synthetic stream-json and a fake `claude` (HUN-169). No live session."""
import fcntl
import json
import os
import stat
import sys
import threading
import time
from pathlib import Path

import pytest

LT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LT / "e4"))

import capture  # noqa: E402  (also puts harness/ on sys.path)
import make_scratch  # noqa: E402

SECRET = "sk-ant-FAKE-SECRET-123"
SENTENCE = "Please summarise my quarterly tax plan for the Zebra account"


def ev(**kw):
    return json.dumps(kw)


def stream_lines(extra_result=True):
    def tu(i, name, parent=None):
        return ev(type="assistant", parent_tool_use_id=parent, session_id="S1", message={
            "id": "msg_" + i, "model": "claude-haiku", "usage": {"input_tokens": 5, "output_tokens": 7,
                                                                  "service_tier": "standard"},
            "content": [{"type": "text", "text": SENTENCE + SECRET},
                        {"type": "tool_use", "id": i, "name": name, "input": {"command": SECRET}}]})

    def tr(i, err=False, parent=None):
        return ev(type="user", parent_tool_use_id=parent, message={"content": [
            {"type": "tool_result", "tool_use_id": i, "is_error": err, "content": SECRET}]})

    lines = [ev(type="system", subtype="init", session_id="S1", cwd="/x", model="haiku",
                tools=[SECRET])]
    for n, name in enumerate(["Read", "Read", "Read", "Bash", "Bash", "Edit", "Agent"]):
        lines += [tu(f"t{n}", name), tr(f"t{n}", err=(n == 4))]
    lines += [tu("t7", "Read", parent="t6"), tr("t7", parent="t6"),
              ev(type="stream_event", event={"type": "content_block_delta", "index": 0,
                                             "delta": {"type": "text_delta", "text": SENTENCE}}),
              ev(type="result", subtype="success", is_error=False, num_turns=9, duration_ms=1234,
                 duration_api_ms=900, total_cost_usd=0.01, result=SENTENCE + SECRET,
                 session_id="S1", stop_reason="end_turn", permission_denials=[],
                 usage={"input_tokens": 10, "output_tokens": 20,
                        "cache_creation": {"ephemeral_5m_input_tokens": 3}},
                 modelUsage={"claude-haiku": {"inputTokens": 10, "costUSD": 0.01}})]
    if extra_result:
        lines.append(ev(type="result", subtype="success", is_error=False, num_turns=0,
                        duration_ms=7, total_cost_usd=0.01, result="", session_id="S1"))
    return lines


def fake_claude(tmp_path: Path, body_lines, tail="", sleep=0, name="claude") -> Path:
    body = tmp_path / f"{name}.jsonl"
    body.write_text("\n".join(body_lines) + ("\n" if not tail else "\n" + tail))
    script = tmp_path / name
    script.write_text(f"#!/bin/sh\ncat {body}\necho '{SECRET}' >&2\nsleep {sleep}\nexit 0\n")
    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    return script


def run(tmp_path, claude, **kw):
    scratch = make_scratch.make_scratch(tmp_path)
    return capture.capture_session(scratch=scratch, runs_dir=tmp_path / "runs", claude_bin=str(claude),
                                   lock_path=tmp_path / "lock", **kw)


def test_e4_capture_records_ids_names_numbers_per_line(tmp_path):
    cap = run(tmp_path, fake_claude(tmp_path, stream_lines()), label="unit")
    assert cap["session_id"] == "S1" and cap["exit_code"] == 0
    assert cap["tool_use_by_name"] == {"Read": 3, "Bash": 2, "Edit": 1, "Agent": 1}
    assert cap["subagent_tool_use_by_name"] == {"Read": 1}
    uses = capture.stream_tool_uses(cap)
    assert set(uses) == {f"t{i}" for i in range(8)}
    assert uses["t7"][2] == "t6"  # subagent line carries its parent tool_use id
    first = [ln for ln in cap["lines"] if ln.get("tool_uses")][0]
    assert first["tool_uses"] == [{"id": "t0", "name": "Read"}]
    assert first["usage"] == {"input_tokens": 5, "output_tokens": 7}
    assert all(isinstance(ln["recv_ms"], float) for ln in cap["lines"])
    assert [ln["recv_ms"] for ln in cap["lines"]] == sorted(ln["recv_ms"] for ln in cap["lines"])


def test_e4_result_line_fields_and_two_result_lines(tmp_path):
    cap = run(tmp_path, fake_claude(tmp_path, stream_lines()))
    res = [ln for ln in cap["lines"] if ln["type"] == "result"]
    assert cap["result_lines"] == 2 and len(res) == 2
    f = res[0]["fields"]
    assert f["num_turns"] == 9 and f["duration_ms"] == 1234 and f["total_cost_usd"] == 0.01
    assert f["usage"]["cache_creation"] == {"ephemeral_5m_input_tokens": 3}
    assert f["modelUsage"] == {"claude-haiku": {"inputTokens": 10, "costUSD": 0.01}}
    assert f["result"] == "<str>"  # text dropped, field name kept
    assert res[1]["fields"]["num_turns"] == 0
    assert all(cap["steps"].values()), cap["steps"]  # normal end uses the real result, not the empty one


def test_e4_planted_secret_never_stored(tmp_path):
    cap = run(tmp_path, fake_claude(tmp_path, stream_lines()), label="secret")
    stored = Path(cap["run_file"]).read_text()
    for bad in (SECRET, SENTENCE, "Zebra", "tax plan"):
        assert bad not in stored and bad not in json.dumps(cap)


def test_e4_stream_event_keeps_type_names_only(tmp_path):
    cap = run(tmp_path, fake_claude(tmp_path, stream_lines()))
    s = capture.stream_event_summary(cap)
    assert s["stream_event_lines"] == 1
    assert s["by_event_type"] == {"content_block_delta": 1} and s["by_delta_type"] == {"text_delta": 1}


def test_e4_killed_run_partial_last_line_no_result(tmp_path):
    lines = stream_lines()[:6]
    claude = fake_claude(tmp_path, lines, tail='{"type":"assistant","message":{"content":[{"type":"te',
                         sleep=30)
    cap = run(tmp_path, claude, timeout=2)
    assert cap["timed_out"] is True and cap["has_result"] is False
    assert cap["last_line_partial"] is True
    assert cap["lines"][-1]["type"] == "unparsed" and cap["unparsed_lines"] == 1
    assert cap["steps"]["normal_end"] is False
    assert set(cap["lines"][-1]) == {"seq", "recv_ms", "bytes", "type"}  # no content from the cut line


def test_e4_evaluate_capture_pct_and_latency(tmp_path):
    cap = run(tmp_path, fake_claude(tmp_path, stream_lines()))
    t0 = cap["start_epoch_ms"] / 1000.0
    from datetime import UTC, datetime

    def iso(offset):
        return datetime.fromtimestamp(t0 - offset, tz=UTC).isoformat()

    ref = {f"t{i}": ("Read", iso(1.0)) for i in range(8)}
    ref["lost"] = ("SubagentHandback", iso(1.0))  # in the transcript, not in the stream
    out = capture.evaluate(cap, reference=ref)
    assert out["reference_ids"] == 9 and out["captured_ids"] == 8
    assert out["capture_pct"] == pytest.approx(88.9, abs=0.05)
    assert out["missing_by_name"] == {"SubagentHandback": 1}
    assert out["subagent_ids_in_stream"] == 1
    assert out["latency_summary"]["n"] == 8 and out["latency_summary"]["median"] > 1000


def test_e4_summarize_latency_median_p95():
    s = capture.summarize_latency([10, 50, 20, 40, 30])
    assert s["median"] == 30 and s["p95"] == 50 and s["n"] == 5 and s["min"] == 10
    assert capture.summarize_latency([])["median"] is None


def test_e4_capture_holds_exclusive_lock_for_the_run(tmp_path):
    lock = tmp_path / "lock"
    fd = os.open(str(lock), os.O_CREAT | os.O_RDWR)
    fcntl.flock(fd, fcntl.LOCK_EX)
    threading.Timer(0.6, lambda: (fcntl.flock(fd, fcntl.LOCK_UN), os.close(fd))).start()
    claude = fake_claude(tmp_path, stream_lines(), sleep=0)
    scratch = make_scratch.make_scratch(tmp_path)
    cap = capture.capture_session(scratch=scratch, runs_dir=tmp_path / "runs", claude_bin=str(claude),
                                  lock_path=lock)
    assert cap["lock_wait_ms"] >= 400
    # lock released after the run
    fd2 = os.open(str(lock), os.O_RDWR)
    fcntl.flock(fd2, fcntl.LOCK_EX | fcntl.LOCK_NB)
    os.close(fd2)


def test_e4_forbidden_env_refused(tmp_path):
    with pytest.raises(ValueError):
        capture.capture_session(extra_env={"OTEL_LOG_TOOL_CONTENT": "1"}, scratch=tmp_path,
                                runs_dir=tmp_path, lock_path=None)


def test_e4_build_cmd_flags():
    c = capture.build_cmd("claude", "p", "haiku", True, "SID")
    assert "--include-partial-messages" in c and c[c.index("--resume") + 1] == "SID"
    assert "--verbose" in c and "stream-json" in c
    assert "--include-partial-messages" not in capture.build_cmd("claude", "p", "haiku", False, None)


def test_e4_unparsed_and_non_dict_lines_do_not_raise():
    assert capture.parse_line("not json " + SECRET, 1.0, 0)["type"] == "unparsed"
    assert capture.parse_line("[1,2]", 1.0, 1)["type"] == "unparsed"
    assert SECRET not in json.dumps(capture.parse_line("not json " + SECRET, 1.0, 0))


def test_e4_planted_secret_absent_from_stored_files():
    """Grep check over everything this task stores: e4/runs/ and e4/RESULT.md."""
    paths = list((LT / "e4" / "runs").glob("*")) + [LT / "e4" / "RESULT.md"]
    for p in paths:
        if p.exists():
            text = p.read_text()
            assert SECRET not in text and SENTENCE not in text, p
