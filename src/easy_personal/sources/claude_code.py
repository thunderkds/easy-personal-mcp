"""`claude_code` source: Claude Code transcripts (~/.claude/projects/**/*.jsonl) -> metadata events.

Read-only. Only the FR-002 fields are taken from a record; text fields (`message.content` text,
`toolUseResult`, `attachment`, `lastPrompt`, `customTitle`, ...) are never read. Shapes come from
the HUN-32 spike (docs/spikes/transcript-shapes.md): records are mapped by `type`; `version` is a
per-record label only. Anything else is abstained (FR-003).
"""

import json
import os
import re
import sqlite3
from collections.abc import Iterator
from datetime import UTC, datetime
from pathlib import Path
from urllib.parse import urlsplit

from easy_personal.core import accounts, config, discovery, store

SOURCE = "claude_code"

# Types that carry `version`, `cwd`, `timestamp`, `sessionId` (HUN-32 spike, 2.1.293-2.1.302).
VERSIONED_TYPES = {"user", "assistant", "attachment", "system"}
# Metadata lines with no `version`/`cwd`: known, never abstained, only type/session/time are kept.
# `cost-state.totalCostUSD` is deliberately never read (owner decision 2026-10-21).
METADATA_TYPES = {
    "agent-name",
    "custom-title",
    "last-prompt",
    "atis-latch",
    "queue-operation",
    "cost-state",
    "file-history-snapshot",
    "file-history-delta",
    "artifact-comment-monitor",
    "artifact-autoreact-ledger",
    "frame-link",
}
TOKEN_FIELDS = (
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
)
# Identifier-like strings only: a value that does not match is dropped, so free text cannot slip in.
_LABEL = re.compile(r"^[A-Za-z0-9_.:@/+-]{1,200}$")
# Connector-suggestion counters (HUN-47): only a Bash command's first word and a WebFetch URL's host.
_WORD = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.+-]{0,63}")
_HOST = re.compile(r"[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?)+")


def _label(value) -> str | None:
    return value if isinstance(value, str) and _LABEL.match(value) else None


def _bash_word(command) -> str | None:
    """First word of a command, as its program name (no path, no arguments); else None.

    The 64-char cap plus the `_WORD` regex are the only guard against a token-like first word.
    """
    if not isinstance(command, str) or not command.split():
        return None
    word = command.split()[0].rsplit("/", 1)[-1]
    return word if _WORD.fullmatch(word) else None


def _host(url) -> str | None:
    """Host name of an http(s) URL; the path, query, user info and port are dropped."""
    try:
        parts = urlsplit(url) if isinstance(url, str) else None
        host = (parts.hostname or "").rstrip(".") if parts and parts.scheme in ("http", "https") else ""
    except ValueError:
        return None
    return host if _HOST.fullmatch(host) else None


def _signals(blocks) -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    for b in blocks:
        if not isinstance(b, dict) or b.get("type") != "tool_use":
            continue
        tool_input = b.get("input") if isinstance(b.get("input"), dict) else {}
        if b.get("name") == "Bash" and (word := _bash_word(tool_input.get("command"))):
            found.append(("bash", word))
        elif b.get("name") == "WebFetch" and (host := _host(tool_input.get("url"))):
            found.append(("domain", host))
    return found


def _utc(value) -> str | None:
    if not isinstance(value, str):
        return None
    try:
        parsed = datetime.fromisoformat(value)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return None
    return parsed.astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def _empty(**fields) -> dict:
    event = dict.fromkeys(store.EVENT_COLUMNS)
    event["abstained"] = 0
    event.update(fields)
    return event


def _abstain(record) -> dict:
    is_dict = isinstance(record, dict)
    return _empty(
        abstained=1,
        record_type=_label(record.get("type")) if is_dict else None,
        version=_label(record.get("version")) if is_dict else None,
        ts_utc=_utc(record.get("timestamp")) if is_dict else None,
        session_id=_label(record.get("sessionId")) if is_dict else None,
    )


def parse_record(record, project_of) -> dict:
    """Map one decoded JSON line to an event dict (store.EVENT_COLUMNS); abstain if unknown."""
    if not isinstance(record, dict):
        return _abstain(record)
    rtype = record.get("type")
    if rtype in METADATA_TYPES:
        return _empty(
            record_type=rtype,
            version=_label(record.get("version")),
            ts_utc=_utc(record.get("timestamp")),
            session_id=_label(record.get("sessionId")),
        )
    if rtype not in VERSIONED_TYPES:
        return _abstain(record)

    ts_utc = _utc(record.get("timestamp"))
    session_id = _label(record.get("sessionId"))
    cwd = record.get("cwd")
    if ts_utc is None or session_id is None or (cwd is not None and not isinstance(cwd, str)):
        return _abstain(record)
    event = _empty(
        record_type=rtype,
        version=_label(record.get("version")),
        ts_utc=ts_utc,
        session_id=session_id,
        project_path=project_of(cwd) if cwd else None,
        git_branch=_label(record.get("gitBranch")),
        is_sidechain=int(record.get("isSidechain") is True),
        entrypoint=_label(record.get("entrypoint")),
    )
    if rtype != "assistant":
        return event

    message = record.get("message")
    if not isinstance(message, dict):
        return _abstain(record)
    usage = message.get("usage")
    message_id = _label(message.get("id"))
    if usage is not None:
        # Usage is deduped on message.id; without one it cannot be counted correctly.
        if (
            message_id is None
            or not isinstance(usage, dict)
            or not all(type(usage.get(f)) is int for f in TOKEN_FIELDS)
        ):
            return _abstain(record)
        event.update({f: usage[f] for f in TOKEN_FIELDS})
    content = message.get("content")
    blocks = content if isinstance(content, list) else []
    tools = [
        name
        for b in blocks
        if isinstance(b, dict) and b.get("type") == "tool_use"
        for name in [_label(b.get("name"))]
        if name
    ]
    event.update(
        model=_label(message.get("model")),
        message_id=message_id,
        is_api_error=int(record.get("isApiErrorMessage") is True),
        tool_names=",".join(tools) or None,
        signals=_signals(blocks),
    )
    return event


def read_complete_lines(path: Path, offset: int) -> Iterator[tuple[int, bytes, int]]:
    """Yield (line_offset, line, next_offset) for each complete line from `offset`.

    A last line without a trailing newline is still being written: it is never yielded, so the
    stored offset never points mid-line. Streams one line at a time (bounded memory).
    """
    with open(path, "rb") as handle:
        handle.seek(offset)
        while True:
            line = handle.readline()
            if not line.endswith(b"\n"):
                return
            yield offset, line, offset + len(line)
            offset += len(line)


def _file_events(path: Path, offset: int, project_of):
    for line_offset, line, next_offset in read_complete_lines(path, offset):
        if not line.strip():
            yield line_offset, next_offset, None
            continue
        try:
            record = json.loads(line)
        except ValueError:
            record = None
        yield line_offset, next_offset, parse_record(record, project_of)


def _scan(conn: sqlite3.Connection, roots: list[tuple[int | None, Path]]) -> dict:
    """Incrementally ingest every *.jsonl under each `(account_id, root)`; return counts of this run.

    Offsets are only forgotten for files under a root that exists now: a missing account folder
    keeps its offsets (and rows), so it resumes where it stopped when the folder returns.
    """
    known = store.file_offsets(conn, SOURCE)
    projects: dict[str, str] = {}

    def project_of(cwd: str) -> str:
        if cwd not in projects:
            projects[cwd] = discovery.resolve_project(cwd)
        return projects[cwd]

    seen: set[str] = set()
    new_events = 0
    scanned = [str(root) + os.sep for _, root in roots if root.is_dir()]
    for account_id, root in roots:
        for path in sorted(root.rglob("*.jsonl")) if root.is_dir() else []:
            key = str(path)
            try:
                info = path.stat()
                known_inode, offset = known.get(key, (info.st_ino, 0))
                if known_inode != info.st_ino or offset > info.st_size:
                    offset = 0  # replaced (new inode) or shrunk file: start over
                rows = _file_events(path, offset, project_of)
                new_events += store.save_file(conn, SOURCE, key, info.st_ino, offset, rows, account_id)
            except FileNotFoundError:
                continue  # removed by cleanup mid-run; the file's transaction rolled back
            seen.add(key)
    gone = {path for path in set(known) - seen if path.startswith(tuple(scanned))}
    store.retire_files(conn, SOURCE, gone)
    return {"files": len(seen), "new_events": new_events}


def ingest(conn: sqlite3.Connection, root: Path, account_id: int | None = None) -> dict:
    """Ingest one transcript folder, tagging its events with `account_id` (None: untagged)."""
    return _scan(conn, [(account_id, Path(root))])


def account_roots(conn: sqlite3.Connection, override: Path | None = None) -> list[tuple[int | None, Path]]:
    """`(account id, <folder>/projects)` of every confirmed Claude account, in id order.

    `override` (else `EASY_PERSONAL_CLAUDE_PROJECTS`) replaces the default account's folder. Without a
    default account row an override is still read, untagged (NULL = default account).
    """
    override = Path(override) if override else config.claude_projects_override()
    rows = conn.execute(
        "SELECT id, folder, config_dir_env FROM agent_accounts "
        "WHERE provider = 'claude' AND status = 'confirmed' ORDER BY id"
    ).fetchall()
    roots = [
        (id_, override if env is None and override else Path(folder) / "projects") for id_, folder, env in rows
    ]
    if override and not any(env is None for _, _, env in rows):
        roots.append((None, override))
    return roots


def ingest_accounts(conn: sqlite3.Connection, override: Path | None = None) -> dict:
    """Ingest every confirmed Claude account (FR-036); commits per file, resumes from stored offsets."""
    default = accounts.default_id(conn)
    if default is not None:  # rows stored before migration 5 carry no account: they are the default's
        with conn:
            conn.execute("UPDATE events SET agent_account_id = ? WHERE agent_account_id IS NULL", (default,))
    return _scan(conn, account_roots(conn, override))
