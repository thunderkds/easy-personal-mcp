"""kanban reader (FR-037): board tasks from each kit project's PROJECT_KANBAN.md, read live.

Per project the answer holds only the folder name, board tasks (id, title, board status), the board
kind and the last board pull (file mtime), both only when `.claude/board.json` names a board. Nothing
is stored and nothing is written. A missing, oversized, non-UTF-8 or board-less file is
`{"status": "no data"}`; rows in an unknown `###` subsection (or without an id) are counted as
`abstained`, never coerced into a status. Parsing is line by line, no markdown library.
"""

import itertools
import json
import re
import sqlite3
from datetime import UTC, datetime
from pathlib import Path

from easy_personal.core import kit

KANBAN = "PROJECT_KANBAN.md"
BOARD_JSON = Path(".claude") / "board.json"
MAX_BYTES = 5_000_000
MAX_TITLE = 300

# `###` subsection name (lower case) -> board status. `## Blocked` is a table section of its own.
STATUSES = {
    s.casefold(): s for s in ("Todo", "In Progress", "Ready for Review", "Done", "Closed (will not do)")
}
BLOCKED = "Blocked"
UNKNOWN = "?"  # a `###` subsection that is not a board status

_ID = r"[A-Za-z]+-?\d+"
_ID_ONLY = re.compile(rf"{_ID}$")
_BULLET = re.compile(rf"\s*[-*]\s+(?:\[[ xX]\]\s+)?(?:\*\*(?P<bold>[^*]+)\*\*|(?P<plain>{_ID})(?=\s|$))\s*[—–:-]*\s*(?P<rest>.*)$")
_SEPARATOR = re.compile(r"\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$")
_LINK = re.compile(r"\[([^\]]*)\]\([^)]*\)")
_ESCAPE = re.compile(r"\\([\\`*_{}\[\]()#+\-.!|<>~])")
_UNESCAPED_BAR = re.compile(r"(?<!\\)\|")
_UNESCAPED_BAR_SPACED = re.compile(r"\s(?<!\\)\|\s")


def _plain(text: str) -> str:
    """A title as plain text: links reduced to their label, backticks dropped, backslash escapes removed."""
    text = _ESCAPE.sub(r"\1", _LINK.sub(r"\1", text))
    return text.replace("`", "").strip()[:MAX_TITLE]


def _clean_id(cell: str) -> str | None:
    cell = cell.strip().strip("*`").strip()
    return cell if _ID_ONLY.match(cell) else None


def _bullet(line: str) -> tuple[str, str] | None:
    m = _BULLET.match(line)
    if not m:
        return None
    task_id = _clean_id(m["bold"] or m["plain"])
    if task_id is None:
        return None
    return task_id, _plain(_UNESCAPED_BAR_SPACED.split(m["rest"], 1)[0])


def _table_cells(line: str) -> list[str] | None:
    if not line.lstrip().startswith("|"):
        return None
    return [c.strip() for c in _UNESCAPED_BAR.split(line.strip().strip("|"))]


def _table_row(cells: list[str]) -> tuple[str, str] | None:
    """(id, title) from the first cell that is a task id (one of the first two), title in the next cell."""
    for i in (0, 1):
        if i < len(cells) and (task_id := _clean_id(cells[i])):
            return task_id, _plain(cells[i + 1]) if i + 1 < len(cells) else ""
    return None


def _board_kind(root: Path) -> str | None:
    try:
        data = json.loads((root / BOARD_JSON).read_text(encoding="utf-8"))
    except (OSError, ValueError):  # missing, unreadable, bad JSON or bad UTF-8 (UnicodeDecodeError is a ValueError)
        return None
    kind = data.get("board") if isinstance(data, dict) else None
    return kind.strip()[:40] if isinstance(kind, str) and kind.strip() else None


def _parse(text: str) -> tuple[list[dict], int] | None:
    """(tasks, abstained), or None when the file has no `## Board` section."""
    lines = text.splitlines()
    tasks, abstained = [], 0
    section: str | None = None  # None outside the board; "board" or "blocked" inside
    status: str | None = None  # current `###` subsection: a status, UNKNOWN, or None before the first one
    found = False
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("## "):
            name = stripped[3:].strip().casefold()
            section = name if name in ("board", "blocked") else None
            found = found or name == "board"
            status = None
            continue
        if section is None:
            continue
        if section == "board" and stripped.startswith("### "):
            status = STATUSES.get(stripped[4:].strip().casefold(), UNKNOWN)
            continue
        current = BLOCKED if section == "blocked" else status
        if current is None:
            continue  # prose between `## Board` and the first subsection
        cells = _table_cells(line)
        if cells is not None:
            if _SEPARATOR.match(stripped) or (i + 1 < len(lines) and _SEPARATOR.match(lines[i + 1].strip())):
                continue  # the separator line, or the header row above it
            if cells[0].startswith("~~"):
                continue  # a struck-through (closed) row
            row = _table_row(cells)
            is_row = any(cells)
        else:
            row = _bullet(line)
            is_row = bool(re.match(r"\s*[-*]\s", line))
        if not is_row:
            continue
        if row is None or current == UNKNOWN:
            abstained += 1
        else:
            tasks.append({"id": row[0], "title": row[1], "status": current})
    return (tasks, abstained) if found else None


def read_project(root: Path) -> dict:
    """The board entry of one project folder; any problem with the file is `no data`."""
    entry = {"project": root.name or str(root), "status": "no data", "board": None, "last_pull": None, "abstained": 0, "tasks": []}
    path = root / KANBAN
    try:
        stat = path.stat()
        if not path.is_file() or stat.st_size > MAX_BYTES:
            return entry
        parsed = _parse(path.read_bytes().decode("utf-8"))
    except (OSError, ValueError):  # unreadable file, or not UTF-8 (UnicodeDecodeError is a ValueError)
        return entry
    if parsed is None:
        return entry
    entry["status"] = "ok"
    entry["tasks"], entry["abstained"] = parsed
    entry["board"] = _board_kind(root)
    if entry["board"]:
        entry["last_pull"] = datetime.fromtimestamp(stat.st_mtime, UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    return entry


def _is_worktree(path: str) -> bool:
    parts = Path(path).parts
    return any(a == ".claude" and b == "worktrees" for a, b in itertools.pairwise(parts))


def board(conn: sqlite3.Connection, workspace: str | None = None, account: str | None = None) -> dict:
    """Every project seen in usage whose folder exists (same scope as the kit), with its board tasks.

    A gone project (folder missing, FR-054) is left out and counted in the top-level `gone`.
    An unknown workspace or account raises ValueError, as for `kit.kit_summary`.
    """
    stored = [p for p in kit.scoped_projects(conn, workspace, account) if not _is_worktree(p)]
    paths, gone = kit.split_gone(stored)
    return {"projects": [read_project(Path(p)) for p in paths], "gone": gone}
