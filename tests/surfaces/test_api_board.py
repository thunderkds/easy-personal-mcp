"""`GET /api/board` (HUN-162): guarded, read-only, scoped like /api/kit."""

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config, store, workspaces
from easy_personal.surfaces import api
from tests_helpers import add_event

TOKEN, PORT = "tok-123", 5555
AUTH = {api.TOKEN_HEADER: TOKEN}


@pytest.fixture
def client(tmp_path):
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://127.0.0.1:{PORT}")


@pytest.fixture
def repos(tmp_path):
    out = {}
    for n, name in enumerate(["alpha", "beta"]):
        root = tmp_path / "repos" / name
        (root / ".claude").mkdir(parents=True)
        (root / "PROJECT_KANBAN.md").write_text(f"## Board\n### Todo\n- [ ] **HUN-{n}** — Task of {name} | a\n")
        out[name] = root
    (out["alpha"] / ".claude" / "board.json").write_text('{"board": "linear"}')
    with store.opened(config.store_path()) as conn:
        for n, root in enumerate(out.values()):
            add_event(conn, n, str(root))
        conn.commit()
        workspaces.add(conn, "acme", repos=[str(out["alpha"])])
    return out


def test_board_needs_the_token(client):
    assert client.get("/api/board").status_code == 403


def test_board_lists_each_project_with_its_tasks_and_board_kind(client, repos):
    res = client.get("/api/board", headers=AUTH)
    assert res.status_code == 200
    projects = {p["project"]: p for p in res.json()["projects"]}
    assert set(projects) == {"alpha", "beta"}
    assert projects["alpha"]["board"] == "linear" and projects["alpha"]["last_pull"].endswith("Z")
    assert projects["beta"]["board"] is None and projects["beta"]["last_pull"] is None
    assert projects["alpha"]["tasks"] == [{"id": "HUN-0", "title": "Task of alpha", "status": "Todo"}]


def test_workspace_scope_filters_and_an_unknown_workspace_is_400(client, repos):
    res = client.get("/api/board?workspace=acme", headers=AUTH)
    assert [p["project"] for p in res.json()["projects"]] == ["alpha"]
    assert client.get("/api/board?workspace=nope", headers=AUTH).status_code == 400


def test_empty_store_gives_no_projects(client):
    assert client.get("/api/board", headers=AUTH).json() == {"projects": []}
