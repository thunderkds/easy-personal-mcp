"""Migration 11: drop the stale index `events_ts` again (HUN-232).

`m0002_message_indexes` already dropped `events_ts ON events (ts_utc)` because `events_ts_message` (leading with
`ts_utc`) supersedes it, yet the owner's v10 store still had it (~14 MB, and it slows every insert). An older
install that predates the migrations most likely re-ran the full schema; that cause is out of scope. No row or
column is touched, nothing is backfilled. `DROP INDEX IF EXISTS`, so a store without it, or a repeated run,
changes nothing. Undo: the index is only a speed aid, nothing reads it by name; the pre-migration copy
`<store>.bak-v10` is the restore path.
"""

import sqlite3


def apply(conn: sqlite3.Connection) -> None:
    conn.execute("DROP INDEX IF EXISTS events_ts")
