"""E1 tail tests on synthetic transcript fixtures (HUN-166). No live session."""
import json
import sys
from datetime import UTC, datetime
from pathlib import Path

E1 = Path(__file__).resolve().parents[1] / "e1"
sys.path.insert(0, str(E1))

import tail  # noqa: E402

SECRET = "sk-ant-FAKE-SECRET-123"
SENTENCE = "Please summarise my quarterly tax plan for the Zebra account"
TS = "2026-10-31T10:00:00.000Z"


def rec(rtype, blocks, usage=None, mid=None, sid="S1", ts=TS, sidechain=False, **extra):
    msg = {"content": blocks}
    if rtype == "assistant":
        msg.update(id=mid or "msg_1", model="claude-haiku", usage=usage)
        if usage is None:
            del msg["usage"]
    return {"type": rtype, "sessionId": sid, "cwd": "/work/demo", "timestamp": ts,
            "version": "2.1.316", "isSidechain": sidechain, "message": msg, **extra}


USAGE = {"input_tokens": 10, "output_tokens": 5, "cache_creation_input_tokens": 2,
         "cache_read_input_tokens": 3}


def tool_use(i, name="Read"):
    return rec("assistant", [{"type": "text", "text": SENTENCE},
                             {"type": "tool_use", "id": i, "name": name,
                              "input": {"command": SECRET, "file_path": SECRET}}],
               usage=USAGE, mid="m_" + i)


def tool_result(i, err=False):
    return rec("user", [{"type": "tool_result", "tool_use_id": i, "is_error": err,
                         "content": SECRET + SENTENCE}])


def line(r):
    return (json.dumps(r) + "\n").encode()


def make(tmp_path):
    proj = tmp_path / "projects" / "-work-demo"
    proj.mkdir(parents=True)
    events = []
    return proj, events, tail.Tailer(tmp_path / "projects", events.append)


def test_e1_half_line_never_parsed_then_emitted_once(tmp_path):
    proj, events, t = make(tmp_path)
    f = proj / "S1.jsonl"
    f.write_bytes(b"")
    t.poll_once()
    data = line(tool_use("t1")) + line(tool_result("t1"))
    cut = len(line(tool_use("t1"))) + 20  # second line cut in the middle
    with open(f, "ab") as fh:
        fh.write(data[:cut])
    t.poll_once()
    assert [e["kind"] for e in events] == ["tool_use", "usage"]  # tool_result line is half: not parsed
    assert t.stats["bad_json"] == 0
    with open(f, "ab") as fh:
        fh.write(data[cut:])
    t.poll_once()
    t.poll_once()  # nothing new: no duplicates
    kinds = [e["kind"] for e in events]
    assert kinds == ["tool_use", "usage", "tool_result"]
    assert [e["tool_use_id"] for e in events if e["kind"] == "tool_use"] == ["t1"]
    assert t.stats["bad_json"] == 0


def test_e1_event_fields_are_metadata(tmp_path):
    proj, events, t = make(tmp_path)
    t.poll_once()
    (proj / "S1.jsonl").write_bytes(line(tool_use("t1", "Bash")) + line(tool_result("t1", err=True)))
    t.poll_once()
    use = next(e for e in events if e["kind"] == "tool_use")
    assert use["tool"] == "Bash" and use["session_id"] == "S1" and use["project"] == "/work/demo"
    assert use["record_ts"] == TS and use["file_kind"] == "main" and use["recv_ms"] > 0
    usage = next(e for e in events if e["kind"] == "usage")
    assert (usage["tokens_in"], usage["tokens_out"], usage["cache_creation"], usage["cache_read"]) == (10, 5, 2, 3)
    assert usage["model"] == "claude-haiku" and usage["cost_usd"] is None
    res = next(e for e in events if e["kind"] == "tool_result")
    assert res["tool_use_id"] == "t1" and res["is_error"] is True


def test_e1_new_subagent_file_mid_session_is_read_from_start(tmp_path):
    proj, events, t = make(tmp_path)
    (proj / "S1.jsonl").write_bytes(b"")
    t.poll_once()
    sub = proj / "S1" / "subagents"
    sub.mkdir(parents=True)
    (sub / "agent-a1.jsonl").write_bytes(line(rec("assistant", [{"type": "tool_use", "id": "s1", "name": "Read",
                                                                "input": {}}], usage=USAGE, mid="ms1",
                                                   sidechain=True)))
    t.poll_once()
    use = [e for e in events if e["kind"] == "tool_use"]
    assert len(use) == 1 and use[0]["file_kind"] == "sub" and use[0]["is_sidechain"] is True
    assert t.stats["subagent_files_seen"] == 1


def test_e1_files_present_at_start_begin_at_end_unless_from_start(tmp_path):
    proj, events, t = make(tmp_path)
    (proj / "S1.jsonl").write_bytes(line(tool_use("old")))
    t.poll_once()
    assert events == []  # existing content skipped (tail -f)
    with open(proj / "S1.jsonl", "ab") as fh:
        fh.write(line(tool_use("new")))
    t.poll_once()
    assert [e["tool_use_id"] for e in events if e["kind"] == "tool_use"] == ["new"]
    ev2 = []
    tail.Tailer(tmp_path / "projects", ev2.append, from_start=True).poll_once()
    assert sorted(e["tool_use_id"] for e in ev2 if e["kind"] == "tool_use") == ["new", "old"]


def test_e1_unknown_record_type_abstained_and_counted(tmp_path):
    proj, events, t = make(tmp_path)
    t.poll_once()
    unknown = {"type": "brand-new-thing", "sessionId": "S1", "timestamp": TS, "payload": SECRET}
    (proj / "S1.jsonl").write_bytes(line(unknown) + line(unknown) + b"not json " + SECRET.encode() + b"\n"
                                    + line(tool_use("t1")))
    t.poll_once()
    assert t.stats["abstained"] == 2 and t.stats["abstained_by_type"] == {"brand-new-thing": 2}
    assert t.stats["bad_json"] == 1
    assert [e["tool_use_id"] for e in events if e["kind"] == "tool_use"] == ["t1"]


def test_e1_resume_by_byte_offset(tmp_path):
    proj, events, t = make(tmp_path)
    state = tmp_path / "state.json"
    f = proj / "S1.jsonl"
    t.state_path = state
    t.poll_once()
    f.write_bytes(line(tool_use("t1")))
    t.poll_once()
    t.save_state()
    with open(f, "ab") as fh:  # lines written while the tailer is down
        fh.write(line(tool_use("t2")))
    ev2 = []
    t2 = tail.Tailer(tmp_path / "projects", ev2.append, state_path=state)
    t2.poll_once()
    t2.poll_once()
    assert [e["tool_use_id"] for e in ev2 if e["kind"] == "tool_use"] == ["t2"]  # no repeat of t1


def test_e1_replaced_or_truncated_file_reread_from_zero(tmp_path):
    proj, events, t = make(tmp_path)
    f = proj / "S1.jsonl"
    f.write_bytes(line(tool_use("a")) + line(tool_use("b")))
    t.poll_once()
    f.write_bytes(line(tool_use("c")))  # truncated in place (shorter than the stored offset)
    t.poll_once()
    assert [e["tool_use_id"] for e in events if e["kind"] == "tool_use"] == ["c"]


def test_e1_planted_secret_never_in_output_or_stats(tmp_path):
    proj, events, t = make(tmp_path)
    t.poll_once()
    out = tmp_path / "events.jsonl"
    fh = open(out, "w")
    t.sink = lambda ev: fh.write(json.dumps(ev) + "\n")
    (proj / "S1.jsonl").write_bytes(
        line(tool_use("t1")) + line(tool_result("t1")) + line({"type": "x", "sessionId": SECRET,
                                                               "timestamp": SENTENCE}))
    t.poll_once()
    fh.close()
    dumped = out.read_text() + json.dumps(t.stats)
    assert out.read_text().count("\n") == 3
    for needle in (SECRET, SENTENCE, "Zebra", "sk-ant"):
        assert needle not in dumped, needle


def test_e1_start_tail_background_thread_and_score(tmp_path):
    proj = tmp_path / "projects" / "-work-demo"
    proj.mkdir(parents=True)
    out = tmp_path / "ev.jsonl"
    h = tail.start_tail(tmp_path / "projects", out, poll_ms=20)
    now = datetime.now(UTC)
    ref_ts = now.isoformat()
    r = tool_use("t1")
    r["timestamp"] = ref_ts
    with open(proj / "S1.jsonl", "ab") as fh:
        fh.write(line(r))
    stats = h.stop()
    ev = tail.read_events(out)
    sc = tail.score(ev, {"t1": ("Read", ref_ts), "t2": ("Read", ref_ts)})
    assert sc["captured"] == 1 and sc["reference_total"] == 2 and sc["missed"] == ["t2"]
    assert 0 <= sc["median_ms"] < 2000 and sc["p95_ms"] == sc["median_ms"]
    assert stats["polls"] >= 2


def test_e1_scan_skips_non_jsonl_and_missing_dir(tmp_path):
    assert tail.scan(tmp_path / "nope") == {}
    proj = tmp_path / "projects" / "p"
    (proj / "S1" / "subagents").mkdir(parents=True)
    (proj / "notes.txt").write_text("x")
    (proj / "S1" / "subagents" / "meta.json").write_text("{}")
    (proj / "S1.jsonl").write_text("")
    assert [k.rsplit("/", 1)[-1] for k in tail.scan(tmp_path / "projects")] == ["S1.jsonl"]


def test_e1_line_over_1mb_is_read_whole(tmp_path):
    proj, events, t = make(tmp_path)
    t.poll_once()
    big = tool_result("t1")
    big["message"]["content"][0]["content"] = "x" * (1_100_000)
    data = line(big)
    assert len(data) > 1_048_576
    (proj / "S1.jsonl").write_bytes(data + line(tool_use("t2")))
    t.poll_once()
    assert [e["kind"] for e in events] == ["tool_result", "tool_use", "usage"]
    assert "xxxx" not in json.dumps(events)
