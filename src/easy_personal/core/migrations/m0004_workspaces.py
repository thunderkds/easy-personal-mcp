"""Migration 4: workspaces and their repo paths (HUN-64). Additive: two tables, three triggers, one seed row.

Rules live here, not only in code: slug name (`orphan` reserved for the one built-in row, which
cannot be edited, renamed or deleted and holds no repos), colour `#rrggbb`, absolute repo paths
without a trailing slash.
"""

import sqlite3


def apply(conn: sqlite3.Connection) -> None:
    conn.execute(
        """
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
        )
        """
    )
    conn.execute("CREATE UNIQUE INDEX workspaces_one_orphan ON workspaces (is_orphan) WHERE is_orphan = 1")
    conn.execute(
        """
        CREATE TABLE workspace_repos (
            workspace_id INTEGER NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
            repo_path    TEXT NOT NULL
                         CHECK (substr(repo_path, 1, 1) = '/'
                                AND (repo_path = '/' OR substr(repo_path, -1) <> '/')),
            PRIMARY KEY (workspace_id, repo_path)
        ) WITHOUT ROWID
        """
    )
    conn.execute(
        "CREATE TRIGGER workspaces_orphan_no_update BEFORE UPDATE ON workspaces "
        "WHEN OLD.is_orphan = 1 BEGIN SELECT RAISE(ABORT, 'the orphan workspace cannot be changed'); END"
    )
    conn.execute(
        "CREATE TRIGGER workspaces_orphan_no_delete BEFORE DELETE ON workspaces "
        "WHEN OLD.is_orphan = 1 BEGIN SELECT RAISE(ABORT, 'the orphan workspace cannot be deleted'); END"
    )
    conn.execute(
        "CREATE TRIGGER workspace_repos_not_orphan BEFORE INSERT ON workspace_repos "
        "WHEN (SELECT is_orphan FROM workspaces WHERE id = NEW.workspace_id) = 1 "
        "BEGIN SELECT RAISE(ABORT, 'the orphan workspace holds no repos'); END"
    )
    conn.execute("INSERT INTO workspaces (name, is_orphan) VALUES ('orphan', 1)")
