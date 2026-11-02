"""HUN-189 (FR-057): the dashboard reads count a gone worktree under its owner project."""

import os
import subprocess

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config, owners, store
from easy_personal.surfaces import api

TOKEN, PORT = "tok-189", 5589
AUTH = {api.TOKEN_HEADER: TOKEN}


def _git(repo, *args):
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null"}
    cmd = ["git", "-c", "user.email=t@example.invalid", "-c", "user.name=t", "-C", str(repo), *args]
    subprocess.run(cmd, check=True, capture_output=True, text=True, env=env)


def _event(conn, n, path, branch=None):
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc,"
        " session_id, project_path, git_branch, model, message_id, input_tokens, output_tokens,"
        " cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,0,?,?,?,?,'model-0',?,10,5,0,0)",
        (n, f"2026-01-01T00:00:{n:02d}.000000Z", f"s{n}", str(path), branch, f"msg{n}"),
    )
    conn.commit()


@pytest.fixture
def client(tmp_path):
    owners.clear_cache()
    repo = (tmp_path / "repo").resolve()
    repo.mkdir()
    _git(repo, "init", "-q", "-b", "main")
    _git(repo, "commit", "-q", "--allow-empty", "-m", "init")
    _git(repo, "branch", "feat/sib")
    with store.opened(config.store_path()) as c:
        _event(c, 0, repo)
        _event(c, 1, repo / ".claude" / "worktrees" / "x")  # rule 1
        _event(c, 2, tmp_path / "wt-sib", "feat/sib")  # rule 2
        _event(c, 3, tmp_path / "wt-other", "feat/none")  # no owner
    yield TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://127.0.0.1:{PORT}")
    owners.clear_cache()


def test_usage_by_project_route_merges_the_owner_rows(client):
    body = client.get("/api/usage/breakdown?by=project", headers=AUTH).json()
    tokens = {g["key"]: g["input_tokens"] + g["output_tokens"] for g in body["groups"]}
    assert tokens == {"repo": 45, "wt-other": 15}
    assert body["total"]["input_tokens"] + body["total"]["output_tokens"] == 60


def test_kit_and_board_list_the_owner_once_and_count_fewer_gone(client):
    kit_out = client.get("/api/kit", headers=AUTH).json()
    assert [p["project"] for p in kit_out["projects"]] == ["repo"] and kit_out["gone"] == 1
    assert client.get("/api/board", headers=AUTH).json()["gone"] == 1
