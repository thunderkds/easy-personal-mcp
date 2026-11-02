"""Live tail (FR-043): follows every confirmed Claude account's transcripts while the dashboard runs.

One thread polls each account's `projects/` tree about every 100 ms and reads only complete new lines
from the offsets the store already holds (the scheduled ingest and the tail share one cursor). Lines
go through `store.save_file`, the normal ingest path, with the single-writer lock taken only for that
one short batch and never kept between batches, so the scheduled ingest still runs. The same lines
become metadata events in a bounded in-memory buffer that the SSE route (next task) reads.

Read-only on every source (NFR-003). Nothing is written to any Claude config. An event holds names,
ids, numbers and timestamps only: message text, tool input and tool output are never read (FR-002).
"""

import contextlib
import itertools
import json
import logging
import os
import sqlite3
import threading
import time
from collections import deque
from collections.abc import Callable, Iterator
from datetime import UTC, datetime, timedelta
from pathlib import Path

from easy_personal.core import accounts, config, discovery, lock, prices, rules, store
from easy_personal.sources import claude_code

log = logging.getLogger(__name__)

POLL_SECONDS = 0.1
BUFFER_SIZE = 2000
ACTIVE_WINDOW_S = 300  # FR-044: a session is active when it wrote a record in the last 5 minutes
FORGET_AFTER_S = 3600  # an idle session's running totals are dropped after an hour (bounded memory)
MAX_BATCH_LINES = 500  # one lock hold = at most this many lines of one file
ROOTS_REFRESH_S = 1.0
OPEN_CALL_MAX_S = 24 * 3600  # an open call older than a day is dropped (the session ended without a result)
WAITING_TOOLS = frozenset({"AskUserQuestion", "ExitPlanMode"})  # an open one waits for the owner (FR-049)
THREAD_NAME = "easy-personal-live-tail"
_STAMP = "%Y-%m-%dT%H:%M:%S.%fZ"

_current: "LiveTail | None" = None


def current() -> "LiveTail | None":
    """The tail running in this process (started by `easy-personal dashboard`), else None."""
    return _current


def _jsonl_files(root: Path) -> Iterator[tuple[str, int, int]]:
    """(path, inode, size) of every *.jsonl under `root`: main files and `subagents/` files alike."""
    stack = [str(root)]
    while stack:
        try:
            scan = os.scandir(stack.pop())
        except OSError:
            continue
        with scan:
            for entry in scan:
                try:
                    if entry.is_dir(follow_symlinks=False):
                        stack.append(entry.path)
                    elif entry.name.endswith(".jsonl") and entry.is_file():
                        info = entry.stat()
                        yield entry.path, info.st_ino, info.st_size
                except OSError:
                    continue  # vanished between listing and stat


def _item(record, parsed: dict) -> dict | None:
    """Metadata of one decoded record for the buffer, or None when it has no session or time.

    Only `parse_record`'s label-checked fields and, from content blocks, `type`, `id`, `tool_use_id`,
    `name` and `is_error` are read: the blocks' `content`, `input` and `text` are never touched. A block
    id (`block_id`) is kept to pair a call with its result in memory; it never reaches a buffered event.
    """
    session, ts = parsed["session_id"], parsed["ts_utc"]
    if parsed["abstained"] or session is None or ts is None:
        return None
    path = parsed["project_path"]
    base = {
        "session_id": session,
        "project": (Path(path).name or path) if path else None,
        "ts": ts,
        "is_sidechain": bool(parsed["is_sidechain"]),
        "message_id": parsed["message_id"],
    }
    message = record.get("message") if parsed["record_type"] in ("assistant", "user") else None
    content = message.get("content") if isinstance(message, dict) else None
    events = []
    for block in content if isinstance(content, list) else []:
        if not isinstance(block, dict):
            continue
        if block.get("type") == "tool_use" and parsed["record_type"] == "assistant":
            if name := claude_code._label(block.get("name")):
                events.append(
                    {"kind": "tool_use", "tool": name, "is_error": False, "block_id": claude_code._label(block.get("id"))}
                )
        elif block.get("type") == "tool_result" and parsed["record_type"] == "user":
            events.append(
                {
                    "kind": "tool_result",
                    "tool": None,
                    "is_error": block.get("is_error") is True,
                    "block_id": claude_code._label(block.get("tool_use_id")),
                }
            )
    usage = None
    if parsed["message_id"] and parsed["input_tokens"] is not None:
        usage = (
            parsed["message_id"],
            parsed["model"],
            parsed["input_tokens"],
            parsed["output_tokens"],
            parsed["cache_creation_input_tokens"],
            parsed["cache_read_input_tokens"],
            bool(parsed["is_api_error"]),
        )
    return {**base, "events": events, "usage": usage}


class _Call:
    """An open call: the tool, the assistant message that made it and when (memory only)."""

    __slots__ = ("message_id", "tool", "ts")

    def __init__(self, tool: str, message_id: str | None, ts: str):
        self.tool, self.message_id, self.ts = tool, message_id, ts


class _Stream:
    """One transcript file of a session: its open calls (by block id) and its run of failed results."""

    def __init__(self):
        self.is_sidechain = False
        self.open: dict[str, _Call] = {}
        self.streak = 0
        self.streak_tool: str | None = None
        self.streak_since = ""


class _Session:
    """Running totals of one session since the tail started; a message counts once (its largest figures)."""

    def __init__(self, account: str):
        self.account = account
        self.project: str | None = None
        self.last_ts = ""
        self.tool_calls = 0
        self.messages: dict[str, tuple[int, int, int, int, float | None]] = {}
        self.streams: dict[str, _Stream] = {}  # by transcript file path: subagents share the session id
        self.attention: list[dict] = []

    @property
    def has_open_call(self) -> bool:
        return any(stream.open for stream in self.streams.values())


class LiveTail:
    """Background tail plus a bounded buffer of recent events with monotonically increasing ids."""

    def __init__(
        self, poll_s: float = POLL_SECONDS, buffer_size: int = BUFFER_SIZE, clock: Callable[[], datetime] | None = None
    ):
        self.poll_s = poll_s
        self._clock = clock or (lambda: datetime.now(UTC))
        try:
            bounds = rules.live_thresholds(config.config_path())
        except ValueError as exc:  # the warnings surface reports a bad config; the tail keeps the defaults
            log.warning("live tail uses default attention bounds: %s", type(exc).__name__)
            bounds = rules.LIVE_DEFAULTS
        self.failing_streak = int(bounds["live_failing_streak"])
        self.open_call_minutes = bounds["live_open_call_minutes"]
        self._state_version = 0
        self._warned: set[str] = set()
        self.buffer_size = buffer_size
        self._mutex = threading.Lock()
        self._buffer: deque[dict] = deque(maxlen=buffer_size)
        self._ids = itertools.count(1)
        self._last_id = 0
        self._sessions: dict[str, _Session] = {}
        self._prices = prices.load()
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None
        self._store_path, self._lock_path = config.store_path(), config.data_dir() / "ingest.lock"
        self._projects: dict[str, str] = {}
        self._roots: list[tuple[int | None, Path]] = []
        self._labels: dict[int | None, str] = {}
        self._roots_at = float("-inf")
        self._last_error = ""

    # -- lifecycle ---------------------------------------------------------------------------------

    def start(self) -> "LiveTail":
        global _current
        if self._thread is None:
            self._thread = threading.Thread(target=self._run, name=THREAD_NAME, daemon=True)
            self._thread.start()
            _current = self
        return self

    def stop(self) -> None:
        """Stop the thread and wait for it; safe to call twice."""
        global _current
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=10)
            self._thread = None
        if _current is self:
            _current = None

    def _run(self) -> None:
        try:
            conn = store.connect(self._store_path)
        except Exception as exc:  # noqa: BLE001 - no store, no tail; the dashboard keeps serving
            log.warning("live tail cannot open the store: %s", type(exc).__name__)
            return
        with contextlib.closing(conn):
            while not self._stop.wait(self.poll_s):
                self._guarded_poll(conn)

    def _guarded_poll(self, conn: sqlite3.Connection) -> None:
        try:
            self.poll_once(conn)
            self._last_error = ""
        except Exception as exc:  # noqa: BLE001 - a bad poll must not end the tail; log the type only
            if type(exc).__name__ != self._last_error:
                self._last_error = type(exc).__name__
                log.warning("live tail poll failed: %s", self._last_error)

    # -- buffer API (read by the SSE route) --------------------------------------------------------

    @property
    def last_id(self) -> int:
        """Id of the newest event so far (0: none yet). Ids never restart while the process runs."""
        with self._mutex:
            return self._last_id

    @property
    def state_version(self) -> int:
        """Counts changes of any session's attention states; an ordinary tool call never moves it."""
        with self._mutex:
            return self._state_version

    def events_since(self, last_id: int) -> list[dict]:
        """Buffered events with `id > last_id`, oldest first (copies)."""
        with self._mutex:
            return [dict(e) for e in self._buffer if e["id"] > last_id]

    def rolled_past(self, last_id: int) -> bool:
        """True when events after `last_id` were already dropped from the buffer (send a fresh snapshot)."""
        with self._mutex:
            return bool(self._buffer) and last_id + 1 < self._buffer[0]["id"]

    def snapshot(self, now: datetime | None = None) -> dict:
        """Active sessions with running totals, and the `last_id` those totals include (one atomic read).

        A session with an open call is listed even when it wrote nothing for 5 minutes (FR-044).
        """
        cutoff = ((now or self._clock()) - timedelta(seconds=ACTIVE_WINDOW_S)).strftime(_STAMP)
        with self._mutex:
            sessions = []
            for session_id, s in self._sessions.items():
                if s.last_ts < cutoff and not s.has_open_call:
                    continue
                figures = list(s.messages.values())
                costs = [f[4] for f in figures]
                sessions.append(
                    {
                        "session_id": session_id,
                        "account": s.account,
                        "project": s.project or "(unknown)",
                        "last_ts": s.last_ts,
                        "tool_calls": s.tool_calls,
                        "tokens_in": sum(f[0] for f in figures),
                        "tokens_out": sum(f[1] for f in figures),
                        "cache_creation": sum(f[2] for f in figures),
                        "cache_read": sum(f[3] for f in figures),
                        "cost_usd": None if None in costs else sum(costs),  # one unpriced model: abstain
                        "attention": [dict(a) for a in s.attention],
                    }
                )
            sessions.sort(key=lambda s: s["last_ts"], reverse=True)
            return {
                "active_window_s": ACTIVE_WINDOW_S,
                "last_id": self._last_id,
                "state_version": self._state_version,
                "open_call_minutes": self.open_call_minutes,
                "sessions": sessions,
            }

    # -- polling -----------------------------------------------------------------------------------

    def poll_once(self, conn: sqlite3.Connection | None = None) -> None:
        """One scan + read pass over every account's tree (the thread calls this every `poll_s`)."""
        if conn is None:
            with store.opened(config.store_path()) as own:
                return self.poll_once(own)
        try:
            self._scan(conn)
        finally:
            self._evaluate()  # the time-based states move even when no line arrived

    def _scan(self, conn: sqlite3.Connection) -> None:
        self._refresh_roots(conn)
        known = store.file_offsets(conn, claude_code.SOURCE)
        for account_id, root in self._roots:
            label = self._labels.get(account_id, accounts.DEFAULT_LABEL)
            for path, inode, size in _jsonl_files(root):
                state = known.get(path)
                offset = state[1] if state and state[0] == inode and state[1] <= size else 0
                if size <= offset:
                    continue
                if self._stop.is_set():
                    return
                try:
                    if not self._follow(conn, account_id, label, path, inode, offset, state):
                        return  # lock busy: the scheduled ingest is writing; try again next poll
                except FileNotFoundError:
                    continue  # removed by cleanup mid-read

    def _refresh_roots(self, conn: sqlite3.Connection) -> None:
        if time.monotonic() - self._roots_at < ROOTS_REFRESH_S:
            return
        self._roots_at = time.monotonic()
        self._roots = claude_code.account_roots(conn)
        labels = {i: label for i, label in conn.execute("SELECT id, label FROM agent_accounts")}
        self._labels = {**labels, None: labels.get(accounts.default_id(conn)) or accounts.DEFAULT_LABEL}

    def _project_of(self, cwd: str) -> str:
        if cwd not in self._projects:
            self._projects[cwd] = discovery.resolve_project(cwd)
        return self._projects[cwd]

    def _follow(self, conn, account_id, label, path, inode, offset, state) -> bool:
        """Read and store a file's new complete lines in short batches; False when the lock was busy."""
        while True:
            rows, items = [], []
            for line_offset, line, next_offset in itertools.islice(
                claude_code.read_complete_lines(Path(path), offset), MAX_BATCH_LINES
            ):
                if not line.strip():
                    rows.append((line_offset, next_offset, None))
                    continue
                try:
                    record = json.loads(line)
                except ValueError:
                    record = None
                parsed = claude_code.parse_record(record, self._project_of)
                rows.append((line_offset, next_offset, parsed))
                if isinstance(record, dict) and (item := _item(record, parsed)):
                    item["path"] = path
                    items.append(item)
            if not rows:
                return True
            try:
                with lock.ingest_lock(self._lock_path, 0):  # held for this batch only
                    if store.file_offsets(conn, claude_code.SOURCE).get(path) != state:
                        return True  # the scheduled ingest moved the cursor first: read again next poll
                    store.save_file(conn, claude_code.SOURCE, path, inode, offset, rows, account_id)
            except lock.LockBusy:
                return False
            self._publish(label, items)
            offset = rows[-1][1]
            state = (inode, offset)
            if len(rows) < MAX_BATCH_LINES:
                return True

    # -- buffer writes -----------------------------------------------------------------------------

    def _publish(self, account: str, items: list[dict]) -> None:
        """Append the stored lines' events and update running totals, in one step for readers."""
        with self._mutex:
            for item in items:
                session = self._sessions.setdefault(item["session_id"], _Session(account))
                session.last_ts = max(session.last_ts, item["ts"])
                session.project = item["project"] or session.project
                stream = session.streams.setdefault(item["path"], _Stream())
                stream.is_sidechain = item["is_sidechain"] or "subagents" in Path(item["path"]).parts
                self._close_on_new_message(stream, item)
                for event in item["events"]:
                    session.tool_calls += event["kind"] == "tool_use"
                    event = self._pair(stream, item, event)
                    self._append(account, item, event)
                if item["usage"]:
                    self._count_usage(session, account, item)
        self._evaluate()

    def _warn_once(self, kind: str) -> None:
        if kind not in self._warned:
            self._warned.add(kind)
            log.warning("live tail: a %s block has no id, so its call is not tracked (format changed?)", kind)

    def _close_on_new_message(self, stream: _Stream, item: dict) -> None:
        """A later assistant message means the tool results of earlier ones were read (or skipped by the ingest)."""
        message_id = item["message_id"]
        if message_id:
            for block_id in [k for k, call in stream.open.items() if call.message_id != message_id]:
                del stream.open[block_id]

    def _pair(self, stream: _Stream, item: dict, event: dict) -> dict:
        """Open a call or close it by block id; keep the failure streak. Returns the event for the buffer."""
        block_id = event["block_id"]
        if block_id is None:
            self._warn_once(event["kind"])
            return event
        if event["kind"] == "tool_use":
            stream.open[block_id] = _Call(event["tool"], item["message_id"], item["ts"])
            return event
        call = stream.open.pop(block_id, None)
        tool = call.tool if call else None  # None: the call began before the tail did, or the tail skipped it
        if event["is_error"]:
            if not stream.streak:
                stream.streak_since, stream.streak_tool = item["ts"], tool
            elif stream.streak_tool != tool:
                stream.streak_tool = None  # mixed tools: the page must not name just the last one
            stream.streak += 1
        else:
            stream.streak = 0
        return {**event, "tool": tool}

    def _evaluate(self) -> None:
        """Drop expired calls and idle sessions, then work out each session's attention states.

        O(sessions + open calls). `state_version` moves only when a session's attention list changes.
        """
        now = self._clock()

        def before(seconds: float) -> str:
            return (now - timedelta(seconds=seconds)).strftime(_STAMP)

        call_horizon, forget_horizon = before(OPEN_CALL_MAX_S), before(FORGET_AFTER_S)
        quiet_cutoff = before(self.open_call_minutes * 60)
        with self._mutex:
            for session_id, session in list(self._sessions.items()):
                for stream in session.streams.values():
                    for block_id in [k for k, call in stream.open.items() if call.ts < call_horizon]:
                        del stream.open[block_id]
                if session.last_ts < forget_horizon and not session.has_open_call:
                    del self._sessions[session_id]
                    continue
                states = self._attention(session, quiet_cutoff)
                if states != session.attention:
                    session.attention = states
                    self._state_version += 1

    def _attention(self, session: _Session, quiet_cutoff: str) -> list[dict]:
        failing, waiting, others = [], [], []
        for stream in session.streams.values():
            if stream.streak >= max(self.failing_streak, 1):
                failing.append(
                    {"state": "failing", "tool": stream.streak_tool, "count": stream.streak,
                     "is_sidechain": stream.is_sidechain, "since": stream.streak_since}
                )  # fmt: skip
            calls = sorted(stream.open.values(), key=lambda c: c.ts)
            asking = [c for c in calls if c.tool in WAITING_TOOLS]
            if asking:
                waiting.append(
                    {"state": "waiting", "tool": asking[0].tool, "count": len(asking),
                     "is_sidechain": stream.is_sidechain, "since": asking[0].ts}
                )  # fmt: skip
            others += [(c, stream.is_sidechain) for c in calls if c.tool not in WAITING_TOOLS]
        states = sorted(failing, key=lambda a: a["since"]) + sorted(waiting, key=lambda a: a["since"])
        if others and session.last_ts <= quiet_cutoff:
            oldest, is_sidechain = min(others, key=lambda pair: pair[0].ts)
            states.append(
                {"state": "needs_attention", "tool": oldest.tool, "count": len(others),
                 "is_sidechain": is_sidechain, "since": oldest.ts}
            )  # fmt: skip
        return states

    def _count_usage(self, session: _Session, account: str, item: dict) -> None:
        message_id, model, *counts, api_error = item["usage"]
        before = session.messages.get(message_id)
        if before is None:  # a message is written as several records: the first one is a usage event
            cost = self._prices.cost(model, *counts)
            self._append(account, item, {"kind": "usage", "tool": None, "is_error": api_error}, (*counts, cost))
            session.messages[message_id] = (*counts, cost)
            return
        largest = [max(a, b) for a, b in zip(counts, before[:4], strict=True)]
        if largest == list(before[:4]):
            return  # a repeat that grew nothing: no event
        cost = self._prices.cost(model, *largest)
        # The page sums snapshot + live events, so growth is an event carrying the difference (cost: None if unpriced).
        delta = [new - old for new, old in zip(largest, before[:4], strict=True)]
        delta_cost = None if cost is None or before[4] is None else cost - before[4]
        self._append(account, item, {"kind": "usage", "tool": None, "is_error": api_error}, (*delta, delta_cost))
        session.messages[message_id] = (*largest, cost)

    def _append(self, account: str, item: dict, event: dict, tokens=(None, None, None, None, None)) -> None:
        self._last_id = next(self._ids)
        self._buffer.append(
            {
                "id": self._last_id,
                "session_id": item["session_id"],
                "account": account,
                "project": item["project"] or "(unknown)",
                "ts": item["ts"],
                "kind": event["kind"],
                "tool": event["tool"],
                "is_error": event["is_error"],
                "is_sidechain": item["is_sidechain"],
                "tokens_in": tokens[0],
                "tokens_out": tokens[1],
                "cache_creation": tokens[2],
                "cache_read": tokens[3],
                "cost_usd": tokens[4],
            }
        )
