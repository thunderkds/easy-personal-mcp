"""`GET /api/reports` and `GET /api/reports/{id}`: saved investigate reports behind the dashboard token (HUN-221, FR-078)."""

import json
from datetime import UTC, datetime, timedelta

import pytest
from starlette.testclient import TestClient

from easy_personal.core import reports, rules, store
from easy_personal.sources import claude_code
from easy_personal.surfaces import api

pytestmark = pytest.mark.usefixtures("real_projects")  # these tests are not about Scratch (FR-069)

TOKEN, PORT = "tok-221", 5555
AUTH = {api.TOKEN_HEADER: TOKEN}
NOW = datetime(2026, 10, 21, 12, 0, tzinfo=UTC)
DAY = timedelta(days=1)


def _msg(n, ago, i, session):
    return {
        "type": "assistant",
        "version": "2.1.295",
        "cwd": "/work/demo",
        "sessionId": session,
        "timestamp": (NOW - ago).strftime("%Y-%m-%dT%H:%M:%S.000Z"),
        "message": {
            "id": f"msg{n}",
            "model": "m-a",
            "content": [{"type": "text", "text": "x"}],
            "usage": {"input_tokens": i, "output_tokens": 0, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0},
        },
    }


def _freeze(monkeypatch, moment):
    class Frozen(datetime):
        @classmethod
        def now(cls, tz=None):
            return moment

    monkeypatch.setattr(rules, "datetime", Frozen)


@pytest.fixture
def home(tmp_path, monkeypatch):
    """A store whose last week holds 2.5x the tokens of the week before: `tokens_jump` fires at NOW."""
    home = tmp_path / "home"
    home.mkdir()
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(home))
    root = tmp_path / "projects"
    root.mkdir()
    records = [_msg("anchor", 15 * DAY, 1, "old"), _msg(1, 10 * DAY, 4000, "prev")]
    records += [_msg(10 + k, DAY, 1000, f"now-{k}") for k in range(10)]
    (root / "s.jsonl").write_text("".join(json.dumps(r) + "\n" for r in records))
    with store.opened(home / "store.sqlite") as conn:
        claude_code.ingest(conn, root)
    return home


@pytest.fixture
def client(home):
    return TestClient(api.create_app(TOKEN, PORT), base_url=f"http://127.0.0.1:{PORT}")


def save(home, n=1, **kw):
    with store.opened(home / "store.sqlite") as conn:
        for k in range(n):
            out = reports.save(conn, f"Report {k}\nmore", [{"text": "Compact earlier"}], now=NOW + timedelta(minutes=k), **kw)
    return out


# AC 6
def test_both_routes_refuse_without_the_dashboard_token(client, home):
    save(home)
    for path in ("/api/reports", "/api/reports/1", "/api/reports/999"):
        assert client.get(path).status_code == 403
        assert client.get(path, headers={api.TOKEN_HEADER: "nope"}).status_code == 403
        assert client.get(path, headers={**AUTH, "Host": "evil.example"}).status_code == 403


def test_empty_list(client):
    res = client.get("/api/reports", headers=AUTH)
    assert res.status_code == 200 and res.json() == {"reports": []}


def test_list_is_newest_first_capped_at_50(client, home):
    save(home, n=52)
    listed = client.get("/api/reports", headers=AUTH).json()["reports"]
    assert len(listed) == 50 and listed[0]["id"] == 52 and listed[-1]["id"] == 3
    assert listed[0] == {
        "id": 52,
        "saved_utc": "2026-10-21T12:51:00Z",
        "workspace": None,
        "account": None,
        "fired_count": 2,
        "summary_first_line": "Report 51",
    }


# AC 5
def test_one_report_shows_before_and_now_per_rule(client, home, monkeypatch):
    save(home)
    _freeze(monkeypatch, NOW + 7 * DAY)
    res = client.get("/api/reports/1", headers=AUTH)
    assert res.status_code == 200
    body = res.json()
    assert body["summary"] == "Report 0\nmore" and body["fixes"] == [{"text": "Compact earlier"}]
    assert body["saved_utc"] == "2026-10-21T12:00:00Z" and body["server_version"]
    jump = next(r for r in body["rules"] if r["rule_id"] == "tokens_jump")
    assert jump["before"]["state"] == "fired" and jump["before"]["measured"] == 1.5
    assert jump["now"]["state"] == "clear" and jump["now"]["measured"] == -1.0
    assert any(r["now"]["state"] == "abstained" and r["now"]["reason"] for r in body["rules"])


def test_now_is_not_stored(client, home, monkeypatch):
    save(home)
    _freeze(monkeypatch, NOW + 7 * DAY)
    first = client.get("/api/reports/1", headers=AUTH).json()
    _freeze(monkeypatch, NOW)
    second = client.get("/api/reports/1", headers=AUTH).json()
    assert next(r for r in first["rules"] if r["rule_id"] == "tokens_jump")["now"]["state"] == "clear"
    assert next(r for r in second["rules"] if r["rule_id"] == "tokens_jump")["now"]["state"] == "fired"


@pytest.mark.parametrize("path", ["/api/reports/999", "/api/reports/0", "/api/reports/abc", "/api/reports/-1", "/api/reports/1.5", "/api/reports/" + "9" * 30, "/api/reports/%D9%A1"])
def test_unknown_or_malformed_id_is_404(client, home, path):
    save(home)
    res = client.get(path, headers=AUTH)
    assert res.status_code == 404 and res.json() == {"error": "no such report"}


def test_the_routes_only_read(client, home):
    save(home)
    for method in ("post", "put", "patch", "delete"):
        assert getattr(client, method)("/api/reports/1", headers=AUTH).status_code == 405
        assert getattr(client, method)("/api/reports", headers=AUTH).status_code == 405
    assert len(client.get("/api/reports", headers=AUTH).json()["reports"]) == 1


def test_a_store_older_than_v10_is_migrated_by_the_read(client, home):
    import sqlite3

    raw = sqlite3.connect(home / "store.sqlite")
    raw.execute("DROP TABLE investigate_reports")
    raw.execute("PRAGMA user_version = 9")
    raw.commit()
    raw.close()
    assert client.get("/api/reports", headers=AUTH).json() == {"reports": []}
