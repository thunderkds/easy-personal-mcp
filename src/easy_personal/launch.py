"""Connector launcher: read secrets from the OS keychain and hand them to the provider server.

Imports only stdlib + keyring (NFR-006). Secrets reach the child only through its environment,
never argv, and no message here ever contains a secret value.
"""
import json
import os
import re
import sys

import keyring
from keyring.errors import KeyringError

SERVICE_PREFIX = "easy-personal/"
ITEM_USER = "connector"
_NAME = re.compile(r"[a-z0-9][a-z0-9_-]{0,63}")

NO_BACKEND = (
    "no usable OS keychain (Secret Service); unlock or install one, e.g. gnome-keyring. "
    "Secrets are never stored in plaintext"
)


class LaunchError(Exception):
    """Expected failure; the message is safe to print."""


def service_name(connector: str) -> str:
    if not _NAME.fullmatch(connector):
        raise LaunchError("invalid connector name")
    return SERVICE_PREFIX + connector


def require_backend() -> None:
    try:
        backend = keyring.get_keyring()
        usable = backend.priority > 0
    except Exception:  # noqa: BLE001 - any backend probe failure means refuse
        usable = False
    if not usable:
        raise LaunchError(NO_BACKEND)


def load(connector: str) -> dict:
    """Return {"command": [...], "env": {...}, "headers": {...}} for a configured connector."""
    service = service_name(connector)
    require_backend()
    try:
        raw = keyring.get_password(service, ITEM_USER)
    except KeyringError:
        raise LaunchError(NO_BACKEND) from None
    if raw is None:
        raise LaunchError(f"connector '{connector}' is not configured")
    try:
        item = json.loads(raw)
        command = item.get("command", [])
        env = item.get("env", {})
        headers = item.get("headers", {})
        ok = (
            isinstance(command, list) and all(isinstance(c, str) for c in command)
            and all(isinstance(v, str) for v in (*env.keys(), *env.values()))
            and all(isinstance(v, str) for v in (*headers.keys(), *headers.values()))
        )
    except (ValueError, AttributeError):
        ok = False
    if not ok:
        raise LaunchError(f"connector '{connector}' has an unreadable keychain entry")
    return {"command": command, "env": env, "headers": headers}


def run_exec(connector: str) -> int:
    item = load(connector)
    if not item["command"]:
        raise LaunchError(f"connector '{connector}' has no provider command")
    env = {**os.environ, **item["env"]}
    try:
        os.execvpe(item["command"][0], item["command"], env)
    except (OSError, ValueError):
        raise LaunchError(f"cannot start provider command for '{connector}'") from None
    return 0  # not reached: exec replaces this process


def run_headers(connector: str) -> int:
    sys.stdout.write(json.dumps(load(connector)["headers"]) + "\n")
    return 0


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if len(args) != 2 or args[0] not in ("exec", "headers"):
        print("usage: easy-personal-launch exec|headers <connector>", file=sys.stderr)
        return 2
    try:
        return run_exec(args[1]) if args[0] == "exec" else run_headers(args[1])
    except LaunchError as exc:
        print(f"easy-personal: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
