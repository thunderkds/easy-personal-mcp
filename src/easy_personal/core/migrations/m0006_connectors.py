"""Migration 6: connectors in workspaces, account picks, what was written per account (HUN-65).

Additive: three new tables, two triggers, indexes; no existing row is touched. No column holds a secret
(secrets live only in the keychain under `easy-personal/<connector id>`).

- `connectors`: fixed `id` (keychain + launcher key) and `alias` (the Claude Code name). One connector
  per provider per workspace and one alias across workspaces, except in orphan.
- `account_workspaces`: the workspaces a confirmed Claude account picked; never orphan.
- `account_registrations`: the aliases easy-personal wrote into an account (the only ones it removes).
"""

import sqlite3

# same rule as launch._NAME: [a-z0-9][a-z0-9_-]{0,63}
_NAME_CHECK = "length({c}) BETWEEN 1 AND 64 AND {c} GLOB '[a-z0-9]*' AND {c} NOT GLOB '*[^a-z0-9_-]*'"


def apply(conn: sqlite3.Connection) -> None:
    # Partial indexes cannot hold a subquery; the orphan row's id never changes (m0004 triggers).
    orphan = conn.execute("SELECT id FROM workspaces WHERE is_orphan = 1").fetchone()[0]
    conn.execute(
        f"""
        CREATE TABLE IF NOT EXISTS connectors (
            id           TEXT PRIMARY KEY CHECK ({_NAME_CHECK.format(c="id")}),
            alias        TEXT NOT NULL CHECK ({_NAME_CHECK.format(c="alias")}),
            alias_custom INTEGER NOT NULL DEFAULT 0 CHECK (alias_custom IN (0, 1)),
            provider     TEXT NOT NULL CHECK ({_NAME_CHECK.format(c="provider")}),
            workspace_id INTEGER NOT NULL REFERENCES workspaces (id)
        ) WITHOUT ROWID
        """
    )
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS connectors_one_provider ON connectors (workspace_id, provider) "
        f"WHERE workspace_id <> {int(orphan)}"
    )
    conn.execute(
        f"CREATE UNIQUE INDEX IF NOT EXISTS connectors_alias ON connectors (alias) WHERE workspace_id <> {int(orphan)}"
    )
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS account_workspaces (
            account_id   INTEGER NOT NULL REFERENCES agent_accounts (id),
            workspace_id INTEGER NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
            PRIMARY KEY (account_id, workspace_id)
        ) WITHOUT ROWID
        """
    )
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS account_workspaces_check BEFORE INSERT ON account_workspaces BEGIN "
        "SELECT RAISE(ABORT, 'the orphan workspace cannot be picked') "
        "WHERE (SELECT is_orphan FROM workspaces WHERE id = NEW.workspace_id) = 1; "
        "SELECT RAISE(ABORT, 'only confirmed Claude accounts pick workspaces') "
        "WHERE NOT EXISTS (SELECT 1 FROM agent_accounts WHERE id = NEW.account_id "
        "AND provider = 'claude' AND status = 'confirmed'); END"
    )
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS account_workspaces_no_update BEFORE UPDATE ON account_workspaces "
        "BEGIN SELECT RAISE(ABORT, 'picks are added or removed, never updated'); END"
    )
    conn.execute(
        f"""
        CREATE TABLE IF NOT EXISTS account_registrations (
            account_id   INTEGER NOT NULL REFERENCES agent_accounts (id),
            alias        TEXT NOT NULL CHECK ({_NAME_CHECK.format(c="alias")}),
            connector_id TEXT NOT NULL REFERENCES connectors (id),
            PRIMARY KEY (account_id, alias)
        ) WITHOUT ROWID
        """
    )
    conn.execute(
        "CREATE INDEX IF NOT EXISTS account_registrations_connector ON account_registrations (connector_id)"
    )
