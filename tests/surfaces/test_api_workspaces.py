"""Dashboard workspace routes: CRUD, `workspace=` on usage routes, and the write guard (HUN-64)."""

import json

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config, store
from easy_personal.surfaces import api
from tests_helpers import add_event

TOKEN, PORT = "tok-123", 5555
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
JSON = {"content-type": "application/json"}


@pytest.fixture
def client(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://{HOST}")


def send(client, method, path, body=None, headers=None):
    headers = AUTH | JSON if headers is None else headers
    return client.request(method, path, headers=headers, content=json.dumps(body or {}))


def create(client, **body):
    res = send(client, "POST", "/api/workspaces", {"name": "acme", **body})
    assert res.status_code == 201, res.text
    return res.json()["workspace"]


def test_list_has_orphan_first(client):
    res = client.get("/api/workspaces", headers=AUTH)
    assert res.status_code == 200
    assert [w["name"] for w in res.json()["workspaces"]] == ["orphan"]
    assert res.json()["workspaces"][0]["is_orphan"] is True


def test_crud_roundtrip_keeps_the_id_on_rename(client):
    ws = create(client, color="#abcdef", email="me@acme.test", repos=["/work/a/"])
    assert ws["repos"] == ["/work/a"]
    res = send(client, "PATCH", f"/api/workspaces/{ws['id']}", {"name": "company", "repos": ["/x", "/y"]})
    assert res.status_code == 200
    assert res.json()["workspace"] == {**ws, "name": "company", "repos": ["/x", "/y"]}
    assert [w["name"] for w in client.get("/api/workspaces", headers=AUTH).json()["workspaces"]] == [
        "orphan", "company"]  # fmt: skip
    res = send(client, "DELETE", f"/api/workspaces/{ws['id']}")
    assert res.status_code == 200 and res.json() == {"deleted": ws["id"]}
    assert send(client, "DELETE", f"/api/workspaces/{ws['id']}").status_code == 404


def test_refusals_and_statuses(client):
    ws = create(client)
    assert send(client, "POST", "/api/workspaces", {"name": "acme"}).status_code == 409
    for bad in ({"name": "orphan"}, {"name": "Bad Name"}, {}, {"name": "x", "color": "red"},
                {"name": "x", "bogus": 1}, {"name": "x", "repos": "/a"}, {"name": "x", "repos": ["rel"]}):  # fmt: skip
        assert send(client, "POST", "/api/workspaces", bad).status_code == 400, bad
    orphan_id = client.get("/api/workspaces", headers=AUTH).json()["workspaces"][0]["id"]
    assert send(client, "PATCH", f"/api/workspaces/{orphan_id}", {"name": "z"}).status_code == 403
    assert send(client, "DELETE", f"/api/workspaces/{orphan_id}").status_code == 403
    assert send(client, "PATCH", "/api/workspaces/999", {"description": "x"}).status_code == 404
    other = create_named(client, "other")
    assert send(client, "PATCH", f"/api/workspaces/{other['id']}", {"name": "acme"}).status_code == 409
    assert send(client, "PATCH", f"/api/workspaces/{ws['id']}", {"id": 5}).status_code == 400
    assert send(client, "PATCH", "/api/workspaces/abc", {}).status_code == 404
    assert [w["name"] for w in client.get("/api/workspaces", headers=AUTH).json()["workspaces"]] == [
        "orphan", "acme", "other"]  # fmt: skip


def create_named(client, name):
    res = send(client, "POST", "/api/workspaces", {"name": name})
    assert res.status_code == 201
    return res.json()["workspace"]


def test_errors_do_not_echo_input(client):
    res = send(client, "POST", "/api/workspaces", {"name": "LEAK-CANARY-123"})
    assert res.status_code == 400 and "LEAK-CANARY-123" not in res.text


@pytest.mark.parametrize(
    "method,path",
    [("POST", "/api/workspaces"), ("PATCH", "/api/workspaces/2"), ("DELETE", "/api/workspaces/2")],
)
def test_write_routes_are_guarded(client, method, path):
    create_named(client, "target")  # id 2: the orphan (id 1) would refuse with 403 anyway
    body = {"name": "guarded"}
    assert send(client, method, path, body).status_code in (200, 201)
    # no token / wrong token
    assert send(client, method, path, body, headers=JSON).status_code == 403
    assert send(client, method, path, body, headers={api.TOKEN_HEADER: "nope"} | JSON).status_code == 403
    # wrong Host
    assert send(client, method, path, body, headers=AUTH | JSON | {"host": "evil.test"}).status_code == 403
    # foreign Origin
    assert send(client, method, path, body, headers=AUTH | JSON | {"origin": "http://evil.test"}).status_code == 403
    # not JSON
    form = client.request(method, path, headers=AUTH | {"content-type": "text/plain"}, content="name=x")
    assert form.status_code == 415
    # malformed JSON
    bad = client.request(method, path, headers=AUTH | JSON, content="{nope")
    assert bad.status_code == 400


def test_guarded_writes_change_nothing(client):
    create_named(client, "target")
    for method, path in (("POST", "/api/workspaces"), ("PATCH", "/api/workspaces/2"), ("DELETE", "/api/workspaces/2")):
        send(client, method, path, {"name": "x"}, headers=JSON)
        send(client, method, path, {"name": "x"}, headers=AUTH | JSON | {"origin": "http://evil.test"})
    assert [w["name"] for w in client.get("/api/workspaces", headers=AUTH).json()["workspaces"]] == ["orphan", "target"]


def test_usage_routes_take_workspace(client, tmp_path):
    with store.opened(config.store_path()) as conn:
        for n, project in enumerate(["/work/a", "/work/a/pkg", "/work/b", "/work/bc"]):
            add_event(conn, n, project)
        conn.commit()
    create(client, repos=["/work/a", "/work/b"])
    create_named(client, "empty")
    get = lambda path, q: client.get(path + q, headers=AUTH)
    assert get("/api/usage/summary", "").json()["messages"] == 4
    assert get("/api/usage/summary", "?workspace=acme").json()["messages"] == 3
    assert get("/api/usage/summary", "?workspace=empty").json()["messages"] == 0
    assert get("/api/usage/summary", "?workspace=").json()["messages"] == 4
    assert get("/api/usage/breakdown", "?by=project&workspace=acme").json()["total"]["messages"] == 3
    assert get("/api/usage/outliers", "?workspace=acme").json()["sessions"] == []
    sessions = get("/api/usage/sessions", "?workspace=acme").json()["sessions"]
    assert len(sessions) == 3
    assert get("/api/usage/summary", "?workspace=nope").status_code == 400
