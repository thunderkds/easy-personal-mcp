"""Migration 6 (HUN-65): connectors, account picks, registrations. Rules tested with raw SQL (NFR-009)."""

import sqlite3

import pytest

from easy_personal.core import store
from easy_personal.core.migrations import MIGRATIONS
from easy_personal.sources import claude_code


def _ws(conn, name):
    return conn.execute("INSERT INTO workspaces (name) VALUES (?)", (name,)).lastrowid


def _orphan(conn):
    return conn.execute("SELECT id FROM workspaces WHERE is_orphan = 1").fetchone()[0]


def _account(conn, provider="claude", status="confirmed", folder="/a/.claude-work"):
    return conn.execute(
        "INSERT INTO agent_accounts (provider, folder, label, status) VALUES (?, ?, ?, ?)",
        (provider, folder, "x" if status == "confirmed" else None, status),
    ).lastrowid


def _connector(conn, id_, alias, provider, ws):
    conn.execute(
        "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES (?, ?, 0, ?, ?)",
        (id_, alias, provider, ws),
    )


def test_tables_exist_with_the_agreed_columns(conn):
    cols = lambda t: [r[1] for r in conn.execute(f"PRAGMA table_info({t})")]
    assert cols("connectors") == ["id", "alias", "alias_custom", "provider", "workspace_id"]
    assert cols("account_workspaces") == ["account_id", "workspace_id"]
    assert cols("account_registrations") == ["account_id", "alias", "connector_id"]


def test_one_connector_per_provider_per_workspace_except_orphan(conn):
    ws, orphan = _ws(conn, "company"), _orphan(conn)
    _connector(conn, "c00000001", "company-slack", "slack", ws)
    with pytest.raises(sqlite3.IntegrityError):
        _connector(conn, "c00000002", "company-slack2", "slack", ws)
    _connector(conn, "slack", "slack", "slack", orphan)
    _connector(conn, "c00000003", "old-slack", "slack", orphan)  # orphan may hold several


def test_alias_unique_among_non_orphan_connectors(conn):
    a, b, orphan = _ws(conn, "a"), _ws(conn, "b"), _orphan(conn)
    _connector(conn, "c00000001", "shared", "slack", a)
    with pytest.raises(sqlite3.IntegrityError):
        _connector(conn, "c00000002", "shared", "gmail", b)
    _connector(conn, "c00000003", "shared", "gmail", orphan)  # orphan is exempt
    _connector(conn, "c00000004", "shared", "slack", orphan)


@pytest.mark.parametrize("bad", ["", "Bad", "-x", "a b", "a" * 65, "a/b", "é"])
def test_id_and_alias_must_match_the_launcher_name_rule(conn, bad):
    ws = _ws(conn, "company")
    with pytest.raises(sqlite3.IntegrityError):
        _connector(conn, bad, "ok", "slack", ws)
    with pytest.raises(sqlite3.IntegrityError):
        _connector(conn, "c00000001", bad, "slack", ws)


def test_connector_needs_an_existing_workspace(conn):
    with pytest.raises(sqlite3.IntegrityError):
        _connector(conn, "c00000001", "x-slack", "slack", 999)


def test_workspace_with_connectors_cannot_be_deleted(conn):
    ws = _ws(conn, "company")
    _connector(conn, "c00000001", "company-slack", "slack", ws)
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("DELETE FROM workspaces WHERE id = ?", (ws,))


def test_picks_refuse_orphan(conn):
    acc = _account(conn)
    with pytest.raises(sqlite3.DatabaseError, match="orphan"):
        conn.execute("INSERT INTO account_workspaces VALUES (?, ?)", (acc, _orphan(conn)))


@pytest.mark.parametrize(("provider", "status"), [("codex", "confirmed"), ("claude", "detected"), ("claude", "ignored")])
def test_picks_refuse_codex_and_unconfirmed_accounts(conn, provider, status):
    acc, ws = _account(conn, provider, status), _ws(conn, "company")
    with pytest.raises(sqlite3.DatabaseError, match="confirmed Claude"):
        conn.execute("INSERT INTO account_workspaces VALUES (?, ?)", (acc, ws))


def test_picks_accept_confirmed_claude_and_are_unique(conn):
    acc, ws = _account(conn), _ws(conn, "company")
    conn.execute("INSERT INTO account_workspaces VALUES (?, ?)", (acc, ws))
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO account_workspaces VALUES (?, ?)", (acc, ws))
    with pytest.raises(sqlite3.DatabaseError):
        conn.execute("UPDATE account_workspaces SET workspace_id = ?", (_orphan(conn),))


def test_deleting_a_workspace_drops_its_picks(conn):
    acc, ws = _account(conn), _ws(conn, "company")
    conn.execute("INSERT INTO account_workspaces VALUES (?, ?)", (acc, ws))
    conn.execute("DELETE FROM workspaces WHERE id = ?", (ws,))
    assert conn.execute("SELECT COUNT(*) FROM account_workspaces").fetchone()[0] == 0


def test_registration_keeps_its_connector(conn):
    acc, ws = _account(conn), _ws(conn, "company")
    _connector(conn, "c00000001", "company-slack", "slack", ws)
    conn.execute("INSERT INTO account_registrations VALUES (?, 'company-slack', 'c00000001')", (acc,))
    with pytest.raises(sqlite3.IntegrityError):  # one alias per account
        conn.execute("INSERT INTO account_registrations VALUES (?, 'company-slack', 'c00000001')", (acc,))
    with pytest.raises(sqlite3.IntegrityError):  # unregister before deleting the connector
        conn.execute("DELETE FROM connectors WHERE id = 'c00000001'")
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO account_registrations VALUES (?, 'x', 'c0000dead')", (acc,))


def test_no_column_can_hold_a_secret_value(conn):
    text_cols = {
        t: [r[1] for r in conn.execute(f"PRAGMA table_info({t})")]
        for t in ("connectors", "account_workspaces", "account_registrations")
    }
    assert not any(c for cols in text_cols.values() for c in cols if c in ("value", "secret", "env", "token"))


def test_v5_store_with_rows_upgrades_keeping_every_row(tmp_path, fixtures_root):
    path = tmp_path / "store.sqlite"
    conn = sqlite3.connect(path)
    conn.execute("PRAGMA foreign_keys=ON")
    store.migrate(conn, path, [m for m in MIGRATIONS if m[0] <= 5])  # schema before m0006
    conn.execute("INSERT INTO meta VALUES ('machine_id', 'm1')")
    conn.commit()
    claude_code.ingest(conn, fixtures_root)
    conn.execute("INSERT INTO agent_accounts (provider, folder, label, status) VALUES ('claude', '/h/.claude', 'd', 'confirmed')")
    conn.execute("INSERT INTO workspaces (name) VALUES ('company')")
    conn.execute("INSERT INTO workspace_repos VALUES (2, '/work/repo')")
    conn.commit()
    tables = ("events", "agent_accounts", "workspaces", "workspace_repos", "source_files")
    before = {t: conn.execute(f"SELECT * FROM {t} ORDER BY 1").fetchall() for t in tables}
    assert conn.execute("PRAGMA user_version").fetchone()[0] == 5 and before["events"]
    conn.close()
    upgraded = store.connect(path)
    assert upgraded.execute("PRAGMA user_version").fetchone()[0] == max(n for n, _ in MIGRATIONS)
    assert {t: upgraded.execute(f"SELECT * FROM {t} ORDER BY 1").fetchall() for t in tables} == before
    assert (tmp_path / "store.sqlite.bak-v5").exists()
    upgraded.close()
