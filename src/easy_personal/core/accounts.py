"""Agent accounts: `~/.claude*` and `~/.codex*` folders, detected, then confirmed and named by the owner.

Reads folder names and the existence of marker entries only; never reads or writes inside an account.
Core code: it must not import connectors.
"""

import sqlite3
from pathlib import Path

from easy_personal.core import config

DEFAULT_LABEL = "Claude default"
MAX_LABEL = 60


class AccountError(ValueError):
    """The request is invalid. The message never echoes the caller's input."""


class NotFound(AccountError):
    pass


def _provider_of(folder: Path) -> str | None:
    """What the folder's content says it is: Claude (`projects/` or `settings.json`), Codex (`config.toml`)."""
    if (folder / "projects").is_dir() or (folder / "settings.json").is_file():
        return "claude"
    if (folder / "config.toml").is_file():
        return "codex"
    return None


def _insert(conn: sqlite3.Connection, provider: str, folder: Path, default: bool = False) -> None:
    label, status = (DEFAULT_LABEL, "confirmed") if default else (None, "detected")
    env = str(folder) if provider == "claude" and not default else None  # default: CLAUDE_CONFIG_DIR unset
    conn.execute(
        "INSERT OR IGNORE INTO agent_accounts (provider, folder, label, status, config_dir_env) "
        "VALUES (?, ?, ?, ?, ?)",
        (provider, str(folder), label, status, env),
    )


def detect(conn: sqlite3.Connection) -> None:
    """Insert accounts not yet known. Rows already there (any status) are never changed."""
    root = config.agent_root()
    try:
        entries = sorted(e for e in root.iterdir() if e.name.startswith((".claude", ".codex")))
    except OSError:
        return
    with conn:
        for entry in entries:
            if not entry.is_dir():  # ~/.claude.json and its backup are files
                continue
            folder = entry.resolve()
            if entry.name == ".claude":
                _insert(conn, "claude", folder, default=True)
            elif entry.name.startswith(".claude") and _provider_of(folder) == "claude":
                _insert(conn, "claude", folder)
            elif entry.name.startswith(".codex") and _provider_of(folder) == "codex":
                _insert(conn, "codex", folder)


def _row(row: tuple) -> dict:
    id_, provider, folder, label, status, env = row
    if status != "ignored" and not Path(folder).is_dir():
        status = "missing"
    return {
        "id": id_, "provider": provider, "label": label, "folder": folder,
        "status": status, "config_dir_env": env,
        "is_default": folder == str((config.agent_root() / ".claude").resolve()),
    }  # fmt: skip


_COLUMNS = "id, provider, folder, label, status, config_dir_env"


def _with_picks(conn: sqlite3.Connection, row: dict) -> dict:
    row["workspaces"] = [
        r[0] for r in conn.execute(
            "SELECT w.name FROM account_workspaces p JOIN workspaces w ON w.id = p.workspace_id "
            "WHERE p.account_id = ? ORDER BY w.name", (row["id"],),
        )
    ]  # fmt: skip
    return row


def list_accounts(conn: sqlite3.Connection) -> list[dict]:
    rows = conn.execute(f"SELECT {_COLUMNS} FROM agent_accounts ORDER BY id").fetchall()
    return [_with_picks(conn, _row(r)) for r in rows]


def group_by_provider(rows: list[dict]) -> dict[str, list[dict]]:
    grouped: dict[str, list[dict]] = {}
    for row in rows:
        grouped.setdefault(row["provider"], []).append(row)
    return grouped


def grouped(conn: sqlite3.Connection, by: str) -> dict:
    """Detect, then list: `by` is `account` (flat list) or `provider`."""
    if by not in ("account", "provider"):
        raise AccountError("by must be account or provider")
    detect(conn)
    rows = list_accounts(conn)
    if by == "provider":
        return {"by": by, "providers": group_by_provider(rows)}
    return {"by": by, "accounts": rows}


def _find(conn: sqlite3.Connection, ref: str) -> int:
    """Account id for an id or a folder path."""
    ref = ref.strip()
    if ref.isdigit():
        row = conn.execute("SELECT id FROM agent_accounts WHERE id = ?", (int(ref),)).fetchone()
    else:
        folder = str(Path(ref).expanduser().resolve())
        row = conn.execute("SELECT id FROM agent_accounts WHERE folder = ?", (folder,)).fetchone()
    if row is None:
        raise NotFound("no such account")
    return row[0]


def _get(conn: sqlite3.Connection, id_: int) -> dict:
    row = conn.execute(f"SELECT {_COLUMNS} FROM agent_accounts WHERE id = ?", (id_,)).fetchone()
    return _with_picks(conn, _row(row))


def confirm(conn: sqlite3.Connection, ref: str, label: str) -> dict:
    """Name an account and mark it confirmed; confirming again with a new label renames it."""
    label = label.strip() if isinstance(label, str) else ""
    if not 1 <= len(label) <= MAX_LABEL:
        raise AccountError(f"label must be 1 to {MAX_LABEL} characters")
    id_ = _find(conn, ref)
    with conn:
        conn.execute("UPDATE agent_accounts SET status = 'confirmed', label = ? WHERE id = ?", (label, id_))
    return _get(conn, id_)


def ignore(conn: sqlite3.Connection, ref: str) -> dict:
    id_ = _find(conn, ref)
    folder = conn.execute("SELECT folder FROM agent_accounts WHERE id = ?", (id_,)).fetchone()[0]
    if folder == str((config.agent_root() / ".claude").resolve()):  # the fallback ingest account
        raise AccountError("the default account cannot be ignored")
    with conn:
        conn.execute("UPDATE agent_accounts SET status = 'ignored' WHERE id = ?", (id_,))
    return _get(conn, id_)


def add(conn: sqlite3.Connection, path: str) -> dict:
    """Add a folder by hand (any place); it must pass the same content check. Starts `detected`."""
    folder = Path(path).expanduser().resolve() if isinstance(path, str) and path.strip() else None
    if folder is None or not folder.is_dir():
        raise AccountError("not an agent folder: it must be an existing directory")
    provider = _provider_of(folder)
    if provider is None:
        raise AccountError("not an agent folder: no projects/, settings.json or config.toml")
    with conn:
        _insert(conn, provider, folder, default=folder == (config.agent_root() / ".claude").resolve())
    return _get(conn, _find(conn, str(folder)))


def default_id(conn: sqlite3.Connection) -> int | None:
    """Id of the default account (`CLAUDE_CONFIG_DIR` unset), or None before it was detected."""
    row = conn.execute(
        "SELECT id FROM agent_accounts WHERE provider = 'claude' AND config_dir_env IS NULL ORDER BY id"
    ).fetchone()
    return row[0] if row else None


def resolve(conn: sqlite3.Connection, ref: str) -> tuple[str, list]:
    """SQL clause and params selecting events of one confirmed account, by id or label (case-insensitive).

    Events with no account (stored before migration 5) belong to the default account. An unknown
    reference raises `AccountError` listing the valid labels; the message never echoes `ref`.
    """
    ref = ref.strip()
    rows = conn.execute("SELECT id, label FROM agent_accounts WHERE status = 'confirmed' ORDER BY id").fetchall()
    found = [i for i, label in rows if str(i) == ref or label.casefold() == ref.casefold()]
    if not found:
        raise AccountError("unknown account; valid labels: " + (", ".join(label for _, label in rows) or "(none)"))
    if found[0] == default_id(conn):
        return "(agent_account_id = ? OR agent_account_id IS NULL)", [found[0]]
    return "agent_account_id = ?", [found[0]]


def use(conn: sqlite3.Connection, ref: str, names: list[str]) -> dict:
    """Set the workspaces a confirmed Claude account picks (replaces the list; empty clears it).

    Changes only the store: `connector sync` applies the result to the account's Claude config.
    Migration 6 triggers refuse the same cases (orphan, Codex, unconfirmed).
    """
    id_ = _find(conn, ref)
    provider, status = conn.execute(
        "SELECT provider, status FROM agent_accounts WHERE id = ?", (id_,)
    ).fetchone()
    if (provider, status) != ("claude", "confirmed"):
        raise AccountError("only confirmed Claude accounts pick workspaces")
    if not isinstance(names, list) or not all(isinstance(n, str) for n in names):
        raise AccountError("workspaces must be a list of workspace names")
    ids = set()
    for name in names:
        row = conn.execute("SELECT id, is_orphan FROM workspaces WHERE name = ?", (name,)).fetchone()
        if row is None:
            raise AccountError("unknown workspace")
        if row[1]:
            raise AccountError("the orphan workspace cannot be picked")
        ids.add(row[0])
    with conn:
        conn.execute("DELETE FROM account_workspaces WHERE account_id = ?", (id_,))
        conn.executemany("INSERT INTO account_workspaces VALUES (?, ?)", ((id_, w) for w in sorted(ids)))
    return _get(conn, id_)
