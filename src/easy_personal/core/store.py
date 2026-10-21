"""SQLite store (~/.easy-personal/store.sqlite): explicit metadata columns only, WAL, one writer.

No column holds a raw record, message text or tool input/output (FR-002, NFR-002).
"""

import sqlite3
import uuid
from collections.abc import Iterator
from contextlib import contextmanager
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


def connect(path: Path) -> sqlite3.Connection:
    """Open (and create if needed) the store in WAL mode and ensure this store's machine_id."""
    conn = sqlite3.connect(path)
    conn.execute("PRAGMA journal_mode=WAL")
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

    def values():
        for line_offset, next_offset, event in rows:
            end[0] = next_offset
            if event is not None:
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
