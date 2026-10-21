"""In-memory keyring backend so tests never touch the owner's real keychain."""
from keyring.backend import KeyringBackend
from keyring.errors import PasswordDeleteError


class MemoryKeyring(KeyringBackend):
    priority = 1

    def __init__(self):
        self.items = {}

    def get_password(self, service, username):
        return self.items.get((service, username))

    def set_password(self, service, username, password):
        self.items[(service, username)] = password

    def delete_password(self, service, username):
        if (service, username) not in self.items:
            raise PasswordDeleteError("absent")
        del self.items[(service, username)]
