"""HUN-240 (FR-089): `GET /api/kit/reports`, behind the token and Host check."""

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config, store, workspaces
from easy_personal.surfaces import api

TOKEN, PORT = "tok-240", 5540
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
URL = "/api/kit/reports"


@pytest.fixture
def world(tmp_path, monkeypatch):
    root = tmp_path.resolve()
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(root / "home"))
    app, lib = root / "app", root / "lib"
    (app / "reports").mkdir(parents=True)
    (app / "reports" / "session-1.json").write_text("{}")
    (app / "reports" / "a.html").write_text("x")
    lib.mkdir()
    with store.opened(config.store_path()) as conn:
        workspaces.add(conn, "acme", repos=[str(app), str(lib)])
        workspaces.add(conn, "nobody", repos=[])
        conn.commit()
    return TestClient(api.create_app(TOKEN, PORT, root / "absent"), base_url=f"http://{HOST}")


def test_route_answers_the_shape_per_workspace(world):
    res = world.get(URL, headers=AUTH)
    assert res.status_code == 200
    acme, nobody = res.json()["workspaces"]
    assert set(res.json()) == {"workspaces"} and acme["name"] == "acme" and nobody == {"name": "nobody", "folders": []}
    (folder,) = acme["folders"]
    assert set(folder) == {"repo", "folder", "files", "files_capped", "easy_ui_sessions", "newest_utc"}
    assert (folder["repo"], folder["folder"], folder["files"], folder["easy_ui_sessions"]) == ("app", "reports", 2, 1)
    assert folder["files_capped"] is False and folder["newest_utc"].endswith("Z")


def test_query_parameters_are_ignored(world):
    assert world.get(URL, params={"since": "not-a-date", "account": "nope"}, headers=AUTH).status_code == 200


def test_no_workspaces_gives_an_empty_list(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    client = TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://{HOST}")
    assert client.get(URL, headers=AUTH).json() == {"workspaces": []}


def test_route_refuses_no_token_wrong_token_and_wrong_host(world):
    assert world.get(URL).status_code == 403
    assert world.get(URL, headers={api.TOKEN_HEADER: "nope"}).status_code == 403
    assert world.get(URL, headers={**AUTH, "Host": "evil.example"}).status_code == 403


def test_it_is_a_read_only_route(world):
    for method in ("POST", "PUT", "PATCH", "DELETE"):
        assert world.request(method, URL, headers=AUTH, content="{}").status_code == 405
