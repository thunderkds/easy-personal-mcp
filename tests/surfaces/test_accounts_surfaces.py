"""`easy-personal account ...` and `/api/accounts` (HUN-62): one core, guarded writes."""

import json

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config
from easy_personal.surfaces import api, cli

TOKEN, PORT = "tok-123", 5555
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
JSON = {"content-type": "application/json"}


@pytest.fixture
def root():
    root = config.agent_root().resolve()
    (root / ".claude").mkdir()
    (root / ".claude-work" / "projects").mkdir(parents=True)
    (root / ".claude.json").write_text("{}")
    (root / ".codex").mkdir()
    (root / ".codex" / "config.toml").write_text("")
    (root / ".claude-junk").mkdir()
    return root


def run(capsys, *argv):
    rc = cli.main(["account", *argv])
    out = capsys.readouterr()
    return rc, out.out, out.err


def test_list_shows_provider_label_folder_status(root, capsys):
    rc, out, _ = run(capsys, "list")
    rows = json.loads(out)["accounts"]
    assert rc == 0
    assert [(r["provider"], r["status"], r["label"]) for r in rows] == [
        ("claude", "confirmed", "Claude default"), ("claude", "detected", None), ("codex", "detected", None)
    ]
    assert str(root / ".claude.json") not in out and ".claude-junk" not in out


def test_confirm_then_list(root, capsys):
    run(capsys, "list")
    work = str(root / ".claude-work")
    rc, _, _ = run(capsys, "confirm", work, "--label", "Claude work")
    assert rc == 0
    rows = {r["folder"]: r for r in json.loads(run(capsys, "list")[1])["accounts"]}
    assert (rows[work]["status"], rows[work]["label"]) == ("confirmed", "Claude work")


def test_confirm_needs_a_good_label_and_a_known_account(root, capsys):
    run(capsys, "list")
    rc, _, err = run(capsys, "confirm", str(root / ".codex"), "--label", "x" * 61)
    assert rc == 2 and "label" in err
    rc, _, err = run(capsys, "confirm", "/no/such", "--label", "L")
    assert rc == 2 and "no such account" in err and "/no/such" not in err


def test_ignore_hides_from_redetection(root, capsys):
    run(capsys, "list")
    assert run(capsys, "ignore", str(root / ".codex"))[0] == 0
    rows = json.loads(run(capsys, "list")[1])["accounts"]
    assert [r["status"] for r in rows if r["provider"] == "codex"] == ["ignored"]


def test_add_by_path(root, tmp_path, capsys):
    good = tmp_path / "outside" / "cfg"
    (good / "projects").mkdir(parents=True)
    rc, out, _ = run(capsys, "add", str(good))
    assert rc == 0 and json.loads(out)["status"] == "detected"
    bad = tmp_path / "outside" / "empty"
    bad.mkdir()
    rc, _, err = run(capsys, "add", str(bad))
    assert rc == 2 and str(bad) not in err


def test_list_by_provider(root, capsys):
    out = json.loads(run(capsys, "list", "--by", "provider")[1])
    assert out["by"] == "provider" and {k: len(v) for k, v in out["providers"].items()} == {"claude": 2, "codex": 1}


def test_bad_by_is_a_usage_error(root):
    with pytest.raises(SystemExit):
        cli.main(["account", "list", "--by", "colour"])


# --- API -------------------------------------------------------------------------------------


@pytest.fixture
def client(tmp_path):
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://{HOST}")


def post(client, path, body, headers=None):
    return client.post(path, content=json.dumps(body), headers=headers or {**AUTH, **JSON})


def test_api_list_and_grouping(root, client):
    res = client.get("/api/accounts", headers=AUTH)
    assert res.status_code == 200 and len(res.json()["accounts"]) == 3
    by_provider = client.get("/api/accounts?by=provider", headers=AUTH).json()
    assert set(by_provider["providers"]) == {"claude", "codex"}
    assert client.get("/api/accounts?by=nope", headers=AUTH).status_code == 400


def test_api_confirm_and_ignore(root, client):
    rows = client.get("/api/accounts", headers=AUTH).json()["accounts"]
    work = next(r for r in rows if r["folder"].endswith(".claude-work"))
    codex = next(r for r in rows if r["provider"] == "codex")
    res = post(client, f"/api/accounts/{work['id']}/confirm", {"label": "Claude work"})
    assert res.status_code == 200 and res.json()["status"] == "confirmed"
    res = post(client, f"/api/accounts/{codex['id']}/ignore", {})
    assert res.json()["status"] == "ignored"
    assert post(client, "/api/accounts/999/ignore", {}).status_code == 404
    assert post(client, "/api/accounts/abc/ignore", {}).status_code == 404
    assert post(client, f"/api/accounts/{work['id']}/confirm", {"label": ""}).status_code == 400
    assert post(client, f"/api/accounts/{work['id']}/confirm", {"label": 5}).status_code == 400
    assert post(client, f"/api/accounts/{work['id']}/confirm", {}).status_code == 400


def test_api_add(root, client, tmp_path):
    good = tmp_path / "o" / "cfg"
    (good / "projects").mkdir(parents=True)
    res = post(client, "/api/accounts", {"path": str(good)})
    assert res.status_code == 201 and res.json()["folder"] == str(good.resolve())
    bad = tmp_path / "o" / "empty"
    bad.mkdir()
    res = post(client, "/api/accounts", {"path": str(bad)})
    assert res.status_code == 400 and str(bad) not in res.text
    assert post(client, "/api/accounts", {"path": 3}).status_code == 400


@pytest.mark.parametrize("path", ["/api/accounts", "/api/accounts/1/confirm", "/api/accounts/1/ignore"])
def test_cross_origin_write_is_403_and_writes_nothing(root, client, path):
    res = post(client, path, {"label": "L", "path": "/x"}, {**AUTH, **JSON, "origin": "http://evil.example"})
    assert res.status_code == 403
    res = post(client, path, {"label": "L"}, JSON)  # no token
    assert res.status_code == 403
    assert client.get("/api/accounts", headers=AUTH).json()["accounts"][-1]["status"] != "ignored"


def test_write_needs_json_content_type(root, client):
    client.get("/api/accounts", headers=AUTH)
    res = client.post("/api/accounts/1/ignore", content="{}", headers={**AUTH, "content-type": "text/plain"})
    assert res.status_code == 415


def test_cli_ignore_default_is_refused(root, capsys):
    run(capsys, "list")
    rc, _, err = run(capsys, "ignore", str(root / ".claude"))
    assert rc == 2 and "default account cannot be ignored" in err


def test_api_ignore_default_is_400(root, client):
    rows = client.get("/api/accounts", headers=AUTH).json()["accounts"]
    default = next(r for r in rows if r["label"] == "Claude default")
    res = post(client, f"/api/accounts/{default['id']}/ignore", {})
    assert res.status_code == 400 and "default account" in res.text
