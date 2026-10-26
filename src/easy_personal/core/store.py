"""SQLite store (~/.easy-personal/store.sqlite): explicit metadata columns only, WAL, one writer.

No column holds a raw record, message text or tool input/output (FR-002, NFR-002).
"""

import sqlite3
import uuid
from collections.abc import Iterator
from contextlib import closing, contextmanager
from datetime import UTC, datetime, timedelta
from pathlib import Path

from easy_personal.core.migrations import MIGRATIONS
from easy_personal.core.migrations.m0001_baseline import SCHEMA  # noqa: F401  (re-export)

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


BUSY_TIMEOUT_MS = 5000


def _has_tables(conn: sqlite3.Connection) -> bool:
    return conn.execute("SELECT 1 FROM sqlite_master LIMIT 1").fetchone() is not None


def migrate(conn: sqlite3.Connection, path: Path, migrations=MIGRATIONS) -> None:
    """Apply pending numbered migrations in order; a migration's number is its `user_version`.

    Each one runs in its own `BEGIN IMMEDIATE` transaction together with its `user_version` bump, so a
    failure rolls back fully. Before the first pending migration on a non-empty store the file is copied
    to `<path>.bak-v<N>` (restore path: put that copy back). A store newer than this code is refused.
    """
    latest = max((number for number, _ in migrations), default=0)
    version = conn.execute("PRAGMA user_version").fetchone()[0]
    if version > latest:
        raise RuntimeError(
            f"store {path} is schema v{version}, newer than this program (v{latest}): upgrade easy-personal"
        )
    if version == latest:
        return
    previous = conn.isolation_level
    conn.isolation_level = None  # explicit BEGIN/COMMIT: DDL must roll back with the version bump
    try:
        if version < latest and _has_tables(conn):
            with closing(sqlite3.connect(f"{path}.bak-v{version}")) as backup:
                conn.backup(backup)
        for number, apply in sorted(migrations, key=lambda m: m[0]):
            conn.execute("BEGIN IMMEDIATE")
            try:
                # re-read under the write lock: another process may have migrated meanwhile
                if number > conn.execute("PRAGMA user_version").fetchone()[0]:
                    apply(conn)
                    conn.execute(f"PRAGMA user_version = {number}")
                conn.execute("COMMIT")
            except BaseException:
                conn.execute("ROLLBACK")
                raise
    finally:
        conn.isolation_level = previous


def connect(path: Path) -> sqlite3.Connection:
    """Open (and create / upgrade) the store: WAL, FKs on, busy wait, this store's machine_id."""
    conn = sqlite3.connect(path)
    conn.execute(f"PRAGMA busy_timeout={BUSY_TIMEOUT_MS}")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.execute("PRAGMA journal_mode=WAL")
    migrate(conn, path)
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
