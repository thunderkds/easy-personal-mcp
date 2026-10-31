"""GET /api/memory-graph (HUN-164): project list, one project's graph, guarded like every /api route."""

import pytest
from starlette.testclient import TestClient

from easy_personal.core import store
from easy_personal.surfaces import api
from tests_helpers import add_event

TOKEN, PORT = "tok-123", 5555
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
PATH = "/api/memory-graph"


@pytest.fixture
def repo(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    (tmp_path / "home").mkdir()
    repo = tmp_path / "alpha"
    (repo / "memory").mkdir(parents=True)
    (repo / "memory" / "MEMORY.md").write_text("---\nname: index\ndescription: hot\n---\nSECRET_Y [[ghost]]\n")
    (repo / "memory" / "decisions.md").write_text("[[index]]\n")
    with store.opened(tmp_path / "home" / "store.sqlite") as conn:
        add_event(conn, 1, str(repo))
        conn.commit()
    return repo


@pytest.fixture
def client(repo):
    return TestClient(api.create_app(TOKEN, PORT), base_url=f"http://{HOST}")


def test_requires_the_token_and_host(client):
    assert client.get(PATH).status_code == 403
    assert client.get(PATH, headers={**AUTH, "Host": "evil.example"}).status_code == 403


def test_without_project_lists_projects(client, repo):
    body = client.get(PATH, headers=AUTH).json()
    assert body == {"projects": [{"project": str(repo), "name": "alpha", "kit_notes": 2, "claude_notes": 0}]}


def test_with_project_returns_graph_without_bodies(client, repo):
    res = client.get(PATH, params={"project": str(repo)}, headers=AUTH)
    assert res.status_code == 200
    body = res.json()
    assert {n["name"] for n in body["nodes"]} == {"index", "decisions", "ghost"}
    assert body["counts"]["dangling"] == 1 and len(body["edges"]) == 2
    assert "SECRET_Y" not in res.text


def test_unknown_project_is_400_without_echo(client):
    res = client.get(PATH, params={"project": "/etc/passwd"}, headers=AUTH)
    assert res.status_code == 400 and "passwd" not in res.text
