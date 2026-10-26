"""Workspaces (FR-028): named groups of repo paths, plus the usage filter that resolves them (FR-035).

The schema (migration 4) enforces the rules; this module validates first so errors are readable,
and never echoes caller input in an error message. Email is stored and shown only.
"""

import os
import re
import sqlite3
from pathlib import Path

from easy_personal.core import discovery

ORPHAN = "orphan"
SLUG = re.compile(r"[a-z0-9][a-z0-9-]{0,39}")
COLOR = re.compile(r"#[0-9a-fA-F]{6}")
TEXT_FIELDS = {"color": 7, "icon": 32, "description": 500, "email": 254}
FIELDS = ("name", *TEXT_FIELDS, "repos")
_COLUMNS = "id, name, color, icon, description, email, is_orphan"


class WorkspaceError(ValueError):
    """A refused workspace operation; `status` is the HTTP status the API answers with."""

    def __init__(self, message: str, status: int = 400) -> None:
        super().__init__(message)
        self.status = status


def normalise_repo(path: str) -> str:
    """`~` expanded, made absolute and resolved, no trailing slash. The folder need not exist."""
    if not isinstance(path, str) or not path.strip() or "\x00" in path:
        raise WorkspaceError("repo path must be a non-empty string")
    expanded = os.path.expanduser(path.strip())
    if not os.path.isabs(expanded):
        raise WorkspaceError("repo path must be absolute or start with ~")
    return str(Path(expanded).resolve())


def _check_name(name) -> str:
    if not isinstance(name, str) or not SLUG.fullmatch(name):
        raise WorkspaceError("name must match [a-z0-9][a-z0-9-]{0,39}")
    if name == ORPHAN:
        raise WorkspaceError("the name 'orphan' is reserved")
    return name


def _check_text(field: str, value) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str) or len(value) > TEXT_FIELDS[field]:
        raise WorkspaceError(f"{field} must be text of at most {TEXT_FIELDS[field]} characters")
    if field == "color" and not COLOR.fullmatch(value):
        raise WorkspaceError("color must look like #rrggbb")
    return value


def _check_repos(repos) -> list[str]:
    if not isinstance(repos, (list, tuple)):
        raise WorkspaceError("repos must be a list of paths")
    return sorted({normalise_repo(r) for r in repos})


def _row(conn: sqlite3.Connection, where: str, value) -> dict:
    row = conn.execute(f"SELECT {_COLUMNS} FROM workspaces WHERE {where} = ?", (value,)).fetchone()
    if row is None:
        raise WorkspaceError("unknown workspace", 404)
    return _dict(conn, row)


def _dict(conn: sqlite3.Connection, row) -> dict:
    ws_id, name, color, icon, description, email, is_orphan = row
    repos = [
        r[0]
        for r in conn.execute(
            "SELECT repo_path FROM workspace_repos WHERE workspace_id = ? ORDER BY repo_path", (ws_id,)
        )
    ]
    return {"id": ws_id, "name": name, "color": color, "icon": icon, "description": description,
            "email": email, "is_orphan": bool(is_orphan), "repos": repos}  # fmt: skip


def get(conn: sqlite3.Connection, ws_id: int) -> dict:
    return _row(conn, "id", ws_id)


def by_name(conn: sqlite3.Connection, name: str) -> dict:
    return _row(conn, "name", name)


def list_all(conn: sqlite3.Connection) -> list[dict]:
    rows = conn.execute(f"SELECT {_COLUMNS} FROM workspaces ORDER BY is_orphan DESC, name").fetchall()
    return [_dict(conn, r) for r in rows]


def _insert_repos(conn: sqlite3.Connection, ws_id: int, repos: list[str]) -> None:
    conn.executemany("INSERT INTO workspace_repos VALUES (?, ?)", ((ws_id, r) for r in repos))


def add(conn: sqlite3.Connection, name, *, repos=(), **text) -> dict:
    """Create a workspace; `text` may hold color, icon, description, email. Nothing is stored on error."""
    unknown = set(text) - set(TEXT_FIELDS)
    if unknown:
        raise WorkspaceError("unknown field")
    values = {f: _check_text(f, text.get(f)) for f in TEXT_FIELDS}
    clean_repos, name = _check_repos(repos), _check_name(name)
    try:
        with conn:
            cur = conn.execute(
                "INSERT INTO workspaces (name, color, icon, description, email) VALUES (?,?,?,?,?)",
                (name, *values.values()),
            )
            _insert_repos(conn, cur.lastrowid, clean_repos)
    except sqlite3.IntegrityError as exc:
        raise WorkspaceError("a workspace with this name exists", 409) from exc
    return get(conn, cur.lastrowid)


def edit(conn: sqlite3.Connection, ws_id: int, changes: dict) -> dict:
    """Apply `changes` (any of name, color, icon, description, email, repos) in one transaction.

    `repos` replaces the whole list. A rename changes the name only; the id is fixed.
    """
    with conn:
        return edit_in_txn(conn, ws_id, changes)


def edit_in_txn(conn: sqlite3.Connection, ws_id: int, changes: dict) -> dict:
    """`edit` without its own commit: the caller owns the transaction (rename + alias plan, HUN-66)."""
    current = get(conn, ws_id)
    if current["is_orphan"]:
        raise WorkspaceError("the orphan workspace cannot be changed", 403)
    if not isinstance(changes, dict) or set(changes) - set(FIELDS):
        raise WorkspaceError("unknown field")
    columns = {f: _check_text(f, v) for f, v in changes.items() if f in TEXT_FIELDS}
    if "name" in changes:
        columns["name"] = _check_name(changes["name"])
    repos = _check_repos(changes["repos"]) if "repos" in changes else None
    try:
        if columns:
            sets = ", ".join(f"{c} = ?" for c in columns)
            conn.execute(f"UPDATE workspaces SET {sets} WHERE id = ?", (*columns.values(), ws_id))
        if repos is not None:
            conn.execute("DELETE FROM workspace_repos WHERE workspace_id = ?", (ws_id,))
            _insert_repos(conn, ws_id, repos)
    except sqlite3.IntegrityError as exc:
        raise WorkspaceError("a workspace with this name exists", 409) from exc
    return get(conn, ws_id)


def delete(conn: sqlite3.Connection, ws_id: int) -> None:
    """Remove a workspace and its repo rows; refused (409) while it still holds connectors."""
    if get(conn, ws_id)["is_orphan"]:
        raise WorkspaceError("the orphan workspace cannot be deleted", 403)
    try:
        with conn:
            conn.execute("DELETE FROM workspaces WHERE id = ?", (ws_id,))
    except sqlite3.IntegrityError as exc:  # connectors.workspace_id references it (migration 6)
        raise WorkspaceError("the workspace still has connectors; remove them first", 409) from exc


def delete_to_orphan(conn: sqlite3.Connection, ws_id: int) -> list[str]:
    """Move the workspace's connectors to orphan (id, alias kept), drop its picks and repo rows, delete it
    (FR-030). No commit: the caller owns the transaction. Returns the moved connector ids."""
    if get(conn, ws_id)["is_orphan"]:
        raise WorkspaceError("the orphan workspace cannot be deleted", 403)
    orphan = conn.execute("SELECT id FROM workspaces WHERE is_orphan = 1").fetchone()[0]
    moved = [r[0] for r in conn.execute("SELECT id FROM connectors WHERE workspace_id = ? ORDER BY id", (ws_id,))]
    conn.execute("UPDATE connectors SET workspace_id = ? WHERE workspace_id = ?", (orphan, ws_id))
    conn.execute("DELETE FROM account_workspaces WHERE workspace_id = ?", (ws_id,))
    conn.execute("DELETE FROM workspaces WHERE id = ?", (ws_id,))  # workspace_repos cascade (migration 4)
    return moved


def _under(path: str, root: str) -> bool:
    return path == root or root == "/" or path.startswith(root + "/")


def project_paths(
    conn: sqlite3.Connection, name: str, since_utc: str | None = None, until_utc: str | None = None
) -> list[str]:
    """Stored `project_path` values equal to or under a repo of workspace `name` (FR-035).

    A repo path is also resolved like `discovery.resolve_project`, so a subfolder or worktree still
    reaches the stored project. Only DISTINCT stored paths (inside the period bounds, for speed) are
    compared, path-aware: `/a/b` never matches `/a/bc`. Raises WorkspaceError for an unknown name.
    """
    roots = set()
    for repo in by_name(conn, name)["repos"]:
        roots.update((repo, discovery.resolve_project(repo)))
    if not roots:
        return []
    clauses, params = ["project_path IS NOT NULL"], []
    if since_utc is not None:
        clauses.append("ts_utc >= ?")
        params.append(since_utc)
    if until_utc is not None:
        clauses.append("ts_utc < ?")
        params.append(until_utc)
    stored = (
        r[0] for r in conn.execute(
            f"SELECT DISTINCT project_path FROM events WHERE {' AND '.join(clauses)}", params
        )
    )  # fmt: skip
    return sorted(p for p in stored if any(_under(p, root) for root in roots))
