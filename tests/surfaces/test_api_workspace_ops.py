"""Dashboard routes for move / alias / workspace rename / delete (HUN-66): plan first, `confirm: true` applies."""

import json

import pytest
from starlette.testclient import TestClient

from easy_personal.core import accounts, config, store, workspaces
from easy_personal.surfaces import api

TOKEN, PORT = "tok-123", 5555
JSON = {api.TOKEN_HEADER: TOKEN, "content-type": "application/json"}
CID = "c0000aaaa"


@pytest.fixture
def world(tmp_path, monkeypatch, claude_state, launcher, mem_keyring):
    root = tmp_path / "agents"
    (root / ".claude").mkdir(parents=True)
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(root))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", "/runner/dir")
    with store.opened(config.store_path()) as conn:
        accounts.detect(conn)
        ws = {n: workspaces.add(conn, n)["id"] for n in ("company", "side")}
        with conn:
            conn.execute("INSERT INTO connectors VALUES (?, 'company-slack', 0, 'slack', ?)", (CID, ws["company"]))
        accounts.use(conn, "1", ["company"])
    client = TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://127.0.0.1:{PORT}")
    assert client.post("/api/sync", content='{"confirm": true}', headers=JSON).json()["status"] == 0
    claude_state.log.unlink()
    return client, ws, claude_state, mem_keyring


def send(client, method, path, body):
    return client.request(method, path, content=json.dumps(body), headers=JSON)


def alias_of(cid=CID):
    with store.opened(config.store_path()) as conn:
        return conn.execute("SELECT alias, workspace_id FROM connectors WHERE id = ?", (cid,)).fetchone()


def test_move_previews_then_applies(world):
    client, ws, fake, keyring = world
    res = send(client, "POST", f"/api/connectors/{CID}/move", {"workspace": "side", "confirm": False})
    assert res.status_code == 200 and res.json()["pending"] is True
    assert any("remove company-slack" in ln for ln in res.json()["lines"])
    assert alias_of() == ("company-slack", ws["company"]) and fake.write_records() == []
    res = send(client, "POST", f"/api/connectors/{CID}/move", {"workspace": "side", "confirm": True})
    assert res.json()["status"] == 0 and res.json()["pending"] is False
    assert alias_of() == ("side-slack", ws["side"])
    assert [(a[1], a[2]) for _, a in fake.write_records()] == [("remove", "company-slack")]  # side is not picked
    assert keyring.items == {}


def test_alias_set_and_reset_and_refusals(world):
    client = world[0]
    res = send(client, "PUT", f"/api/connectors/{CID}/alias", {"alias": "team-chat", "confirm": True})
    assert res.json()["status"] == 0 and alias_of()[0] == "team-chat"
    res = send(client, "PUT", f"/api/connectors/{CID}/alias", {"alias": None, "confirm": True})
    assert res.json()["status"] == 0 and alias_of()[0] == "company-slack"
    for body in ({"alias": "Bad Alias!", "confirm": True}, {"alias": 7}, {"alias": "x", "confirm": "yes"}):
        assert send(client, "PUT", f"/api/connectors/{CID}/alias", body).status_code == 400, body
    assert send(client, "PUT", "/api/connectors/nope/alias", {"alias": "x"}).status_code == 400
    assert alias_of()[0] == "company-slack"


def test_move_refusals(world):
    client, ws, _, _ = world
    assert send(client, "POST", f"/api/connectors/{CID}/move", {"confirm": True}).status_code == 400
    assert send(client, "POST", f"/api/connectors/{CID}/move", {"workspace": "missing"}).status_code == 404
    assert send(client, "POST", f"/api/connectors/{CID}/move", {"workspace": "company"}).status_code == 400
    assert alias_of() == ("company-slack", ws["company"])


def test_rename_and_delete_workspace_with_plan(world):
    client, ws, fake, _ = world
    path = f"/api/workspaces/{ws['company']}"
    res = send(client, "PATCH", path, {"name": "firm", "confirm": False})
    body = res.json()
    assert res.status_code == 200 and body["pending"] is True and body["workspace"]["name"] == "company"
    res = send(client, "PATCH", path, {"name": "firm", "confirm": True})
    assert res.json()["workspace"]["name"] == "firm" and alias_of()[0] == "firm-slack"
    res = send(client, "DELETE", path, {})  # no confirm: plan only
    assert res.json()["pending"] is True and res.json()["deleted"] is None and alias_of()[0] == "firm-slack"
    res = send(client, "DELETE", path, {"confirm": True})
    assert res.json()["deleted"] == ws["company"] and res.json()["status"] == 0
    assert alias_of()[0] == "firm-slack" and alias_of()[1] not in ws.values()  # now in orphan
    assert fake.state()["<unset>"] == {}
    assert send(client, "DELETE", path, {"confirm": True}).status_code == 404
    assert send(client, "DELETE", f"/api/workspaces/{ws['side']}", {"confirm": "yes"}).status_code == 400
