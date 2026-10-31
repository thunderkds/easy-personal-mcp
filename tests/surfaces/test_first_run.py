"""`GET /api/first-run` (HUN-160): four done / not-done setup steps, read-only."""

import pytest
from starlette.testclient import TestClient

from easy_personal.core import accounts, config, first_run, schedule, store, workspaces
from easy_personal.surfaces import api
from tests_helpers import add_event

TOKEN, PORT = "tok-123", 5555
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
IDS = ["first_ingest", "ingest_timer", "accounts_confirmed", "first_workspace"]


@pytest.fixture
def xdg(tmp_path, monkeypatch):
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "xdg"))
    monkeypatch.setattr(schedule, "systemd_available", lambda: True)
    return tmp_path / "xdg"


@pytest.fixture
def client(tmp_path, xdg):
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://{HOST}")


def steps(client) -> dict[str, dict]:
    res = client.get("/api/first-run", headers=AUTH)
    assert res.status_code == 200
    return {s["id"]: s for s in res.json()["steps"]}


def test_fresh_home_has_four_steps_all_not_done(client):
    res = client.get("/api/first-run", headers=AUTH)
    assert [s["id"] for s in res.json()["steps"]] == IDS
    assert [s["done"] for s in res.json()["steps"]] == [False] * 4
    assert all(isinstance(s["hint"], str) and s["hint"] for s in res.json()["steps"])


def test_hints_are_the_existing_commands(client):
    got = steps(client)
    assert got["first_ingest"]["hint"] == "easy-personal ingest"
    assert got["ingest_timer"]["hint"] == "easy-personal schedule install"


def test_everything_set_up_is_all_done(client, xdg):
    (xdg / "systemd" / "user").mkdir(parents=True)
    (xdg / "systemd" / "user" / schedule.TIMER).write_text("[Timer]\n")
    (config.agent_root() / ".claude").mkdir()
    with store.opened(config.store_path()) as conn:
        add_event(conn, 1, "/p")
        conn.commit()
        accounts.detect(conn)  # the default Claude account is confirmed on detection
        workspaces.add(conn, "acme")
    assert {k: v["done"] for k, v in steps(client).items()} == dict.fromkeys(IDS, True)


def test_detected_but_unconfirmed_accounts_do_not_count(client):
    root = config.agent_root()
    (root / ".claude-work" / "projects").mkdir(parents=True)
    with store.opened(config.store_path()) as conn:
        accounts.detect(conn)
        assert [a["status"] for a in accounts.list_accounts(conn)] == ["detected"]
    assert steps(client)["accounts_confirmed"]["done"] is False


def test_only_the_orphan_workspace_does_not_count(client):
    with store.opened(config.store_path()) as conn:
        assert [w["name"] for w in workspaces.list_all(conn)] == ["orphan"]
    assert steps(client)["first_workspace"]["done"] is False


def test_timer_not_installed_without_unit_file(client):
    assert steps(client)["ingest_timer"]["done"] is False


def test_without_systemd_the_timer_hint_is_a_cron_line_and_still_not_done(client, monkeypatch):
    monkeypatch.setattr(schedule, "systemd_available", lambda: False)
    step = steps(client)["ingest_timer"]
    assert step["done"] is False
    assert "crontab" in step["hint"] and "ingest" in step["hint"]


def test_post_is_405(client):
    assert client.post("/api/first-run", headers=AUTH).status_code == 405


def test_needs_the_token(client):
    assert client.get("/api/first-run").status_code == 403


def test_core_steps_match_the_route(tmp_path, xdg):
    with store.opened(config.store_path()) as conn:
        assert [s["id"] for s in first_run.steps(conn)] == IDS
