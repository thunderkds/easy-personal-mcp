"""`--account` on the CLI and `account=` on the API, alone and ANDed with the workspace filter (HUN-63)."""

import json

import pytest
from starlette.testclient import TestClient

from easy_personal.core import store, workspaces
from easy_personal.surfaces import api, cli
from tests_helpers import add_event

TOKEN, PORT = "tok-123", 5555
AUTH = {api.TOKEN_HEADER: TOKEN}
# (project, account id): 2 accounts x 2 workspaces
EVENTS = [("/work/a", 1), ("/work/a", 1), ("/work/b", 1), ("/work/a", 2), ("/work/b", 2)]


@pytest.fixture
def seeded(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    (tmp_path / "home").mkdir()
    with store.opened(tmp_path / "home" / "store.sqlite") as conn:
        conn.execute(
            "INSERT INTO agent_accounts (provider, folder, label, status, config_dir_env) VALUES "
            "('claude', '/a/.claude', 'Claude default', 'confirmed', NULL),"
            "('claude', '/a/.claude-work', 'Claude work', 'confirmed', '/a/.claude-work')"
        )
        for n, (project, account) in enumerate(EVENTS, 1):
            add_event(conn, n, project)
            conn.execute("UPDATE events SET agent_account_id = ? WHERE byte_offset = ?", (account, n))
        workspaces.add(conn, "wa", repos=["/work/a"])
        workspaces.add(conn, "wb", repos=["/work/b"])
        conn.commit()


def run(capsys, *argv, code=0):
    rc = cli.main(list(argv))
    out = capsys.readouterr()
    assert rc == code, out
    return out


def messages(capsys, *argv):
    return json.loads(run(capsys, "usage", *argv).out)["messages"]


def test_cli_account_by_label_and_id(seeded, capsys):
    assert messages(capsys) == 5
    assert messages(capsys, "--account", "Claude work") == 2
    assert messages(capsys, "--account", "claude WORK") == 2
    assert messages(capsys, "--account", "1") == 3
    out = json.loads(run(capsys, "usage", "--by", "project", "--account", "2").out)
    assert out["total"]["messages"] == 2
    assert "sessions" in json.loads(run(capsys, "outliers", "--account", "2").out)


@pytest.mark.parametrize("command", [["usage"], ["usage", "--by", "day"], ["outliers"]])
def test_cli_unknown_account_exits_2_and_lists_labels(seeded, capsys, command):
    out = run(capsys, *command, "--account", "nope-secret", code=2)
    assert "Claude default" in out.err and "Claude work" in out.err
    assert "nope-secret" not in out.err + out.out and out.out == ""


def test_cli_account_and_workspace_are_anded(seeded, capsys):
    expected = {("wa", "Claude default"): 2, ("wa", "Claude work"): 1,
                ("wb", "Claude default"): 1, ("wb", "Claude work"): 1}  # fmt: skip
    for (ws, acct), n in expected.items():
        assert messages(capsys, "--workspace", ws, "--account", acct) == n, (ws, acct)
    run(capsys, "usage", "--workspace", "nope", "--account", "Claude work", code=2)


@pytest.fixture
def client(seeded):
    return TestClient(api.create_app(TOKEN, PORT), base_url=f"http://127.0.0.1:{PORT}")


def get(client, path, query=""):
    return client.get(path + query, headers=AUTH)


def test_api_account_param(client):
    assert get(client, "/api/usage/summary").json()["messages"] == 5
    assert get(client, "/api/usage/summary", "?account=Claude%20work").json()["messages"] == 2
    assert get(client, "/api/usage/summary", "?account=1").json()["messages"] == 3
    assert get(client, "/api/usage/summary", "?account=").json()["messages"] == 5
    assert get(client, "/api/usage/breakdown", "?by=project&account=2").json()["total"]["messages"] == 2
    assert get(client, "/api/usage/outliers", "?account=2").status_code == 200
    assert len(get(client, "/api/usage/sessions", "?account=2").json()["sessions"]) == 2


def test_api_unknown_account_is_400_with_labels(client):
    for path in ("/api/usage/summary", "/api/usage/breakdown?by=day", "/api/usage/outliers", "/api/usage/sessions"):
        sep = "&" if "?" in path else "?"
        res = client.get(f"{path}{sep}account=nope-secret", headers=AUTH)
        assert res.status_code == 400, path
        assert "Claude work" in res.json()["error"] and "nope-secret" not in res.text


def test_api_account_and_workspace_are_anded(client):
    expected = {("wa", "Claude default"): 2, ("wa", "Claude work"): 1,
                ("wb", "Claude default"): 1, ("wb", "Claude work"): 1}  # fmt: skip
    for (ws, acct), n in expected.items():
        query = f"?workspace={ws}&account={acct.replace(' ', '%20')}"
        assert get(client, "/api/usage/summary", query).json()["messages"] == n, (ws, acct)
        assert get(client, "/api/usage/breakdown", query + "&by=model").json()["total"]["messages"] == n
        assert sum(len(s["points"]) for s in get(client, "/api/usage/sessions", query).json()["sessions"]) == n
