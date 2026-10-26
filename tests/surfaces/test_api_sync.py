"""Dashboard routes for picks and sync (HUN-65): guarded, plan first, writes only with `confirm: true`."""

import json

import pytest
from starlette.testclient import TestClient

from easy_personal.core import accounts, config, store, workspaces
from easy_personal.surfaces import api

TOKEN, PORT = "tok-123", 5555
AUTH = {api.TOKEN_HEADER: TOKEN}
JSON = {**AUTH, "content-type": "application/json"}
CID = "c0000aaaa"


@pytest.fixture
def world(tmp_path, monkeypatch, claude_state, launcher, mem_keyring):
    root = tmp_path / "agents"
    (root / ".claude").mkdir(parents=True)
    (root / ".codex").mkdir()
    (root / ".codex" / "config.toml").write_text("")
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(root))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", "/runner/dir")
    with store.opened(config.store_path()) as conn:
        accounts.detect(conn)
        ws = workspaces.add(conn, "company")["id"]
        with conn:
            conn.execute("INSERT INTO connectors VALUES (?, 'company-slack', 0, 'slack', ?)", (CID, ws))
        ids = {a["provider"]: a["id"] for a in accounts.list_accounts(conn)}
    client = TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://127.0.0.1:{PORT}")
    return client, ids, claude_state


def put_picks(client, account_id, names, headers=JSON):
    return client.put(f"/api/accounts/{account_id}/workspaces", content=json.dumps({"workspaces": names}), headers=headers)


def test_picks_then_plan_then_confirm(world):
    client, ids, fake = world
    res = put_picks(client, ids["claude"], ["company"])
    assert res.status_code == 200 and res.json()["workspaces"] == ["company"]
    plan = client.get("/api/sync", headers=AUTH).json()
    assert [s["line"] for s in plan["steps"]] == [f"Claude default: add company-slack -> exec {CID}"]
    assert plan["steps"][0]["action"] == "add" and fake.write_records() == []
    preview = client.post("/api/sync", content='{"confirm": false}', headers=JSON).json()
    assert preview["pending"] is True and preview["lines"] == [s["line"] for s in plan["steps"]]
    assert fake.write_records() == []
    done = client.post("/api/sync", content='{"confirm": true}', headers=JSON).json()
    assert done["status"] == 0 and done["pending"] is False
    assert [d for d, _ in fake.write_records()] == ["<unset>"]
    assert client.get("/api/sync", headers=AUTH).json()["steps"] == []
    again = client.post("/api/sync", content='{"confirm": true}', headers=JSON).json()
    assert again["status"] == 0 and again["pending"] is False and "nothing to change" in again["lines"]


def test_sync_for_one_account(world):
    client, ids, _ = world
    put_picks(client, ids["claude"], ["company"])
    body = json.dumps({"account": ids["claude"], "confirm": True})
    assert client.post("/api/sync", content=body, headers=JSON).json()["status"] == 0
    assert client.get(f"/api/sync?account={ids['claude']}", headers=AUTH).json()["steps"] == []
    assert client.get("/api/sync?account=999", headers=AUTH).status_code == 404
    assert client.post("/api/sync", content='{"account": 999, "confirm": true}', headers=JSON).status_code == 404


def test_codex_orphan_and_bad_bodies_are_refused(world):
    client, ids, fake = world
    assert put_picks(client, ids["codex"], ["company"]).status_code == 400
    assert put_picks(client, ids["claude"], ["orphan"]).status_code == 400
    res = put_picks(client, ids["claude"], ["PLANTED-nope"])
    assert res.status_code == 400 and "PLANTED" not in res.text
    assert put_picks(client, ids["claude"], "company").status_code == 400
    assert put_picks(client, 999, ["company"]).status_code == 404
    assert client.post("/api/sync", content='{"confirm": "yes"}', headers=JSON).status_code == 400
    assert client.post("/api/sync", content='{"account": "x", "confirm": true}', headers=JSON).status_code == 400
    assert fake.write_records() == []


@pytest.mark.parametrize(("method", "path"), [("GET", "/api/sync"), ("POST", "/api/sync"),
                                              ("PUT", "/api/accounts/1/workspaces")])  # fmt: skip
def test_guard_applies(world, method, path):
    client, _, fake = world
    kw = {"content": '{"confirm": true, "workspaces": []}'}
    assert client.request(method, path, headers={"content-type": "application/json"}, **kw).status_code == 403
    evil = {**JSON, "Origin": "http://evil.example"}
    assert client.request(method, path, headers=evil, **kw).status_code == 403
    assert fake.write_records() == []
