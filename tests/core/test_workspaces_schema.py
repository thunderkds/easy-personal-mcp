"""Migration 4 (HUN-64): workspace tables; the business rules live in the schema, tested with raw SQL."""

import sqlite3

import pytest

from easy_personal.core import store
from easy_personal.core.migrations import MIGRATIONS
from easy_personal.sources import claude_code


def _insert(conn, name, **cols):
    cols = {"name": name, **cols}
    keys = ", ".join(cols)
    conn.execute(f"INSERT INTO workspaces ({keys}) VALUES ({', '.join('?' * len(cols))})", list(cols.values()))


def test_orphan_row_exists_after_migration(conn):
    row = conn.execute("SELECT name, is_orphan FROM workspaces").fetchall()
    assert row == [("orphan", 1)]


@pytest.mark.parametrize(
    "name", ["", "Acme", "-acme", "a_b", "a b", "a.b", "a" * 41, "orphan", "é", "a\nb"]
)
def test_schema_refuses_bad_names(conn, name):
    with pytest.raises(sqlite3.IntegrityError):
        _insert(conn, name)


@pytest.mark.parametrize("name", ["a", "0", "acme-2", "a" * 40])
def test_schema_accepts_slugs(conn, name):
    _insert(conn, name)


def test_schema_refuses_duplicate_name(conn):
    _insert(conn, "acme")
    with pytest.raises(sqlite3.IntegrityError):
        _insert(conn, "acme")



def test_only_the_orphan_row_may_carry_the_flag(conn):
    with pytest.raises(sqlite3.IntegrityError):
        _insert(conn, "second", is_orphan=1)


def test_name_orphan_without_flag_is_refused(conn):
    conn.execute("DELETE FROM workspaces WHERE is_orphan = 0")
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO workspaces (name, is_orphan) VALUES ('orphan', 0)")


def test_orphan_cannot_be_updated_or_deleted(conn):
    for sql in (
        "UPDATE workspaces SET name = 'other' WHERE is_orphan = 1",
        "UPDATE workspaces SET description = 'x' WHERE is_orphan = 1",
        "DELETE FROM workspaces WHERE is_orphan = 1",
    ):
        with pytest.raises(sqlite3.DatabaseError, match="orphan"):
            conn.execute(sql)
    assert conn.execute("SELECT name FROM workspaces WHERE is_orphan = 1").fetchone() == ("orphan",)


def test_orphan_cannot_hold_repos(conn):
    orphan = conn.execute("SELECT id FROM workspaces WHERE is_orphan = 1").fetchone()[0]
    with pytest.raises(sqlite3.DatabaseError, match="orphan"):
        conn.execute("INSERT INTO workspace_repos VALUES (?, '/a')", (orphan,))


@pytest.mark.parametrize("color", ["red", "#12345", "#1234567", "#GGGGGG"])
def test_schema_refuses_bad_colors(conn, color):
    with pytest.raises(sqlite3.IntegrityError):
        _insert(conn, "acme", color=color)


def test_schema_accepts_color_and_nulls(conn):
    _insert(conn, "acme", color="#a1B2c3")
    _insert(conn, "plain")


@pytest.mark.parametrize("path", ["", "rel/path", "/a/", "~/a"])
def test_schema_refuses_unnormalised_repo_paths(conn, path):
    _insert(conn, "acme")
    ws = conn.execute("SELECT id FROM workspaces WHERE name = 'acme'").fetchone()[0]
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO workspace_repos VALUES (?, ?)", (ws, path))


def test_repo_in_several_workspaces_but_once_per_workspace(conn):
    _insert(conn, "a")
    _insert(conn, "b")
    ids = [r[0] for r in conn.execute("SELECT id FROM workspaces WHERE is_orphan = 0")]
    for i in ids:
        conn.execute("INSERT INTO workspace_repos VALUES (?, '/work/x')", (i,))
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO workspace_repos VALUES (?, '/work/x')", (ids[0],))
    conn.execute("INSERT INTO workspace_repos VALUES (?, '/')", (ids[0],))  # root is valid


def test_repo_needs_an_existing_workspace_and_cascades_on_delete(conn):
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO workspace_repos VALUES (999, '/a')")
    _insert(conn, "a")
    ws = conn.execute("SELECT id FROM workspaces WHERE name = 'a'").fetchone()[0]
    conn.execute("INSERT INTO workspace_repos VALUES (?, '/a')", (ws,))
    conn.execute("DELETE FROM workspaces WHERE id = ?", (ws,))
    assert conn.execute("SELECT COUNT(*) FROM workspace_repos").fetchone()[0] == 0


def test_rename_keeps_the_id(conn):
    _insert(conn, "a")
    ws = conn.execute("SELECT id FROM workspaces WHERE name = 'a'").fetchone()[0]
    conn.execute("UPDATE workspaces SET name = 'b' WHERE id = ?", (ws,))
    assert conn.execute("SELECT name FROM workspaces WHERE id = ?", (ws,)).fetchone() == ("b",)


# --- upgrade path (migration-safety) -----------------------------------------------------------


def _v2_store(path, fixtures_root):
    """A store as HUN-61 left it: migrations 1 and 2 only, with ingested rows."""
    conn = sqlite3.connect(path)
    store.migrate(conn, path, [m for m in MIGRATIONS if m[0] <= 2])
    conn.close()
    conn = store.connect(path)
    claude_code.ingest(conn, fixtures_root)
    n = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    conn.execute("DROP TABLE workspace_repos")
    conn.execute("DROP TRIGGER workspaces_orphan_no_update")
    conn.execute("DROP TRIGGER workspaces_orphan_no_delete")
    conn.execute("DROP TABLE workspaces")
    conn.execute("PRAGMA user_version = 2")
    conn.commit()
    conn.close()
    return n


def test_v2_store_upgrades_to_v4_keeping_every_row(tmp_path, fixtures_root):
    path = tmp_path / "store.sqlite"
    n = _v2_store(path, fixtures_root)
    assert n > 0
    conn = store.connect(path)
    assert conn.execute("PRAGMA user_version").fetchone()[0] == max(m[0] for m in MIGRATIONS)
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == n
    assert conn.execute("SELECT name FROM workspaces").fetchall() == [("orphan",)]
    backup = sqlite3.connect(f"{path}.bak-v2")
    assert backup.execute("SELECT COUNT(*) FROM events").fetchone()[0] == n
    assert backup.execute("PRAGMA user_version").fetchone()[0] == 2


def test_runner_tolerates_a_gap_in_numbers(tmp_path):
    """This branch has m0001, m0002, m0004 (no m0003): user_version jumps 2 -> 4."""
    numbers = [m[0] for m in MIGRATIONS]
    assert 4 in numbers
    path = tmp_path / "s.sqlite"
    conn = sqlite3.connect(path)
    calls = []
    store.migrate(conn, path, [(1, lambda c: calls.append(1)), (2, lambda c: calls.append(2)),
                               (4, lambda c: calls.append(4))])
    assert calls == [1, 2, 4]
    assert conn.execute("PRAGMA user_version").fetchone()[0] == 4


def test_known_limitation_m0003_arriving_after_v4_is_skipped(tmp_path):
    """A store already at v4 never runs a later-merged m0003 (its number is below user_version).

    LIMITATION, not fixed here (Supervisor decides at merge): the runner compares only against
    `user_version`, so a store upgraded on this branch before HUN-62's m0003 lands keeps the
    schema without m0003's changes. Stores created after both merge run 1, 2, 3, 4 normally.
    """
    path = tmp_path / "s.sqlite"
    conn = sqlite3.connect(path)
    calls = []
    store.migrate(conn, path, [(1, lambda c: calls.append(1)), (4, lambda c: calls.append(4))])
    assert conn.execute("PRAGMA user_version").fetchone()[0] == 4
    store.migrate(conn, path, [(1, lambda c: calls.append(1)), (3, lambda c: calls.append(3)),
                               (4, lambda c: calls.append(4))])
    assert calls == [1, 4]  # 3 never ran
    assert conn.execute("PRAGMA user_version").fetchone()[0] == 4
