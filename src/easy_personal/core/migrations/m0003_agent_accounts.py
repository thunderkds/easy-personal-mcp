"""Migration 3: agent accounts (HUN-62). Additive: one new table, no existing row touched.

`missing` is not a status: it is computed at read time from the folder.
"""

import sqlite3


def apply(conn: sqlite3.Connection) -> None:
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS agent_accounts (
            id             INTEGER PRIMARY KEY,
            provider       TEXT NOT NULL CHECK (provider IN ('claude', 'codex')),
            folder         TEXT NOT NULL UNIQUE,
            label          TEXT CHECK (label IS NULL OR length(label) BETWEEN 1 AND 60),
            status         TEXT NOT NULL CHECK (status IN ('detected', 'confirmed', 'ignored')),
            config_dir_env TEXT,
            CHECK (status <> 'confirmed' OR label IS NOT NULL)
        )
        """
    )
