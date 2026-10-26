"""GET/POST /api/connectors/legacy: same core as `connector adopt-legacy` (HUN-67)."""

import json

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config, store
from easy_personal.surfaces import api

TOKEN, PORT = "tok-123", 5555
AUTH = {api.TOKEN_HEADER: TOKEN}
JSON = {"content-type": "application/json"}
LAUNCH = {"command": "/opt/fake/bin/easy-personal-launch", "args": ["exec", "slack"]}


@pytest.fixture
def client(tmp_path, monkeypatch, claude_state, launcher, mem_keyring):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    (tmp_path / "home").mkdir()
    monkeypatch.delenv("CLAUDE_CONFIG_DIR", raising=False)
    mem_keyring.items[("easy-personal/slack", "connector")] = "{}"
    (tmp_path / "claude-state.json").write_text(json.dumps({"<unset>": {"slack": LAUNCH}}))
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://127.0.0.1:{PORT}")


def test_get_lists_legacy_then_post_preview_then_confirm(client, claude_state):
    got = client.get("/api/connectors/legacy", headers=AUTH).json()
    assert [x["id"] for x in got["legacy"]] == ["slack"]
    assert got["legacy"][0]["keychain"] and got["legacy"][0]["claude"]
    preview = client.post("/api/connectors/legacy", content='{"confirm": false}', headers={**AUTH, **JSON}).json()
    assert preview["pending"] is True and claude_state.write_records() == []
    done = client.post("/api/connectors/legacy", content='{"confirm": true}', headers={**AUTH, **JSON}).json()
    assert done["status"] == 0 and claude_state.state()["<unset>"] == {}
    assert client.get("/api/connectors/legacy", headers=AUTH).json() == {"legacy": []}
    with store.opened(config.store_path()) as conn:
        assert conn.execute("SELECT id, alias FROM connectors").fetchall() == [("slack", "slack")]


def test_post_needs_the_token_and_json(client):
    assert client.post("/api/connectors/legacy", content="{}", headers=JSON).status_code in (401, 403)
    assert client.post("/api/connectors/legacy", content="{}", headers=AUTH).status_code == 415


def test_get_without_a_keychain_backend_is_an_error_not_a_crash(client, no_keyring):
    res = client.get("/api/connectors/legacy", headers=AUTH)
    assert res.status_code == 503 and "keychain" in res.json()["error"]
