import json

import pytest

from easy_personal.connectors import keychain
from easy_personal.launch import LaunchError

SECRET = "xoxb-PLANTED-secret-é\nline2"


def test_store_writes_one_item_under_service_prefix(mem_keyring):
    keychain.store("slack", ["npx", "slack-mcp"], env={"SLACK_TOKEN": SECRET})
    (service, user), payload = next(iter(mem_keyring.items.items()))
    assert service == "easy-personal/slack" and user == "connector"
    assert json.loads(payload)["env"]["SLACK_TOKEN"] == SECRET


def test_store_refuses_without_backend(no_keyring, tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    with pytest.raises(LaunchError) as exc:
        keychain.store("slack", ["x"], env={"T": SECRET})
    assert "plaintext" in str(exc.value) and SECRET not in str(exc.value)
    assert list(tmp_path.iterdir()) == []  # nothing written to disk


def test_invalid_connector_name_rejected(mem_keyring):
    with pytest.raises(LaunchError):
        keychain.store("../evil", ["x"])


def test_remove_is_idempotent(mem_keyring):
    keychain.store("slack", ["x"])
    keychain.remove("slack")
    keychain.remove("slack")
    assert mem_keyring.items == {}
