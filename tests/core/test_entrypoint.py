"""Migration 7 (events.entrypoint), the typed / agent / unknown split and its backfill (HUN-161)."""

import sqlite3

import pytest

from easy_personal.core import metrics, store
from easy_personal.core.migrations import MIGRATIONS
from easy_personal.sources import claude_code
from tests_helpers import TOKENS, rec, write


def split(conn, **kwargs):
    return metrics.usage_summary(conn, **kwargs)["split"]


def test_migration_7_is_registered():
    assert 7 in [n for n, _ in MIGRATIONS]


def _v6_store(tmp_path, root):
    """A store as m0006 left it: the transcripts are ingested, `entrypoint` does not exist yet."""
    path = tmp_path / "store.sqlite"
    conn = store.connect(path)
    claude_code.ingest(conn, root)
    conn.execute("INSERT INTO source_files VALUES ('m', 'kit_trace', '/kt.jsonl', 1, 77)")
    conn.commit()
    rows = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    conn.close()
    raw = sqlite3.connect(path)
    raw.execute("ALTER TABLE events DROP COLUMN entrypoint")
    raw.execute("PRAGMA user_version = 6")
    raw.commit()
    raw.close()
    return path, rows


def test_v6_store_upgrades_keeps_rows_resets_offsets_and_backfills_on_next_ingest(tmp_path):
    root = write(
        tmp_path / "projects",
        "a.jsonl",
        [rec(1, "s-cli"), rec(2, "s-cli", "user"), rec(3, "s-sdk", entry="sdk-cli"), rec(4, "s-x", sidechain=True)],
    )
    path, rows = _v6_store(tmp_path, root)
    conn = store.connect(path)
    columns = {r[1]: r for r in conn.execute("PRAGMA table_info(events)")}
    assert "entrypoint" in columns and columns["entrypoint"][3] == 0  # nullable
    assert conn.execute("PRAGMA user_version").fetchone()[0] == 7
    assert list(tmp_path.glob("*.bak-v6"))
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == rows > 0
    assert conn.execute("SELECT COUNT(*) FROM events WHERE entrypoint IS NOT NULL").fetchone()[0] == 0
    assert conn.execute(
        "SELECT byte_offset FROM source_files WHERE source = 'claude_code'"
    ).fetchall() == [(0,)]
    assert conn.execute("SELECT byte_offset FROM source_files WHERE source = 'kit_trace'").fetchone() == (77,)
    # until the next ingest every old row is unknown (sidechain rows are already agent work)
    before = split(conn)
    assert before == {"typed": 0, "agent": TOKENS, "unknown": 2 * TOKENS}
    result = claude_code.ingest(conn, root)
    assert result["new_events"] == 0  # same events, only entrypoint changes
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == rows
    assert split(conn) == {"typed": TOKENS, "agent": 2 * TOKENS, "unknown": 0}


def test_migration_7_is_idempotent(conn):
    from easy_personal.core.migrations import m0007_entrypoint as m

    conn.execute("BEGIN")
    m.apply(conn)  # column exists already: no error, still one column
    m.apply(conn)
    names = [r[1] for r in conn.execute("PRAGMA table_info(events)")]
    conn.execute("ROLLBACK")
    assert names.count("entrypoint") == 1


def test_backfill_twice_gives_the_same_rows(tmp_path):
    root = write(tmp_path / "p", "a.jsonl", [rec(1, "s1"), rec(2, "s1"), rec(3, "s2", entry="sdk-cli")])
    path, rows = _v6_store(tmp_path, root)
    conn = store.connect(path)
    claude_code.ingest(conn, root)
    first = conn.execute("SELECT id, entrypoint FROM events ORDER BY id").fetchall()
    conn.execute("UPDATE source_files SET byte_offset = 0")  # interrupted backfill / manual re-run
    conn.commit()
    assert claude_code.ingest(conn, root)["new_events"] == 0
    assert conn.execute("SELECT id, entrypoint FROM events ORDER BY id").fetchall() == first
    assert len(first) == rows


@pytest.mark.parametrize(
    "records, expected",
    [
        ([rec(1, "s", entry="cli")], "typed"),
        ([rec(1, "s", entry="sdk-cli")], "agent"),
        ([rec(1, "s", entry="ide")], "agent"),  # any known value other than cli is agent work
        ([rec(1, "s", entry="cli", sidechain=True)], "agent"),
        ([rec(1, "s", entry=None, sidechain=True)], "agent"),  # sidechain wins over a missing entrypoint
        ([rec(1, "s", entry=None)], "unknown"),
    ],
)
def test_classification_table(conn, tmp_path, records, expected):
    claude_code.ingest(conn, write(tmp_path / "p", "a.jsonl", records))
    result = split(conn)
    assert {k: v for k, v in result.items() if v} == {expected: TOKENS}


def test_headless_session_with_sidechain_counts_every_message_once(conn, tmp_path):
    records = [
        rec(1, "main", entry="cli"),
        rec(2, "main", entry="cli", sidechain=True),
        rec(3, "head", entry="sdk-cli"),
        rec(4, "head", entry="sdk-cli", sidechain=True),
        rec(5, "none", entry=None),
    ]
    claude_code.ingest(conn, write(tmp_path / "p", "a.jsonl", records))
    summary = metrics.usage_summary(conn)
    assert summary["split"] == {"typed": TOKENS, "agent": 3 * TOKENS, "unknown": TOKENS}
    assert sum(summary["split"].values()) == summary["messages"] * TOKENS == 5 * TOKENS


def test_entrypoint_found_later_in_the_session_is_applied_to_the_whole_session(conn, tmp_path):
    root = write(tmp_path / "p", "a.jsonl", [rec(1, "s", "user", entry=None), rec(2, "s", entry=None), rec(3, "s", entry="cli")])
    claude_code.ingest(conn, root)
    assert split(conn) == {"typed": 2 * TOKENS, "agent": 0, "unknown": 0}
    assert conn.execute("SELECT COUNT(*) FROM events WHERE entrypoint IS NULL").fetchone()[0] == 0


def test_entrypoint_arriving_in_a_later_ingest_fills_earlier_rows_and_later_rows_inherit_it(conn, tmp_path):
    root = write(tmp_path / "p", "a.jsonl", [rec(1, "s", entry=None)])
    claude_code.ingest(conn, root)
    assert split(conn)["unknown"] == TOKENS
    write(root, "a.jsonl", [rec(2, "s", entry="sdk-cli")], mode="a")
    assert claude_code.ingest(conn, root)["new_events"] == 1
    assert split(conn) == {"typed": 0, "agent": 2 * TOKENS, "unknown": 0}
    write(root, "a.jsonl", [rec(3, "s", entry=None)], mode="a")
    claude_code.ingest(conn, root)
    assert split(conn) == {"typed": 0, "agent": 3 * TOKENS, "unknown": 0}


def test_summary_lists_tokens_per_entrypoint_value_so_a_wrong_call_is_visible(conn, tmp_path):
    records = [rec(1, "a", entry="cli"), rec(2, "b", entry="ide"), rec(3, "b", entry="ide"), rec(4, "c", entry=None)]
    claude_code.ingest(conn, write(tmp_path / "p", "a.jsonl", records))
    assert metrics.usage_summary(conn)["entrypoints"] == {"cli": TOKENS, "ide": 2 * TOKENS}


def test_split_sums_to_the_total_in_every_breakdown_group(conn, tmp_path):
    records = [
        rec(1, "a", entry="cli"),
        rec(2, "a", entry="cli", sidechain=True),
        rec(3, "b", entry="sdk-cli", day=16),
        rec(4, "c", entry=None, day=16),
    ]
    claude_code.ingest(conn, write(tmp_path / "p", "a.jsonl", records))
    summary = metrics.usage_summary(conn)
    for by in metrics.DIMENSIONS:
        out = metrics.usage_breakdown(conn, by)
        assert out["total"]["split"] == summary["split"], by
        for group in [out["total"], *out["groups"]]:
            tokens = sum(group[k] for k in ("input_tokens", "output_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"))
            assert sum(group["split"].values()) == tokens, (by, group.get("key"))
    days = {g["key"]: g["split"] for g in metrics.usage_breakdown(conn, "day")["groups"]}
    assert days == {
        "2026-01-15": {"typed": TOKENS, "agent": TOKENS, "unknown": 0},
        "2026-01-16": {"typed": 0, "agent": TOKENS, "unknown": TOKENS},
    }


def test_split_follows_the_period_and_scope_filters(conn, tmp_path):
    records = [rec(1, "a", entry="cli", day=15), rec(2, "b", entry="sdk-cli", day=20)]
    claude_code.ingest(conn, write(tmp_path / "p", "a.jsonl", records))
    assert split(conn, until="2026-01-16T00:00:00Z") == {"typed": TOKENS, "agent": 0, "unknown": 0}
    assert split(conn, since="2026-01-16T00:00:00Z") == {"typed": 0, "agent": TOKENS, "unknown": 0}
    assert metrics.usage_breakdown(conn, "day", since="2026-01-16T00:00:00Z")["total"]["split"]["agent"] == TOKENS


def test_backfill_after_a_prune_brings_pruned_rows_back_only_until_the_next_prune(tmp_path):
    from datetime import UTC, datetime

    now = datetime(2026, 1, 30, tzinfo=UTC)
    root = write(tmp_path / "p", "a.jsonl", [rec(1, "old", day=1), rec(2, "new", day=28)])
    path, _ = _v6_store(tmp_path, root)
    conn = store.connect(path)
    assert store.prune(conn, 10, now=now) == 1  # the old row is gone, the file is still there
    assert claude_code.ingest(conn, root)["new_events"] == 1  # the backfill re-reads it ...
    assert store.prune(conn, 10, now=now) == 1  # ... and the prune after every CLI ingest removes it again
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == 1
    assert split(conn) == {"typed": TOKENS, "agent": 0, "unknown": 0}


def test_split_query_plans_use_the_time_index(conn):
    since, until = "2026-01-10T00:00:00.000000Z", "2026-01-20T00:00:00.000000Z"
    inner, params = metrics._inner_window(since, until)
    plan = " | ".join(
        r[3]
        for r in conn.execute(
            "EXPLAIN QUERY PLAN SELECT MIN(ts_utc), MAX(is_sidechain), MAX(entrypoint) FROM events WHERE abstained = 0 AND "
            f"message_id IS NOT NULL AND input_tokens IS NOT NULL AND {inner} GROUP BY message_id",
            params,
        )
    )
    assert "events_ts_message" in plan and "SCAN events" not in plan, plan


@pytest.mark.perf
def test_thirty_day_workspace_summary_over_a_million_events_is_under_500ms(tmp_path):
    """Same shape as the HUN-61 workspace benchmark: 2 events per message, 40 projects, a year of days."""
    import time

    from easy_personal.core import workspaces

    conn = store.connect(tmp_path / "big.sqlite")
    day = 86400
    base = 1_767_225_600  # 2026-01-01
    stamp = lambda t: time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(t)) + ".000000Z"
    conn.executemany(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,"
        " project_path, is_sidechain, entrypoint, model, message_id, input_tokens, output_tokens,"
        " cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,0,?,?,?,?,?,'model-0',?,10,5,0,0)",
        (
            (
                m * 2 + k,
                stamp(base + (m * 364 * day) // 500_000 + k * 60),
                f"s{m % 50}",
                f"/w/p{m % 40}/sub",
                m % 5 == 0,
                ("cli", "sdk-cli", None)[m % 3],
                f"x{m}",
            )
            for m in range(500_000)
            for k in range(2)
        ),
    )
    conn.commit()
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == 1_000_000
    workspaces.add(conn, "company", repos=["/w/p7/sub", "/w/p9/sub", "/w/p11/sub", "/w/p13/sub"])
    timings = []
    for _ in range(3):
        t = time.perf_counter()
        out = metrics.usage_summary(conn, "2026-06-01", "2026-07-01", workspace="company")
        timings.append(time.perf_counter() - t)
    print(f"perf usage_summary+split 30d workspace over 1M events: {[round(x * 1000) for x in timings]} ms")
    assert out["messages"] > 0 and sum(out["split"].values()) > 0
    assert min(timings) < 0.5, timings
    conn.close()
