"""Store connector secrets in the OS keychain only; refuse when no backend exists (FR-017)."""
import json

import keyring
from keyring.errors import KeyringError

from easy_personal.launch import ITEM_USER, NO_BACKEND, LaunchError, require_backend, service_name


def store(connector: str, command: list[str], env: dict[str, str] | None = None,
          headers: dict[str, str] | None = None) -> None:
    """Write one keychain item holding the provider command and its secrets."""
    service = service_name(connector)
    require_backend()
    payload = json.dumps({"command": command, "env": env or {}, "headers": headers or {}})
    try:
        keyring.set_password(service, ITEM_USER, payload)
    except KeyringError:
        raise LaunchError(NO_BACKEND) from None


def remove(connector: str) -> None:
    service = service_name(connector)
    require_backend()
    try:
        keyring.delete_password(service, ITEM_USER)
    except keyring.errors.PasswordDeleteError:
        pass  # already absent
    except KeyringError:
        raise LaunchError(NO_BACKEND) from None
