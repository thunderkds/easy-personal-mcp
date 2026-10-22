"""`verifier` source: easy-verifier score JSON -> newest overall score per project (FR-011).

Read-only. For each project that still exists, the newest `evidence-report-*.json` (by the UTC time in the file name) is read from `<project>/reports/` and from `<container>/<project folder name>/`
(default `~/.easy-verifier-reports`). Only `schema_version` 1 is read; HTML is never opened. If the newest file has another integer
`schema_version`, the project is "abstained" (unknown_schema_version). Only
scope, `generated_at` and `overall` are kept; `dimensions` and any other key are dropped with the
decoded file. A file that is unreadable, oversized, not a regular file, a symlink, or of another
shape is skipped (counted); it never fails ingest. A project with no readable JSON is "no data".
"""

import json
import os
import re
import sqlite3
import stat
from pathlib import Path

from easy_personal.core import config
from easy_personal.sources.claude_code import _label, _utc
from easy_personal.sources.kit_trace import _projects

SCHEMA_VERSION = 1
MAX_BYTES = 1_000_000  # a larger file is skipped, not decoded
UNKNOWN = "unknown"  # a readable JSON object whose integer schema_version is not 1
_STAMP = re.compile(r"(\d{8}T\d{6}-\d+Z)\.json$")


def _number(value) -> float | None:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    return float(value) if 0 <= value <= 100 else None


def parse_report(raw: bytes) -> tuple | str | None:
    """(scope, generated_at_utc, value, abstained, reason_code); `UNKNOWN` for another schema
    version; None when it cannot be read."""
    try:
        record = json.loads(raw)
    except (ValueError, RecursionError):
        return None
    if not isinstance(record, dict):
        return None
    version = record.get("schema_version")
    if isinstance(version, bool) or not isinstance(version, int):
        return None
    if version != SCHEMA_VERSION:
        return UNKNOWN
    scope, generated = _label(record.get("scope")), _utc(record.get("generated_at"))
    overall = record.get("overall")
    if scope is None or generated is None or not isinstance(overall, dict):
        return None
    abstained, reason = overall.get("abstained"), overall.get("reason_code")
    if not isinstance(abstained, bool) or not (reason is None or _label(reason)):
        return None
    value = _number(overall.get("value"))
    if abstained != (value is None) or (overall.get("value") is not None and value is None):
        return None  # abstained must carry a null value, a scored report a number in 0..100
    return scope, generated, value, int(abstained), reason


def _read(path: Path, folder: Path) -> bytes | None:
    """File bytes, or None for a symlink, a non-regular or oversized file, or an unreadable one."""
    try:
        if path.resolve().parent != folder.resolve():
            return None
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    except OSError:
        return None
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_BYTES:
            return None
        with os.fdopen(fd, "rb", closefd=False) as handle:
            return handle.read(MAX_BYTES + 1)
    except OSError:
        return None
    finally:
        os.close(fd)


def _candidates(folder: Path, root: Path) -> tuple[list, int]:
    """([(order key, row or UNKNOWN)], skipped count) for the readable reports of the folder."""
    found, skipped = [], 0
    try:
        if not folder.is_dir() or folder.resolve().parent != root.resolve():
            return [], 0
        paths = sorted(folder.glob("evidence-report-*.json"))
    except OSError:
        return [], 0
    for path in paths:
        raw = _read(path, folder)
        row = parse_report(raw) if raw is not None and len(raw) <= MAX_BYTES else None
        if row is None:
            skipped += 1
        else:  # the file name carries the UTC time; an unknown schema's own fields are not trusted
            stamp = _STAMP.search(path.name)
            found.append(((stamp.group(1) if stamp else "", path.name), row))
    return found, skipped


def ingest(conn: sqlite3.Connection, container: Path | None = None) -> dict:
    """Store the newest report per existing project; return this run's counts. Never raises.

    The newest file (by the UTC time in its name) decides: a readable report of an unknown schema
    version makes the project "abstained" rather than showing an older score as current.
    """
    try:
        return _ingest(conn, container)
    except Exception as exc:  # noqa: BLE001 - FR-011: never fail the run; type name is not content
        return {"status": "failed", "reason": type(exc).__name__}


def _ingest(conn: sqlite3.Connection, container: Path | None) -> dict:
    container = container if container is not None else config.verifier_reports_dir()
    projects = _projects(conn)
    names = [Path(p).name for p in projects]
    rows, skipped = [], 0
    for project in projects:
        repo = Path(project)
        folders = [(repo / "reports", repo)]
        if names.count(repo.name) == 1:  # two projects with one folder name: shared dir is ambiguous
            folders.append((container / repo.name, container))
        found = []
        for folder, root in folders:
            items, bad = _candidates(folder, root)
            found += items
            skipped += bad
        if not found:
            continue
        row = max(found, key=lambda item: item[0])[1]
        if row == UNKNOWN:
            rows.append((project, UNKNOWN, None, None, None, 0, None))
        else:
            rows.append((project, "ok", *row))
    with conn:
        conn.execute("DELETE FROM verifier_scores")
        conn.executemany("INSERT INTO verifier_scores VALUES (?, ?, ?, ?, ?, ?, ?)", rows)
    return {"status": "ok" if rows else "no data", "projects": len(rows), "skipped": skipped}


def verifier_part(conn: sqlite3.Connection, project: str) -> dict:
    """The verifier entry of kit_summary for one project; "no data" when no score JSON was read."""
    row = conn.execute(
        "SELECT status, scope, generated_at_utc, overall_value, overall_abstained, reason_code "
        "FROM verifier_scores WHERE project_path = ?",
        (project,),
    ).fetchone()
    if row is None:
        return {"status": "no data"}
    status, scope, generated, value, abstained, reason = row
    if status == UNKNOWN:
        return {"status": "abstained", "reason": "unknown_schema_version"}
    return {
        "status": "ok",
        "scope": scope,
        "generated_at": generated,
        "overall": {
            "value": None if value is None else int(value) if value == int(value) else value,
            "abstained": bool(abstained),
            "reason_code": reason,
        },
    }
