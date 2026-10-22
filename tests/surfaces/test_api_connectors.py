"""Dashboard connector routes: guarded, one core with `connector add/remove`, no secret echoed.

The keychain is an in-memory fake and `claude` a fake script on PATH (tests/connectors/conftest.py).
"""
import json

import pytest
from starlette.testclient import TestClient

from easy_personal.surfaces import api

TOKEN, PORT = "tok-123", 5555
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
JSON = {"content-type": "application/json"}
SECRET = "xoxb-PLANTED-secret-123"
SLACK = {"provider": "slack", "values": {"SLACK_MCP_XOXB_TOKEN": SECRET}}


@pytest.fixture
def client(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://{HOST}")


def post(client, body, headers=None):
    return client.post("/api/connectors", content=json.dumps(body), headers=headers or {**AUTH, **JSON})


def test_get_lists_catalog_in_cli_order_without_secrets(client, mem_keyring, fake_claude):
    res = client.get("/api/connectors", headers=AUTH)
    assert res.status_code == 200
    items = res.json()["connectors"]
    assert {i["id"] for i in items} >= {"slack", "gmail", "sendgrid", "teams", "discord"}
    keys = [(i["category"], not i["claude_ai_connector"], i["provider"]) for i in items]
    assert keys == sorted(keys)
    slack = next(i for i in items if i["id"] == "slack")
    assert slack["claude_ai_link"].startswith("https://")
    assert slack["fields"] == [{"env": "SLACK_MCP_XOXB_TOKEN", "label": "Bot token (xoxb-...)", "secret": True}]
    assert not any(i["configured"] for i in items)


def test_configured_needs_keychain_item_and_claude_entry(client, mem_keyring, fake_claude, monkeypatch):
    mem_keyring.items[("easy-personal/slack", "connector")] = '{"env": {"K": "' + SECRET + '"}}'
    got = {i["id"]: i["configured"] for i in client.get("/api/connectors", headers=AUTH).json()["connectors"]}
    assert got["slack"] is False  # keychain alone is not configured
    monkeypatch.setenv("FAKE_CLAUDE_GET_RC", "0")
    monkeypatch.setenv("FAKE_CLAUDE_GET_ONLY", "slack")
    res = client.get("/api/connectors", headers=AUTH)
    got = {i["id"]: i["configured"] for i in res.json()["connectors"]}
    assert got["slack"] is True and got["gmail"] is False
    assert SECRET not in res.text


def test_get_without_keychain_backend_is_not_configured(client, no_keyring, fake_claude):
    res = client.get("/api/connectors", headers=AUTH)
    assert res.status_code == 200
    assert not any(i["configured"] for i in res.json()["connectors"])


@pytest.mark.parametrize("method", ["get", "post", "delete"])
def test_guard_applies_to_connector_routes(client, method):
    path = "/api/connectors" if method != "delete" else "/api/connectors/slack"
    send = lambda path, **kw: client.request(method.upper(), path, **kw)
    kw = {"content": "{}"} if method != "get" else {}
    assert send(path, **kw, headers=JSON).status_code == 403  # no token
    assert send(path, **kw, headers={**AUTH, **JSON, "Origin": "http://evil.example"}).status_code == 403
    assert send(path, **kw, headers={**AUTH, **JSON, "Host": "evil.example"}).status_code == 403


def test_cross_origin_write_is_403_and_writes_nothing(client, mem_keyring, fake_claude, launcher):
    body = {**SLACK, "confirm": True}
    res = post(client, body, {**AUTH, **JSON, "Origin": "http://evil.example"})
    assert res.status_code == 403
    assert mem_keyring.items == {} and fake_claude.calls() == []


@pytest.mark.parametrize("ctype", ["text/plain", "application/x-www-form-urlencoded", None])
def test_write_needs_json_content_type(client, mem_keyring, fake_claude, launcher, ctype):
    headers = dict(AUTH) | ({"content-type": ctype} if ctype else {})
    res = client.post("/api/connectors", content=json.dumps({**SLACK, "confirm": True}), headers=headers)
    assert res.status_code == 415
    assert client.request("DELETE", "/api/connectors/slack", content="{}", headers=headers).status_code == 415
    assert mem_keyring.items == {} and fake_claude.writes() == []


def test_bad_bodies_are_400(client, mem_keyring, fake_claude, launcher):
    assert client.post("/api/connectors", content="not json", headers={**AUTH, **JSON}).status_code == 400
    assert post(client, ["x"]).status_code == 400
    assert post(client, {"provider": "slack", "values": "x", "confirm": False}).status_code == 400
    assert post(client, {"provider": "slack", "values": {"A": 1}, "confirm": False}).status_code == 400
    assert post(client, {"provider": "slack", "values": {}, "confirm": "yes"}).status_code == 400
    res = post(client, {"provider": "nope", "values": {}, "confirm": False})
    assert res.status_code == 400 and "unknown provider" in res.json()["error"]
    assert mem_keyring.items == {} and fake_claude.writes() == []


def test_preview_shows_plan_masks_secret_writes_nothing(client, mem_keyring, fake_claude, launcher):
    res = post(client, {**SLACK, "confirm": False})
    body = res.json()
    assert res.status_code == 200 and body["pending"] is True
    text = "\n".join(body["lines"])
    assert "will change" in text and "mcp add-json slack" in text and "SLACK_MCP_XOXB_TOKEN=***" in text
    assert SECRET not in res.text
    assert mem_keyring.items == {} and fake_claude.writes() == []


def test_confirm_writes_through_the_cli_core_and_never_echoes(client, mem_keyring, fake_claude, launcher):
    res = post(client, {**SLACK, "confirm": True})
    body = res.json()
    assert body == {"status": 0, "pending": False, "lines": body["lines"]}
    assert "configured" in body["lines"][-1]
    item = json.loads(mem_keyring.items[("easy-personal/slack", "connector")])
    assert item["env"] == {"SLACK_MCP_XOXB_TOKEN": SECRET}
    adds = fake_claude.writes()
    assert adds[0][:3] == ["mcp", "add-json", "slack"] and adds[0][-2:] == ["--scope", "user"]
    assert SECRET not in res.text and SECRET not in json.dumps(adds)


def test_double_submit_is_refused_not_overwritten(client, mem_keyring, fake_claude, launcher, monkeypatch):
    assert post(client, {**SLACK, "confirm": True}).json()["status"] == 0
    monkeypatch.setenv("FAKE_CLAUDE_GET_RC", "0")
    again = post(client, {**SLACK, "values": {"SLACK_MCP_XOXB_TOKEN": "other-value"}, "confirm": True})
    body = again.json()
    assert body["status"] == 1 and "already exists" in "\n".join(body["lines"])
    assert "other-value" not in again.text and SECRET not in again.text
    assert len(fake_claude.writes()) == 1


def test_empty_value_writes_nothing(client, mem_keyring, fake_claude, launcher):
    res = post(client, {"provider": "slack", "values": {}, "confirm": True})
    assert res.json()["status"] == 2 and res.json()["pending"] is False
    assert mem_keyring.items == {} and fake_claude.writes() == []


def test_claude_failure_rolls_back_keychain(client, mem_keyring, fake_claude, launcher, monkeypatch):
    monkeypatch.setenv("FAKE_CLAUDE_RC", "1")
    res = post(client, {**SLACK, "confirm": True})
    assert res.json()["status"] == 1 and "rolled back" in "\n".join(res.json()["lines"])
    assert mem_keyring.items == {} and SECRET not in res.text


def test_missing_claude_returns_snippet(client, mem_keyring, no_claude, launcher):
    res = post(client, {**SLACK, "confirm": True})
    text = "\n".join(res.json()["lines"])
    assert res.json()["status"] == 0 and "claude mcp add-json slack" in text
    assert mem_keyring.items == {} and SECRET not in res.text


def test_missing_keychain_shows_refusal_guidance(client, no_keyring, fake_claude, launcher):
    res = post(client, {**SLACK, "confirm": True})
    body = res.json()
    assert res.status_code == 200 and body["status"] == 1 and "keychain" in "\n".join(body["lines"])
    assert SECRET not in res.text and fake_claude.writes() == []


def test_delete_previews_then_removes(client, mem_keyring, fake_claude, launcher):
    post(client, {**SLACK, "confirm": True})
    fake_claude.log.unlink()
    preview = client.request("DELETE", "/api/connectors/slack", content='{"confirm": false}', headers={**AUTH, **JSON}).json()
    assert preview["pending"] is True and "will change" in "\n".join(preview["lines"])
    assert ("easy-personal/slack", "connector") in mem_keyring.items and fake_claude.writes() == []
    done = client.request("DELETE", "/api/connectors/slack", content='{"confirm": true}', headers={**AUTH, **JSON}).json()
    assert done["status"] == 0 and "removed" in done["lines"][-1]
    assert mem_keyring.items == {} and fake_claude.writes() == [["mcp", "remove", "slack", "--scope", "user"]]


def test_delete_invalid_name_is_not_found(client, mem_keyring, fake_claude):
    res = client.request("DELETE", "/api/connectors/Bad Name", content='{"confirm": true}', headers={**AUTH, **JSON})
    assert res.status_code == 404
    assert fake_claude.writes() == []


def test_delete_of_non_catalog_name_is_404_and_runs_no_claude(client, mem_keyring, fake_claude):
    res = client.request("DELETE", "/api/connectors/github", content='{"confirm": true}', headers={**AUTH, **JSON})
    assert res.status_code == 404
    assert "github" not in res.text
    assert fake_claude.calls() == []


def test_get_survives_a_broken_keychain(client, fake_claude):
    import keyring
    from keyring.backend import KeyringBackend
    from keyring.errors import KeyringError

    class Locked(KeyringBackend):
        priority = 1

        def get_password(self, service, username):
            raise KeyringError("locked")

        def set_password(self, service, username, password):
            raise KeyringError("locked")

        def delete_password(self, service, username):
            raise KeyringError("locked")

    old = keyring.get_keyring()
    keyring.set_keyring(Locked())
    try:
        res = client.get("/api/connectors", headers=AUTH)
    finally:
        keyring.set_keyring(old)
    assert res.status_code == 200
    assert not any(i["configured"] for i in res.json()["connectors"])
