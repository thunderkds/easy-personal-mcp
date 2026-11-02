"""The live tail (HUN-176) over the leak suite's planted transcripts: no canary leaves it (NFR-002, FR-002)."""

import logging

from easy_personal.core import config, live, store


def test_live_tail_buffer_snapshot_store_and_logs_hold_no_canary(canaries, tmp_path, monkeypatch, caplog):
    caplog.set_level(logging.DEBUG)
    projects = canaries.render_fixture(tmp_path / "projects")  # unknown to the store: the tail reads it all
    monkeypatch.setenv(config.CLAUDE_PROJECTS_ENV, str(projects))
    tail = live.LiveTail()
    tail.poll_once()
    events = tail.events_since(0)
    assert {"tool_use", "usage"} <= {e["kind"] for e in events}  # not a vacuous pass: it really read the file
    with store.opened(config.store_path()) as conn:
        dump = "\n".join(conn.iterdump())
    assert 'INSERT INTO "events"' in dump
    for surface, blob in (
        ("live events", events),
        ("live snapshot", tail.snapshot()),
        ("store dump", dump),
        ("log records", caplog.text),
    ):
        canaries.assert_clean(surface, blob)


def test_live_stream_really_ran_over_planted_transcripts(leak_run):
    """Guards against a vacuous pass: the SSE routes streamed a snapshot and live events built from the planted lines."""
    body = leak_run.outputs["api /api/live"]
    assert body.startswith("200 ") and "event: snapshot" in body and "text/event-stream" in body
    assert '"session_id"' in body  # the snapshot lists sessions
    assert "event: snapshot" in leak_run.outputs["api /api/live?token=leak-test-token"]
    assert "event: live" in leak_run.outputs["api /api/live resumed"]
    resumed = leak_run.outputs["api /api/live resumed"]  # replayed events first, then one fresh snapshot (HUN-181)
    assert resumed.index("event: live") < resumed.index("event: snapshot")


def test_attention_states_leak_no_planted_block_id_or_content(tmp_path, monkeypatch, caplog):
    """HUN-181: block ids and planted text in `input` / `content` / `text` stay out of every live output (NFR-002)."""
    import json
    from datetime import UTC, datetime

    from easy_personal.surfaces import api

    caplog.set_level(logging.DEBUG)
    projects = tmp_path / "projects"
    (projects / "demo").mkdir(parents=True)
    monkeypatch.setenv(config.CLAUDE_PROJECTS_ENV, str(projects))
    secrets = ("PLANTED-TEXT-aa11", "PLANTED-INPUT-bb22", "PLANTED-OUTPUT-cc33")
    ids = ("toolu_PLANTED_ID_0001", "toolu_PLANTED_ID_0002", "toolu_PLANTED_ID_0003", "toolu_PLANTED_ID_ASK")
    stamp = datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"

    def line(kind, content, **extra):
        message = {"id": extra.get("mid"), "model": "m", "content": content,
                   "usage": {"input_tokens": 1, "output_tokens": 1, "cache_creation_input_tokens": 0,
                             "cache_read_input_tokens": 0}} if kind == "assistant" else {"content": content}  # fmt: skip
        return json.dumps({"type": kind, "version": "2.1.316", "cwd": "/work/demo", "sessionId": "s1",
                           "isSidechain": False, "timestamp": stamp, "message": message}) + "\n"  # fmt: skip

    text = []
    for i, block_id in enumerate(ids[:3]):
        text.append(line("assistant", [{"type": "text", "text": secrets[0]},
                                       {"type": "tool_use", "id": block_id, "name": "Bash", "input": {"c": secrets[1]}}],
                         mid=f"msg_{i}"))  # fmt: skip
        text.append(line("user", [{"type": "tool_result", "tool_use_id": block_id, "is_error": True, "content": secrets[2]}]))
    text.append(line("assistant", [{"type": "tool_use", "id": ids[3], "name": "AskUserQuestion", "input": {"q": secrets[1]}}], mid="msg_ask"))  # fmt: skip
    (projects / "demo" / "s1.jsonl").write_text("".join(text))
    tail = live.LiveTail()
    tail.poll_once()
    snapshot = tail.snapshot()
    assert {a["state"] for a in snapshot["sessions"][0]["attention"]} == {"failing", "waiting"}  # not a vacuous pass
    with store.opened(config.store_path()) as conn:
        dump = "\n".join(conn.iterdump())
    wire = "".join(
        [api._sse("snapshot", api._snapshot_data(snapshot)).decode()]
        + [api._sse("live", {k: e[k] for k in api._EVENT_KEYS}, e["id"]).decode() for e in tail.events_since(0)]
    )
    blob = json.dumps(tail.events_since(0)) + json.dumps(snapshot) + wire + dump + caplog.text
    for planted in (*secrets, *ids):
        assert planted not in blob, planted
