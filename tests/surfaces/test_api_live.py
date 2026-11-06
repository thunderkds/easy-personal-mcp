"""GET /api/live (HUN-177, FR-044): the live tail as Server-Sent Events, behind the dashboard guard.

The stream never ends by itself, so the tests drive the ASGI app directly and "close the browser"
(an `http.disconnect`) once they have read enough. Transcripts live in temp folders only.
"""

import asyncio
import json
import logging
from datetime import UTC, datetime

import pytest

from easy_personal.core import config, live, store
from easy_personal.sources import claude_code
from easy_personal.surfaces import api

TOKEN, PORT = "tok-LIVE-123", 5556
HOST = f"127.0.0.1:{PORT}"
SECRET = "SECRET-TEXT-do-not-leak"
CONTRACT_EVENT = {
    "id", "session_id", "account", "project", "ts", "kind", "tool", "is_error", "is_sidechain",
    "tokens_in", "tokens_out", "cost_usd", "label", "agent",
}  # fmt: skip
CONTRACT_SESSION = {
    "session_id", "account", "project", "last_ts", "tool_calls", "tokens_in", "tokens_out", "cost_usd", "attention",
    "stage", "stage_skill",
}  # fmt: skip


def stamp() -> str:
    return datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def record(session="s1", tool="Bash", message="msg_1", out=5, use_id="toolu_1"):
    return {
        "type": "assistant", "version": "2.1.316", "cwd": "/work/demo-app", "sessionId": session,
        "isSidechain": False, "entrypoint": "cli", "timestamp": stamp(),
        "message": {
            "id": message, "model": "claude-fixture-model-1",
            "content": [{"type": "text", "text": SECRET},
                        {"type": "tool_use", "id": use_id, "name": tool, "input": {"command": SECRET}}],
            "usage": {"input_tokens": 10, "output_tokens": out, "cache_creation_input_tokens": 2,
                      "cache_read_input_tokens": 3},
        },
    }  # fmt: skip


@pytest.fixture
def projects(tmp_path, monkeypatch):
    root = tmp_path / "claude" / "projects"
    root.mkdir(parents=True)
    monkeypatch.setenv(config.CLAUDE_PROJECTS_ENV, str(root))
    with store.opened(config.store_path()) as conn:
        claude_code.ingest_accounts(conn)
    monkeypatch.setattr(api, "POLL_S", 0.01)
    return root


@pytest.fixture
def add(projects):
    def _add(*records):
        with open(projects / "demo" / "s1.jsonl", "a") as handle:
            handle.write("".join(json.dumps(r) + "\n" for r in records))

    (projects / "demo").mkdir()
    return _add


@pytest.fixture
def tail(monkeypatch, projects):
    t = live.LiveTail()
    monkeypatch.setattr(live, "_current", t)
    return t


@pytest.fixture
def app(tmp_path):
    return api.create_app(TOKEN, PORT, tmp_path)


def headers(token=TOKEN, host=HOST, origin=None, extra=()):
    result = [(b"host", host.encode())]
    if token is not None:
        result.append((api.TOKEN_HEADER.encode(), token.encode()))
    if origin:
        result.append((b"origin", origin.encode()))
    return result + [(k.encode(), v.encode()) for k, v in extra]


def get(app, path="/api/live", hdrs=None, step=None, timeout=5.0):
    """Run one GET; call `step(text)` after every body chunk and disconnect when it returns True."""
    path, _, query = path.partition("?")
    scope = {
        "type": "http", "asgi": {"version": "3.0", "spec_version": "2.3"}, "http_version": "1.1", "method": "GET",
        "scheme": "http", "path": path, "raw_path": path.encode(), "query_string": query.encode(),
        "headers": headers() if hdrs is None else hdrs, "client": ("127.0.0.1", 1), "server": ("127.0.0.1", PORT),
    }  # fmt: skip
    out = {"status": None, "headers": {}, "text": "", "ended": False}

    async def run():
        gone, first = asyncio.Event(), True

        async def receive():
            nonlocal first
            if first:
                first = False
                return {"type": "http.request", "body": b"", "more_body": False}
            await gone.wait()
            return {"type": "http.disconnect"}

        async def send(message):
            if message["type"] == "http.response.start":
                out["status"] = message["status"]
                out["headers"] = {k.decode().lower(): v.decode() for k, v in message["headers"]}
            elif message["type"] == "http.response.body":
                out["text"] += message.get("body", b"").decode()
                if step and step(out["text"]):
                    gone.set()

        await app(scope, receive, send)
        out["ended"] = True

    asyncio.run(asyncio.wait_for(run(), timeout))
    return out


def messages(text):
    """The SSE text as a list of {event, id, data, comment} dicts (data decoded from JSON)."""
    found = []
    for block in text.split("\n\n"):
        if not block.strip():
            continue
        fields = {"event": None, "id": None, "data": None, "comment": None}
        for line in block.split("\n"):
            key, _, value = line.partition(": ")
            if line.startswith(":"):
                fields["comment"] = line[1:].strip()
            elif key == "data":
                fields["data"] = json.loads(value)
            else:
                fields[key] = value
        found.append(fields)
    return found


def count(kind):
    return lambda text: sum(m["event"] == kind for m in messages(text)) >= 1


# -- guard (AC 1) -----------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "hdrs, query",
    [
        (headers(token=None), ""),
        (headers(token=None), "?token=wrong"),
        (headers(token="wrong"), ""),
        (headers(token=None, host="evil.example:5556"), f"?token={TOKEN}"),
        (headers(token=None, origin="http://evil.example"), f"?token={TOKEN}"),
        (headers(origin="http://127.0.0.1:1"), ""),
    ],
)
def test_live_refused_without_token_or_with_wrong_host_or_origin(app, tail, hdrs, query):
    out = get(app, "/api/live" + query, hdrs)
    assert (out["status"], out["text"]) == (403, '{"error":"forbidden"}')


def test_live_token_in_the_query_is_accepted_only_on_the_live_route(app, tail):
    assert get(app, "/api/first-run?token=" + TOKEN, headers(token=None))["status"] == 403
    out = get(app, f"/api/live?token={TOKEN}", headers(token=None), step=count("snapshot"))
    assert out["status"] == 200


def test_live_token_in_the_header_is_accepted_too(app, tail):
    assert get(app, step=count("snapshot"))["status"] == 200


def test_live_response_is_an_uncached_event_stream_with_csp(app, tail):
    out = get(app, f"/api/live?token={TOKEN}", headers(token=None), step=count("snapshot"))
    assert out["headers"]["content-type"].startswith("text/event-stream")
    assert out["headers"]["cache-control"] == "no-store"
    assert out["headers"]["content-security-policy"] == api.CSP


# -- snapshot, then live events (AC 2) --------------------------------------------------------------


def test_live_sends_a_snapshot_then_one_live_event_per_new_buffer_entry(app, tail, add):
    add(record(message="msg_old"))
    tail.poll_once()  # before the browser connects: counted in the snapshot, never replayed
    snap_id = tail.last_id
    state = {"added": False}

    def step(text):
        got = messages(text)
        if got and not state["added"]:
            state["added"] = True
            add(record(message="msg_new", tool="Read", out=7))
            tail.poll_once()
        return sum(m["event"] == "live" for m in got) >= 2

    out = get(app, step=step)
    first, *rest = messages(out["text"])
    assert first["event"] == "snapshot" and first["id"] is None
    assert first["data"]["last_id"] == snap_id and first["data"]["active_window_s"] == 300
    [session] = first["data"]["sessions"]
    assert set(session) == CONTRACT_SESSION
    assert (session["session_id"], session["project"], session["tool_calls"]) == ("s1", "demo-app", 1)
    assert [(m["event"], m["data"]["kind"], m["data"]["tool"]) for m in rest[:2]] == [
        ("live", "tool_use", "Read"),
        ("live", "usage", None),
    ]
    for message in rest[:2]:
        assert set(message["data"]) == CONTRACT_EVENT
        assert message["id"] == str(message["data"]["id"]) and message["data"]["id"] > snap_id
    assert out["status"] == 200 and SECRET not in out["text"]


def test_live_events_pass_the_tail_values_through(app, tail, add):
    def step(text):
        if len(messages(text)) == 1:
            add(record(message="msg_v", out=9))
            tail.poll_once()
        return len(messages(text)) >= 3

    out = get(app, step=step)
    usage = next(m["data"] for m in messages(out["text"]) if m["data"].get("kind") == "usage")
    expected = next(e for e in tail.events_since(0) if e["kind"] == "usage")
    assert usage == {k: expected[k] for k in CONTRACT_EVENT}
    assert (usage["tokens_in"], usage["tokens_out"]) == (10, 9) and usage["ts"].endswith("Z")


def test_live_without_a_running_tail_sends_an_empty_snapshot_and_keep_alives(app, monkeypatch, projects):
    monkeypatch.setattr(live, "_current", None)
    monkeypatch.setattr(api, "PING_S", 0.05)
    out = get(app, step=lambda text: ": ping" in text)
    first, *rest = messages(out["text"])
    assert (out["status"], first["event"], first["data"]) == (
        200, "snapshot", {"active_window_s": 300, "last_id": 0, "open_call_minutes": 10, "sessions": []})
    assert rest and rest[0]["comment"] == "ping"


def test_live_resume_id_without_a_running_tail_still_starts_with_a_snapshot(app, monkeypatch, projects):
    monkeypatch.setattr(live, "_current", None)
    out = get(app, hdrs=headers(extra=[("last-event-id", "5")]), step=count("snapshot"))
    assert messages(out["text"])[0]["data"]["last_id"] == 0


# -- resume (AC 3) ----------------------------------------------------------------------------------


def fill(tail, add, n):
    add(*[record(message=f"msg_{i}", tool="Read") for i in range(n)])
    tail.poll_once()


def test_live_reconnect_with_last_event_id_replays_later_events_then_one_fresh_snapshot(app, tail, add):
    fill(tail, add, 2)
    ids = [e["id"] for e in tail.events_since(0)]
    assert len(ids) == 4  # two tool_use + two usage
    out = get(app, hdrs=headers(extra=[("last-event-id", str(ids[1]))]), step=count("snapshot"))
    got = messages(out["text"])
    assert [m["event"] for m in got[:3]] == ["live", "live", "snapshot"]
    assert [m["data"]["id"] for m in got[:2]] == ids[2:]  # replayed once, in order
    assert got[2]["data"]["last_id"] == ids[-1]
    assert sum(m["event"] == "snapshot" for m in got) == 1


def test_live_reconnect_at_the_newest_id_gets_a_snapshot_then_only_new_events(app, tail, add, monkeypatch):
    monkeypatch.setattr(api, "PING_S", 0.05)
    fill(tail, add, 1)
    newest = tail.last_id

    def step(text):
        if ": ping" in text and not state["added"]:
            state["added"] = True
            add(record(message="msg_after", tool="Grep"))
            tail.poll_once()
        return sum(m["event"] == "live" for m in messages(text)) >= 1

    state = {"added": False}
    out = get(app, hdrs=headers(extra=[("last-event-id", str(newest))]), step=step)
    got = messages(out["text"])
    assert got[0]["event"] == "snapshot" and got[0]["data"]["last_id"] == newest
    assert next(m["data"]["id"] for m in got if m["event"] == "live") == newest + 1
    assert sum(m["event"] == "snapshot" for m in got) == 1


def test_live_a_state_change_while_disconnected_reaches_the_page_in_the_snapshot_after_resume(app, tail, add):
    add(record(message="msg_old"))
    tail.poll_once()
    newest = tail.last_id
    add(record(message="msg_ask", tool="AskUserQuestion", use_id="toolu_q"))  # the page was away
    tail.poll_once()
    out = get(app, hdrs=headers(extra=[("last-event-id", str(newest))]), step=count("snapshot"))
    got = messages(out["text"])
    ids = [m["data"]["id"] for m in got if m["event"] == "live"]
    assert ids == list(range(newest + 1, tail.last_id + 1))
    snapshot = got[-1]
    assert snapshot["event"] == "snapshot" and snapshot["data"]["last_id"] == tail.last_id
    assert [a["state"] for a in snapshot["data"]["sessions"][0]["attention"]] == ["waiting"]


def test_live_resume_older_than_the_buffer_starts_with_a_fresh_snapshot(app, monkeypatch, projects, add):
    small = live.LiveTail(buffer_size=3)
    monkeypatch.setattr(live, "_current", small)
    fill(small, add, 3)  # 6 events, ids 1..6, buffer holds 4..6
    assert small.rolled_past(1)
    out = get(app, hdrs=headers(extra=[("last-event-id", "1")]), step=count("snapshot"))
    first = messages(out["text"])[0]
    assert first["event"] == "snapshot" and first["data"]["last_id"] == 6
    assert [m["data"]["id"] for m in messages(out["text"]) if m["event"] == "live"] == []


@pytest.mark.parametrize("sent", ["999", "-1", "abc", ""])
def test_live_a_stale_or_bad_last_event_id_starts_with_a_fresh_snapshot(app, tail, add, sent):
    fill(tail, add, 1)
    out = get(app, hdrs=headers(extra=[("last-event-id", sent)]), step=count("snapshot"))
    assert messages(out["text"])[0]["event"] == "snapshot"


# -- attention states re-send the snapshot (HUN-181, AC 9) -------------------------------------------


def test_live_resends_a_snapshot_with_attention_after_a_state_change_and_keeps_event_ids_continuous(app, tail, add):
    add(record(message="msg_old"))
    tail.poll_once()
    state = {"phase": 0}

    def step(text):
        got = messages(text)
        snapshots = [m for m in got if m["event"] == "snapshot"]
        if state["phase"] == 0 and snapshots:
            state["phase"] = 1
            add(record(message="msg_a", tool="Read", use_id="toolu_r"))  # ordinary call: no re-send
            tail.poll_once()
            add(record(message="msg_ask", tool="AskUserQuestion", use_id="toolu_q"))  # state change
            tail.poll_once()
        if state["phase"] == 1 and len(snapshots) >= 2:
            state["phase"] = 2
            add(record(message="msg_ask", tool="Read", use_id="toolu_r2"))  # a live event after the re-send
            tail.poll_once()
        return state["phase"] == 2 and any(m["data"]["id"] == tail.last_id for m in got if m["event"] == "live")

    out = get(app, step=step)
    got = messages(out["text"])
    snapshots = [m["data"] for m in got if m["event"] == "snapshot"]
    assert len(snapshots) == 2  # the ordinary call did not trigger a third message
    first, second = snapshots
    assert first["sessions"][0]["attention"] == [] and first["open_call_minutes"] == 10
    [session] = second["sessions"]
    assert set(session) == CONTRACT_SESSION and second["open_call_minutes"] == 10
    assert [(a["state"], a["tool"], a["count"], a["is_sidechain"]) for a in session["attention"]] == [
        ("waiting", "AskUserQuestion", 1, False)
    ]
    assert set(session["attention"][0]) == {"state", "tool", "count", "is_sidechain", "since"}
    ids = [m["data"]["id"] for m in got if m["event"] == "live"]
    assert ids == list(range(first["last_id"] + 1, ids[-1] + 1))  # no gap, no duplicate across the re-send
    after = got[[m["event"] for m in got].index("snapshot", 1) :]
    assert second["last_id"] == max(i for i in ids if i <= second["last_id"])
    assert all(m["data"]["id"] > second["last_id"] for m in after if m["event"] == "live")


# -- call labels, sub-agent type and kit stage (HUN-233, FR-081..FR-083) ----------------------------


def skill_record(skill, message, use_id, args=SECRET):
    rec = record(message=message, tool="Skill", use_id=use_id)
    rec["message"]["content"][1]["input"] = {"skill": skill, "args": args}
    return rec


def test_live_event_carries_label_and_agent_and_the_snapshot_carries_the_stage(app, tail, add):
    add(record(message="msg_old"))
    tail.poll_once()
    state = {"added": False}

    def step(text):
        got = messages(text)
        if got and not state["added"]:
            state["added"] = True
            add(skill_record("tdd", "msg_skill", "toolu_skill"))
            tail.poll_once()
        return sum(m["event"] == "snapshot" for m in got) >= 2 and sum(m["event"] == "live" for m in got) >= 2

    out = get(app, step=step)
    got = messages(out["text"])
    first, second = [m["data"] for m in got if m["event"] == "snapshot"]
    assert (first["sessions"][0]["stage"], first["sessions"][0]["stage_skill"]) == (None, None)
    assert (second["sessions"][0]["stage"], second["sessions"][0]["stage_skill"]) == ("3", "tdd")
    [use] = [m["data"] for m in got if m["event"] == "live" and m["data"]["kind"] == "tool_use"]
    assert (use["tool"], use["label"], use["agent"]) == ("Skill", "tdd", None)
    assert SECRET not in out["text"]


def test_live_resume_replays_labels_from_the_buffer(app, tail, add):
    add(record(message="msg_old"))
    tail.poll_once()
    newest = tail.last_id
    add(skill_record("code-review", "msg_skill", "toolu_skill"))
    tail.poll_once()
    out = get(app, hdrs=headers(extra=[("last-event-id", str(newest))]), step=count("snapshot"))
    got = messages(out["text"])
    [use] = [m["data"] for m in got if m["event"] == "live" and m["data"]["kind"] == "tool_use"]
    assert use["label"] == "code-review"
    assert got[-1]["data"]["sessions"][0]["stage"] == "4"  # the reconnect snapshot has the stage


# -- lifetime, logs (edge cases, AC 5) --------------------------------------------------------------


def test_live_two_tabs_each_get_their_own_full_stream(app, tail, add):
    fill(tail, add, 1)
    a, b = (get(app, step=count("snapshot")) for _ in range(2))
    assert messages(a["text"])[0]["data"] == messages(b["text"])[0]["data"]


def test_live_generator_ends_when_the_client_disconnects(app, tail):
    out = get(app, step=count("snapshot"), timeout=2.0)  # asyncio.wait_for raises if it hangs
    assert out["ended"] is True


def test_live_generator_ends_when_the_server_is_stopping(app, tail):
    stop = {"now": False}
    app.state.should_stop = lambda: stop["now"]

    def step(text):
        stop["now"] = True  # no disconnect is ever sent: only the shutdown flag can end the stream

    out = get(app, step=step, timeout=2.0)
    assert out["ended"] is True and messages(out["text"])[0]["event"] == "snapshot"


def test_live_token_never_appears_in_any_log_line(app, tail, add, caplog):
    caplog.set_level(logging.DEBUG)
    fill(tail, add, 1)
    get(app, f"/api/live?token={TOKEN}", headers(token=None), step=count("snapshot"))
    get(app, f"/api/live?token=nope-{TOKEN}", headers(token=None))
    assert TOKEN not in caplog.text


def test_serve_runs_uvicorn_without_an_access_log():
    import inspect

    assert "access_log=False" in inspect.getsource(api.serve)  # uvicorn prints the query string, token included
