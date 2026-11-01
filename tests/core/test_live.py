"""Live tail (HUN-176, FR-043): follows transcripts while the dashboard runs, writes through the store.

Every test points `EASY_PERSONAL_CLAUDE_PROJECTS` / the agent root at temp folders and uses the
isolated `EASY_PERSONAL_HOME` from conftest: nothing here reads a real Claude folder or the real store.
"""

import itertools
import json
import logging
import threading
import time
from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest

from easy_personal.core import accounts, config, live, lock, store
from easy_personal.sources import claude_code
from easy_personal.surfaces import cli

SECRET_TEXT = "SECRET-PROMPT-TEXT-do-not-leak"
SECRET_INPUT = "SECRET-TOOL-INPUT-do-not-leak"
SECRET_OUTPUT = "SECRET-TOOL-OUTPUT-do-not-leak"
EVENT_KEYS = {
    "id", "session_id", "account", "project", "ts", "kind", "tool", "is_error", "is_sidechain",
    "tokens_in", "tokens_out", "cache_creation", "cache_read", "cost_usd",
}  # fmt: skip

_ids = itertools.count(1)


def now_ts(offset_s: float = 0.0) -> str:
    return (datetime.now(UTC) + timedelta(seconds=offset_s)).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def assistant(session="s1", tools=("Bash",), message=None, ts=None, sidechain=False, out=5):
    """One assistant record: usage plus one tool_use block per tool name (planted secrets inside)."""
    blocks = [{"type": "text", "text": SECRET_TEXT}]
    blocks += [
        {"type": "tool_use", "id": f"toolu_{next(_ids)}", "name": t, "input": {"command": f"ls {SECRET_INPUT}"}}
        for t in tools
    ]
    return {
        "type": "assistant", "version": "2.1.316", "cwd": "/work/demo-app", "sessionId": session,
        "isSidechain": sidechain, "entrypoint": "cli", "timestamp": ts or now_ts(),
        "message": {
            "id": message or f"msg_{next(_ids)}", "model": "claude-fixture-model-1", "content": blocks,
            "usage": {"input_tokens": 10, "output_tokens": out, "cache_creation_input_tokens": 2,
                      "cache_read_input_tokens": 3},
        },
    }  # fmt: skip


def tool_result(session="s1", error=False, ts=None):
    return {
        "type": "user", "version": "2.1.316", "cwd": "/work/demo-app", "sessionId": session,
        "isSidechain": False, "entrypoint": "cli", "timestamp": ts or now_ts(),
        "message": {"content": [{"type": "tool_result", "tool_use_id": "toolu_x", "is_error": error,
                                 "content": SECRET_OUTPUT}]},
    }  # fmt: skip


def append(path: Path, *records, raw: str = "") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a") as handle:
        handle.write("".join(json.dumps(r) + "\n" for r in records) + raw)


def wait_until(predicate, timeout=3.0, step=0.01):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if result := predicate():
            return result
        time.sleep(step)
    return predicate()


def store_rows(sql="SELECT * FROM events"):
    with store.opened(config.store_path()) as conn:
        return conn.execute(sql).fetchall()


@pytest.fixture
def projects(tmp_path, monkeypatch):
    root = tmp_path / "claude" / "projects"
    root.mkdir(parents=True)
    monkeypatch.setenv(config.CLAUDE_PROJECTS_ENV, str(root))
    return root


@pytest.fixture
def start(projects):
    """Ingest once (what `easy-personal dashboard` does first), then start a tail; stopped at teardown."""
    tails = []

    def _start(**kwargs):
        with store.opened(config.store_path()) as conn:
            claude_code.ingest_accounts(conn)
        tail = live.LiveTail(**kwargs)
        tail.start()
        tails.append(tail)
        return tail

    yield _start
    for tail in tails:
        tail.stop()


def test_tool_use_line_is_stored_and_in_the_buffer_within_500_ms(projects, start):
    append(projects / "demo" / "s1.jsonl", assistant(tools=("Read",)))
    tail = start()
    assert tail.last_id == 0  # what existed before the tail started is not "live"
    sent = time.monotonic()
    append(projects / "demo" / "s1.jsonl", assistant(tools=("Bash",), message="msg_live"))
    found = wait_until(lambda: [e for e in tail.events_since(0) if e["kind"] == "tool_use"], timeout=0.5)
    assert time.monotonic() - sent < 0.5
    assert [(e["tool"], e["session_id"], e["project"]) for e in found] == [("Bash", "s1", "demo-app")]
    assert found[0]["ts"].endswith("Z") and found[0]["id"] == 1
    assert store_rows("SELECT tool_names, message_id FROM events WHERE message_id = 'msg_live'") == [
        ("Bash", "msg_live")
    ]


def test_partial_last_line_is_never_parsed_and_is_taken_once_complete(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant(tools=("Read",)))
    tail = start()
    line = json.dumps(assistant(tools=("Grep",), message="msg_part"))
    append(path, raw=line[: len(line) // 2])
    time.sleep(0.4)
    assert tail.events_since(0) == []
    assert store_rows("SELECT 1 FROM events WHERE message_id = 'msg_part'") == []
    append(path, raw=line[len(line) // 2 :] + "\n")
    got = wait_until(lambda: [e for e in tail.events_since(0) if e["kind"] == "tool_use"])
    assert [e["tool"] for e in got] == ["Grep"]
    assert len(store_rows("SELECT 1 FROM events WHERE message_id = 'msg_part'")) == 1


def test_a_subagents_file_appearing_mid_session_is_followed(projects, start):
    append(projects / "demo" / "s1.jsonl", assistant())
    tail = start()
    sub = projects / "demo" / "s1" / "subagents" / "agent-a1.jsonl"
    append(sub, assistant(tools=("Edit",), sidechain=True, message="msg_sub"))
    got = wait_until(lambda: [e for e in tail.events_since(0) if e["tool"] == "Edit"])
    assert got and got[0]["is_sidechain"] is True
    append(sub, assistant(tools=("Write",), sidechain=True))
    assert wait_until(lambda: [e for e in tail.events_since(0) if e["tool"] == "Write"])


def test_the_tail_does_not_hold_the_lock_so_a_scheduled_ingest_completes(projects, start, monkeypatch, capsys):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()
    stop_writing = threading.Event()

    def writer():
        while not stop_writing.is_set():
            append(path, assistant(tools=("Bash",)))
            time.sleep(0.005)

    thread = threading.Thread(target=writer)
    thread.start()
    try:
        assert wait_until(lambda: tail.last_id > 3)
        monkeypatch.setenv("EASY_PERSONAL_LOCK_TIMEOUT", "5")
        for _ in range(5):
            assert cli.main(["ingest"]) == 0  # 0, not 75 (LockBusy)
            time.sleep(0.02)
    finally:
        stop_writing.set()
        thread.join()
    capsys.readouterr()
    append(path, assistant(tools=("Zzz",)))
    assert wait_until(lambda: [e for e in tail.events_since(0) if e["tool"] == "Zzz"])  # still alive
    with lock.ingest_lock(config.data_dir() / "ingest.lock", timeout=0.5):
        pass  # idle tail holds no lock


def test_events_are_not_lost_or_doubled_when_a_scheduled_ingest_holds_the_lock(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()
    with lock.ingest_lock(config.data_dir() / "ingest.lock", timeout=1):
        append(path, assistant(tools=("Bash",), message="msg_busy"))
        time.sleep(0.4)  # the tail tries, finds the lock busy and leaves the line for later
        assert tail.events_since(0) == []
    got = wait_until(lambda: [e for e in tail.events_since(0) if e["kind"] == "tool_use"])
    assert len(got) == 1
    assert len(store_rows("SELECT 1 FROM events WHERE message_id = 'msg_busy'")) == 1


def test_a_scheduled_ingest_that_reads_first_leaves_one_row_and_no_cursor_rewind(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start(poll_s=5)  # effectively idle: this test drives the ingest by hand
    append(path, assistant(message="msg_a"), assistant(message="msg_b"))
    with store.opened(config.store_path()) as conn:
        claude_code.ingest_accounts(conn)
        offset = conn.execute("SELECT byte_offset FROM source_files").fetchone()[0]
    assert offset == path.stat().st_size
    tail.poll_once()  # sees nothing new: the shared cursor already moved
    assert tail.events_since(0) == []
    assert store_rows("SELECT byte_offset FROM source_files") == [(offset,)]
    assert len(store_rows("SELECT 1 FROM events WHERE message_id IN ('msg_a', 'msg_b')")) == 2


def test_buffer_is_bounded_and_ids_keep_counting(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start(buffer_size=5)
    for _ in range(4):
        append(path, *(assistant(tools=("Bash", "Read")) for _ in range(3)))
        time.sleep(0.15)
    assert wait_until(lambda: tail.last_id >= 36)  # 12 records x (2 tool_use + 1 usage)
    events = tail.events_since(0)
    assert len(events) == 5
    assert [e["id"] for e in events] == list(range(tail.last_id - 4, tail.last_id + 1))
    assert tail.events_since(tail.last_id) == []
    assert tail.events_since(tail.last_id - 2) == events[-2:]


def test_default_buffer_size_is_2000():
    assert live.LiveTail().buffer_size == 2000


def test_events_carry_metadata_only_and_nothing_secret_reaches_store_or_logs(projects, start, caplog):
    caplog.set_level(logging.DEBUG)
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()
    append(path, assistant(tools=("Bash",), message="msg_m"), tool_result(error=True))
    assert wait_until(lambda: {e["kind"] for e in tail.events_since(0)} == {"tool_use", "usage", "tool_result"})
    events = tail.events_since(0)
    assert all(set(e) == EVENT_KEYS for e in events)
    by_kind = {e["kind"]: e for e in events}
    assert by_kind["tool_use"]["tool"] == "Bash" and by_kind["tool_use"]["is_error"] is False
    assert by_kind["tool_result"]["tool"] is None and by_kind["tool_result"]["is_error"] is True
    assert (by_kind["usage"]["tokens_in"], by_kind["usage"]["tokens_out"]) == (10, 5)
    assert by_kind["usage"]["cost_usd"] is None  # fixture model is not in the price table
    assert by_kind["tool_use"]["tokens_in"] is None
    blob = json.dumps(events) + json.dumps(tail.snapshot()) + caplog.text
    with store.opened(config.store_path()) as conn:
        blob += "\n".join(conn.iterdump())
    for secret in (SECRET_TEXT, SECRET_INPUT, SECRET_OUTPUT):
        assert secret not in blob


def test_known_model_gets_an_api_equivalent_cost(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()
    record = assistant(message="msg_cost")
    record["message"]["model"] = "claude-sonnet-5-5"
    append(path, record)
    usage = wait_until(lambda: [e for e in tail.events_since(0) if e["kind"] == "usage"])
    from easy_personal.core import prices

    assert usage[0]["cost_usd"] == prices.load().cost("claude-sonnet-5-5", 10, 5, 2, 3) > 0


def test_stop_ends_the_thread_and_is_repeatable(projects):
    tail = live.LiveTail()
    tail.start()
    assert live.current() is tail
    assert any(t.name == live.THREAD_NAME for t in threading.enumerate())
    tail.stop()
    tail.stop()
    assert not any(t.name == live.THREAD_NAME for t in threading.enumerate())
    assert live.current() is None


def test_a_replaced_file_is_read_from_the_start(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant(), assistant())
    tail = start()
    path.unlink()
    append(path, assistant(tools=("Glob",), message="msg_new"))  # new inode, shorter than the old offset
    got = wait_until(lambda: [e for e in tail.events_since(0) if e["tool"] == "Glob"])
    assert len(got) == 1


def test_a_truncated_file_is_read_from_the_start(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant(), assistant(), assistant())
    tail = start()
    path.write_text(json.dumps(assistant(tools=("Glob",), message="msg_trunc")) + "\n")  # same inode, smaller
    got = wait_until(lambda: [e for e in tail.events_since(0) if e["tool"] == "Glob"])
    assert len(got) == 1


def test_a_vanished_file_and_garbage_lines_do_not_stop_the_tail(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()
    append(path, raw="not json at all\n\n[1, 2]\n")
    append(projects / "demo" / "gone.jsonl", assistant())
    (projects / "demo" / "gone.jsonl").unlink()
    append(path, assistant(tools=("Bash",)))
    assert wait_until(lambda: [e for e in tail.events_since(0) if e["tool"] == "Bash"])


def test_two_accounts_are_followed_and_labelled(tmp_path, monkeypatch):
    agent_root = Path(config.agent_root())
    monkeypatch.delenv(config.CLAUDE_PROJECTS_ENV, raising=False)
    for name in (".claude", ".claude-work"):
        (agent_root / name / "projects").mkdir(parents=True)
    with store.opened(config.store_path()) as conn:
        accounts.detect(conn)
        accounts.confirm(conn, str(agent_root / ".claude-work"), "Work")
        claude_code.ingest_accounts(conn)
    tail = live.LiveTail()
    tail.start()
    try:
        append(agent_root / ".claude" / "projects" / "a" / "s-home.jsonl", assistant("s-home", ("Bash",)))
        append(agent_root / ".claude-work" / "projects" / "b" / "s-work.jsonl", assistant("s-work", ("Read",)))
        def by_session():
            found = {e["session_id"]: e["account"] for e in tail.events_since(0) if e["tool"]}
            return found if len(found) == 2 else None

        got = wait_until(by_session)
        assert got == {"s-home": accounts.DEFAULT_LABEL, "s-work": "Work"}
        tagged = store_rows(
            "SELECT e.session_id, a.label FROM events e JOIN agent_accounts a ON a.id = e.agent_account_id"
            " WHERE e.session_id IN ('s-home', 's-work') GROUP BY 1, 2 ORDER BY 1"
        )
        assert tagged == [("s-home", accounts.DEFAULT_LABEL), ("s-work", "Work")]
    finally:
        tail.stop()


def test_ids_are_monotonic_and_events_since_reports_a_rolled_buffer(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start(buffer_size=4)
    assert tail.rolled_past(0) is False  # nothing happened yet
    append(path, *(assistant(tools=("Bash",)) for _ in range(6)))
    assert wait_until(lambda: tail.last_id >= 12)  # 6 records x (1 tool_use + 1 usage)
    first_kept = tail.events_since(0)[0]["id"]
    assert tail.rolled_past(0) is True and tail.rolled_past(first_kept - 2) is True
    assert tail.rolled_past(first_kept - 1) is False  # the next id is still buffered
    assert tail.rolled_past(tail.last_id) is False


def test_snapshot_totals_count_a_message_once_and_cover_active_sessions_only(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()
    # one message written as two records (thinking/tool_use blocks): same message id, growing output
    append(path, assistant(message="msg_dup", tools=(), out=3), assistant(message="msg_dup", tools=("Bash",), out=9))
    append(path, assistant(session="s-old", ts=now_ts(-3600), message="msg_old"))
    append(path, tool_result())
    assert wait_until(lambda: tail.snapshot()["sessions"] and tail.last_id >= 5)
    snap = tail.snapshot()
    assert snap["active_window_s"] == 300 and snap["last_id"] == tail.last_id
    [session] = snap["sessions"]
    assert session["session_id"] == "s1" and session["account"] and session["project"] == "demo-app"
    assert session["tool_calls"] == 1
    assert (session["tokens_in"], session["tokens_out"]) == (10, 9)  # max of the two records, once
    assert session["last_ts"].endswith("Z")
    assert set(session) == {"session_id", "account", "project", "last_ts", "tool_calls", "tokens_in",
                            "tokens_out", "cache_creation", "cache_read", "cost_usd"}  # fmt: skip
    usage_events = [e for e in tail.events_since(0) if e["kind"] == "usage" and e["session_id"] == "s1"]
    assert len(usage_events) == 1


def test_idle_sessions_are_forgotten_so_memory_stays_bounded(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()
    append(path, assistant(session="s-stale", ts=now_ts(-2 * live.FORGET_AFTER_S)))
    append(path, assistant(session="s-fresh"))
    assert wait_until(lambda: tail.last_id >= 4)
    assert set(tail._sessions) == {"s-fresh"}  # the stale one was dropped as soon as it was seen


def test_cli_dashboard_runs_the_tail_around_serve_and_stops_it(projects, monkeypatch):
    from easy_personal.surfaces import api

    seen = {}

    def serve():
        seen["tail"] = live.current()
        seen["alive"] = any(t.name == live.THREAD_NAME for t in threading.enumerate())

    monkeypatch.setattr(api, "serve", serve)
    assert cli.main(["dashboard"]) == 0
    assert seen["tail"] is not None and seen["alive"]
    assert live.current() is None
    assert not any(t.name == live.THREAD_NAME for t in threading.enumerate())


def test_cli_dashboard_stops_the_tail_when_serve_raises(projects, monkeypatch):
    from easy_personal.surfaces import api

    def serve():
        raise RuntimeError("boom")

    monkeypatch.setattr(api, "serve", serve)
    with pytest.raises(RuntimeError):
        cli.main(["dashboard"])
    assert live.current() is None
    assert not any(t.name == live.THREAD_NAME for t in threading.enumerate())
