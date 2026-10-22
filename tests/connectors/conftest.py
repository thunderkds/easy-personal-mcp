import keyring
import pytest
from fake_backend import MemoryKeyring
from keyring.backends import fail


@pytest.fixture
def mem_keyring():
    old = keyring.get_keyring()
    backend = MemoryKeyring()
    keyring.set_keyring(backend)
    yield backend
    keyring.set_keyring(old)


@pytest.fixture
def no_keyring():
    old = keyring.get_keyring()
    keyring.set_keyring(fail.Keyring())
    yield
    keyring.set_keyring(old)
