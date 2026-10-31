"""Migration 7: the session entrypoint per event, for the typed / agent / unknown split (HUN-161).

`events.entrypoint` is nullable and added without a backfill (`ADD COLUMN` rewrites no row). NULL means
*unknown*: the row was ingested before this column existed. A migration must not read the home folder,
so the backfill is the next normal ingest: this migration resets the stored offset of every Claude Code
transcript file to 0, and ingest re-reads each file and writes `entrypoint` into the rows it already has
(no row is added; see `store.save_file`). Files that cleanup deleted are not re-read, so their rows stay
unknown. Only the `claude_code` source carries an entrypoint, so other sources keep their offsets.
Idempotent like migrations 1, 4 and 5: it checks for the column first; a repeated offset reset only
causes one more harmless re-read. Undo: stop using the column (nothing reads it before this version);
the pre-migration copy `<store>.bak-v6` is the restore path.
"""

import sqlite3


def apply(conn: sqlite3.Connection) -> None:
    if "entrypoint" not in [row[1] for row in conn.execute("PRAGMA table_info(events)")]:
        conn.execute("ALTER TABLE events ADD COLUMN entrypoint TEXT")
    conn.execute("UPDATE source_files SET byte_offset = 0 WHERE source = 'claude_code'")
