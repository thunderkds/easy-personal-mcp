"""`ui` source: easy-ui `session-*.json` files -> one overall UI-check row per session (FR-013).

Read-only. Only `id`, `status`, `startedAt`, `endedAt` and the number of `actions` are taken. `target`,
`issues` and the contents of `actions` are caller-authored text: they are never looked up, and the
decoded file is dropped as soon as the five values are read. A file that cannot be read is skipped
(abstain); it never fails ingest. Sessions are overall, never attributed to a project.
"""

import json
import sqlite3
from pathlib import Path

from easy_personal.core import config
from easy_personal.sources.claude_code import _label, _utc

MAX_BYTES = 20_000_000  # a larger file is skipped, not decoded


def parse_session(raw: bytes) -> tuple | None:
    """(id, status, started_utc, ended_utc, action_count), or None when it cannot be read."""
    try:
        record = json.loads(raw)
    except (ValueError, RecursionError):
        return None
    if not isinstance(record, dict):
        return None
    sid, status = _label(record.get("id")), _label(record.get("status"))
    started, ended = _utc(record.get("startedAt")), _utc(record.get("endedAt"))
    actions = record.get("actions")
    if sid is None or status is None or started is None or not isinstance(actions, list):
        return None
    if record.get("endedAt") is not None and ended is None:
        return None
    return sid, status, started, ended, len(actions)


def ingest(conn: sqlite3.Connection, folder: Path | None = None) -> dict:
    """Store every readable session in the folder; return this run's counts. Never raises."""
    folder = folder if folder is not None else config.ui_reports_dir()
    rows: dict[str, tuple] = {}
    skipped = 0
    try:
        paths = sorted(folder.glob("session-*.json")) if folder and folder.is_dir() else None
    except OSError:
        paths = None
    if paths is None:  # unset or missing folder: nothing to show
        with conn:
            conn.execute("DELETE FROM ui_sessions")
        return {"status": "no data", "sessions": 0, "skipped": 0}
    for path in paths:
        try:
            if path.stat().st_size > MAX_BYTES:
                skipped += 1
                continue
            row = parse_session(path.read_bytes())
        except OSError:
            skipped += 1
            continue
        if row is None:
            skipped += 1
        else:
            rows[row[0]] = row
    with conn:
        conn.executemany("INSERT OR REPLACE INTO ui_sessions VALUES (?, ?, ?, ?, ?)", rows.values())
    return {"status": "ok", "sessions": len(rows), "skipped": skipped}


def ui_part(conn: sqlite3.Connection) -> dict:
    """The overall UI-check entry of kit_summary; "no data" when nothing was ingested."""
    count, passed, failed, actions, first, last = conn.execute(
        "SELECT COUNT(*), COALESCE(SUM(status = 'passed'), 0), COALESCE(SUM(status = 'failed'), 0), "
        "COALESCE(SUM(action_count), 0), MIN(started_at_utc), MAX(started_at_utc) FROM ui_sessions"
    ).fetchone()
    if not count:
        return {"status": "no data"}
    return {
        "status": "ok",
        "sessions": count,
        "passed": passed,
        "failed": failed,
        "other": count - passed - failed,
        "actions": actions,
        "first_utc": first,
        "last_utc": last,
    }
