"""`GET /api/jobs` (HUN-163): the scheduled-jobs reader behind the token guard; no argument ever leaves."""

import pytest
from starlette.testclient import TestClient

from easy_personal.surfaces import api
from fakes import jobs_fakes
from fakes.jobs_fakes import exec_start

TOKEN, PORT = "tok-123", 5555
AUTH = {api.TOKEN_HEADER: TOKEN}
SECRET = "SECRET_X"


@pytest.fixture
def client(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("PATH", str(tmp_path / "bin"))
    jobs_fakes.install(
        tmp_path / "bin",
        timers=[{"unit": "easy-personal-ingest.timer", "activates": "easy-personal-ingest.service", "next": 1_793_444_400_000_000, "last": 0}],
        props={
            "easy-personal-ingest.service": {"ExecStart": exec_start(f"/usr/bin/easy-personal ingest --k {SECRET}"), "ExecMainStatus": "0"},
        },
        crontab=f'0 7 * * 1-5 claude -p "{SECRET}"\n',
    )
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://127.0.0.1:{PORT}")


def test_jobs_route_lists_jobs_hidden_count_and_sources(client):
    res = client.get("/api/jobs", headers=AUTH)
    assert res.status_code == 200
    body = res.json()
    assert [j["kind"] for j in body["jobs"]] == ["systemd", "cron"]
    assert body["hidden"] == 0
    assert body["sources"] == {"systemd": "ok", "cron": "ok"}
    assert set(body["jobs"][0]) == {"name", "kind", "command", "schedule", "next_run", "last_run", "last_exit"}


def test_secret_in_a_cron_line_and_in_an_exec_start_is_nowhere_in_the_response(client):
    res = client.get("/api/jobs", headers=AUTH)
    assert SECRET not in res.text
    assert "--k" not in res.text and "-p" not in res.text.replace("-personal", "")


def test_jobs_route_needs_the_token(client):
    assert client.get("/api/jobs").status_code == 403
