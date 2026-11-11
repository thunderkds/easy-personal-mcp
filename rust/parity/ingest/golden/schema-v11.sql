-- Schema v11 of a fresh Python store (`store.opened` on a new file), frozen by rust/parity/gen_ingest.py (T026).
-- Executable as one script on an empty file. One block per sqlite_master row, sorted by name:
-- `-- <type> <name> ON <tbl_name>`, then the SQL text SQLite stored, verbatim, plus `;`.
-- Rows with no SQL (sqlite_autoindex_*) are comments only: SQLite makes them for PRIMARY KEY / UNIQUE.
-- Python sets on every connection: PRAGMA busy_timeout = 5000; PRAGMA foreign_keys = ON (reads 1); PRAGMA journal_mode = WAL (reads wal; kept in the file once set).
-- meta.machine_id is a random uuid4 that store.connect writes on the first open: not in this file.

-- table account_registrations ON account_registrations
CREATE TABLE account_registrations (
            account_id   INTEGER NOT NULL REFERENCES agent_accounts (id),
            alias        TEXT NOT NULL CHECK (length(alias) BETWEEN 1 AND 64 AND alias GLOB '[a-z0-9]*' AND alias NOT GLOB '*[^a-z0-9_-]*'),
            connector_id TEXT NOT NULL REFERENCES connectors (id),
            PRIMARY KEY (account_id, alias)
        ) WITHOUT ROWID
        ;

-- index account_registrations_connector ON account_registrations
CREATE INDEX account_registrations_connector ON account_registrations (connector_id);

-- table account_workspaces ON account_workspaces
CREATE TABLE account_workspaces (
            account_id   INTEGER NOT NULL REFERENCES agent_accounts (id),
            workspace_id INTEGER NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
            PRIMARY KEY (account_id, workspace_id)
        ) WITHOUT ROWID
        ;

-- trigger account_workspaces_check ON account_workspaces
CREATE TRIGGER account_workspaces_check BEFORE INSERT ON account_workspaces BEGIN SELECT RAISE(ABORT, 'the orphan workspace cannot be picked') WHERE (SELECT is_orphan FROM workspaces WHERE id = NEW.workspace_id) = 1; SELECT RAISE(ABORT, 'only confirmed Claude accounts pick workspaces') WHERE NOT EXISTS (SELECT 1 FROM agent_accounts WHERE id = NEW.account_id AND provider = 'claude' AND status = 'confirmed'); END;

-- trigger account_workspaces_no_update ON account_workspaces
CREATE TRIGGER account_workspaces_no_update BEFORE UPDATE ON account_workspaces BEGIN SELECT RAISE(ABORT, 'picks are added or removed, never updated'); END;

-- table agent_accounts ON agent_accounts
CREATE TABLE agent_accounts (
            id             INTEGER PRIMARY KEY,
            provider       TEXT NOT NULL CHECK (provider IN ('claude', 'codex')),
            folder         TEXT NOT NULL UNIQUE,
            label          TEXT CHECK (label IS NULL OR length(label) BETWEEN 1 AND 60),
            status         TEXT NOT NULL CHECK (status IN ('detected', 'confirmed', 'ignored')),
            config_dir_env TEXT,
            CHECK (status <> 'confirmed' OR label IS NOT NULL)
        );

-- table connectors ON connectors
CREATE TABLE connectors (
            id           TEXT PRIMARY KEY CHECK (length(id) BETWEEN 1 AND 64 AND id GLOB '[a-z0-9]*' AND id NOT GLOB '*[^a-z0-9_-]*'),
            alias        TEXT NOT NULL CHECK (length(alias) BETWEEN 1 AND 64 AND alias GLOB '[a-z0-9]*' AND alias NOT GLOB '*[^a-z0-9_-]*'),
            alias_custom INTEGER NOT NULL DEFAULT 0 CHECK (alias_custom IN (0, 1)),
            provider     TEXT NOT NULL CHECK (length(provider) BETWEEN 1 AND 64 AND provider GLOB '[a-z0-9]*' AND provider NOT GLOB '*[^a-z0-9_-]*'),
            workspace_id INTEGER NOT NULL REFERENCES workspaces (id)
        ) WITHOUT ROWID
        ;

-- index connectors_alias ON connectors
CREATE UNIQUE INDEX connectors_alias ON connectors (alias) WHERE workspace_id <> 1;

-- index connectors_one_provider ON connectors
CREATE UNIQUE INDEX connectors_one_provider ON connectors (workspace_id, provider) WHERE workspace_id <> 1;

-- table context_index ON context_index
CREATE TABLE context_index (
    project_path      TEXT PRIMARY KEY,
    last_index_at_utc TEXT
);

-- table events ON events
CREATE TABLE events (
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
    cache_read_input_tokens     INTEGER, agent_account_id INTEGER REFERENCES agent_accounts (id), entrypoint TEXT, tool_error_count INTEGER,
    UNIQUE (machine_id, source, source_path, inode, byte_offset)
);

-- index events_account_ts ON events
CREATE INDEX events_account_ts ON events (agent_account_id, ts_utc);

-- index events_message ON events
CREATE INDEX events_message ON events (message_id);

-- index events_ts_message ON events
CREATE INDEX events_ts_message ON events (ts_utc, message_id);

-- table investigate_reports ON investigate_reports
CREATE TABLE investigate_reports (
            id             INTEGER PRIMARY KEY,
            saved_utc      TEXT NOT NULL,
            server_version TEXT NOT NULL,
            workspace      TEXT,
            account        TEXT,
            summary        TEXT NOT NULL,
            fixes_json     TEXT NOT NULL,
            snapshot_json  TEXT NOT NULL
        );

-- table kit_trace_events ON kit_trace_events
CREATE TABLE kit_trace_events (
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

-- index kit_trace_ts ON kit_trace_events
CREATE INDEX kit_trace_ts ON kit_trace_events (ts_utc);

-- table meta ON meta
CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- table source_files ON source_files
CREATE TABLE source_files (
    machine_id  TEXT NOT NULL,
    source      TEXT NOT NULL,
    path        TEXT NOT NULL,
    inode       INTEGER NOT NULL,
    byte_offset INTEGER NOT NULL,
    PRIMARY KEY (machine_id, source, path)
);

-- index sqlite_autoindex_agent_accounts_1 ON agent_accounts

-- index sqlite_autoindex_context_index_1 ON context_index

-- index sqlite_autoindex_events_1 ON events

-- index sqlite_autoindex_kit_trace_events_1 ON kit_trace_events

-- index sqlite_autoindex_meta_1 ON meta

-- index sqlite_autoindex_source_files_1 ON source_files

-- index sqlite_autoindex_tool_signals_1 ON tool_signals

-- index sqlite_autoindex_ui_sessions_1 ON ui_sessions

-- index sqlite_autoindex_verifier_scores_1 ON verifier_scores

-- index sqlite_autoindex_workspaces_1 ON workspaces

-- table tool_signals ON tool_signals
CREATE TABLE tool_signals (
    machine_id  TEXT NOT NULL,
    source_path TEXT NOT NULL,
    inode       INTEGER NOT NULL,
    byte_offset INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    value       TEXT NOT NULL,
    ts_utc      TEXT,
    UNIQUE (machine_id, source_path, inode, byte_offset, kind, value)
);

-- index tool_signals_kind ON tool_signals
CREATE INDEX tool_signals_kind ON tool_signals (kind, value);

-- table ui_sessions ON ui_sessions
CREATE TABLE ui_sessions (
    id             TEXT PRIMARY KEY,
    status         TEXT NOT NULL,
    started_at_utc TEXT NOT NULL,
    ended_at_utc   TEXT,
    action_count   INTEGER NOT NULL
);

-- table verifier_scores ON verifier_scores
CREATE TABLE verifier_scores (
    project_path       TEXT PRIMARY KEY,
    status             TEXT NOT NULL,
    scope              TEXT,
    generated_at_utc   TEXT,
    overall_value      REAL,
    overall_abstained  INTEGER NOT NULL,
    reason_code        TEXT
);

-- table workspace_repos ON workspace_repos
CREATE TABLE workspace_repos (
            workspace_id INTEGER NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
            repo_path    TEXT NOT NULL
                         CHECK (substr(repo_path, 1, 1) = '/'
                                AND (repo_path = '/' OR substr(repo_path, -1) <> '/')),
            PRIMARY KEY (workspace_id, repo_path)
        ) WITHOUT ROWID
        ;

-- trigger workspace_repos_not_orphan ON workspace_repos
CREATE TRIGGER workspace_repos_not_orphan BEFORE INSERT ON workspace_repos WHEN (SELECT is_orphan FROM workspaces WHERE id = NEW.workspace_id) = 1 BEGIN SELECT RAISE(ABORT, 'the orphan workspace holds no repos'); END;

-- table workspaces ON workspaces
CREATE TABLE workspaces (
            id          INTEGER PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE
                        CHECK (length(name) BETWEEN 1 AND 40
                               AND name GLOB '[a-z0-9]*'
                               AND name NOT GLOB '*[^a-z0-9-]*'),
            color       TEXT CHECK (color IS NULL OR (length(color) = 7
                               AND color GLOB '#[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]')),
            icon        TEXT CHECK (icon IS NULL OR length(icon) <= 32),
            description TEXT CHECK (description IS NULL OR length(description) <= 500),
            email       TEXT CHECK (email IS NULL OR length(email) <= 254),
            is_orphan   INTEGER NOT NULL DEFAULT 0 CHECK (is_orphan IN (0, 1)),
            CHECK ((is_orphan = 1) = (name = 'orphan'))
        );

-- index workspaces_one_orphan ON workspaces
CREATE UNIQUE INDEX workspaces_one_orphan ON workspaces (is_orphan) WHERE is_orphan = 1;

-- trigger workspaces_orphan_no_delete ON workspaces
CREATE TRIGGER workspaces_orphan_no_delete BEFORE DELETE ON workspaces WHEN OLD.is_orphan = 1 BEGIN SELECT RAISE(ABORT, 'the orphan workspace cannot be deleted'); END;

-- trigger workspaces_orphan_no_update ON workspaces
CREATE TRIGGER workspaces_orphan_no_update BEFORE UPDATE ON workspaces WHEN OLD.is_orphan = 1 BEGIN SELECT RAISE(ABORT, 'the orphan workspace cannot be changed'); END;

-- seed rows
INSERT INTO workspaces (id, name, color, icon, description, email, is_orphan) VALUES (1, 'orphan', NULL, NULL, NULL, NULL, 1);

PRAGMA user_version = 11;
