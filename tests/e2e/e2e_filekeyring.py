"""File-backed keyring for the HUN-71 end-to-end suite.

Both the CLI subprocesses and the in-process API share one keychain through the JSON file named by
`E2E_KEYRING_FILE`. It is a test double: the owner's real keychain is never used.
"""

import json
import os
from pathlib import Path

from keyring.backend import KeyringBackend
from keyring.errors import PasswordDeleteError


class Keyring(KeyringBackend):
    priority = 5

    @staticmethod
    def _path() -> Path:
        return Path(os.environ["E2E_KEYRING_FILE"])

    def _load(self) -> dict:
        path = self._path()
        return json.loads(path.read_text()) if path.exists() else {}

    @staticmethod
    def _key(service, username) -> str:
        return f"{service}\x00{username}"

    def get_password(self, service, username):
        return self._load().get(self._key(service, username))

    def set_password(self, service, username, password):
        items = self._load()
        items[self._key(service, username)] = password
        self._path().write_text(json.dumps(items))

    def delete_password(self, service, username):
        items = self._load()
        if self._key(service, username) not in items:
            raise PasswordDeleteError("absent")
        del items[self._key(service, username)]
        self._path().write_text(json.dumps(items))
