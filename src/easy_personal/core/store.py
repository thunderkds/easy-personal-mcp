"""SQLite store (~/.easy-personal/store.sqlite): explicit metadata columns only, WAL, one writer.

No column holds a raw record, message text or tool input/output (FR-002, NFR-002).
"""

import sqlite3
import uuid
from collections.abc import Iterator
from contextlib import contextmanager
from datetime import UTC, datetime, timedelta
from pathlib import Path

SCHEMA = """
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS source_files (
    machine_id  TEXT NOT NULL,
    source      TEXT NOT NULL,
    path        TEXT NOT NULL,
    inode       INTEGER NOT NULL,
    byte_offset INTEGER NOT NULL,
    PRIMARY KEY (machine_id, source, path)
);
CREATE TABLE IF NOT EXISTS events (
    id                          INTEGER PRIMARY KEY,
    machine_id                  TEXT NOT NULL,
    source                      TEXT NOT NULL,
    source_path                 TEXT NOT NULL,
    inode                       INTEGER NOT NULL,
    byte_offset                 INTEGER NOT NULL,
    abstained                   INTEGER NOT NULL,
    record_type                 TEXT,
    version                     TEXT,
    ts_utc                      TEXT,
    session_id                  TEXT,
    project_path                TEXT,
    git_branch                  TEXT,
    is_sidechain                INTEGER,
    is_api_error                INTEGER,
    model                       TEXT,
    message_id                  TEXT,
    tool_names                  TEXT,
    input_tokens                INTEGER,
    output_tokens               INTEGER,
    cache_creation_input_tokens INTEGER,
    cache_read_input_tokens     INTEGER,
    UNIQUE (machine_id, source, source_path, inode, byte_offset)
);
CREATE TABLE IF NOT EXISTS kit_trace_events (
    id           INTEGER PRIMARY KEY,
    machine_id   TEXT NOT NULL,
    source_path  TEXT NOT NULL,
    inode        INTEGER NOT NULL,
    byte_offset  INTEGER NOT NULL,
    project_path TEXT NOT NULL,
    kit_task_id  TEXT,
    abstained    INTEGER NOT NULL,
    ts_utc       TEXT,
    tool_name    TEXT,
    is_error     INTEGER,
    UNIQUE (machine_id, source_path, inode, byte_offset)
);
CREATE TABLE IF NOT EXISTS context_index (
    project_path      TEXT PRIMARY KEY,
    last_index_at_utc TEXT
);
CREATE TABLE IF NOT EXISTS ui_sessions (
    id             TEXT PRIMARY KEY,
    status         TEXT NOT NULL,
    started_at_utc TEXT NOT NULL,
    ended_at_utc   TEXT,
    action_count   INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS verifier_scores (
    project_path       TEXT PRIMARY KEY,
    status             TEXT NOT NULL,
    scope              TEXT,
    generated_at_utc   TEXT,
    overall_value      REAL,
    overall_abstained  INTEGER NOT NULL,
    reason_code        TEXT
);
CREATE TABLE IF NOT EXISTS tool_signals (
    machine_id  TEXT NOT NULL,
    source_path TEXT NOT NULL,
    inode       INTEGER NOT NULL,
    byte_offset INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    value       TEXT NOT NULL,
    ts_utc      TEXT,
    UNIQUE (machine_id, source_path, inode, byte_offset, kind, value)
);
CREATE INDEX IF NOT EXISTS tool_signals_kind ON tool_signals (kind, value);
CREATE INDEX IF NOT EXISTS kit_trace_ts ON kit_trace_events (ts_utc);
CREATE INDEX IF NOT EXISTS events_ts ON events (ts_utc);
CREATE INDEX IF NOT EXISTS events_message ON events (message_id);
"""

EVENT_COLUMNS = (
    "abstained",
    "record_type",
    "version",
    "ts_utc",
    "session_id",
    "project_path",
    "git_branch",
    "is_sidechain",
    "is_api_error",
    "model",
    "message_id",
    "tool_names",
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
)


def _rebuild_old_verifier_scores(conn: sqlite3.Connection) -> None:
    """Drop the pre-HUN-42 verifier_scores (no `status` column); the next ingest refills this cache."""
    columns = [row[1] for row in conn.execute("PRAGMA table_info(verifier_scores)")]
    if columns and "status" not in columns:
        with conn:
            conn.execute("DROP TABLE verifier_scores")


def connect(path: Path) -> sqlite3.Connection:
    """Open (and create if needed) the store in WAL mode and ensure this store's machine_id."""
    conn = sqlite3.connect(path)
    conn.execute("PRAGMA journal_mode=WAL")
    _rebuild_old_verifier_scores(conn)
    conn.executescript(SCHEMA)
    with conn:
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('machine_id', ?)", (str(uuid.uuid4()),)
        )
    return conn


@contextmanager
def opened(path: Path) -> Iterator[sqlite3.Connection]:
    conn = connect(path)
    try:
        yield conn
    finally:
        conn.close()


def machine_id(conn: sqlite3.Connection) -> str:
    return conn.execute("SELECT value FROM meta WHERE key = 'machine_id'").fetchone()[0]


def file_offsets(conn: sqlite3.Connection, source: str) -> dict[str, tuple[int, int]]:
    """Known files of a source: path -> (inode, byte_offset of the next unread line)."""
    rows = conn.execute(
        "SELECT path, inode, byte_offset FROM source_files WHERE machine_id = ? AND source = ?",
        (machine_id(conn), source),
    )
    return {path: (inode, offset) for path, inode, offset in rows}


def save_file(
    conn: sqlite3.Connection, source: str, path: str, inode: int, offset: int, rows
) -> int:
    """Insert one file's new events and its new offset in one transaction; return rows added.

    `rows` yields (line_offset, next_offset, event or None); the stored offset becomes the last
    `next_offset` seen, i.e. the start of the first line not yet consumed.
    """
    mid = machine_id(conn)
    end = [offset]
    signals: list[tuple] = []

    def values():
        for line_offset, next_offset, event in rows:
            end[0] = next_offset
            if event is not None:
                signals.extend(
                    (mid, path, inode, line_offset, kind, value, event["ts_utc"])
                    for kind, value in event.get("signals", ())
                )
                yield (mid, source, path, inode, line_offset, *(event[c] for c in EVENT_COLUMNS))

    placeholders = ", ".join("?" * (5 + len(EVENT_COLUMNS)))
    sql = (
        "INSERT OR IGNORE INTO events (machine_id, source, source_path, inode, byte_offset, "
        f"{', '.join(EVENT_COLUMNS)}) VALUES ({placeholders})"
    )
    with conn:
        before = conn.total_changes
        conn.executemany(sql, values())
        added = conn.total_changes - before
        conn.executemany(
            "INSERT OR IGNORE INTO tool_signals (machine_id, source_path, inode, byte_offset, "
            "kind, value, ts_utc) VALUES (?, ?, ?, ?, ?, ?, ?)",
            signals,
        )
        conn.execute(
            "INSERT INTO source_files (machine_id, source, path, inode, byte_offset) "
            "VALUES (?, ?, ?, ?, ?) ON CONFLICT (machine_id, source, path) "
            "DO UPDATE SET inode = excluded.inode, byte_offset = excluded.byte_offset",
            (mid, source, path, inode, end[0]),
        )
    return added


def retire_files(conn: sqlite3.Connection, source: str, paths: set[str]) -> None:
    """Forget offsets of files that no longer exist; their events stay (history survives cleanup)."""
    mid = machine_id(conn)
    with conn:
        conn.executemany(
            "DELETE FROM source_files WHERE machine_id = ? AND source = ? AND path = ?",
            ((mid, source, p) for p in paths),
        )


def prune(conn: sqlite3.Connection, retention_days: int | None, now: datetime | None = None) -> int:
    """Delete events older than `retention_days`; None keeps everything. Return rows deleted.

    Events without a timestamp are kept: their age is unknown.
    """
    if retention_days is None:
        return 0
    cutoff = (now or datetime.now(UTC)) - timedelta(days=retention_days)
    with conn:
        cur = conn.execute(
            "DELETE FROM events WHERE ts_utc IS NOT NULL AND ts_utc < ?",
            (cutoff.strftime("%Y-%m-%dT%H:%M:%S.%fZ"),),
        )
        kit = conn.execute(
            "DELETE FROM kit_trace_events WHERE ts_utc IS NOT NULL AND ts_utc < ?",
            (cutoff.strftime("%Y-%m-%dT%H:%M:%S.%fZ"),),
        )
        sig = conn.execute(
            "DELETE FROM tool_signals WHERE ts_utc IS NOT NULL AND ts_utc < ?",
            (cutoff.strftime("%Y-%m-%dT%H:%M:%S.%fZ"),),
        )
    return cur.rowcount + kit.rowcount + sig.rowcount
