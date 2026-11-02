"""GET /api/memory-graph (HUN-164): project list, one project's graph, guarded like every /api route."""

import hashlib
import json
import os

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
    assert body == {"projects": [{"project": str(repo), "name": "alpha", "kit_notes": 2, "claude_notes": 0, "easy_memory_notes": 0}], "gone": 0}


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


@pytest.fixture
def em_store(repo, tmp_path, monkeypatch):
    """An easy-memory store for `repo`: one note citing a changed file and a kit file, plus an easy-memory-only project."""
    (repo / ".git").mkdir()
    (repo / "a.py").write_text("now")
    home = tmp_path / "em-home"
    monkeypatch.setenv("EASY_MEMORY_HOME", str(home))

    def plant(project, files):
        folder = home / hashlib.sha256(os.path.realpath(project / ".git").encode()).hexdigest()[:16]
        (folder / "notes").mkdir(parents=True)
        (folder / "notes" / "n.json").write_text(json.dumps(
            {"v": 1, "time": "2026-11-01T10:00:00Z", "prompt": "INVENTED_P", "answer": "INVENTED_A", "session_id": "INVENTED_S", "files": files}))
        return folder

    plant(repo, {"a.py": "0" * 64, "memory/decisions.md": "1" * 64})
    only = tmp_path / "only-em"
    (only / ".git").mkdir(parents=True)
    plant(only, {})
    from easy_personal.core import store

    with store.opened(tmp_path / "home" / "store.sqlite") as conn:
        add_event(conn, 2, str(only))
        conn.commit()
    return only


def test_list_has_easy_memory_counts_and_easy_memory_only_projects(client, repo, em_store):
    rows = {p["name"]: p for p in client.get(PATH, headers=AUTH).json()["projects"]}
    assert rows["alpha"]["easy_memory_notes"] == 1
    assert rows["only-em"] == {"project": str(em_store), "name": "only-em", "kit_notes": 0, "claude_notes": 0, "easy_memory_notes": 1}


def test_graph_has_cited_file_nodes_and_panel(client, repo, em_store):
    res = client.get(PATH, params={"project": str(repo)}, headers=AUTH)
    body = res.json()
    node = next(n for n in body["nodes"] if n["id"] == "easymem:a.py")
    assert node["kind"] == "easy_memory" and node["ring"] == 4 and node["stale"] == "stale" and node["cited"] == 1
    kit = next(n for n in body["nodes"] if n["path"] == "memory/decisions.md")
    assert kit["kind"] == "kit" and kit["stale"] == "stale"
    assert body["easy_memory"]["notes"] == 1 and len(body["easy_memory"]["days"]) == 30
    for planted in ("INVENTED_P", "INVENTED_A", "INVENTED_S"):
        assert planted not in res.text


def test_graph_without_a_store_has_a_null_block(client, repo):
    assert client.get(PATH, params={"project": str(repo)}, headers=AUTH).json()["easy_memory"] is None
