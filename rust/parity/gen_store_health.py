#!/usr/bin/env python3
"""Store health oracle (T059 / RS-003): freeze what Python's `core/store_health.check` answers over hand-made stores, as
golden JSON and text in `rust/parity/golden-store-health/`.

    rust/parity/.venv/bin/python rust/parity/gen_store_health.py           # (re)write rust/parity/golden-store-health/
    rust/parity/.venv/bin/python rust/parity/gen_store_health.py --check   # regenerate in a temp folder and compare: exit 0 equal

Each case is a folder of files (the data folder of one store: `store.sqlite`, maybe `-wal`, `-shm`, `bak-v<N>`) plus
`<case>.json` (the whole result of `check()`) and `<case>.txt` (its `lines()`, the human form). `manifest.json` lists the
cases with the options that Python ran them with (`budget_s`, `full`, `full_check_max_bytes`: the time budget is forced the
same way in Rust) and the `setup` a test must repeat before it runs a case (`lock`: a writer in exclusive mode holds the
store). The fixture stores are binary files that Python made once; only the answers are compared by `--check`.

Left out on purpose: the time-dependent cases (a real 1.5 s cut-off is raced, not fixed); a symlinked store and a store
with a live `-shm` (a symlink does not survive a git checkout everywhere; a live store needs a running writer): the Rust
tests in `crates/core/tests/store_health_parity.rs` make both on the fly.
"""

import argparse
import gc
import json
import os
import shutil
import sqlite3
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
PARITY = Path(__file__).resolve().parent
REPO = PARITY.parents[1]
sys.path.insert(0, str(REPO / "src"))
sys.path.insert(0, str(PARITY))
OUT = PARITY / "golden-store-health"
CANARY = "CANARY-SECRET-244"


def add_events(conn, count: int, source: str = "claude_code") -> None:
    conn.executemany(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,"
        " project_path, model, message_id, input_tokens, output_tokens)"
        " VALUES ('m', ?, '/x', 1, ?, 0, ?, 's', ?, 'model-0', ?, 10, 5)",
        ((source, n, f"2026-01-01T00:{n // 60 % 60:02d}:{n % 60:02d}.000000Z", f"/work/{CANARY}", f"m{n}") for n in range(count)),
    )


def migrated(folder: Path):
    """A migrated, closed store in `folder`; returns its path."""
    from easy_personal.core import store

    path = folder / "store.sqlite"
    with store.opened(path):
        pass
    return path


def checkpointed(path: Path) -> None:
    with sqlite3.connect(path) as conn:
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    for side in ("-wal", "-shm"):
        Path(f"{path}{side}").unlink(missing_ok=True)


def seeded(folder: Path, events: int = 5):
    """The `clean` store: a few events (one abstained), a kit trace row, a ui row."""
    from easy_personal.core import store

    path = migrated(folder)
    with store.opened(path) as conn:
        add_events(conn, events)
        conn.execute("UPDATE events SET abstained = 1 WHERE byte_offset = 0")
        conn.execute(
            "INSERT INTO kit_trace_events (machine_id, source_path, inode, byte_offset, project_path, abstained, ts_utc)"
            " VALUES ('m', '/t', 1, 0, ?, 1, '2026-02-01T00:00:00.000000Z')", (f"/work/{CANARY}",))
        conn.execute("INSERT INTO ui_sessions VALUES ('u1', 'ok', '2026-03-01T00:00:00Z', NULL, 2)")
        conn.commit()
    checkpointed(path)
    return path


def corrupt_page(path: Path, page: int) -> None:
    size = sqlite3.connect(path).execute("PRAGMA page_size").fetchone()[0]
    with open(path, "r+b") as handle:
        handle.seek(size * (page - 1))
        handle.write(b"\xff" * size)


def user_version(path: Path, value: int) -> None:
    with sqlite3.connect(path) as conn:
        conn.execute(f"PRAGMA user_version = {value}")


def latest() -> int:
    from easy_personal.core.migrations import MIGRATIONS

    return max(n for n, _ in MIGRATIONS)


# ---- the cases: name -> (builder of the folder, options, setup) -------------------------------------------------------


def c_clean(f):
    seeded(f)


def c_empty(f):
    checkpointed(migrated(f))


def c_behind(f):
    user_version(seeded(f), latest() - 1)


def c_newer(f):
    user_version(seeded(f), latest() + 1)


def big(f, count=1500):
    from easy_personal.core import store

    path = migrated(f)
    with store.opened(path) as conn:
        add_events(conn, count)
        conn.commit()
    checkpointed(path)
    return path


def c_corrupt_page(f):
    path = big(f)
    pages = path.stat().st_size // sqlite3.connect(path).execute("PRAGMA page_size").fetchone()[0]
    corrupt_page(path, pages // 2)


def c_damaged_index(f):
    """An index whose key no longer equals its table row: a byte of the key is changed inside the index page."""
    path = seeded(f)
    with sqlite3.connect(path) as conn:
        conn.execute("CREATE TABLE ix (k TEXT)")
        conn.execute("CREATE INDEX ix_k ON ix (k)")
        conn.executemany("INSERT INTO ix VALUES (?)", [(f"key-{n:04d}",) for n in range(20)])
    checkpointed(path)
    with sqlite3.connect(path) as conn:
        root = conn.execute("SELECT rootpage FROM sqlite_master WHERE name = 'ix_k'").fetchone()[0]
        size = conn.execute("PRAGMA page_size").fetchone()[0]
    data = bytearray(path.read_bytes())
    start = (root - 1) * size
    at = bytes(data[start:start + size]).index(b"key-0007") + 7
    data[start + at] = ord("8")
    path.write_bytes(bytes(data))


def c_wal_no_shm(f):
    """Rows that live only in the `-wal` of a store that has no `-shm` (a crashed or copied store)."""
    path = seeded(f)
    writer = sqlite3.connect(path)
    writer.execute("PRAGMA journal_mode=WAL")
    writer.execute("PRAGMA wal_autocheckpoint=0")
    writer.execute("INSERT INTO meta (key, value) VALUES ('k', 'v')")
    writer.commit()
    shutil.copyfile(path, f"{path}.copy")
    shutil.copyfile(f"{path}-wal", f"{path}.copy-wal")
    writer.close()
    for side in ("", "-wal", "-shm"):
        Path(f"{path}{side}").unlink(missing_ok=True)
    os.rename(f"{path}.copy", path)
    os.rename(f"{path}.copy-wal", f"{path}-wal")


def c_locked(f):
    """A store whose `-wal` and `-shm` are there and whose writer (the test) holds it exclusively. See `setup`."""
    path = seeded(f)
    gc.collect()  # an unclosed connection of the builders would remove the two files below when it is collected
    Path(f"{path}-wal").write_bytes(b"")
    Path(f"{path}-shm").write_bytes(b"")


def c_hostile(f):
    """Names and times from the store that are long, hold control characters, or collide once cleaned."""
    path = seeded(f)
    with sqlite3.connect(path) as conn:
        conn.execute('CREATE TABLE "evil\x1b[31mtable\nline' + "x" * 200 + '" (a)')
        conn.execute('CREATE TABLE "a\x01b" (a)')
        conn.execute('CREATE TABLE "a\x02b" (a)')
        conn.executemany('INSERT INTO "a\x02b" VALUES (?)', [(1,), (2,)])
        conn.execute('CREATE TABLE "café ☕ ​ zero width" (a)')
        conn.execute("UPDATE events SET source = ?, ts_utc = ? WHERE byte_offset = 1",
                     ("bad\x1b]0;title\x07src\n" + "y" * 200, "2030\x1b[2J" + "z" * 200))
        conn.execute("UPDATE events SET source = ? WHERE byte_offset = 2", ("c\x01ollide",))
        conn.execute("UPDATE events SET source = ? WHERE byte_offset = 3", ("c\x02ollide",))
    checkpointed(path)


def c_blob_time(f):
    path = seeded(f)
    with sqlite3.connect(path) as conn:
        conn.execute("UPDATE events SET ts_utc = X'0102ff' WHERE byte_offset = 1")
    checkpointed(path)


def c_int_time(f):
    path = seeded(f)
    with sqlite3.connect(path) as conn:
        conn.execute("UPDATE ui_sessions SET started_at_utc = 5")
    checkpointed(path)


def c_missing(f):
    f.mkdir(parents=True, exist_ok=True)


def c_missing_with_wal(f):
    f.mkdir(parents=True, exist_ok=True)
    (f / "store.sqlite-wal").write_bytes(b"w" * 10)
    (f / "store.sqlite.bak-v3").write_bytes(b"x")


def c_zero_byte(f):
    f.mkdir(parents=True, exist_ok=True)
    (f / "store.sqlite").write_bytes(b"")


def c_not_a_database(f):
    f.mkdir(parents=True, exist_ok=True)
    (f / "store.sqlite").write_bytes(b"this is not a database " * 200)


def c_backups(f):
    seeded(f)
    for name in ("store.sqlite.bak-v10", "store.sqlite.bak-v3", "store.sqlite.bak-v4", f"store.sqlite.bak-v5-{CANARY}",
                 "store.sqlite.bak-v3-wal", f"{CANARY}.txt", "store.sqlite.bak-v", "store.sqlite.bak-v007", "store.sqlite.bak-v7"):
        (f / name).write_bytes(b"x")


def c_fk_violation(f):
    path = seeded(f)
    with sqlite3.connect(path) as conn:
        conn.execute("INSERT INTO workspace_repos (workspace_id, repo_path) VALUES (999, ?)", (f"/{CANARY}",))
    checkpointed(path)


def c_later_table(f):
    path = seeded(f)
    with sqlite3.connect(path) as conn:
        conn.execute("CREATE TABLE added_by_a_later_migration (x)")
        conn.executemany("INSERT INTO added_by_a_later_migration VALUES (?)", [(1,), (2,)])
    checkpointed(path)


def c_bad_table_name(f):
    """A table name that is not UTF-8: SQLite itself refuses the schema."""
    path = seeded(f)
    conn = sqlite3.connect(path)
    conn.execute("CREATE TABLE badname (a)")
    conn.execute("PRAGMA writable_schema=ON")
    conn.execute("UPDATE sqlite_master SET name = CAST(X'c328ff' AS TEXT) WHERE name = 'badname'")
    conn.execute("PRAGMA writable_schema=OFF")
    conn.commit()
    conn.close()  # the last connection merges and removes the `-wal` (a checkpoint would have to read the bad name)


def c_big(f):
    big(f, 1500)


def c_no_events(f):
    """Every optional table absent: an old schema (the per-source queries find no table)."""
    path = seeded(f)
    with sqlite3.connect(path) as conn:
        for table in ("context_index", "ui_sessions", "verifier_scores", "kit_trace_events"):
            conn.execute(f"DROP TABLE {table}")
    checkpointed(path)


FIXTURES = {  # fixture folder -> builder (several cases may read one fixture)
    "clean": c_clean, "empty": c_empty, "behind": c_behind, "newer": c_newer, "corrupt_page": c_corrupt_page,
    "damaged_index": c_damaged_index, "wal_no_shm": c_wal_no_shm, "locked": c_locked, "hostile": c_hostile,
    "blob_time": c_blob_time, "int_time": c_int_time, "missing": c_missing, "missing_with_wal": c_missing_with_wal,
    "zero_byte": c_zero_byte, "not_a_database": c_not_a_database, "backups": c_backups, "fk_violation": c_fk_violation,
    "later_table": c_later_table, "bad_table_name": c_bad_table_name, "no_optional_tables": c_no_events, "big": c_big,
}  # fmt: skip
CASES = [  # (case, fixture, options, setup)
    ("clean", "clean", {}, None),
    ("empty", "empty", {}, None),
    ("behind", "behind", {}, None),
    ("newer", "newer", {}, None),
    ("corrupt_page", "corrupt_page", {}, None),
    ("corrupt_page_full", "corrupt_page", {"full": True}, None),
    ("damaged_index", "damaged_index", {}, None),
    ("wal_no_shm", "wal_no_shm", {}, None),
    ("locked", "locked", {"budget_s": 0.3}, "lock"),
    ("hostile", "hostile", {}, None),
    ("blob_time", "blob_time", {}, None),
    ("int_time", "int_time", {}, None),
    ("missing", "missing", {}, None),
    ("missing_with_wal", "missing_with_wal", {}, None),
    ("zero_byte", "zero_byte", {}, None),
    ("not_a_database", "not_a_database", {}, None),
    ("backups", "backups", {}, None),
    ("fk_violation", "fk_violation", {}, None),
    ("later_table", "later_table", {}, None),
    ("bad_table_name", "bad_table_name", {}, None),
    ("no_optional_tables", "no_optional_tables", {}, None),
    ("budget_zero", "big", {"budget_s": 0.0}, None),
    ("budget_zero_and_behind", "behind", {"budget_s": 0.0}, None),
    ("quick_check", "clean", {"full_check_max_bytes": 1}, None),
    ("full_no_budget", "big", {"full": True, "budget_s": 0.0}, None),
    ("full_on_a_clean_store", "clean", {"full": True}, None),
]


def run_case(path: Path, options: dict, setup: str | None) -> tuple[dict, list[str]]:
    """What Python answers for the store at `path` (after the `setup`), with the same constants forced."""
    from easy_personal.core import store_health

    held = None
    if setup == "lock":
        held = sqlite3.connect(path, isolation_level=None)
        held.execute("PRAGMA locking_mode=EXCLUSIVE")
        held.execute("SELECT count(*) FROM meta").fetchone()
    saved = (store_health.BUDGET_S, store_health.FULL_CHECK_MAX_BYTES)
    try:
        store_health.BUDGET_S = options.get("budget_s", saved[0])
        store_health.FULL_CHECK_MAX_BYTES = options.get("full_check_max_bytes", saved[1])
        result = store_health.check(path, full=options.get("full", False))
        return result, store_health.lines(result)
    finally:
        store_health.BUDGET_S, store_health.FULL_CHECK_MAX_BYTES = saved
        if held is not None:
            held.close()


def generate(out: Path) -> None:
    import gen_golden

    gen_golden.refuse_real_paths(out)
    gen_golden.guard_sqlite()
    work = Path(tempfile.mkdtemp(prefix="easy-personal-health-oracle-")).resolve()
    os.environ["HOME"] = str(work / "nowhere")  # nothing here can reach a real home
    os.environ["EASY_PERSONAL_HOME"] = str(work / "home")
    try:
        if out.exists():
            for entry in out.iterdir():
                if entry.is_dir():
                    shutil.rmtree(entry)
                else:
                    entry.unlink()
        out.mkdir(parents=True, exist_ok=True)
        manifest = {"generator": "rust/parity/gen_store_health.py", "python_sqlite": sqlite3.sqlite_version,
                    "latest_migration": latest(), "cases": []}
        for fixture, build in FIXTURES.items():
            (out / fixture).mkdir(parents=True)
            build(out / fixture)
        for name, fixture, options, setup in CASES:
            folder = out / fixture
            # Python runs over a copy, so the committed fixture is what the Rust test copies too
            live = work / "run" / name
            shutil.copytree(folder, live)
            result, lines = run_case(live / "store.sqlite", options, setup)
            (out / f"{name}.json").write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
            (out / f"{name}.txt").write_text("\n".join(lines) + "\n")
            manifest["cases"].append({"name": name, "fixture": fixture, "options": options, "setup": setup,
                                      "files": sorted(p.name for p in folder.iterdir())})  # fmt: skip
        (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        answers = [p for p in [*out.glob("*.json"), *out.glob("*.txt")] if p.name != "manifest.json"]
        leaked = [p for p in answers if any(bad in p.read_text() for bad in (str(work), "/home/", CANARY))]
        if leaked:
            raise SystemExit(f"an answer holds the work folder, a path of the real home or a planted canary: {leaked}")
    finally:
        shutil.rmtree(work, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if not args.check:
        generate(OUT)
        print(f"wrote {OUT}")
        return 0
    with tempfile.TemporaryDirectory() as tmp:
        fresh = Path(tmp) / "golden-store-health"
        generate(fresh)
        names = sorted(p.name for p in fresh.iterdir() if p.is_file())
        stored = sorted(p.name for p in OUT.iterdir() if p.is_file())
        bad = [n for n in names if n not in stored or (fresh / n).read_bytes() != (OUT / n).read_bytes()]
        if bad or names != stored:
            print("different:", bad or "file lists differ")
            return 1
    print("golden-store-health is equal to a fresh run")
    return 0


if __name__ == "__main__":
    sys.exit(main())
