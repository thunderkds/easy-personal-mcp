"""Read-only health check of the local store (FR-091): one plain dict for the CLI and the dashboard route.

The store is opened with `mode=ro`: no migration, no `VACUUM`, no backup, no row written, and no side file
next to the store. The answer holds counts, versions, sizes, times and file names only. It never holds a row's
content, an absolute path or the text of an exception (a fixed word stands in), so it is safe to print and to
send to the page (NFR-002). Every text that comes from the store (table and source names, times) is capped at
`TEXT_LIMIT` characters and has its non-printable characters replaced, so it cannot drive a terminal.

`last_ingest_utc` is the newest record time the store holds for a source: the store keeps no ingest clock.

`healthy` is three-valued: True (every check finished and found nothing), False (a problem was found; it always
wins) and None (no problem found, but a check did not finish: "not fully checked").

NFR-013 (< 2 s): `PRAGMA integrity_check` needs about 30 ms per MB (9 to 12 s for a 360 MB store), so a store over
`FULL_CHECK_MAX_BYTES` gets `PRAGMA quick_check` instead (`mode: "quick"`; it skips the cross-check of each index
against its table). The four reads run in parallel threads, each on its own read-only connection, and all stop
at one deadline (`BUDGET_S`). A read that did not finish is `None` and is named in `incomplete`.
`check(path, full=True)` has no deadline and always runs the full `integrity_check` (CLI `--full` only).
"""

import contextlib
import re
import shutil
import sqlite3
import tempfile
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from easy_personal.core.migrations import MIGRATIONS

BUSY_TIMEOUT_MS = 5000  # the wait for a lock when there is no deadline (`full`)
BUDGET_S = 1.5
FULL_CHECK_MAX_BYTES = 32_000_000
PROGRESS_OPS = 1000  # how many SQLite VM steps between two looks at the clock
TEXT_LIMIT = 64
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


def _clean(value, limit: int = TEXT_LIMIT) -> str | None:
    """A text from the store, safe to print: capped, non-printable characters replaced. Not a text -> TypeError."""
    if value is None:
        return None
    if not isinstance(value, str):
        raise TypeError("not a text")
    return "".join(c if c.isprintable() else "?" for c in value[:limit])


@contextlib.contextmanager
def _target(real: Path):
    """The URI to read, decided once before any thread starts. Nothing is ever created beside the store.

    - No `-wal`: no process has the store open, so `immutable=1` (no locks, no side files) is safe.
    - `-wal` and `-shm`: a live store; `mode=ro` creates nothing.
    - `-wal` but no `-shm`: `mode=ro` would create the `-shm`. Read a private copy in a temp folder instead.
    """
    wal, shm = Path(f"{real}-wal"), Path(f"{real}-shm")
    if not wal.exists():
        yield f"{real.as_uri()}?mode=ro&immutable=1"
    elif shm.exists():
        yield f"{real.as_uri()}?mode=ro"
    else:
        with tempfile.TemporaryDirectory(prefix="easy-personal-check-") as folder:
            copy = Path(folder) / "store.sqlite"
            shutil.copyfile(real, copy)
            shutil.copyfile(wal, Path(f"{copy}-wal"))
            yield f"{copy.as_uri()}?mode=ro"


def _open(uri: str, deadline: float | None, cancel: threading.Event) -> sqlite3.Connection:
    """A read-only connection that aborts its running statement at `deadline` (monotonic) or when `cancel` is set."""
    wait = BUSY_TIMEOUT_MS / 1000 if deadline is None else max(0.0, deadline - time.monotonic())
    conn = sqlite3.connect(uri, uri=True, timeout=wait)
    conn.execute(f"PRAGMA busy_timeout={int(wait * 1000)}")
    conn.set_progress_handler(
        lambda: cancel.is_set() or (deadline is not None and time.monotonic() >= deadline),
        PROGRESS_OPS,
    )
    return conn


def _verdict(pragma: str):
    """A read of a check pragma: `{"ok", "problems", "mode"}`; `problems` is the number of rows SQLite returned."""

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
    """Row count of every table in the schema (None for a table SQLite cannot read; a cut-off read raises)."""
    names = [r[0] for r in conn.execute(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]  # fmt: skip
    counts: dict[str, int | None] = {}
    for name in names:
        try:
            count = conn.execute(
                f'SELECT COUNT(*) FROM "{name.replace(chr(34), chr(34) * 2)}"'
            ).fetchone()[0]
        except sqlite3.DatabaseError as exc:
            if "interrupted" in str(exc):
                raise
            count = None
        counts[_clean(name)] = count
    return counts


def _no_sources() -> dict[str, dict]:
    return {name: {"last_ingest_utc": None, "abstained": None} for name in SOURCES}


def _count(value) -> int | None:
    return None if value is None else int(value)


def _sources(conn: sqlite3.Connection) -> dict[str, dict]:
    """Per source: newest record time and abstained rows; both None when the source has no rows ("no data")."""
    found = _no_sources()
    for name, last, abstained in conn.execute(
        "SELECT source, MAX(ts_utc), SUM(abstained) FROM events GROUP BY source"
    ):
        found[_clean(name)] = {"last_ingest_utc": _clean(last), "abstained": _count(abstained) or 0}
    for name, sql in _SOURCE_QUERIES.items():
        try:
            last, abstained = conn.execute(sql).fetchone()
        except sqlite3.OperationalError as exc:  # a table missing in an old schema: no data
            if "interrupted" in str(exc):
                raise
            continue
        if last is not None:
            found[name] = {"last_ingest_utc": _clean(last), "abstained": _count(abstained)}
    return found


def _version(conn: sqlite3.Connection) -> int:
    conn.execute("SELECT 1 FROM sqlite_master LIMIT 1").fetchone()  # raises on "not a database"
    return conn.execute("PRAGMA user_version").fetchone()[0]


def _read(uri: str, deadline: float | None, cancel: threading.Event, work):
    """Run `work(conn)` on its own connection: `(status, value)`, status ok, late, locked or error. Never raises."""
    conn = None
    try:
        conn = _open(uri, deadline, cancel)
        return "ok", work(conn)
    except (
        sqlite3.Error,
        ValueError,
        TypeError,
    ) as exc:  # ValueError covers UnicodeDecodeError; TypeError is `_clean`
        message = str(exc)
        if "interrupted" in message:
            return "late", None
        if "locked" in message or "busy" in message:
            return "locked", None
        return "error", None
    finally:
        if conn is not None:
            with contextlib.suppress(sqlite3.Error):
                conn.close()


def check(path: Path, full: bool = False, cancel: threading.Event | None = None) -> dict:
    """The health of the store file at `path`. Never writes, never migrates, never raises for a bad store.

    `full`: no deadline, always the full `integrity_check`. `cancel`: set it to stop every running read.
    """
    cancel = cancel or threading.Event()
    deadline = None if full else time.monotonic() + BUDGET_S
    latest = max((number for number, _ in MIGRATIONS), default=0)
    real = path.resolve()  # SQLite names `-wal` and `-shm` after the real file, not after a symlink
    exists = path.is_file()
    out = {
        "store_file": path.name,
        "exists": exists,
        "readable": False,
        "locked": False,
        "size_bytes": _size(path) if exists else None,
        "wal_bytes": _size(Path(f"{real}-wal")) or 0,
        "user_version": None,
        "latest_migration": latest,
        "migration_pending": False,
        "newer_than_code": False,
        "integrity_check": None,
        "foreign_key_check": None,
        "tables": None,
        "sources": None,
        "backups": _backups(path),
        "incomplete": [],
        "unreadable": [],
    }
    if exists:
        try:
            with _target(real) as uri:
                _fill(out, uri, deadline, cancel, latest, full)
        except OSError:  # the private copy could not be made
            out["unreadable"].append("file")
    if not exists or out["unreadable"] or (not out["readable"] and not out["incomplete"]):
        out["healthy"] = False  # no store, or a file SQLite cannot read: a problem found
    else:
        out["healthy"] = _healthy(out)
    return out


def _fill(out: dict, uri: str, deadline, cancel, latest: int, full: bool) -> None:
    status, version = _read(uri, deadline, cancel, _version)
    if status != "ok":
        out["locked"] = status == "locked"
        if status != "error":
            out["incomplete"].append("schema")
        return
    out.update(
        readable=True,
        user_version=version,
        migration_pending=version < latest,
        newer_than_code=version > latest,
    )
    pragma = (
        "integrity_check" if full or out["size_bytes"] <= FULL_CHECK_MAX_BYTES else "quick_check"
    )
    mode = "quick" if pragma == "quick_check" else "full"
    reads = {  # name -> (work, value when SQLite raises; None = the part is unreadable)
        "integrity_check": (_verdict(pragma), {"ok": False, "problems": 1, "mode": mode}),
        "foreign_key_check": (_foreign_keys, {"ok": False, "problems": 1}),
        "tables": (_tables, None),
        "sources": (_sources, None),
    }
    with ThreadPoolExecutor(len(reads)) as pool:
        futures = {
            name: pool.submit(_read, uri, deadline, cancel, work)
            for name, (work, _) in reads.items()
        }
        try:
            results = {name: future.result() for name, future in futures.items()}
        except KeyboardInterrupt:
            cancel.set()  # the running statements stop at the next look at the clock
            raise
    for name, (status, value) in results.items():
        if status == "ok":
            out[name] = value
        elif status == "error" and reads[name][1] is not None:
            out[name] = reads[name][1]
        elif status == "error":
            out["unreadable"].append(name)
        else:
            out["incomplete"].append(name)


def _healthy(out: dict) -> bool | None:
    """False when a problem was found (it always wins), None when a check did not finish, else True."""
    if out["migration_pending"] or out["newer_than_code"] or out["unreadable"]:
        return False
    if any(
        out[k] is not None and not out[k]["ok"] for k in ("integrity_check", "foreign_key_check")
    ):
        return False
    if out["tables"] is not None and any(c is None for c in out["tables"].values()):
        return False
    return None if out["incomplete"] else True


def exit_code(result: dict) -> int:
    """0 healthy, 1 a problem was found, 3 not fully checked."""
    return {True: 0, False: 1, None: 3}[result["healthy"]]


def lines(result: dict) -> list[str]:
    """The human form of `check()`: one line per field group, then the verdict."""
    late = result["incomplete"]

    def part(name: str, text) -> str:
        if name in late:
            return "not finished in time"
        return "unreadable" if name in result["unreadable"] else text()

    def verdict(name: str) -> str:
        v = result[name]
        mode = " (quick check)" if v.get("mode") == "quick" else ""
        return ("ok" if v["ok"] else f"{v['problems']} problem(s)") + mode

    size = f"{result['size_bytes']} bytes (wal {result['wal_bytes']} bytes)"
    if not result["exists"]:
        file_line = f"{result['store_file']} does not exist (wal {result['wal_bytes']} bytes)"
    elif result["locked"]:
        file_line = f"{result['store_file']} {size}, unreadable (locked)"
    elif not result["readable"]:
        file_line = f"{result['store_file']} {size}, not a readable database"
    else:
        file_line = f"{result['store_file']} {size}"
    schema = "unknown"
    if result["user_version"] is not None:
        schema = (
            f"user_version {result['user_version']}, code has {result['latest_migration']}, "
            f"migration pending: {'yes' if result['migration_pending'] else 'no'}"
            + (", store is newer than this program" if result["newer_than_code"] else "")
        )

    def table_text() -> str:
        return (
            ", ".join(f"{n} {'?' if c is None else c}" for n, c in result["tables"].items())
            or "none"
        )

    def source_text() -> str:
        return "; ".join(
            f"{name} "
            + (
                "no data"
                if s["last_ingest_utc"] is None
                else f"last {s['last_ingest_utc']}"
                + ("" if s["abstained"] is None else f", abstained {s['abstained']}")
            )
            for name, s in result["sources"].items()
        )

    def check_text(name: str) -> str:
        return "not checked" if result[name] is None else verdict(name)

    word = {True: "healthy", False: "NOT healthy"}.get(result["healthy"])
    if word is None:
        word = f"not fully checked ({', '.join(late)} not finished in time)"
        if "integrity_check" in late:
            word += "; run `easy-personal store-check --full` for the integrity verdict"
    return [
        f"file: {file_line}",
        f"schema: {schema}",
        f"integrity_check: {part('integrity_check', lambda: check_text('integrity_check'))}",
        f"foreign_key_check: {part('foreign_key_check', lambda: check_text('foreign_key_check'))}",
        f"tables: {part('tables', lambda: table_text() if result['tables'] is not None else 'not checked')}",
        f"sources: {part('sources', lambda: source_text() if result['sources'] is not None else 'not checked')}",
        f"backups: {', '.join(result['backups']) or 'none'}",
        f"verdict: {word}",
    ]
