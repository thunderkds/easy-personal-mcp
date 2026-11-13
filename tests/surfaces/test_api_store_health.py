"""HUN-244 (FR-091): `GET /api/store/health`, the read-only store health check, behind the dashboard guard."""

import hashlib
import json
import threading
import time

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config, store
from easy_personal.core.migrations import MIGRATIONS
from easy_personal.surfaces import api

TOKEN, PORT = "tok-244", 5544
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
URL = "/api/store/health"


@pytest.fixture(autouse=True)
def _fresh_cache():
    api._health_cache.clear()
    yield
    api._health_cache.clear()


@pytest.fixture
def client(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "secret-home-244"))
    with store.opened(config.store_path()):
        pass
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://{HOST}")


def test_route_answers_the_health_check(client):
    res = client.get(URL, headers=AUTH)
    assert res.status_code == 200
    body = res.json()
    assert body["store_file"] == "store.sqlite" and body["healthy"] is True
    assert body["user_version"] == max(n for n, _ in MIGRATIONS) and body["migration_pending"] is False
    assert body["integrity_check"]["ok"] is True and body["foreign_key_check"]["ok"] is True
    assert "events" in body["tables"] and set(body["sources"]) >= {"claude_code", "kit_trace"}


def test_route_never_names_the_store_directory(client):
    assert "secret-home-244" not in client.get(URL, headers=AUTH).text


def test_a_missing_store_is_a_200_answer_that_creates_nothing(client):
    config.store_path().unlink()
    for side in ("-wal", "-shm"):
        (config.store_path().parent / f"store.sqlite{side}").unlink(missing_ok=True)
    res = client.get(URL, headers=AUTH)
    assert res.status_code == 200 and res.json()["exists"] is False and res.json()["healthy"] is False
    assert not config.store_path().exists()


def test_route_refuses_no_token_wrong_token_wrong_host_and_foreign_origin(client):
    assert client.get(URL).status_code == 403
    assert client.get(URL, headers={api.TOKEN_HEADER: "nope"}).status_code == 403
    assert client.get(URL, headers={**AUTH, "Host": "evil.example"}).status_code == 403
    assert client.get(URL, headers={**AUTH, "Origin": "http://evil.example"}).status_code == 403
    assert client.get(URL, headers={**AUTH, "Origin": f"http://{HOST}"}).status_code == 200


def test_the_token_is_not_accepted_from_the_query(client):
    assert client.get(f"{URL}?token={TOKEN}").status_code == 403


@pytest.mark.parametrize("method", ["POST", "PUT", "PATCH", "DELETE"])
def test_only_get_is_allowed(client, method):
    assert client.request(method, URL, headers=AUTH, content="{}").status_code == 405


def test_the_route_writes_nothing(client):
    path = config.store_path()
    before = (hashlib.sha256(path.read_bytes()).hexdigest(), path.stat().st_mtime_ns, sorted(p.name for p in path.parent.iterdir()))
    for _ in range(2):
        client.get(URL, headers=AUTH)
    assert (hashlib.sha256(path.read_bytes()).hexdigest(), path.stat().st_mtime_ns, sorted(p.name for p in path.parent.iterdir())) == before


def test_the_route_does_not_migrate_a_store_one_migration_behind(client):
    import sqlite3

    latest = max(n for n, _ in MIGRATIONS)
    with sqlite3.connect(config.store_path()) as conn:
        conn.execute(f"PRAGMA user_version = {latest - 1}")
    body = client.get(URL, headers=AUTH).json()
    assert body["migration_pending"] is True and body["healthy"] is False
    with sqlite3.connect(config.store_path()) as conn:
        assert conn.execute("PRAGMA user_version").fetchone()[0] == latest - 1
    assert not list(config.store_path().parent.glob("store.sqlite.bak-v*"))
    assert json.loads(client.get(URL, headers=AUTH).text)["user_version"] == latest - 1


def _counting_check(monkeypatch, delay=0.0):
    calls = []
    real = api.store_health.check

    def check(path, *args, **kwargs):
        calls.append(1)
        time.sleep(delay)
        return real(path, *args, **kwargs)

    monkeypatch.setattr(api.store_health, "check", check)
    return calls


def test_concurrent_requests_share_one_check_and_wait_for_it(client, monkeypatch):
    calls = _counting_check(monkeypatch, delay=0.3)
    results = []

    def ask():
        results.append(client.get(URL, headers=AUTH))

    threads = [threading.Thread(target=ask) for _ in range(12)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    assert len(calls) == 1 and len(results) == 12
    assert {r.status_code for r in results} == {200} and len({r.text for r in results}) == 1


def test_an_answer_is_reused_for_five_seconds_then_checked_again(client, monkeypatch):
    calls = _counting_check(monkeypatch)
    assert api.HEALTH_CACHE_S == 5.0
    client.get(URL, headers=AUTH)
    client.get(URL, headers=AUTH)
    assert len(calls) == 1
    monkeypatch.setattr(api, "HEALTH_CACHE_S", 0.0)
    client.get(URL, headers=AUTH)
    assert len(calls) == 2


def test_head_answers_headers_only_and_never_runs_the_check(client, monkeypatch):
    calls = _counting_check(monkeypatch)
    res = client.head(URL, headers=AUTH)
    assert res.status_code == 200 and res.content == b"" and calls == []
    assert client.head(URL).status_code == 403  # the guard still applies


def test_a_cut_off_check_is_null_in_the_answer(client, monkeypatch):
    monkeypatch.setattr(api.store_health, "BUDGET_S", 0.0)
    with store.opened(config.store_path()) as conn:
        for n in range(3000):
            conn.execute(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc)"
                " VALUES ('m', 'claude_code', '/x', 1, ?, 0, '2026-01-01T00:00:00.000000Z')", (n,))
        conn.commit()
    body = client.get(URL, headers=AUTH).json()
    assert body["healthy"] is None and "integrity_check" in body["incomplete"]
