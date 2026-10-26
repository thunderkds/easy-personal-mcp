"""Migration 2: index for time-first usage queries (HUN-61)."""

import sqlite3


def apply(conn: sqlite3.Connection) -> None:
    conn.execute("CREATE INDEX IF NOT EXISTS events_ts_message ON events (ts_utc, message_id)")
    conn.execute("DROP INDEX IF EXISTS events_ts")  # the new index leads with ts_utc
