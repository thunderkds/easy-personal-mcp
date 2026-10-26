"""Migration 1: the schema as of HUN-58 (every statement IF NOT EXISTS, so a v0 store passes).

Also absorbs the old ad-hoc `_rebuild_old_verifier_scores` patch (the pre-HUN-42 table is a cache).
"""

import sqlite3

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


def apply(conn: sqlite3.Connection) -> None:
    columns = [row[1] for row in conn.execute("PRAGMA table_info(verifier_scores)")]
    if columns and "status" not in columns:
        # pre-HUN-42 verifier_scores has no `status`; the next ingest refills this cache
        conn.execute("DROP TABLE verifier_scores")
    for statement in SCHEMA.split(";"):  # no executescript: it would commit our transaction
        if statement.strip():
            conn.execute(statement)
