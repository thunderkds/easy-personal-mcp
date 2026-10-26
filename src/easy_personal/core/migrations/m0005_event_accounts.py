"""Migration 5: which agent account an event came from (HUN-63).

`events.agent_account_id` is nullable and added without a backfill. A migration must not read the
home folder, and the default account's row may not exist yet (store migrated before any detect), so
the rule is: **NULL means the default account**. Reads count NULL as the default account; the next
ingest tags those rows with the default account's id. `ADD COLUMN` rewrites no row (cheap at 1M+).
Idempotent like migrations 1 and 4: it checks for the column first.
"""

import sqlite3


def apply(conn: sqlite3.Connection) -> None:
    if "agent_account_id" not in [row[1] for row in conn.execute("PRAGMA table_info(events)")]:
        conn.execute("ALTER TABLE events ADD COLUMN agent_account_id INTEGER REFERENCES agent_accounts (id)")
    conn.execute("CREATE INDEX IF NOT EXISTS events_account_ts ON events (agent_account_id, ts_utc)")
