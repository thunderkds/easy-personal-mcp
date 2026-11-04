"""HUN-206 (FR-070): `GET /api/workspaces/unassigned` lists repos in no workspace, behind the token and Host check."""

import os
import subprocess

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config, owners, store, workspaces
from easy_personal.surfaces import api
from tests_helpers import add_event

TOKEN, PORT = "tok-206", 5506
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
URL = "/api/workspaces/unassigned"


def _repo(path):
    path.mkdir(parents=True)
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null"}
    subprocess.run(["git", "-C", str(path), "init", "-q", "-b", "main"], check=True, capture_output=True, env=env)


@pytest.fixture
def world(tmp_path, monkeypatch):
    root = tmp_path.resolve()
    monkeypatch.setattr(owners, "_tmp_roots", lambda: (str(root / "systmp"),))
    owners.clear_cache()
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(root / "home"))
    covered, open_repo = root / "covered", root / "open"
    _repo(covered)
    _repo(open_repo)
    with store.opened(config.store_path()) as conn:
        add_event(conn, 0, str(covered))
        add_event(conn, 1, str(open_repo))
        add_event(conn, 2, str(root / "systmp" / "box"))
        workspaces.add(conn, "acme", repos=[str(covered)])
        conn.commit()
    yield {"open": open_repo, "client": TestClient(api.create_app(TOKEN, PORT, root / "absent"), base_url=f"http://{HOST}")}
    owners.clear_cache()


def test_route_answers_repos_in_no_workspace(world):
    res = world["client"].get(URL, headers=AUTH)
    assert res.status_code == 200
    (repo,) = res.json()["repos"]
    assert repo["path"] == str(world["open"]) and repo["name"] == "open" and repo["sessions"] == 1
    assert set(repo) == {"path", "name", "sessions", "api_equivalent_cost", "last_used_utc"}
    assert repo["api_equivalent_cost"]["abstained_models"] == ["model-0"]


def test_route_is_empty_when_every_repo_is_covered(world):
    with store.opened(config.store_path()) as conn:
        workspaces.add(conn, "other", repos=[str(world["open"])])
    assert world["client"].get(URL, headers=AUTH).json() == {"repos": []}


def test_route_refuses_no_token_wrong_token_and_wrong_host(world):
    client = world["client"]
    assert client.get(URL).status_code == 403
    assert client.get(URL, headers={api.TOKEN_HEADER: "nope"}).status_code == 403
    assert client.get(URL, headers={**AUTH, "Host": "evil.example"}).status_code == 403


def test_it_is_not_taken_for_a_workspace_id(world):
    assert world["client"].get("/api/workspaces", headers=AUTH).status_code == 200
    assert world["client"].request("PATCH", URL, headers=AUTH, content="{}").status_code != 200
