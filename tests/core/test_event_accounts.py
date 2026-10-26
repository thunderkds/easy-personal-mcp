"""Migration 5 (events.agent_account_id) and the account filter in the usage functions (HUN-63)."""

import sqlite3

import pytest

from easy_personal.core import accounts, metrics, store, workspaces
from easy_personal.core.migrations import MIGRATIONS
from tests_helpers import add_event

LABELS = ("Claude default", "Claude work")


@pytest.fixture
def two(conn):
    """Default (id 1, label 'Claude default') and work (id 2) accounts; 3 + 2 events, 1 shared message."""
    conn.execute(
        "INSERT INTO agent_accounts (provider, folder, label, status, config_dir_env) VALUES "
        "('claude', '/a/.claude', 'Claude default', 'confirmed', NULL),"
        "('claude', '/a/.claude-work', 'Claude work', 'confirmed', '/a/.claude-work')"
    )
    for n, project, account in ((1, "/work/a", 1), (2, "/work/a", 1), (3, "/work/b", 1), (4, "/work/a", 2), (5, "/work/b", 2)):
        add_event(conn, n, project)
        conn.execute("UPDATE events SET agent_account_id = ? WHERE byte_offset = ?", (account, n))
    conn.commit()
    return conn


def test_migration_5_registered():
    assert 5 in [n for n, _ in MIGRATIONS]


def test_v4_store_upgrades_keeping_rows_and_adds_column_and_index(tmp_path, fixtures_root):
    from easy_personal.sources import claude_code

    path = tmp_path / "store.sqlite"
    conn = store.connect(path)
    claude_code.ingest(conn, fixtures_root)
    rows = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    conn.close()
    raw = sqlite3.connect(path)
    raw.execute("DROP INDEX events_account_ts")
    raw.execute("ALTER TABLE events DROP COLUMN agent_account_id")
    raw.execute("PRAGMA user_version = 4")
    raw.commit()
    raw.close()
    conn = store.connect(path)
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == rows > 0
    columns = {r[1]: r for r in conn.execute("PRAGMA table_info(events)")}
    assert "agent_account_id" in columns and columns["agent_account_id"][3] == 0  # nullable
    assert conn.execute("SELECT COUNT(*) FROM events WHERE agent_account_id IS NOT NULL").fetchone()[0] == 0
    index = [r[2] for r in conn.execute("PRAGMA index_info(events_account_ts)")]
    assert index == ["agent_account_id", "ts_utc"]
    assert list(tmp_path.glob("*.bak-v4"))
    fk = conn.execute("PRAGMA foreign_key_list(events)").fetchall()
    assert [(r[2], r[3], r[4]) for r in fk] == [("agent_accounts", "agent_account_id", "id")]


def test_migration_5_is_idempotent_when_the_column_exists(conn):
    from easy_personal.core.migrations import m0005_event_accounts as m

    conn.execute("BEGIN")
    m.apply(conn)  # column and index already there: no error
    conn.execute("ROLLBACK")


def test_foreign_key_refuses_an_unknown_account(conn):
    add_event(conn, 1, "/p")
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("UPDATE events SET agent_account_id = 99")


def test_filter_by_id_or_label_case_insensitive(two):
    assert metrics.usage_summary(two)["messages"] == 5
    for ref in ("2", "Claude work", "claude WORK"):
        assert metrics.usage_summary(two, account=ref)["messages"] == 2, ref
    assert metrics.usage_summary(two, account="Claude default")["messages"] == 3


def test_unknown_account_lists_valid_labels(two):
    with pytest.raises(accounts.AccountError) as err:
        metrics.usage_summary(two, account="nope")
    assert "Claude default" in str(err.value) and "Claude work" in str(err.value)
    assert "nope" not in str(err.value)
    assert isinstance(err.value, ValueError)


def test_account_filter_covers_every_usage_function(two):
    assert metrics.usage_breakdown(two, "project", account="Claude work")["total"]["messages"] == 2
    assert [g["key"] for g in metrics.usage_breakdown(two, "project", account="2")["groups"]] == ["a", "b"]
    assert len(metrics.session_timeline(two, account="Claude work")["sessions"]) == 2
    assert metrics.session_outliers(two, account="Claude work")["sessions"] == []


def test_legacy_null_rows_belong_to_the_default_account(two):
    two.execute("UPDATE events SET agent_account_id = NULL WHERE agent_account_id = 1")
    assert metrics.usage_summary(two, account="Claude default")["messages"] == 3
    assert metrics.usage_summary(two, account="Claude work")["messages"] == 2


def test_account_and_workspace_filters_are_anded(two):
    for name, repo in (("wa", "/work/a"), ("wb", "/work/b")):
        workspaces.add(two, name, repos=[repo])
    combos = {("wa", "Claude default"): 2, ("wa", "Claude work"): 1, ("wb", "Claude default"): 1, ("wb", "Claude work"): 1}
    for (ws, acct), expected in combos.items():
        assert metrics.usage_summary(two, workspace=ws, account=acct)["messages"] == expected, (ws, acct)
        assert metrics.usage_breakdown(two, "day", workspace=ws, account=acct)["total"]["messages"] == expected
    assert metrics.usage_summary(two, workspace="wa")["messages"] == 3
    with pytest.raises(workspaces.WorkspaceError):
        metrics.usage_summary(two, workspace="nope", account="Claude work")


def test_filter_plan_uses_an_index(two):
    scope, params = metrics._scope(two, None, None, None, account="2")
    plan = " | ".join(r[3] for r in two.execute(f"EXPLAIN QUERY PLAN SELECT 1 FROM events WHERE ts_utc >= ? AND {scope}", ["2026"] + params))
    assert "SCAN events" not in plan, plan


@pytest.mark.perf
def test_migration_5_and_default_backfill_cost_over_a_million_rows(tmp_path):
    import time

    path = tmp_path / "big.sqlite"
    conn = store.connect(path)
    conn.executemany(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, message_id, input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,0,'2026-01-01T00:00:00.000000Z',?,1)",
        ((i, f"m{i}") for i in range(1_000_000)),
    )
    conn.commit()
    conn.close()
    raw = sqlite3.connect(path)
    raw.execute("DROP INDEX events_account_ts")
    raw.execute("ALTER TABLE events DROP COLUMN agent_account_id")
    raw.execute("PRAGMA user_version = 4")
    raw.commit()
    raw.close()
    start = time.perf_counter()
    conn = store.connect(path)  # runs m0005 (plus the online backup of the file)
    total = time.perf_counter() - start
    conn.execute("INSERT INTO agent_accounts (provider, folder, label, status) VALUES ('claude', '/d', 'd', 'confirmed')")
    start = time.perf_counter()
    with conn:
        conn.execute("UPDATE events SET agent_account_id = 1 WHERE agent_account_id IS NULL")
    backfill = time.perf_counter() - start
    start = time.perf_counter()
    with conn:
        conn.execute("UPDATE events SET agent_account_id = 1 WHERE agent_account_id IS NULL")
    noop = time.perf_counter() - start
    print(f"perf m0005 on 1M rows incl. backup: {total:.2f}s; one-time NULL backfill {backfill:.2f}s; no-op backfill {noop * 1000:.1f}ms")
    assert noop < 0.1
