"""E2 tests (HUN-167): collector reduction, settings template, analysis. No live session."""
import json
import sys
import threading
import time
import urllib.request
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "harness"))
sys.path.insert(0, str(ROOT / "e2"))

import collector
import run_e2

SECRET = "sk-ant-FAKE-SECRET-123"
SENTENCE = "Please summarise my quarterly tax plan for the Zebra account"
ALLOWED = {"hook_event_name", "session_id", "tool_use_id", "tool_name", "agent_id", "recv_ms"}


def post(port, body, path="/hook", raw=None):
    data = raw if raw is not None else json.dumps(body).encode()
    req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", data=data,
                                 headers={"Content-Type": "application/json"}, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


def payload(event="PreToolUse", sid="S1", tid="t1", **kw):
    return {"hook_event_name": event, "session_id": sid, "tool_use_id": tid, "tool_name": "Bash",
            "transcript_path": "/x/y.jsonl", "cwd": "/x", "prompt": SENTENCE,
            "tool_input": {"command": SECRET}, "tool_response": {"stdout": SECRET}, **kw}


@pytest.fixture
def col(tmp_path):
    c = collector.Collector(tmp_path / "ev.jsonl", port=0).start()
    yield c, tmp_path / "ev.jsonl"
    try:
        c.stop()
    except Exception:
        pass


def test_e2_collector_keeps_only_allowed_keys_and_replies_2xx_empty(col):
    c, out = col
    status, body = post(c.port, payload(agent_id="a1"))
    assert status == 200 and body == b""
    c.stop()
    lines = out.read_text().splitlines()
    assert len(lines) == 1
    rec = json.loads(lines[0])
    assert set(rec) <= ALLOWED
    assert rec["hook_event_name"] == "PreToolUse" and rec["agent_id"] == "a1"
    assert isinstance(rec["recv_ms"], float) and abs(rec["recv_ms"] - time.time() * 1000) < 60000


def test_e2_planted_secret_and_prompt_never_stored(col, tmp_path):
    c, out = col
    post(c.port, payload())
    post(c.port, payload("UserPromptSubmit", tid=None) | {"tool_use_id": None})
    c.stop()
    fields = Path(str(out) + ".fields.json")
    for f in (out, fields):
        text = f.read_text()
        assert SECRET not in text and SENTENCE not in text
    names = json.loads(fields.read_text())
    assert "tool_input" in names["PreToolUse"]  # NAMES only
    assert "prompt" in names["PreToolUse"]


def test_e2_large_tool_response_dropped_and_fast(col):
    c, out = col
    big = payload("PostToolUse", tid="big")
    big["tool_response"] = {"stdout": ("x" * 1024 + SECRET) * 1200}  # > 1 MB
    assert len(json.dumps(big)) > 1_000_000
    t0 = time.monotonic()
    status, _ = post(c.port, big)
    elapsed = time.monotonic() - t0
    c.stop()
    assert status == 200 and elapsed < 2.0
    text = out.read_text()
    assert SECRET not in text and len(text) < 500


def test_e2_subagent_events_carry_agent_id(col):
    c, out = col
    post(c.port, payload("SubagentStart", tid=None, agent_id="ag-9") | {"tool_use_id": None})
    post(c.port, payload("PreToolUse", tid="t2", agent_id="ag-9"))
    c.stop()
    recs = [json.loads(x) for x in out.read_text().splitlines()]
    assert [r.get("agent_id") for r in recs] == ["ag-9", "ag-9"]
    a = run_e2.analyze(recs, {"t2": ("Read", "2026-10-31T10:00:00.000Z")})
    assert a["subagent_events_with_agent_id"] == 2


def test_e2_two_sessions_separated_by_session_id(col):
    c, out = col
    ts = []
    for i in range(20):
        sid = "SA" if i % 2 == 0 else "SB"
        th = threading.Thread(target=post, args=(c.port, payload(sid=sid, tid=f"{sid}-{i}")))
        ts.append(th)
        th.start()
    for th in ts:
        th.join()
    c.stop()
    recs = run_e2.read_events(out)
    assert len(recs) == 20
    ref_a = {r["tool_use_id"]: ("Read", "2026-10-31T10:00:00.000Z") for r in recs if r["session_id"] == "SA"}
    a = run_e2.analyze(recs, ref_a, "SA")
    assert a["all"]["pre_capture_pct"] == 100.0 and a["pre_post_extra_ids"] == []


def test_e2_bad_requests_rejected_without_storing(col):
    c, out = col
    assert post(c.port, None, raw=b"not json " + SECRET.encode())[0] == 400
    assert post(c.port, None, raw=b"[1,2]")[0] == 400
    assert post(c.port, payload(), path="/other")[0] == 404
    c.stop()
    assert out.read_text() == ""


def test_e2_settings_template_matches_acceptance_criteria(tmp_path):
    tpl = json.loads((ROOT / "e2" / "settings.json").read_text())
    expected = ["SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse", "PostToolUseFailure",
                "SubagentStart", "SubagentStop", "Stop", "SessionEnd"]
    assert sorted(tpl["hooks"]) == sorted(expected)
    for ev, groups in tpl["hooks"].items():
        h = groups[0]["hooks"][0]
        assert h["type"] == "http" and h["async"] is True and h["url"] == "http://127.0.0.1:4390/hook"
    sync = json.loads(Path(run_e2.write_settings(tmp_path / "s.json", async_=False)).read_text())
    assert all("async" not in g[0]["hooks"][0] for g in sync["hooks"].values())
    assert json.loads(Path(run_e2.write_settings(tmp_path / "a.json")).read_text()) == tpl


def test_e2_analyze_latency_and_capture():
    ref = {"t1": ("Read", "2026-10-31T10:00:00.000Z"), "t2": ("Bash", "2026-10-31T10:00:01.000Z"),
           "t3": ("SubagentHandback", "2026-10-31T10:00:02.000Z")}
    base = run_e2.analyze([], ref)["all"]["ref_ids"]
    assert base == 3
    t0 = run_e2._iso_ms("2026-10-31T10:00:00.000Z")
    ev = [{"hook_event_name": "PreToolUse", "session_id": "S", "tool_use_id": "t1", "recv_ms": t0 + 100},
          {"hook_event_name": "PostToolUse", "session_id": "S", "tool_use_id": "t1", "recv_ms": t0 + 300},
          {"hook_event_name": "PreToolUse", "session_id": "S", "tool_use_id": "t2", "recv_ms": t0 + 1200},
          {"hook_event_name": "PostToolUseFailure", "session_id": "S", "tool_use_id": "t2", "recv_ms": t0 + 1500}]
    a = run_e2.analyze(ev, ref, "S")
    assert a["all"]["pre_capture_pct"] == pytest.approx(66.7, abs=0.1)
    assert a["no_handback"]["pre_capture_pct"] == 100.0 and a["no_handback"]["ref_ids"] == 2
    assert a["pre_latency"]["median_ms"] == 150.0 and a["post_latency"]["max_ms"] == 500.0


def test_e2_collector_delay_knob_and_no_content_in_source_folder(tmp_path):
    c = collector.Collector(tmp_path / "d.jsonl", port=0, delay_ms=300).start()
    t0 = time.monotonic()
    status, _ = post(c.port, payload())
    el = time.monotonic() - t0
    c.stop()
    assert status == 200 and el >= 0.3
    # planted-secret grep over everything this task stores (run records, events, results)
    for f in (ROOT / "e2").rglob("*"):
        if f.is_file() and f.suffix in {".json", ".jsonl", ".md", ".txt"}:
            text = f.read_text()
            assert SECRET not in text and SENTENCE not in text, f
