"""Register connectors in Claude Code user scope, only via `claude mcp add-json|remove` (DDR-0001).

The JSON handed to `claude` holds the launcher command and its args, never a secret. Secrets go to
the OS keychain. `~/.claude.json` is never opened. Every `claude` call is an argument list.
"""
import json
import shlex
import shutil
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path

import keyring

from easy_personal.connectors import catalog, keychain
from easy_personal.launch import ITEM_USER, LaunchError, require_backend, service_name

LAUNCHER = "easy-personal-launch"
_SHOWN = ("Scope:", "Type:", "Command:")  # lines of `claude mcp get` safe to show (no env, no args)

Prompt = Callable[[str, bool], str]  # (label, secret) -> typed value
Confirm = Callable[[], bool]
Out = Callable[[str], None]


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


def _claude(*args: str) -> subprocess.CompletedProcess:
    exe = shutil.which("claude")
    try:
        return subprocess.run([exe, *args], capture_output=True, text=True, timeout=60, check=False)
    except (OSError, subprocess.SubprocessError) as exc:
        return subprocess.CompletedProcess(args, 127, "", f"cannot run claude: {exc}")


def _entry_exists(name: str) -> subprocess.CompletedProcess | None:
    got = _claude("mcp", "get", name)
    return got if got.returncode == 0 else None


def _safe_lines(text: str) -> list[str]:
    return [ln.rstrip() for ln in text.splitlines() if ln.strip().startswith(_SHOWN)]


def _keychain_item(name: str) -> bool:
    require_backend()
    return keyring.get_password(service_name(name), ITEM_USER) is not None


def add(entry: catalog.Entry, prompt: Prompt, confirm: Confirm, out: Out, assume_yes: bool = False) -> int:
    name = entry.id
    launcher = launcher_path()
    json_text = add_json(name, launcher)
    if not shutil.which("claude"):
        out("the `claude` CLI is not on PATH; nothing was written. Run this once it is installed:")
        out(f"claude mcp add-json {shlex.quote(name)} {shlex.quote(json_text)} --scope user")
        return 0
    require_backend()
    existing = _entry_exists(name)
    if existing is not None or _keychain_item(name):
        out(f"connector '{name}' already exists; refusing to overwrite it.")
        out("existing:")
        if existing is not None:
            for line in _safe_lines(existing.stdout):
                out(f"  {line}")
        if _keychain_item(name):
            out("  keychain item present (values hidden)")
        out(f"new: claude mcp add-json {name} {json_text} --scope user")
        out(f"run `easy-personal connector remove {name}` first to replace it.")
        return 1
    env: dict[str, str] = {}
    for field in entry.fields:
        value = prompt(field.label, field.secret)
        if not value:
            out(f"{field.label} is empty; nothing was written.")
            return 2
        env[field.env] = value
    out(f"will change ({entry.provider}):")
    out(f"  keychain: service easy-personal/{name}, fields "
        + ", ".join(f"{f.env}=***" for f in entry.fields))  # fmt: skip
    out(f"  claude user scope: mcp add-json {name} {json_text}")
    if not assume_yes and not confirm():
        out("aborted; nothing was written.")
        return 1
    keychain.store(name, entry.command, env=env)
    done = _claude("mcp", "add-json", name, json_text, "--scope", "user")
    if done.returncode != 0:
        keychain.remove(name)  # roll back: no half-configured connector
        out(f"`claude mcp add-json` failed (exit {done.returncode}); keychain write rolled back.")
        if done.stderr.strip():
            out(done.stderr.strip())
        return 1
    out(f"connector '{name}' is configured. Restart Claude Code to use it.")
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
