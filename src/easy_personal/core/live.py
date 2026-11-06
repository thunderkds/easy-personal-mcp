"""Live tail (FR-043): follows every confirmed Claude account's transcripts while the dashboard runs.

One thread polls each account's `projects/` tree about every 100 ms and reads only complete new lines.
The tail keeps its own read offset per file. A file of the tail's first scan starts at the store cursor
(no history replay); a file that appears later is read from its start. A line from the store cursor on goes through `store.save_file`, the normal ingest path, with the
single-writer lock taken only for that one short batch and never kept between batches, so the scheduled
ingest still runs. A line below the cursor was stored by another process (a second dashboard, the
scheduled ingest): it is published and not stored again (FR-071). Every published line becomes a
metadata event in a bounded in-memory buffer that the SSE route reads.

Read-only on every source (NFR-003). Nothing is written to any Claude config. An event holds names,
ids, numbers and timestamps only: message text and tool output are never read (FR-002). Of a tool call's
input, only the fields `_call_label` names are read, and only to show a label on the page (FR-081,
DDR-0004); labels, sub-agent types and the kit stage live in memory and are never stored.
"""

import contextlib
import itertools
import json
import logging
import os
import re
import sqlite3
import threading
import time
from collections import deque
from collections.abc import Callable, Iterator
from datetime import UTC, datetime, timedelta
from pathlib import Path

from easy_personal.core import accounts, config, discovery, jobs, lock, prices, rules, store
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
LABEL_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9:._-]{0,63}$")  # FR-081: anything else is no label
COMMAND_READ_CHARS = 4096  # a Bash command is cut here before its first word is looked for
META_MAX_BYTES = 4096  # FR-082: a sub-agent meta file larger than this is not read
META_WAIT_S = 30  # a meta file still missing or unusable this long after the first look gives SUB_AGENT for good
SUB_AGENT = "sub-agent"
# FR-083: the kit stage of a skill (owner choice 2026-11-06); a skill not listed leaves the stage unchanged.
STAGE_MAP = {
    **dict.fromkeys(("brainstorming", "ideate"), "0.5"),
    **dict.fromkeys(("git-guardrails-claude-code", "map-codebase"), "1"),
    "craft-agent": "1.5",
    **dict.fromkeys(("grill-with-docs", "to-issues"), "2"),
    **dict.fromkeys(("tdd", "bugfix", "diagnose", "craft-spawn-prompt", "migration-safety"), "3"),
    **dict.fromkeys(("code-review", "security-review", "blast-radius", "html-report"), "4"),
    **dict.fromkeys(("verify", "ship", "delivery-report"), "5"),
}
_CD_PREFIX = re.compile(r"""\s*cd\s+(?:"[^"]*"|'[^']*'|[^\s;&|]+)\s*(?:&&|;)\s*""")
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


def _cursor(state: tuple[int, int] | None, inode: int, size: int) -> int:
    """Where the store cursor stands for this file: 0 when the file is unknown, replaced or shorter."""
    return state[1] if state and state[0] == inode and state[1] <= size else 0


def _safe(value) -> str | None:
    """`value` when it is a string that passes the FR-081 label pattern, else None."""
    return value if isinstance(value, str) and LABEL_PATTERN.fullmatch(value) else None


def _bash_label(command) -> str | None:
    """First word of a Bash command after any leading `cd <dir> &&` / `cd <dir>;` (live-only wrapper)."""
    if not isinstance(command, str):
        return None
    command = command[:COMMAND_READ_CHARS]
    while match := _CD_PREFIX.match(command):
        command = command[match.end() :]
    return _safe(jobs.command_name(command))


def _call_label(name: str, tool_input) -> str | dict | None:
    """The FR-081 call label of one `tool_use` block. The only place a tool's input is read.

    Reads `skill` (Skill), `subagent_type` (Agent, Task) or the first word of `command` (Bash); an MCP
    tool is split from its name and its input is not read. Every other field and tool gives no label.
    """
    if name.startswith("mcp__"):
        server, _, tool = name[len("mcp__") :].partition("__")
        server, tool = _safe(server), _safe(tool)
        return {"server": server, "tool": tool} if server and tool else None
    field = {"Skill": "skill", "Agent": "subagent_type", "Task": "subagent_type", "Bash": "command"}.get(name)
    if field is None or not isinstance(tool_input, dict):
        return None
    value = tool_input.get(field)
    return _bash_label(value) if name == "Bash" else _safe(value)


def _meta_agent_type(stream_path: str) -> str | None:
    """`agentType` of `<stream>.meta.json` beside a sub-agent stream file, else None (FR-082).

    Reads at most META_MAX_BYTES of a regular file (a symlink is not followed) and takes that one field.
    """
    meta = stream_path[: -len(".jsonl")] + ".meta.json"
    try:
        fd = os.open(meta, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, "rb") as handle:
            raw = handle.read(META_MAX_BYTES + 1)
        data = json.loads(raw) if len(raw) <= META_MAX_BYTES else None
    except (OSError, ValueError, RecursionError):
        return None
    return _safe(data.get("agentType")) if isinstance(data, dict) else None


def _item(record, parsed: dict) -> dict | None:
    """Metadata of one decoded record for the buffer, or None when it has no session or time.

    Only `parse_record`'s label-checked fields and, from content blocks, `type`, `id`, `tool_use_id`,
    `name` and `is_error` are read: the blocks' `content` and `text` are never touched. A `tool_use`
    block's `input` goes to `_call_label` alone (DDR-0004), which reads `skill`, `subagent_type` or a
    Bash command's first word and nothing else. A block id (`block_id`) is kept to pair a call with its
    result in memory; it never reaches a buffered event.
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
                    {"kind": "tool_use", "tool": name, "is_error": False, "block_id": claude_code._label(block.get("id")),
                     "label": _call_label(name, block.get("input"))}
                )  # fmt: skip
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
        self.sub_agent = False  # a file under `subagents/`: its calls carry an agent type (FR-082)
        self.agent: str | None = None  # settled type: from the meta file, or SUB_AGENT after META_WAIT_S
        self.meta_since: datetime | None = None  # when the meta file was first looked for
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
        self.stage: str | None = None  # FR-083: stage of the newest mapped Skill call, and that skill's bare name
        self.stage_skill: str | None = None
        self.stage_ts = ""

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
        self._offsets: dict[str, tuple[int, int]] = {}  # path -> (inode, next byte this tail has not published)
        self._primed = False  # True once a first full scan has finished (no early return)
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
        """Counts changes of any session's attention states or kit stage; an ordinary tool call never moves it."""
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
                        "stage": s.stage,
                        "stage_skill": s.stage_skill,
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
        seen: set[str] = set()
        for account_id, root in self._roots:
            label = self._labels.get(account_id, accounts.DEFAULT_LABEL)
            for path, inode, size in _jsonl_files(root):
                seen.add(path)
                own = self._offsets.get(path)
                if own is None or own[0] != inode or size < own[1]:
                    # A file of the first scan (history, not live), or a truncated one, starts where the
                    # store cursor stands. A file first seen later, or one replaced by a new inode, is
                    # new content written while the tail runs: read it from byte 0, so a line another
                    # process stored first is still published (never stored twice). Claude Code appends
                    # to transcripts and never rewrites them, so a new inode means a genuinely new file.
                    fresh = self._primed and (own is None or own[0] != inode)
                    own = (inode, 0 if fresh else _cursor(known.get(path), inode, size))
                    self._offsets[path] = own
                if size <= own[1]:
                    continue
                if self._stop.is_set():
                    return
                try:
                    if not self._follow(conn, account_id, label, path, inode, size, known.get(path)):
                        return  # lock busy: the scheduled ingest is writing; try again next poll
                except FileNotFoundError:
                    continue  # removed by cleanup mid-read
        for path in self._offsets.keys() - seen:
            del self._offsets[path]  # the file is gone: memory stays bounded
        self._primed = True  # the files of this first full scan are history; every later file is live

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

    def _follow(self, conn, account_id, label, path, inode, size, state) -> bool:
        """Read a file's new complete lines in short batches; False when the lock was busy.

        The tail's own offset says what it has published; the store cursor says what is stored. A line
        below the cursor was stored by someone else: it is published only. A line from the cursor on is
        stored (lock held for this batch only), then published. A batch starts at the lower of the two.
        """
        own = self._offsets[path][1]
        cursor = _cursor(state, inode, size)
        while True:
            start = min(own, cursor)
            rows, items = [], []
            for line_offset, line, next_offset in itertools.islice(
                claude_code.read_complete_lines(Path(path), start), MAX_BATCH_LINES
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
                if line_offset >= own and isinstance(record, dict) and (item := _item(record, parsed)):
                    item["path"] = path
                    items.append(item)
            if not rows:
                return True
            if rows[-1][1] > cursor:  # some line is not stored yet
                try:
                    with lock.ingest_lock(self._lock_path, 0):  # held for this batch only
                        fresh = store.file_offsets(conn, claude_code.SOURCE).get(path)
                        cursor = _cursor(fresh, inode, os.stat(path).st_size)  # the file may have grown since the scan
                        if cursor < start:
                            return True  # the cursor went back (store reset): read again next poll
                        to_store = [row for row in rows if row[0] >= cursor]
                        if to_store:  # the other process may have stored part of the batch since
                            store.save_file(conn, claude_code.SOURCE, path, inode, to_store[0][0], to_store, account_id)
                            cursor = to_store[-1][1]
                except lock.LockBusy:
                    return False
            self._publish(label, items)
            own = rows[-1][1]
            self._offsets[path] = (inode, own)
            if len(rows) < MAX_BATCH_LINES:
                return True

    # -- buffer writes -----------------------------------------------------------------------------

    def _publish(self, account: str, items: list[dict]) -> None:
        """Append the stored lines' events and update running totals, in one step for readers."""
        tried: set[str] = set()  # sub-agent streams whose meta file was looked for in this step
        with self._mutex:
            for item in items:
                session = self._sessions.setdefault(item["session_id"], _Session(account))
                session.last_ts = max(session.last_ts, item["ts"])
                session.project = item["project"] or session.project
                stream = session.streams.setdefault(item["path"], _Stream())
                stream.sub_agent = "subagents" in Path(item["path"]).parts
                stream.is_sidechain = item["is_sidechain"] or stream.sub_agent
                item["agent"] = self._agent_of(stream, item["path"], tried)
                self._close_on_new_message(stream, item)
                for event in item["events"]:
                    session.tool_calls += event["kind"] == "tool_use"
                    if event["kind"] == "tool_use" and event["tool"] == "Skill":
                        self._set_stage(session, event["label"], item["ts"])
                    event = self._pair(stream, item, event)
                    self._append(account, item, event)
                if item["usage"]:
                    self._count_usage(session, account, item)
        self._evaluate()

    def _agent_of(self, stream: _Stream, path: str, tried: set[str]) -> str | None:
        """FR-082: None on the main thread, else the stream's agent type (SUB_AGENT while it is not known).

        The meta file is looked for at most once per stream per step, until it gives a type or META_WAIT_S
        has passed since the first look. A settled type is never read again.
        """
        if not stream.sub_agent:
            return None
        if stream.agent is None and path not in tried:
            tried.add(path)
            now = self._clock()
            stream.meta_since = stream.meta_since or now
            if agent := _meta_agent_type(path):
                stream.agent = agent
            elif now - stream.meta_since >= timedelta(seconds=META_WAIT_S):
                stream.agent = SUB_AGENT
        return stream.agent or SUB_AGENT

    def _set_stage(self, session: _Session, skill: str | None, ts: str) -> None:
        """FR-083: a mapped Skill call not older than the current one sets the stage; a change moves `state_version`."""
        bare = skill.rsplit(":", 1)[-1] if skill else None
        if bare in STAGE_MAP and ts >= session.stage_ts and (STAGE_MAP[bare], bare) != (session.stage, session.stage_skill):
            session.stage, session.stage_skill, session.stage_ts = STAGE_MAP[bare], bare, ts
            self._state_version += 1

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
                "label": event.get("label"),
                "agent": item["agent"],
                "tokens_in": tokens[0],
                "tokens_out": tokens[1],
                "cache_creation": tokens[2],
                "cache_read": tokens[3],
                "cost_usd": tokens[4],
            }
        )
