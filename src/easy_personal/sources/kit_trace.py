"""`kit_trace` source: kernel traces (<project>/memory/event-trace/*.jsonl) -> metadata rows.

Read-only. Only `timestamp`, `tool_name`, `is_error` and the kit task id (the file name, with
`_untagged` meaning no task) are taken from a line. The `summary` field holds raw tool input: it
is never looked up, and the decoded line is dropped as soon as the four values are read (FR-010).
Kit sources read only projects that still exist on disk (FR-009).
"""

import json
import sqlite3
from pathlib import Path

from easy_personal.core import store
from easy_personal.sources.claude_code import _label, _utc, read_complete_lines

SOURCE = "kit_trace"
UNTAGGED = "_untagged"


def parse_line(line: bytes) -> tuple[str | None, str | None, int | None]:
    """(ts_utc, tool_name, is_error) of one line; all None when it cannot be read (abstain)."""
    try:
        record = json.loads(line)
    except ValueError:
        return None, None, None
    if not isinstance(record, dict):
        return None, None, None
    ts, tool, error = (
        _utc(record.get("timestamp")),
        _label(record.get("tool_name")),
        record.get("is_error"),
    )
    if ts is None or tool is None or not isinstance(error, bool):
        return None, None, None
    return ts, tool, int(error)


def _projects(conn: sqlite3.Connection) -> list[str]:
    rows = conn.execute(
        "SELECT DISTINCT project_path FROM events "
        "WHERE source = 'claude_code' AND project_path IS NOT NULL ORDER BY project_path"
    )
    return [p for (p,) in rows if Path(p).is_dir()]


def _rows(path: Path, offset: int, project: str, task_id: str | None):
    for line_offset, line, next_offset in read_complete_lines(path, offset):
        if not line.strip():
            yield next_offset, None
            continue
        ts, tool, error = parse_line(line)
        yield next_offset, (line_offset, project, task_id, int(ts is None), ts, tool, error)


def _save(conn, path: str, inode: int, offset: int, rows) -> int:
    mid = store.machine_id(conn)
    end = [offset]

    def values():
        for next_offset, row in rows:
            end[0] = next_offset
            if row is not None:
                yield (mid, path, inode, *row)

    with conn:
        before = conn.total_changes
        conn.executemany(
            "INSERT OR IGNORE INTO kit_trace_events (machine_id, source_path, inode, byte_offset, "
            "project_path, kit_task_id, abstained, ts_utc, tool_name, is_error) "
            "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            values(),
        )
        added = conn.total_changes - before
        conn.execute(
            "INSERT INTO source_files (machine_id, source, path, inode, byte_offset) "
            "VALUES (?, ?, ?, ?, ?) ON CONFLICT (machine_id, source, path) "
            "DO UPDATE SET inode = excluded.inode, byte_offset = excluded.byte_offset",
            (mid, SOURCE, path, inode, end[0]),
        )
    return added


def ingest(conn: sqlite3.Connection) -> dict:
    """Incrementally ingest the trace files of every existing project; return this run's counts."""
    known = store.file_offsets(conn, SOURCE)
    seen: set[str] = set()
    new_rows = 0
    for project in _projects(conn):
        trace_dir = Path(project) / "memory" / "event-trace"
        for path in sorted(trace_dir.glob("*.jsonl")) if trace_dir.is_dir() else []:
            key = str(path)
            try:
                info = path.stat()
                known_inode, offset = known.get(key, (info.st_ino, 0))
                if known_inode != info.st_ino or offset > info.st_size:
                    offset = 0  # replaced or shrunk file: start over
                task_id = None if path.stem == UNTAGGED else _label(path.stem)
                new_rows += _save(
                    conn, key, info.st_ino, offset, _rows(path, offset, project, task_id)
                )
            except FileNotFoundError:
                continue
            seen.add(key)
    store.retire_files(conn, SOURCE, set(known) - seen)
    return {"files": len(seen), "new_events": new_rows}
