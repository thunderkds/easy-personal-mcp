"""Register connectors in Claude Code user scope, only via `claude mcp add-json|remove` (DDR-0001).

The JSON handed to `claude` holds the launcher command and its args, never a secret. Secrets go to
the OS keychain. `~/.claude.json` is never opened. Every `claude` call is an argument list.
"""
import json
import os
import secrets
import shlex
import shutil
import sqlite3
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path

import keyring

from easy_personal.connectors import catalog, keychain
from easy_personal.core import workspaces
from easy_personal.launch import _NAME, ITEM_USER, LaunchError, require_backend, service_name

LAUNCHER = "easy-personal-launch"
_SHOWN = ("Scope:", "Type:", "Command:")  # lines of `claude mcp get` safe to show (no env, no args)

Prompt = Callable[[str, bool], str]  # (label, secret) -> typed value
Confirm = Callable[[], bool]
Out = Callable[[str], None]


class ConnectorError(ValueError):
    """Invalid connector input; the message is safe to print and never echoes the caller's input."""


def launcher_path() -> str:
    """Absolute path of the launcher, so Claude Code finds it whatever its PATH is."""
    beside = Path(sys.executable).parent / LAUNCHER
    found = str(beside) if beside.is_file() else shutil.which(LAUNCHER)
    if not found:
        raise LaunchError(
            f"cannot find {LAUNCHER}; reinstall easy-personal (uv tool install) so it is on disk"
        )
    return found


def add_json(name: str, launcher: str) -> str:
    return json.dumps({"type": "stdio", "command": launcher, "args": ["exec", name]})


def account_env(config_dir_env: str | None) -> dict[str, str]:
    """Environment for `claude` acting on one account: the default account (None) has
    CLAUDE_CONFIG_DIR removed (its config is ~/.claude.json); any other has it set to its folder."""
    env = {k: v for k, v in os.environ.items() if k != "CLAUDE_CONFIG_DIR"}
    if config_dir_env is not None:
        env["CLAUDE_CONFIG_DIR"] = config_dir_env
    return env


def _claude(*args: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
    exe = shutil.which("claude")
    try:
        return subprocess.run(
            [exe, *args], capture_output=True, text=True, timeout=60, check=False, env=env
        )
    except (OSError, subprocess.SubprocessError) as exc:
        return subprocess.CompletedProcess(args, 127, "", f"cannot run claude: {exc}")


def _entry_exists(name: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess | None:
    got = _claude("mcp", "get", name, env=env)
    return got if got.returncode == 0 else None


def entry_target(got: subprocess.CompletedProcess) -> tuple[str, str]:
    """(command, args) of a `claude mcp get` answer; for our own checks only, never printed."""
    found = {}
    for ln in got.stdout.splitlines():
        key, sep, value = ln.strip().partition(":")
        if sep and key in ("Command", "Args"):
            found.setdefault(key, value.strip())
    return found.get("Command", ""), found.get("Args", "")


def points_at_launcher(got: subprocess.CompletedProcess, connector_id: str) -> bool:
    command, args = entry_target(got)
    return Path(command).name == LAUNCHER and args == f"exec {connector_id}"


def add_entry(alias: str, connector_id: str, launcher: str, env: dict[str, str]) -> subprocess.CompletedProcess:
    """Register `alias` -> `easy-personal-launch exec <id>` in one account's user scope."""
    return _claude("mcp", "add-json", alias, add_json(connector_id, launcher), "--scope", "user", env=env)


def remove_entry(alias: str, env: dict[str, str]) -> subprocess.CompletedProcess:
    return _claude("mcp", "remove", alias, "--scope", "user", env=env)


def _safe_lines(text: str) -> list[str]:
    return [ln.rstrip() for ln in text.splitlines() if ln.strip().startswith(_SHOWN)]


def _keychain_item(name: str) -> bool:
    require_backend()
    return keyring.get_password(service_name(name), ITEM_USER) is not None


def new_id() -> str:
    return "c" + secrets.token_hex(4)


def add_to_workspace(
    conn: sqlite3.Connection, entry: catalog.Entry, workspace: str, alias: str | None,
    prompt: Prompt, confirm: Confirm, out: Out, assume_yes: bool = False,
) -> int:  # fmt: skip
    """Store a connector in a workspace: secrets in keychain `easy-personal/<new id>`, one row in the
    store. Writes nothing to Claude Code: `connector sync` does that per account (FR-029, FR-033)."""
    ws = workspaces.by_name(conn, workspace)  # WorkspaceError: unknown workspace
    if ws["is_orphan"]:
        raise ConnectorError("connectors are added to a workspace, never to orphan")
    custom = alias is not None
    alias = alias if custom else f"{ws['name']}-{entry.id}"
    if not _NAME.fullmatch(alias):
        raise ConnectorError(f"alias must match {_NAME.pattern}")
    require_backend()
    if conn.execute(
        "SELECT 1 FROM connectors WHERE workspace_id = ? AND provider = ?", (ws["id"], entry.id)
    ).fetchone():
        out(f"workspace '{ws['name']}' already has a {entry.id} connector; refusing to add another.")
        return 1
    if conn.execute(
        "SELECT 1 FROM connectors c JOIN workspaces w ON w.id = c.workspace_id "
        "WHERE c.alias = ? AND w.is_orphan = 0", (alias,),
    ).fetchone():
        out(f"alias '{alias}' is in use by another connector; choose another with --alias.")
        return 1
    connector_id = new_id()
    if _keychain_item(connector_id):  # today's refusal: never overwrite a keychain item
        out(f"keychain item easy-personal/{connector_id} already exists; refusing to overwrite it.")
        return 1
    env: dict[str, str] = {}
    for field in entry.fields:
        value = prompt(field.label, field.secret)
        if not value:
            out(f"{field.label} is empty; nothing was written.")
            return 2
        env[field.env] = value
    out(f"will change ({entry.provider}):")
    out(f"  keychain: service easy-personal/{connector_id}, fields "
        + ", ".join(f"{f.env}=***" for f in entry.fields))  # fmt: skip
    out(f"  store: connector {connector_id}, alias {alias}, workspace {ws['name']}")
    out("  Claude Code: no change now; `easy-personal connector sync` registers it")
    if not assume_yes and not confirm():
        out("aborted; nothing was written.")
        return 1
    keychain.store(connector_id, entry.command, env=env)
    try:
        with conn:
            conn.execute(
                "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES (?, ?, ?, ?, ?)",
                (connector_id, alias, int(custom), entry.id, ws["id"]),
            )
    except sqlite3.IntegrityError:
        keychain.remove(connector_id)  # roll back: no secret without its row
        out("the store refused the connector (changed meanwhile); keychain write rolled back.")
        return 1
    out(f"connector {connector_id} ({alias}) added to workspace '{ws['name']}'.")
    out("run `easy-personal connector sync` to register it in the accounts that picked this workspace.")
    return 0


def remove(name: str, confirm: Confirm, out: Out, assume_yes: bool = False) -> int:
    service_name(name)  # validates the name
    snippet = f"claude mcp remove {shlex.quote(name)} --scope user"
    if not shutil.which("claude"):
        out(f"the `claude` CLI is not on PATH; nothing was removed. Run `{snippet}` once it is installed.")
        return 0
    out(f"will change: delete keychain service easy-personal/{name}; run `{snippet}`")
    if not assume_yes and not confirm():
        out("aborted; nothing was removed.")
        return 1
    done = _claude("mcp", "remove", name, "--scope", "user")
    if done.returncode != 0:
        out(f"`claude mcp remove` failed (exit {done.returncode}); keychain left as it was.")
        if done.stderr.strip():
            out(done.stderr.strip())
        return 1
    keychain.remove(name)
    out(f"connector '{name}' removed.")
    return 0


def doctor(out: Out) -> int:
    """Check keychain backend, `claude` on PATH, the launcher, and each connector entry."""
    bad = 0

    def report(ok: bool, what: str, detail: str = "") -> None:
        nonlocal bad
        bad += not ok
        out(f"{'ok' if ok else 'FAIL'} {what}" + (f": {detail}" if detail else ""))

    try:
        require_backend()
        report(True, "keychain", type(keyring.get_keyring()).__name__)
        backend_ok = True
    except LaunchError as exc:
        report(False, "keychain", str(exc))
        backend_ok = False
    has_claude = shutil.which("claude") is not None
    report(has_claude, "claude", "on PATH" if has_claude else "not on PATH")
    try:
        report(True, "launcher", launcher_path())
    except LaunchError as exc:
        report(False, "launcher", str(exc))
    for name in catalog.ids():
        in_keychain = backend_ok and _keychain_item(name)
        got = _entry_exists(name) if has_claude else None
        if not in_keychain and got is None:
            continue  # not a configured connector
        if got is None:
            report(False, name, "keychain item present but no Claude Code entry; run connector add")
            continue
        command = next((ln.split(":", 1)[1].strip() for ln in _safe_lines(got.stdout)
                        if ln.strip().startswith("Command:")), "")  # fmt: skip
        points = Path(command).name == LAUNCHER
        report(points, name, "entry points at the launcher" if points
               else "entry does not point at easy-personal-launch")  # fmt: skip
        if not in_keychain:
            report(False, name, "Claude Code entry present but no keychain item")
    return 1 if bad else 0
