"""Migration 10: saved investigate reports (HUN-221, FR-076, DDR-0003).

Additive: one new table, no existing table or row is touched, nothing is backfilled. `investigate_reports`
holds the only model-written text in the store: the `summary` and `fixes_json` the owner's own Claude Code
session sends to MCP `save_report` (limits and secret check in `core/reports.py`), plus `snapshot_json`, the
server's own rule snapshot taken at save time. "Now" is never stored. `workspace` / `account` are the scope
the report used, as given (NULL = all). `CREATE TABLE IF NOT EXISTS`, so a repeated run changes nothing.
Undo: stop using the table (code older than v10 refuses a v10 store); the pre-migration copy
`<store>.bak-v9` is the restore path.
"""

import sqlite3


def apply(conn: sqlite3.Connection) -> None:
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS investigate_reports (
            id             INTEGER PRIMARY KEY,
            saved_utc      TEXT NOT NULL,
            server_version TEXT NOT NULL,
            workspace      TEXT,
            account        TEXT,
            summary        TEXT NOT NULL,
            fixes_json     TEXT NOT NULL,
            snapshot_json  TEXT NOT NULL
        )
        """
    )
