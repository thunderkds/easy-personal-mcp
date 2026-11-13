"""Read-only health check of the local store (FR-091): one plain dict for the CLI and the dashboard route.

The store is opened with `mode=ro`: no migration, no `VACUUM`, no backup, no row written. The answer holds
counts, versions, sizes, times and file names only. It never holds a row's content, an absolute path or the
text of an exception (a fixed word stands in), so it is safe to print and to send to the page (NFR-002).

`last_ingest_utc` is the newest record time the store holds for a source: the store keeps no ingest clock.

NFR-013 (< 2 s): `PRAGMA integrity_check` needs about 30 ms per MB (9 to 12 s for a 360 MB store), so a store over
`FULL_CHECK_MAX_BYTES` gets `PRAGMA quick_check` instead (`mode: "quick"`; it skips the cross-check of each index
against its table). The four reads run in parallel threads, each on its own read-only connection, and all stop
at one deadline (`BUDGET_S`). A read that did not finish is `None` and is named in `incomplete`; it is never a problem.
"""

import re
import sqlite3
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from easy_personal.core.migrations import MIGRATIONS

BUSY_TIMEOUT_MS = 5000
BUDGET_S = 1.5
FULL_CHECK_MAX_BYTES = 32_000_000
PROGRESS_OPS = 1000  # how many SQLite VM steps between two looks at the clock
SOURCES = ("claude_code", "kit_trace", "context", "ui", "verifier")
# source -> (query giving (last time, abstained count); the count is None for a source with no such column)
_SOURCE_QUERIES = {
    "kit_trace": "SELECT MAX(ts_utc), SUM(abstained) FROM kit_trace_events",
    "context": "SELECT MAX(last_index_at_utc), NULL FROM context_index",
    "ui": "SELECT MAX(started_at_utc), NULL FROM ui_sessions",
    "verifier": "SELECT MAX(generated_at_utc), SUM(overall_abstained) FROM verifier_scores",
}


def _size(path: Path) -> int | None:
    try:
        return path.stat().st_size
    except OSError:
        return None


def _backups(path: Path) -> list[str]:
    """Names of the `<store>.bak-v<N>` files beside the store, oldest version first (never `-wal` / `-shm`)."""
    pattern = re.compile(re.escape(path.name) + r"\.bak-v(\d+)")
    try:
        found = [
            (int(m[1]), e.name) for e in path.parent.iterdir() if (m := pattern.fullmatch(e.name))
        ]
    except OSError:
        return []
    return [name for _, name in sorted(found)]


class _Expired(Exception):
    """The deadline passed while a read was running."""


def _open(path: Path, deadline: float) -> sqlite3.Connection:
    """A read-only connection that aborts its running statement once `deadline` (monotonic) has passed."""
    # A WAL store read through `mode=ro` still creates empty `-wal` and `-shm` files when none exist. With no `-wal`
    # no process has the store open, so `immutable=1` (no locks, no side files) is safe and keeps "nothing written" true.
    flags = "mode=ro" if Path(f"{path}-wal").exists() else "mode=ro&immutable=1"
    conn = sqlite3.connect(
        f"{path.resolve().as_uri()}?{flags}", uri=True, timeout=BUSY_TIMEOUT_MS / 1000
    )
    conn.execute(f"PRAGMA busy_timeout={BUSY_TIMEOUT_MS}")
    conn.set_progress_handler(lambda: time.monotonic() >= deadline, PROGRESS_OPS)
    return conn


def _verdict(pragma: str):
    """A read of a check pragma: `{"ok", "problems", "mode"}`. A pragma that raises counts as one problem."""

    def read(conn: sqlite3.Connection) -> dict:
        mode = "quick" if pragma == "quick_check" else "full"
        rows = conn.execute(f"PRAGMA {pragma}").fetchall()
        if rows == [("ok",)]:
            return {"ok": True, "problems": 0, "mode": mode}
        return {"ok": False, "problems": len(rows), "mode": mode}

    return read


def _foreign_keys(conn: sqlite3.Connection) -> dict:
    rows = conn.execute("PRAGMA foreign_key_check").fetchall()
    return {"ok": not rows, "problems": len(rows)}


def _tables(conn: sqlite3.Connection) -> dict[str, int | None]:
    """Row count of every table in the schema (None for a table SQLite cannot read)."""
    names = [r[0] for r in conn.execute(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]  # fmt: skip
    counts: dict[str, int | None] = {}
    for name in names:
        try:
            counts[name] = conn.execute(
                f'SELECT COUNT(*) FROM "{name.replace(chr(34), chr(34) * 2)}"'
            ).fetchone()[0]
        except sqlite3.DatabaseError:
            counts[name] = None
    return counts


def _no_sources() -> dict[str, dict]:
    return {name: {"last_ingest_utc": None, "abstained": None} for name in SOURCES}


def _sources(conn: sqlite3.Connection) -> dict[str, dict]:
    """Per source: newest record time and abstained rows; both None when the source has no rows ("no data")."""
    found = _no_sources()
    try:
        for name, last, abstained in conn.execute(
            "SELECT source, MAX(ts_utc), SUM(abstained) FROM events GROUP BY source"
        ):
            found[name] = {"last_ingest_utc": last, "abstained": abstained or 0}
    except sqlite3.OperationalError as exc:
        if "interrupted" in str(exc):
            raise
    for name, sql in _SOURCE_QUERIES.items():
        try:
            last, abstained = conn.execute(sql).fetchone()
        except sqlite3.OperationalError as exc:  # a table missing in an old schema: no data
            if "interrupted" in str(exc):
                raise
            continue
        if last is not None:
            found[name] = {
                "last_ingest_utc": last,
                "abstained": abstained if abstained is None else int(abstained),
            }
    return found


def _version(conn: sqlite3.Connection) -> int:
    conn.execute("SELECT 1 FROM sqlite_master LIMIT 1").fetchone()  # raises on "not a database"
    return conn.execute("PRAGMA user_version").fetchone()[0]


def _read(path: Path, deadline: float, work, on_error):
    """Run `work(conn)` on its own connection. Past the deadline -> None; any other SQLite error -> `on_error`."""
    conn = None
    try:
        conn = _open(path, deadline)
        return work(conn)
    except (
        sqlite3.DatabaseError
    ) as exc:  # "interrupted" is how the progress handler stops a statement
        return None if "interrupted" in str(exc) else on_error
    finally:
        if conn is not None:
            conn.close()


def check(path: Path) -> dict:
    """The health of the store file at `path`. Never writes, never migrates, never raises for a bad store."""
    deadline = time.monotonic() + BUDGET_S
    latest = max((number for number, _ in MIGRATIONS), default=0)
    exists = path.is_file()
    out = {
        "store_file": path.name,
        "exists": exists,
        "readable": False,
        "size_bytes": _size(path) if exists else None,
        "wal_bytes": _size(Path(f"{path}-wal")) or 0,
        "user_version": None,
        "latest_migration": latest,
        "migration_pending": False,
        "newer_than_code": False,
        "integrity_check": None,
        "foreign_key_check": None,
        "tables": {},
        "sources": _no_sources(),
        "backups": _backups(path),
        "incomplete": [],
    }
    version = _read(path, deadline, _version, None) if exists else None
    if version is not None:
        out.update(
            readable=True,
            user_version=version,
            migration_pending=version < latest,
            newer_than_code=version > latest,
        )
        pragma = "integrity_check" if out["size_bytes"] <= FULL_CHECK_MAX_BYTES else "quick_check"
        corrupt = {"ok": False, "problems": 1}
        reads = {
            "integrity_check": (
                _verdict(pragma),
                corrupt | {"mode": "quick" if pragma == "quick_check" else "full"},
            ),
            "foreign_key_check": (_foreign_keys, corrupt),
            "tables": (_tables, {}),
            "sources": (_sources, _no_sources()),
        }
        with ThreadPoolExecutor(len(reads)) as pool:
            futures = {
                name: pool.submit(_read, path, deadline, work, on_error)
                for name, (work, on_error) in reads.items()
            }
            for name, future in futures.items():
                out[name] = future.result()
                if out[name] is None:
                    out["incomplete"].append(name)
        out["tables"], out["sources"] = out["tables"] or {}, out["sources"] or _no_sources()
    out["healthy"] = bool(
        out["readable"]
        and not out["migration_pending"]
        and not out["newer_than_code"]
        and all(out[k] is None or out[k]["ok"] for k in ("integrity_check", "foreign_key_check"))
    )
    return out


def lines(result: dict) -> list[str]:
    """The human form of `check()`: one line per field group, then the verdict."""
    late = result["incomplete"]

    def verdict(name: str) -> str:
        v = result[name]
        if v is None:
            return "not finished in time" if name in late else "not checked"
        mode = " (quick check)" if v.get("mode") == "quick" else ""
        return ("ok" if v["ok"] else f"{v['problems']} problem(s)") + mode

    if not result["exists"]:
        file_line = f"{result['store_file']} does not exist (wal {result['wal_bytes']} bytes)"
    elif not result["readable"]:
        file_line = f"{result['store_file']} {result['size_bytes']} bytes (wal {result['wal_bytes']} bytes), not a readable database"
    else:
        file_line = (
            f"{result['store_file']} {result['size_bytes']} bytes (wal {result['wal_bytes']} bytes)"
        )
    schema = (
        "unknown"
        if result["user_version"] is None
        else (
            f"user_version {result['user_version']}, code has {result['latest_migration']}, migration pending: "
            f"{'yes' if result['migration_pending'] else 'no'}"
            + (", store is newer than this program" if result["newer_than_code"] else "")
        )
    )
    tables = (
        "not finished in time"
        if "tables" in late
        else (
            ", ".join(f"{n} {'?' if c is None else c}" for n, c in result["tables"].items())
            or "none"
        )
    )
    sources = (
        "not finished in time"
        if "sources" in late
        else "; ".join(
            f"{name} "
            + (
                "no data"
                if s["last_ingest_utc"] is None
                else f"last {s['last_ingest_utc']}"
                + ("" if s["abstained"] is None else f", abstained {s['abstained']}")
            )
            for name, s in result["sources"].items()
        )
    )
    return [
        f"file: {file_line}",
        f"schema: {schema}",
        f"integrity_check: {verdict('integrity_check')}",
        f"foreign_key_check: {verdict('foreign_key_check')}",
        f"tables: {tables}",
        f"sources: {sources}",
        f"backups: {', '.join(result['backups']) or 'none'}",
        f"verdict: {'healthy' if result['healthy'] else 'NOT healthy'}",
    ]
