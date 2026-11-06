"""HUN-238 (FR-087, FR-088): `GET /api/kit/workspaces`, behind the token and Host check."""

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config, owners, store, workspaces
from easy_personal.surfaces import api
from tests_helpers import add_event

TOKEN, PORT = "tok-238", 5538
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
URL = "/api/kit/workspaces"
UI = "mcp__easy-ui__"


@pytest.fixture
def world(tmp_path, monkeypatch):
    root = tmp_path.resolve()
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(root / "home"))
    owners.clear_cache()
    repo, empty = root / "repo", root / "empty"
    repo.mkdir()
    empty.mkdir()
    with store.opened(config.store_path()) as conn:
        conn.execute("INSERT INTO verifier_scores VALUES (?, 'ok', 'project', '2026-02-01T09:00:00Z', 80.0, 0, NULL)", (str(repo),))
        add_event(conn, 1, str(repo), ts="2026-02-10T12:00:00.000000Z")
        conn.execute("UPDATE events SET tool_names = ?", (f"{UI}open,{UI}click",))
        add_event(conn, 2, str(repo), ts="2026-04-10T12:00:00.000000Z")
        conn.execute("UPDATE events SET tool_names = ? WHERE byte_offset = 2", (f"{UI}open",))
        conn.execute(
            "INSERT INTO agent_accounts (provider, folder, label, status, config_dir_env) VALUES "
            "('claude', '/a/.claude', 'Claude default', 'confirmed', NULL)"
        )
        workspaces.add(conn, "acme", color="#336699", repos=[str(repo)])
        workspaces.add(conn, "nobody", repos=[str(empty)])
        conn.commit()
    yield TestClient(api.create_app(TOKEN, PORT, root / "absent"), base_url=f"http://{HOST}")
    owners.clear_cache()


def test_route_answers_the_shape_per_workspace(world):
    res = world.get(URL, headers=AUTH)
    assert res.status_code == 200
    body = res.json()
    assert set(body) == {"workspaces", "ui_checks"}
    acme, nobody = body["workspaces"]
    assert acme["name"] == "acme" and acme["color"] == "#336699" and acme["repos"] == 1
    assert acme["verifier"] == {
        "status": "ok", "repos_scored": 1, "repos_total": 1, "mean": 80.0,
        "lowest": {"project": "repo", "value": 80.0}, "newest_utc": "2026-02-01T09:00:00Z",
    }  # fmt: skip
    assert acme["ui"] == {"status": "ok", "sessions": 1, "calls": 3, "last_utc": "2026-04-10T12:00:00.000000Z"}
    assert nobody["verifier"]["status"] == "no data" and nobody["ui"]["status"] == "not used"
    assert body["ui_checks"] == {"status": "no data"}


def test_since_and_until_limit_the_easy_ui_part_only(world):
    body = world.get(URL, params={"since": "2026-02-01T00:00:00Z", "until": "2026-03-01T00:00:00Z"}, headers=AUTH).json()
    acme = body["workspaces"][0]
    assert acme["ui"]["calls"] == 2 and acme["ui"]["last_utc"] == "2026-02-10T12:00:00.000000Z"
    assert acme["verifier"]["repos_scored"] == 1


def test_account_is_honoured_and_an_unknown_one_is_a_400(world):
    assert world.get(URL, params={"account": "Claude default"}, headers=AUTH).status_code == 200
    bad = world.get(URL, params={"account": "nope"}, headers=AUTH)
    assert bad.status_code == 400 and "nope" not in bad.text
    assert world.get(URL, params={"account": ""}, headers=AUTH).status_code == 200  # empty = no filter


def test_a_bad_date_is_a_400(world):
    assert world.get(URL, params={"since": "not-a-date"}, headers=AUTH).status_code == 400


def test_no_workspaces_gives_an_empty_list(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    client = TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://{HOST}")
    assert client.get(URL, headers=AUTH).json() == {"workspaces": [], "ui_checks": {"status": "no data"}}


def test_route_refuses_no_token_wrong_token_and_wrong_host(world):
    assert world.get(URL).status_code == 403
    assert world.get(URL, headers={api.TOKEN_HEADER: "nope"}).status_code == 403
    assert world.get(URL, headers={**AUTH, "Host": "evil.example"}).status_code == 403


def test_it_is_a_read_only_route(world):
    assert world.request("POST", URL, headers=AUTH, content="{}").status_code == 405


def test_kit_route_is_unchanged(world):
    assert world.get("/api/kit", headers=AUTH).status_code == 200
