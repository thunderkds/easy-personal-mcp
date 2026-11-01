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
    assert "event: snapshot" not in leak_run.outputs["api /api/live resumed"]
