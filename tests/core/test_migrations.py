import sqlite3
import time
from pathlib import Path

import pytest

from easy_personal.core import memo, metrics, store
from easy_personal.core.migrations import MIGRATIONS
from easy_personal.sources import claude_code

LATEST = max(number for number, _ in MIGRATIONS)


def _make_v0(path: Path, fixtures_root: Path, old_verifier: bool = False) -> int:
    """A store as the pre-HUN-61 code left it: full schema, rows, user_version 0."""
    conn = store.connect(path)
    claude_code.ingest(conn, fixtures_root)
    n = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    conn.close()
    raw = sqlite3.connect(path)
    raw.execute("PRAGMA user_version = 0")
    if old_verifier:
        raw.execute("DROP TABLE verifier_scores")
        raw.execute("CREATE TABLE verifier_scores (project_path TEXT PRIMARY KEY, score REAL)")
        raw.execute("INSERT INTO verifier_scores VALUES ('/p', 0.5)")
    raw.execute("DROP INDEX IF EXISTS events_ts_message")
    raw.execute("CREATE INDEX IF NOT EXISTS events_ts ON events (ts_utc)")
    raw.commit()
    raw.close()
    return n


def test_runner_is_ordered_and_numbers_are_versions():
    numbers = [number for number, _ in MIGRATIONS]
    assert numbers == sorted(numbers) and numbers[0] == 1 and len(set(numbers)) == len(numbers)
    assert LATEST >= 2


def test_new_store_runs_all_migrations_and_makes_no_backup(tmp_path):
    path = tmp_path / "store.sqlite"
    conn = store.connect(path)
    assert conn.execute("PRAGMA user_version").fetchone()[0] == LATEST
    conn.close()
    assert not list(tmp_path.glob("*.bak-v*"))


def test_rerun_is_a_noop(tmp_path):
    path = tmp_path / "store.sqlite"
    store.connect(path).close()
    before = path.stat().st_mtime_ns
    conn = store.connect(path)
    assert conn.execute("PRAGMA user_version").fetchone()[0] == LATEST
    conn.close()
    assert not list(tmp_path.glob("*.bak-v*"))
    assert path.stat().st_mtime_ns >= before


def test_v0_store_upgrades_keeping_every_row_and_leaves_a_backup(tmp_path, fixtures_root):
    path = tmp_path / "store.sqlite"
    n = _make_v0(path, fixtures_root)
    assert n > 0
    assert sqlite3.connect(path).execute("PRAGMA user_version").fetchone()[0] == 0
    conn = store.connect(path)
    assert conn.execute("PRAGMA user_version").fetchone()[0] == LATEST
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == n
    backup = tmp_path / "store.sqlite.bak-v0"
    assert backup.exists()
    assert sqlite3.connect(backup).execute("SELECT COUNT(*) FROM events").fetchone()[0] == n
    assert sqlite3.connect(backup).execute("PRAGMA user_version").fetchone()[0] == 0
    names = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='index'")}
    assert "events_ts_message" in names and "events_ts" not in names


def test_pre_hun42_verifier_scores_is_rebuilt_by_the_baseline(tmp_path, fixtures_root):
    path = tmp_path / "store.sqlite"
    n = _make_v0(path, fixtures_root, old_verifier=True)
    conn = store.connect(path)
    assert "status" in {r[1] for r in conn.execute("PRAGMA table_info(verifier_scores)")}
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == n


def test_failing_migration_rolls_back_fully_and_keeps_user_version(tmp_path):
    path = tmp_path / "store.sqlite"
    conn = sqlite3.connect(path)

    def good(c):
        c.execute("CREATE TABLE t1 (x)")

    def bad(c):
        c.execute("CREATE TABLE t2 (x)")
        raise RuntimeError("boom")

    with pytest.raises(RuntimeError, match="boom"):
        store.migrate(conn, path, [(1, good), (2, bad)])
    assert conn.execute("PRAGMA user_version").fetchone()[0] == 1
    tables = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    assert tables == {"t1"}
    assert not conn.in_transaction


def test_failing_first_migration_leaves_version_zero(tmp_path):
    path = tmp_path / "store.sqlite"
    conn = sqlite3.connect(path)

    def bad(c):
        c.execute("CREATE TABLE t (x)")
        raise RuntimeError("boom")

    with pytest.raises(RuntimeError):
        store.migrate(conn, path, [(1, bad)])
    assert conn.execute("PRAGMA user_version").fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM sqlite_master").fetchone()[0] == 0


def test_newer_store_is_refused_not_downgraded(tmp_path):
    path = tmp_path / "store.sqlite"
    raw = sqlite3.connect(path)
    raw.execute(f"PRAGMA user_version = {LATEST + 1}")
    raw.close()
    with pytest.raises(RuntimeError, match="newer"):
        store.connect(path)
    assert sqlite3.connect(path).execute("PRAGMA user_version").fetchone()[0] == LATEST + 1


def test_connection_pragmas(conn):
    assert conn.execute("PRAGMA foreign_keys").fetchone()[0] == 1
    assert conn.execute("PRAGMA busy_timeout").fetchone()[0] >= 5000
    assert conn.execute("PRAGMA journal_mode").fetchone()[0] == "wal"


def test_foreign_keys_are_enforced(conn):
    conn.execute("CREATE TABLE parent (id INTEGER PRIMARY KEY)")
    conn.execute("CREATE TABLE child (p INTEGER REFERENCES parent(id))")
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO child VALUES (99)")


# --- usage queries: time first, same numbers -------------------------------------------------

DAY = 86400


def _ts(seconds: int) -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(seconds)) + ".000000Z"


BASE = 1_767_225_600  # 2026-01-01T00:00:00Z


def seed(conn, messages: int, span_days: int = 60, spread: int = 1) -> None:
    """Deterministic synthetic events: `spread` events per message, minutes apart."""
    rows = []
    for m in range(messages):
        t0 = BASE + (m * span_days * DAY) // messages
        for k in range(spread):
            rows.append(
                (
                    "m",
                    "claude_code",
                    f"/p/{m % 7}.jsonl",
                    1,
                    m * 10 + k,
                    0,
                    "assistant",
                    "2.1.295",
                    _ts(t0 + k * 60),
                    f"s{m % 13}",
                    f"/work/app{m % 5}",
                    0,
                    0,
                    f"model-{m % 3}",
                    f"msg{m}",
                    "Read,Edit" if m % 4 == 0 else None,
                    100 + k,
                    10 + m % 50,
                    m % 7,
                    m % 11,
                )
            )
    conn.executemany(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained,"
        " record_type, version, ts_utc, session_id, project_path, is_sidechain, is_api_error,"
        " model, message_id, tool_names, input_tokens, output_tokens,"
        " cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        rows,
    )
    conn.commit()


WINDOWS = [
    (None, None),
    ("2026-01-10", None),
    (None, "2026-01-20"),
    ("2026-01-10", "2026-01-20"),
    ("2026-01-10T05:30:00Z", "2026-01-20T17:45:00Z"),
    ("2025-12-01", "2025-12-31"),
]


def _all_numbers(conn, since, until):
    return (
        metrics.usage_summary(conn, since, until),
        metrics._messages(conn, metrics._utc_bound(since), metrics._utc_bound(until)),
        [metrics.usage_breakdown(conn, by, since, until) for by in metrics.DIMENSIONS],
        metrics.session_outliers(conn, since, until),
        metrics.session_timeline(conn, since, until),
        metrics.session_contexts(conn, since, until),
    )


@pytest.mark.parametrize("since,until", WINDOWS)
def test_time_first_numbers_equal_the_group_everything_numbers(conn, monkeypatch, since, until):
    seed(conn, 400, spread=3)
    claude_code.ingest(conn, Path(__file__).parents[1] / "fixtures" / "claude_code")
    new = _all_numbers(conn, since, until)
    monkeypatch.setattr(metrics, "_inner_window", lambda s, u: ("1", []))  # legacy semantics
    memo.reset()  # recompute with the legacy query, not from the shared cache (HUN-227)
    assert _all_numbers(conn, since, until) == new


def test_fixture_store_numbers_are_unchanged(conn, fixtures_root, monkeypatch):
    claude_code.ingest(conn, fixtures_root)
    new = _all_numbers(conn, "2026-01-15", "2026-01-16")
    monkeypatch.setattr(metrics, "_inner_window", lambda s, u: ("1", []))
    memo.reset()  # recompute with the legacy query, not from the shared cache (HUN-227)
    assert _all_numbers(conn, "2026-01-15", "2026-01-16") == new
    assert new[0]["messages"] == 3


def test_message_straddling_the_window_start_is_counted_by_its_first_event(conn):
    seed(conn, 1, spread=3)  # events at BASE, +60, +120
    start = _ts(BASE + 30)
    assert len(metrics._messages(conn, start, None)) == 0  # first event is before `since`
    assert len(metrics._messages(conn, _ts(BASE), None)) == 1


def test_message_plan_uses_the_ts_index_not_a_scan(conn):
    since, until = "2026-01-10T00:00:00.000000Z", "2026-01-20T00:00:00.000000Z"
    where, params = metrics._inner_window(since, until)
    plan = " | ".join(
        row[3]
        for row in conn.execute(
            "EXPLAIN QUERY PLAN SELECT MIN(ts_utc) FROM events WHERE abstained = 0 AND "
            f"message_id IS NOT NULL AND input_tokens IS NOT NULL AND {where} GROUP BY message_id",
            params,
        )
    )
    assert "events_ts_message" in plan, plan
    assert "SCAN events" not in plan, plan


@pytest.mark.perf
def test_thirty_day_summary_over_a_million_events_is_under_500ms(tmp_path):
    conn = store.connect(tmp_path / "big.sqlite")
    seed(conn, 400_000, span_days=365, spread=2)  # 800k
    seed_extra = 200_000
    conn.executemany(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained,"
        " ts_utc, session_id, project_path, model, message_id, tool_names, input_tokens,"
        " output_tokens, cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,0,?,?,?,?,?,NULL,10,5,0,0)",
        (
            (
                10**9 + i,
                _ts(BASE + (i * 365 * DAY) // seed_extra),
                f"s{i % 50}",
                "/w/a",
                "model-0",
                f"x{i}",
            )
            for i in range(seed_extra)
        ),
    )
    conn.commit()
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == 1_000_000
    timings = []
    for _ in range(3):
        t = time.perf_counter()
        out = metrics.usage_summary(conn, "2026-06-01", "2026-07-01")
        timings.append(time.perf_counter() - t)
    print(f"perf usage_summary 30d over 1M events: {[round(x * 1000) for x in timings]} ms")
    assert out["messages"] > 0
    assert min(timings) < 0.5, timings
    conn.close()


# Migration 9 (HUN-209): stale `abstained = 1` on metadata rows stored before HUN-175 listed them.
_FROZEN_METADATA_TYPES = (
    "agent-name", "custom-title", "last-prompt", "atis-latch", "queue-operation", "cost-state",
    "file-history-snapshot", "file-history-delta", "artifact-comment-monitor",
    "artifact-autoreact-ledger", "frame-link", "api-request", "api-request-blob",
    "api-request-shape", "tools-used", "ai-title", "mode", "permission-mode", "bridge-session",
)  # fmt: skip


def _v8_store_with_abstained_rows(path: Path) -> list[tuple]:
    """A v8 store: one abstained row per (source, record_type) case; returns the rows as (source, type)."""
    store.connect(path).close()
    raw = sqlite3.connect(path)
    raw.execute("PRAGMA user_version = 8")
    cases = [("claude_code", t) for t in _FROZEN_METADATA_TYPES]
    cases += [("claude_code", t) for t in ("user", "assistant", "attachment", "system", "mystery", None)]
    cases += [("kit_trace", "api-request"), ("git_local", "api-request-shape")]
    for n, (source, rtype) in enumerate(cases):
        raw.execute(
            "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type,"
            " version, ts_utc, session_id, project_path, tool_error_count) VALUES"
            " ('m', ?, '/f', 1, ?, 1, ?, '2.1.312', '2026-10-30T10:00:00.000Z', 's1', '/p', 2)",
            (source, n, rtype),
        )
    raw.commit()
    raw.close()
    return cases


def test_migration_9_clears_stale_metadata_rows_and_nothing_else(tmp_path):
    path = tmp_path / "store.sqlite"
    cases = _v8_store_with_abstained_rows(path)
    raw = sqlite3.connect(path)
    before = raw.execute("SELECT * FROM events ORDER BY id").fetchall()
    columns = [c[1] for c in raw.execute("PRAGMA table_info(events)")]
    raw.close()

    conn = store.connect(path)  # AC 1: v8 -> latest, with a backup
    assert conn.execute("PRAGMA user_version").fetchone()[0] == LATEST >= 9
    assert (tmp_path / "store.sqlite.bak-v8").exists()
    after = conn.execute("SELECT * FROM events ORDER BY id").fetchall()
    assert len(after) == len(before) == len(cases)  # AC 4: no row added or deleted

    flag = columns.index("abstained")
    for (source, rtype), old, new in zip(cases, before, after, strict=True):
        cleared = source == "claude_code" and rtype in _FROZEN_METADATA_TYPES
        assert new[flag] == (0 if cleared else 1), (source, rtype)  # AC 2 and AC 3
        assert [v for i, v in enumerate(new) if i != flag] == [v for i, v in enumerate(old) if i != flag]
    conn.close()


def test_migration_9_is_idempotent_and_a_v9_store_is_not_touched(tmp_path):
    from easy_personal.core.migrations import m0009_metadata_abstained as m9

    path = tmp_path / "store.sqlite"
    _v8_store_with_abstained_rows(path)
    conn = store.connect(path)
    first = conn.execute("SELECT id, abstained FROM events ORDER BY id").fetchall()
    m9.apply(conn)
    m9.apply(conn)
    assert conn.execute("SELECT id, abstained FROM events ORDER BY id").fetchall() == first
    conn.close()
    store.connect(path).close()  # already at the latest version: no second backup, no change
    assert sorted(p.name for p in tmp_path.glob("*.bak-v*")) == ["store.sqlite.bak-v8"]


def test_migration_9_frozen_list_is_part_of_the_parser_list():
    # The migration keeps its own copy (frozen in time); the parser list may grow later, never lose these.
    assert set(_FROZEN_METADATA_TYPES) <= claude_code.METADATA_TYPES


def _v9_store(path: Path, fixtures_root: Path) -> int:
    """A store as HUN-209 left it: full schema without `investigate_reports`, rows, user_version 9."""
    conn = store.connect(path)
    claude_code.ingest(conn, fixtures_root)
    n = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    conn.close()
    raw = sqlite3.connect(path)
    raw.execute("DROP TABLE investigate_reports")
    raw.execute("PRAGMA user_version = 9")
    raw.commit()
    raw.close()
    return n


def test_migration_10_adds_the_reports_table_keeps_every_row_and_leaves_a_backup(tmp_path, fixtures_root):
    path = tmp_path / "store.sqlite"
    n = _v9_store(path, fixtures_root)
    raw = sqlite3.connect(path)
    before = raw.execute("SELECT * FROM events ORDER BY id").fetchall()
    raw.close()

    conn = store.connect(path)  # AC 1: v9 -> latest, with a backup
    assert conn.execute("PRAGMA user_version").fetchone()[0] == LATEST >= 10
    assert (tmp_path / "store.sqlite.bak-v9").exists()
    assert conn.execute("SELECT * FROM events ORDER BY id").fetchall() == before and len(before) == n > 0
    assert [c[1] for c in conn.execute("PRAGMA table_info(investigate_reports)")] == [
        "id", "saved_utc", "server_version", "workspace", "account", "summary", "fixes_json", "snapshot_json",
    ]  # fmt: skip
    assert conn.execute("SELECT COUNT(*) FROM investigate_reports").fetchone()[0] == 0
    conn.close()
    backup = sqlite3.connect(tmp_path / "store.sqlite.bak-v9")  # the rollback copy is the v9 store, rows intact
    assert backup.execute("PRAGMA user_version").fetchone()[0] == 9
    assert backup.execute("SELECT COUNT(*) FROM events").fetchone()[0] == n
    backup.close()


def test_migration_10_is_idempotent_and_a_v10_store_is_not_touched(tmp_path, fixtures_root):
    from easy_personal.core.migrations import m0010_investigate_reports as m10

    path = tmp_path / "store.sqlite"
    _v9_store(path, fixtures_root)
    conn = store.connect(path)
    conn.execute(
        "INSERT INTO investigate_reports (saved_utc, server_version, summary, fixes_json, snapshot_json)"
        " VALUES ('2026-11-05T00:00:00Z', '0', 's', '[]', '{}')"
    )
    conn.commit()
    m10.apply(conn)  # CREATE TABLE IF NOT EXISTS: the saved row survives a repeated run
    assert conn.execute("SELECT COUNT(*) FROM investigate_reports").fetchone()[0] == 1
    conn.close()
    store.connect(path).close()
    assert sorted(p.name for p in tmp_path.glob("*.bak-v*")) == ["store.sqlite.bak-v9"]


# --- HUN-231: after a successful migration only the newest backup stays ---

OLD_BACKUPS = ("bak-v0", "bak-v3", "bak-v3-wal", "bak-v3-shm")


def _names(folder: Path) -> list[str]:
    return sorted(p.name for p in folder.iterdir())


def _store_one_version_behind(path: Path) -> int:
    """A store with data whose user_version is latest - 1 (the last migration is safe to run again)."""
    conn = store.connect(path)
    conn.execute("INSERT INTO meta (key, value) VALUES ('marker', 'kept')")
    conn.commit()
    conn.close()
    raw = sqlite3.connect(path)
    raw.execute(f"PRAGMA user_version = {LATEST - 1}")
    raw.commit()
    raw.close()
    return LATEST - 1


def _plant_file(file: Path, content: bytes = b"old") -> None:
    """An old backup: a real (empty) database file, or junk bytes for a `-wal`/`-shm` side file."""
    if file.name.endswith(("-wal", "-shm")):
        file.write_bytes(content)
    else:
        raw = sqlite3.connect(file)
        raw.execute("CREATE TABLE stale (x)")
        raw.commit()
        raw.close()


def _plant(folder: Path, names, content=b"old") -> None:
    for name in names:
        _plant_file(folder / f"store.sqlite.{name}", content)


def test_connect_keeps_only_the_newest_backup_and_leaves_other_files(tmp_path):
    path = tmp_path / "store.sqlite"
    n = _store_one_version_behind(path)
    _plant(tmp_path, OLD_BACKUPS)
    _plant(tmp_path, ["pre-v0.4.0", "bak-vX"])  # not backups of this store
    (tmp_path / "other.sqlite.bak-v1").write_bytes(b"other")
    store.connect(path).close()
    assert _names(tmp_path) == [
        "other.sqlite.bak-v1", "store.sqlite", f"store.sqlite.bak-v{n}", "store.sqlite.bak-vX",
        "store.sqlite.pre-v0.4.0",
    ]  # fmt: skip
    backup = sqlite3.connect(tmp_path / f"store.sqlite.bak-v{n}")
    assert backup.execute("PRAGMA user_version").fetchone()[0] == n
    assert backup.execute("SELECT value FROM meta WHERE key = 'marker'").fetchone()[0] == "kept"
    backup.close()


def test_stale_backup_of_the_same_number_is_replaced_and_its_side_files_and_higher_numbers_go(tmp_path):
    path = tmp_path / "store.sqlite"
    n = _store_one_version_behind(path)
    _plant(tmp_path, [f"bak-v{n}", f"bak-v{n}-wal", f"bak-v{n}-shm", f"bak-v{n + 5}"], b"stale")
    store.connect(path).close()
    assert _names(tmp_path) == ["store.sqlite", f"store.sqlite.bak-v{n}"]
    backup = sqlite3.connect(tmp_path / f"store.sqlite.bak-v{n}")
    assert backup.execute("SELECT value FROM meta WHERE key = 'marker'").fetchone()[0] == "kept"
    backup.close()


def test_backup_names_with_regex_characters_in_the_store_name(tmp_path):
    path = tmp_path / "st+re (1).sqlite"
    conn = sqlite3.connect(path)
    conn.execute("CREATE TABLE t (x)")
    conn.commit()

    def noop(c):
        c.execute("INSERT INTO t VALUES (1)")

    _plant_named = [f"{path.name}.bak-v0", f"{path.name}.bak-v1-wal", "stXre (1).sqlite.bak-v1"]
    for name in _plant_named:
        _plant_file(tmp_path / name)
    store.migrate(conn, path, [(1, noop)])
    conn.close()
    assert _names(tmp_path) == [path.name, f"{path.name}.bak-v0", "stXre (1).sqlite.bak-v1"]


def test_failing_migration_deletes_no_old_backup(tmp_path):
    path = tmp_path / "store.sqlite"
    conn = sqlite3.connect(path)
    conn.execute("CREATE TABLE t (x)")
    conn.commit()
    _plant(tmp_path, OLD_BACKUPS)

    def good(c):
        c.execute("INSERT INTO t VALUES (1)")

    def bad(c):
        raise RuntimeError("boom")

    with pytest.raises(RuntimeError, match="boom"):
        store.migrate(conn, path, [(1, good), (2, bad)])
    assert conn.execute("PRAGMA user_version").fetchone()[0] == 1
    assert _names(tmp_path) == sorted(["store.sqlite", "store.sqlite.bak-v0", *(f"store.sqlite.{b}" for b in OLD_BACKUPS[1:])])
    conn.close()


def test_up_to_date_store_deletes_nothing(tmp_path):
    path = tmp_path / "store.sqlite"
    store.connect(path).close()
    _plant(tmp_path, OLD_BACKUPS)
    store.connect(path).close()
    assert [n for n in _names(tmp_path) if ".bak-v" in n] == sorted(f"store.sqlite.{b}" for b in OLD_BACKUPS)


def test_a_failing_delete_never_fails_connect(tmp_path, monkeypatch):
    path = tmp_path / "store.sqlite"
    _store_one_version_behind(path)
    _plant(tmp_path, OLD_BACKUPS)

    def refuse(*args, **kwargs):
        raise PermissionError("no")

    monkeypatch.setattr("os.remove", refuse)
    monkeypatch.setattr("os.unlink", refuse)
    monkeypatch.setattr("pathlib.Path.unlink", refuse)
    conn = store.connect(path)
    assert conn.execute("PRAGMA user_version").fetchone()[0] == LATEST
    conn.close()
    assert (tmp_path / "store.sqlite.bak-v0").exists()  # nothing could be deleted, and that was fine


def test_new_store_deletes_nothing(tmp_path):
    path = tmp_path / "store.sqlite"
    _plant(tmp_path, OLD_BACKUPS)  # stray backups beside a store with no tables: no backup written, no cleanup
    store.connect(path).close()
    assert [n for n in _names(tmp_path) if ".bak-v" in n] == sorted(f"store.sqlite.{b}" for b in OLD_BACKUPS)
