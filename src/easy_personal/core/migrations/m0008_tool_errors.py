"""Migration 8: failed tool results per user record, for the kit performance comparison (HUN-192).

`events.tool_error_count` is nullable and added without a backfill (`ADD COLUMN` rewrites no row). NULL
means *unknown*: the row is not a user record with tool results, or it was ingested before this column
existed. A migration must not read the home folder, so the backfill is the next normal ingest: this
migration resets the stored offset of every Claude Code transcript file to 0, and ingest re-reads each
file and writes the count into the rows it already has (no row is added; see `store.save_file`). Files
that cleanup deleted are not re-read, so their rows stay unknown, never 0. Only the `claude_code` source
carries a count, so other sources keep their offsets. Idempotent like migration 7: it checks for the
column first; a repeated offset reset only causes one more harmless re-read. Undo: stop using the column
(nothing reads it before the next task); the pre-migration copy `<store>.bak-v7` is the restore path.
"""

import sqlite3


def apply(conn: sqlite3.Connection) -> None:
    if "tool_error_count" not in [row[1] for row in conn.execute("PRAGMA table_info(events)")]:
        conn.execute("ALTER TABLE events ADD COLUMN tool_error_count INTEGER")
    conn.execute("UPDATE source_files SET byte_offset = 0 WHERE source = 'claude_code'")
