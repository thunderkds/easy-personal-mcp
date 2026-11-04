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


def assistant(session="s1", tools=("Bash",), message=None, ts=None, sidechain=False, out=5, use_ids=None):
    """One assistant record: usage plus one tool_use block per tool name (planted secrets inside)."""
    blocks = [{"type": "text", "text": SECRET_TEXT}]
    blocks += [
        {"type": "tool_use", "id": (use_ids[i] if use_ids else f"toolu_{next(_ids)}"), "name": t,
         "input": {"command": f"ls {SECRET_INPUT}"}}
        for i, t in enumerate(tools)
    ]  # fmt: skip
    return {
        "type": "assistant", "version": "2.1.316", "cwd": "/work/demo-app", "sessionId": session,
        "isSidechain": sidechain, "entrypoint": "cli", "timestamp": ts or now_ts(),
        "message": {
            "id": message or f"msg_{next(_ids)}", "model": "claude-fixture-model-1", "content": blocks,
            "usage": {"input_tokens": 10, "output_tokens": out, "cache_creation_input_tokens": 2,
                      "cache_read_input_tokens": 3},
        },
    }  # fmt: skip


def tool_result(session="s1", error=False, ts=None, use_id="toolu_x", sidechain=False):
    return {
        "type": "user", "version": "2.1.316", "cwd": "/work/demo-app", "sessionId": session,
        "isSidechain": sidechain, "entrypoint": "cli", "timestamp": ts or now_ts(),
        "message": {"content": [{"type": "tool_result", "tool_use_id": use_id, "is_error": error,
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
    append(path, assistant(session="s-old", tools=(), ts=now_ts(-3600), message="msg_old"))
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
                            "tokens_out", "cache_creation", "cache_read", "cost_usd", "attention"}  # fmt: skip
    assert session["attention"] == []
    usage_events = [e for e in tail.events_since(0) if e["kind"] == "usage" and e["session_id"] == "s1"]
    assert [e["tokens_out"] for e in usage_events] == [3, 6]  # first sight, then the growth (delta)


def _sums(events):
    keys = ("tokens_in", "tokens_out", "cache_creation", "cache_read")
    return {k: sum(e[k] for e in events if e["kind"] == "usage") for k in keys}


def test_usage_delta_events_keep_live_totals_equal_to_the_snapshot(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()

    def message(out, cache_read=3, model="claude-sonnet-5-5"):
        record = assistant(message="msg_grow", tools=(), out=out)
        record["message"]["model"] = model
        record["message"]["usage"]["cache_read_input_tokens"] = cache_read
        return record

    # one message written as several records: figures grow, one repeat changes nothing, then a new message
    for records in ([message(3)], [message(7, cache_read=9)], [message(7, cache_read=9)], [message(12, cache_read=9)]):
        append(path, *records)
        time.sleep(0.15)
    other = assistant(message="msg_other", tools=(), out=4)
    other["message"]["model"] = "claude-sonnet-5-5"
    append(path, other)
    assert wait_until(lambda: tail.last_id >= 4)
    snap = tail.snapshot()
    events = [e for e in tail.events_since(0) if e["id"] <= snap["last_id"]]
    usage = [e for e in events if e["kind"] == "usage"]
    assert [e["tokens_out"] for e in usage if e["tokens_out"]] == [3, 4, 5, 4]  # 3, +4, (repeat: nothing), +5, other 4
    [session] = snap["sessions"]
    totals = _sums(events)
    assert (session["tokens_in"], session["tokens_out"], session["cache_creation"], session["cache_read"]) == (
        totals["tokens_in"], totals["tokens_out"], totals["cache_creation"], totals["cache_read"]
    )  # fmt: skip
    assert session["tokens_out"] == 12 + 4 and session["cache_read"] == 9 + 3
    assert sum(e["cost_usd"] for e in usage) == pytest.approx(session["cost_usd"])
    assert all(e["cost_usd"] is not None for e in usage)


def test_unpriced_model_growth_keeps_cost_unknown_in_deltas_and_snapshot(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()
    append(path, assistant(message="msg_u", tools=(), out=2))
    time.sleep(0.2)
    append(path, assistant(message="msg_u", tools=(), out=9))
    assert wait_until(lambda: tail.last_id >= 2)
    snap = tail.snapshot()
    usage = [e for e in tail.events_since(0) if e["kind"] == "usage"]
    assert [e["tokens_out"] for e in usage] == [2, 7] and all(e["cost_usd"] is None for e in usage)
    assert snap["sessions"][0]["tokens_out"] == 9 and snap["sessions"][0]["cost_usd"] is None


def test_idle_sessions_are_forgotten_so_memory_stays_bounded(projects, start):
    path = projects / "demo" / "s1.jsonl"
    append(path, assistant())
    tail = start()
    append(path, assistant(session="s-stale", tools=(), ts=now_ts(-2 * live.FORGET_AFTER_S)))
    append(path, assistant(session="s-fresh"))
    assert wait_until(lambda: tail.last_id >= 3)
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


# -- attention states (HUN-181, FR-047..049, NFR-012) ------------------------------------------------


class Clock:
    """A settable clock; record timestamps come from it so the 10-minute and 24-hour rules need no sleeping."""

    def __init__(self):
        self.at = datetime.now(UTC)

    def __call__(self):
        return self.at

    def stamp(self, plus_s: float = 0.0) -> str:
        return (self.at + timedelta(seconds=plus_s)).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"

    def advance(self, **kwargs):
        self.at += timedelta(**kwargs)


@pytest.fixture
def clock():
    return Clock()


@pytest.fixture
def manual(projects, clock):
    """A tail with a fake clock that the test drives with `poll_once()`; the files exist (ingested) first."""

    def _make():
        with store.opened(config.store_path()) as conn:
            claude_code.ingest_accounts(conn)
        return live.LiveTail(clock=clock)

    return _make


MAIN = "demo/s1.jsonl"
SUB = "demo/s1/subagents/agent-a1.jsonl"


def use(clock, ids, tools=("Bash",), message=None, session="s1", sidechain=False, plus_s=0.0):
    return assistant(session=session, tools=tools, use_ids=ids, message=message, ts=clock.stamp(plus_s), sidechain=sidechain)


def result(clock, use_id, error=False, session="s1", sidechain=False, plus_s=0.0):
    return tool_result(session=session, error=error, use_id=use_id, ts=clock.stamp(plus_s), sidechain=sidechain)


def attention(tail, session="s1"):
    [s] = [s for s in tail.snapshot(now=tail._clock())["sessions"] if s["session_id"] == session]
    return s["attention"]


def seeded(projects, manual, clock, *paths):
    """Files holding one old record each (what exists before the dashboard starts), then the tail."""
    for rel in paths:
        append(projects / rel, assistant(tools=(), ts=clock.stamp(), sidechain="subagents" in rel))
    return manual()


def test_a_result_carries_the_tool_name_of_its_use_across_polls_and_unknown_ids_stay_null(projects, manual, clock):
    tail = seeded(projects, manual, clock, MAIN)
    append(projects / MAIN, use(clock, ["toolu_A", "toolu_B"], tools=("Bash", "Read"), message="msg_1"))
    tail.poll_once()
    append(projects / MAIN, result(clock, "toolu_B"), result(clock, "toolu_A", error=True), result(clock, "toolu_old"))
    tail.poll_once()
    results = [(e["tool"], e["is_error"]) for e in tail.events_since(0) if e["kind"] == "tool_result"]
    assert results == [("Read", False), ("Bash", True), (None, False)]
    assert all(set(e) == EVENT_KEYS for e in tail.events_since(0))  # no block id in the event


def test_streams_are_separate_three_failures_in_a_subagent_file_mark_failing_and_a_main_success_keeps_it(
    projects, manual, clock
):
    tail = seeded(projects, manual, clock, MAIN, SUB)
    append(projects / SUB, *[r for i in range(3) for r in (use(clock, [f"sub_{i}"], sidechain=True, message=f"m_{i}"),
                                                           result(clock, f"sub_{i}", error=True, sidechain=True))])
    append(projects / MAIN, use(clock, ["main_1"]), result(clock, "main_1"))
    tail.poll_once()
    assert attention(tail) == [
        {"state": "failing", "tool": "Bash", "count": 3, "is_sidechain": True, "since": attention(tail)[0]["since"]}
    ]
    assert attention(tail)[0]["since"].endswith("Z")


def test_three_failures_in_a_row_mark_failing_and_the_next_success_clears_it(projects, manual, clock):
    tail = seeded(projects, manual, clock, MAIN)
    for i in range(2):
        append(projects / MAIN, use(clock, [f"t{i}"], tools=("Grep",), message=f"m{i}"), result(clock, f"t{i}", error=True))
    tail.poll_once()
    assert attention(tail) == []  # two are not enough
    append(projects / MAIN, use(clock, ["t2"], tools=("Grep",), message="m2"), result(clock, "t2", error=True))
    tail.poll_once()
    [item] = attention(tail)
    assert (item["state"], item["tool"], item["count"], item["is_sidechain"]) == ("failing", "Grep", 3, False)
    append(projects / MAIN, use(clock, ["t3"], message="m3"), result(clock, "t3"))
    tail.poll_once()
    assert attention(tail) == []


def test_a_streak_of_different_tools_names_no_tool(projects, manual, clock):
    tail = seeded(projects, manual, clock, MAIN)
    for i, tool in enumerate(("Bash", "Bash", "Edit")):
        append(projects / MAIN, use(clock, [f"t{i}"], tools=(tool,), message=f"m{i}"), result(clock, f"t{i}", error=True))
    tail.poll_once()
    [item] = attention(tail)
    assert (item["state"], item["tool"], item["count"]) == ("failing", None, 3)  # not "Edit"


def test_unpaired_failed_results_count_in_the_streak_with_a_null_tool(projects, manual, clock):
    tail = seeded(projects, manual, clock, MAIN)
    append(projects / MAIN, *[result(clock, f"before_start_{i}", error=True) for i in range(3)])
    tail.poll_once()
    [item] = attention(tail)
    assert (item["state"], item["tool"], item["count"]) == ("failing", None, 3)


def test_the_failing_bound_comes_from_thresholds_in_the_config(projects, manual, clock):
    config.config_path().write_text("thresholds:\n  live_failing_streak: 5\n")
    tail = seeded(projects, manual, clock, MAIN)
    for i in range(4):
        append(projects / MAIN, use(clock, [f"t{i}"], message=f"m{i}"), result(clock, f"t{i}", error=True))
    tail.poll_once()
    assert attention(tail) == []
    append(projects / MAIN, use(clock, ["t4"], message="m4"), result(clock, "t4", error=True))
    tail.poll_once()
    assert [(a["state"], a["count"]) for a in attention(tail)] == [("failing", 5)]


@pytest.mark.parametrize("tool", ["AskUserQuestion", "ExitPlanMode"])
def test_an_open_question_or_plan_prompt_is_waiting_at_once_and_its_result_clears_it(projects, manual, clock, tool):
    tail = seeded(projects, manual, clock, MAIN)
    append(projects / MAIN, use(clock, ["q1"], tools=(tool,), message="mq"))
    tail.poll_once()
    [item] = attention(tail)
    assert (item["state"], item["tool"], item["count"], item["is_sidechain"]) == ("waiting", tool, 1, False)
    append(projects / MAIN, result(clock, "q1"))
    tail.poll_once()
    assert attention(tail) == []


def test_another_call_open_ten_minutes_in_a_silent_session_needs_attention_and_a_record_clears_it(
    projects, manual, clock
):
    tail = seeded(projects, manual, clock, MAIN)
    append(projects / MAIN, use(clock, ["slow"], tools=("Bash",), message="ms"))
    tail.poll_once()
    clock.advance(minutes=9, seconds=59)
    tail.poll_once()
    assert attention(tail) == []
    clock.advance(seconds=1)
    tail.poll_once()  # the poll loop alone does it: no new line arrived
    [item] = attention(tail)
    assert (item["state"], item["tool"], item["count"]) == ("needs_attention", "Bash", 1)
    append(projects / MAIN, result(clock, "slow"))
    tail.poll_once()
    assert attention(tail) == []


def test_a_thread_started_tail_sets_needs_attention_within_two_seconds(projects, manual, clock):
    tail = seeded(projects, manual, clock, MAIN)
    append(projects / MAIN, use(clock, ["slow"], message="ms"))
    tail.poll_once()
    tail.start()
    try:
        clock.advance(minutes=10)
        assert wait_until(lambda: attention(tail), timeout=2.0)
    finally:
        tail.stop()


def test_a_record_in_another_stream_of_the_session_prevents_needs_attention(projects, manual, clock):
    tail = seeded(projects, manual, clock, MAIN, SUB)
    append(projects / MAIN, use(clock, ["agent_call"], tools=("Agent",), message="ma"))
    tail.poll_once()
    clock.advance(minutes=9)
    append(projects / SUB, assistant(tools=(), ts=clock.stamp(), sidechain=True, message="msub"))  # the subagent works
    tail.poll_once()
    clock.advance(minutes=9)
    tail.poll_once()
    assert attention(tail) == []  # 18 min since the call, 9 since the session last wrote
    clock.advance(minutes=1)
    tail.poll_once()
    assert [a["state"] for a in attention(tail)] == ["needs_attention"]


def test_the_open_call_minutes_come_from_thresholds_and_sit_in_the_snapshot(projects, manual, clock):
    assert manual().snapshot()["open_call_minutes"] == 10
    config.config_path().write_text("thresholds:\n  live_open_call_minutes: 1\n")
    tail = seeded(projects, manual, clock, MAIN)
    assert tail.snapshot()["open_call_minutes"] == 1
    append(projects / MAIN, use(clock, ["slow"], message="ms"))
    tail.poll_once()
    clock.advance(seconds=60)
    tail.poll_once()
    assert [a["state"] for a in attention(tail)] == ["needs_attention"]


def test_a_later_assistant_message_closes_earlier_open_calls_but_parallel_blocks_of_one_message_stay_open(
    projects, manual, clock
):
    tail = seeded(projects, manual, clock, MAIN)
    # one message, two tool_use blocks written as two records (same message id): both stay open
    append(projects / MAIN, use(clock, ["p1"], message="mp"), use(clock, ["p2"], message="mp"))
    tail.poll_once()
    append(projects / MAIN, result(clock, "p1"))
    tail.poll_once()
    session = next(iter(tail._sessions.values()))
    assert len(session.streams[str(projects / MAIN)].open) == 1  # p2 is still open
    append(projects / MAIN, use(clock, ["n1"], message="mn"))  # a new message: p2's result was skipped
    tail.poll_once()
    assert list(session.streams[str(projects / MAIN)].open) == ["n1"]
    clock.advance(minutes=10)
    tail.poll_once()
    [item] = attention(tail)
    assert (item["state"], item["count"]) == ("needs_attention", 1)


def test_a_session_with_an_open_call_stays_listed_past_five_minutes_and_one_hour_and_the_call_is_dropped_at_24_h(
    projects, manual, clock
):
    tail = seeded(projects, manual, clock, MAIN)
    append(projects / MAIN, use(clock, ["slow"], message="ms"))
    append(projects / "demo" / "other.jsonl", assistant(session="s-closed", tools=(), ts=clock.stamp()))
    tail.poll_once()
    clock.advance(seconds=live.ACTIVE_WINDOW_S + 1)
    tail.poll_once()
    ids = {s["session_id"] for s in tail.snapshot(now=clock())["sessions"]}
    assert ids == {"s1"}  # the session with the open call stays; the quiet one without it leaves the list
    clock.advance(seconds=live.FORGET_AFTER_S)
    tail.poll_once()
    assert "s1" in {s["session_id"] for s in tail.snapshot(now=clock())["sessions"]}
    assert "s-closed" not in tail._sessions  # forgotten at 1 h
    clock.advance(hours=23)
    tail.poll_once()
    assert tail.snapshot(now=clock())["sessions"] == []  # the call is older than 24 h: dropped, session idle
    assert "s1" not in tail._sessions


def test_state_version_ignores_ordinary_tool_calls_and_moves_once_per_state_set_change(projects, manual, clock):
    tail = seeded(projects, manual, clock, MAIN)
    before = tail.state_version
    for i in range(100):
        append(projects / MAIN, use(clock, [f"ok{i}"], tools=("Read",), message=f"m{i}"), result(clock, f"ok{i}"))
        tail.poll_once()
    assert tail.state_version == before
    append(projects / MAIN, use(clock, ["q"], tools=("AskUserQuestion",), message="mq"))
    tail.poll_once()
    assert tail.state_version == before + 1
    tail.poll_once()
    assert tail.state_version == before + 1  # a poll that changes nothing
    append(projects / MAIN, result(clock, "q"))
    tail.poll_once()
    assert tail.state_version == before + 2
    assert tail.snapshot()["sessions"][0]["attention"] == []


def test_a_call_or_result_without_a_block_id_shows_no_state_and_is_logged_once_by_type(
    projects, manual, clock, caplog
):
    caplog.set_level(logging.WARNING)
    tail = seeded(projects, manual, clock, MAIN)
    bare_use = use(clock, ["x"], tools=("AskUserQuestion",), message="m1")
    del bare_use["message"]["content"][1]["id"]
    bare_result = result(clock, "x", error=True)
    del bare_result["message"]["content"][0]["tool_use_id"]
    append(projects / MAIN, bare_use, bare_result, bare_use, bare_result, bare_result, bare_result)
    tail.poll_once()
    assert attention(tail) == []
    assert sum("has no id" in r.getMessage() for r in caplog.records) == 2  # one line per block type


# -- several readers of one store (HUN-212, FR-071) --------------------------------------------------


def session_file(n: int) -> str:
    return f"proj{n}/sess{n}.jsonl"


def write_line(clock, projects, n, tool="Bash"):
    """One tool call of session n in its own project (distinct cwd)."""
    record = use(clock, [f"call_{n}_{next(_ids)}"], tools=(tool,), session=f"sess{n}", message=f"msg_{next(_ids)}")
    record["cwd"] = f"/work/app-{n}"
    append(projects / session_file(n), record)


def tool_events(tail, session):
    return [e for e in tail.events_since(0) if e["kind"] == "tool_use" and e["session_id"] == session]


def seed_three(projects, manual, clock):
    """Three sessions in three projects, one old record each, then two tails that start on that store."""
    for n in (1, 2, 3):
        record = assistant(session=f"sess{n}", tools=(), ts=clock.stamp())
        record["cwd"] = f"/work/app-{n}"
        append(projects / session_file(n), record)
    return manual(), manual()


def test_two_tails_on_one_store_each_list_every_session_and_every_tool_event_once_in_the_store(
    projects, manual, clock
):
    first, second = seed_three(projects, manual, clock)
    written = 0
    for _round in range(3):
        for n in (1, 2, 3):
            write_line(clock, projects, n)
            written += 1
            first.poll_once()  # the first tail always wins the store cursor
            second.poll_once()
    for tail in (first, second):
        assert {s["session_id"] for s in tail.snapshot(now=clock())["sessions"]} == {"sess1", "sess2", "sess3"}
        assert [len(tool_events(tail, f"sess{n}")) for n in (1, 2, 3)] == [3, 3, 3]
    assert len(store_rows("SELECT 1 FROM events")) == 3 + written  # 3 seed lines + every new line once


def test_a_line_stored_by_another_caller_between_two_polls_is_still_published_and_not_stored_again(
    projects, manual, clock
):
    tail, _other = seed_three(projects, manual, clock)
    write_line(clock, projects, 1, tool="Read")
    with store.opened(config.store_path()) as conn:
        claude_code.ingest_accounts(conn)  # what the scheduled ingest does
    assert len(store_rows("SELECT 1 FROM events")) == 4
    tail.poll_once()
    assert [e["tool"] for e in tool_events(tail, "sess1")] == ["Read"]
    assert len(store_rows("SELECT 1 FROM events")) == 4  # published, not stored twice
    write_line(clock, projects, 1, tool="Write")  # now past the cursor: stored and published as before
    tail.poll_once()
    assert [e["tool"] for e in tool_events(tail, "sess1")] == ["Read", "Write"]
    assert len(store_rows("SELECT 1 FROM events")) == 5


def test_a_tail_started_with_the_cursor_at_the_end_of_a_file_publishes_nothing_until_a_new_line(
    projects, manual, clock
):
    first, second = seed_three(projects, manual, clock)
    first.poll_once()
    second.poll_once()
    second.poll_once()
    for tail in (first, second):
        assert tail.events_since(0) == [] and tail.snapshot(now=clock())["sessions"] == []  # no history replay
    write_line(clock, projects, 2)
    first.poll_once()
    second.poll_once()
    for tail in (first, second):
        assert len(tool_events(tail, "sess2")) == 1 and tail.last_id == 2  # one tool_use, one usage; no more


def test_a_replaced_file_is_followed_by_both_tails_without_publishing_old_lines_twice(projects, manual, clock):
    first, second = seed_three(projects, manual, clock)
    write_line(clock, projects, 1, tool="Old")
    first.poll_once()
    second.poll_once()
    (projects / session_file(1)).unlink()
    write_line(clock, projects, 1, tool="New")  # new inode, shorter than the old offset
    first.poll_once()
    second.poll_once()
    for tail in (first, second):
        assert [e["tool"] for e in tool_events(tail, "sess1")] == ["Old", "New"]


def test_a_truncated_file_is_followed_by_both_tails_without_an_error_or_a_second_publish(projects, manual, clock):
    first, second = seed_three(projects, manual, clock)
    for _ in range(3):
        write_line(clock, projects, 1, tool="Old")
    first.poll_once()
    second.poll_once()
    (projects / session_file(1)).write_text("")  # same inode, now smaller than both offsets
    write_line(clock, projects, 1, tool="New")
    first.poll_once()
    second.poll_once()
    write_line(clock, projects, 1, tool="Newer")
    first.poll_once()
    second.poll_once()
    for tail in (first, second):
        tools = [e["tool"] for e in tool_events(tail, "sess1")]
        assert tools.count("Old") == 3 and tools.count("Newer") == 1 and tools.count("New") <= 1
    assert tool_events(second, "sess1")[-1]["tool"] == "Newer"

