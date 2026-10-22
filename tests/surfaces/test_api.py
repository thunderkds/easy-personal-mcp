import asyncio
import json

import pytest
from starlette.testclient import TestClient

from easy_personal.core import metrics, store
from easy_personal.sources import claude_code
from easy_personal.surfaces import api, cli

TOKEN, PORT = "tok-123", 5555
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}


@pytest.fixture
def home(tmp_path, monkeypatch, fixtures_root):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(fixtures_root))
    (tmp_path / "home").mkdir()
    with store.opened(tmp_path / "home" / "store.sqlite") as conn:
        claude_code.ingest(conn, fixtures_root)
    return tmp_path / "home"


@pytest.fixture
def static(tmp_path):
    d = tmp_path / "static"
    d.mkdir()
    (d / "index.html").write_text("<html>app</html>")
    return d


@pytest.fixture
def client(home, static):
    return TestClient(api.create_app(TOKEN, PORT, static), base_url=f"http://{HOST}")


def test_summary_with_token_ok(client):
    res = client.get("/api/usage/summary", headers=AUTH)
    assert res.status_code == 200
    assert res.json()["sessions"] == 4


def test_missing_or_wrong_token_is_403(client):
    assert client.get("/api/usage/summary").status_code == 403
    assert client.get("/api/usage/summary", headers={api.TOKEN_HEADER: "nope"}).status_code == 403


def test_dns_rebinding_host_is_403(client):
    res = client.get("/api/usage/summary", headers={**AUTH, "Host": "evil.example"})
    assert res.status_code == 403
    res = client.get("/api/usage/summary", headers={**AUTH, "Host": f"evil.example:{PORT}"})
    assert res.status_code == 403


def test_other_origin_is_403(client):
    res = client.get("/api/usage/summary", headers={**AUTH, "Origin": "http://evil.example"})
    assert res.status_code == 403
    ok = client.get("/api/usage/summary", headers={**AUTH, "Origin": f"http://{HOST}"})
    assert ok.status_code == 200


def test_page_checks_host_but_needs_no_token(client):
    assert client.get("/").status_code == 200
    assert client.get("/", headers={"Host": "evil.example"}).status_code == 403


def test_csp_on_every_response(client):
    for res in (
        client.get("/"),
        client.get("/missing.js"),
        client.get("/api/usage/summary"),  # 403
        client.get("/api/usage/summary", headers=AUTH),
        client.get("/", headers={"Host": "evil.example"}),
    ):
        assert (
            res.headers["content-security-policy"] == "default-src 'self'; frame-ancestors 'none'"
        )
        assert "frame-ancestors 'none'" in res.headers["content-security-policy"]


def test_no_static_dir_still_serves_api(home, tmp_path):
    app = api.create_app(TOKEN, PORT, tmp_path / "absent")
    c = TestClient(app, base_url=f"http://{HOST}")
    assert c.get("/api/usage/summary", headers=AUTH).status_code == 200


def test_bad_input_is_400(client):
    assert client.get("/api/usage/breakdown?by=bad", headers=AUTH).status_code == 400
    assert client.get("/api/usage/summary?since=notadate", headers=AUTH).status_code == 400


def test_parity_with_core_and_mcp_numbers(client, home):
    from easy_personal.surfaces import mcp as mcp_surface

    app = mcp_surface.create_app()
    with store.opened(home / "store.sqlite") as conn:
        want = {
            "/api/usage/summary": metrics.usage_summary(conn),
            "/api/usage/breakdown?by=model": metrics.usage_breakdown(conn, "model"),
            "/api/usage/outliers": metrics.session_outliers(conn),
        }
    for path, expected in want.items():
        assert client.get(path, headers=AUTH).json() == expected, path
    _, mcp_summary = asyncio.run(app.call_tool("usage_summary", {}))
    assert client.get("/api/usage/summary", headers=AUTH).json() == mcp_summary


def test_warnings_and_plan_routes(client):
    assert "abstained" in client.get("/api/warnings", headers=AUTH).json()
    assert client.get("/api/plan", headers=AUTH).status_code == 200


def test_bind_free_port_is_loopback():
    sock = api.bind_free_port()
    try:
        host, port = sock.getsockname()
        assert host == "127.0.0.1" and port > 0
    finally:
        sock.close()


def test_serve_prints_url_opens_browser_and_hides_token_from_logs(monkeypatch, capsys):
    import uvicorn

    seen = {}

    def fake_run(self, sockets=None):
        seen["config"] = self.config
        seen["sockets"] = sockets

    monkeypatch.setattr(uvicorn.Server, "run", fake_run)
    opened = []
    api.serve(open_browser=opened.append)
    url = opened[0]
    assert url.startswith("http://127.0.0.1:") and "/#" in url
    assert url in capsys.readouterr().out
    assert seen["config"].access_log is False and seen["config"].log_config is None
    assert seen["sockets"][0].getsockname()[0] == "127.0.0.1"
    seen["sockets"][0].close()


def test_serve_survives_headless_opener_failure(monkeypatch, capsys):
    import uvicorn

    monkeypatch.setattr(uvicorn.Server, "run", lambda self, sockets=None: sockets[0].close())

    def boom(url):
        raise RuntimeError("no display")

    api.serve(open_browser=boom)
    assert "dashboard: http://127.0.0.1:" in capsys.readouterr().out


def test_cli_dashboard_ingests_then_serves(home, monkeypatch, capsys):
    calls = []
    monkeypatch.setattr(api, "serve", lambda: calls.append("serve"))
    assert cli.main(["dashboard"]) == 0
    assert calls == ["serve"]
    assert json.loads(capsys.readouterr().out)["new_events"] == 0


def test_cli_dashboard_lock_busy_does_not_serve(home, monkeypatch, capsys):
    from easy_personal.core import lock

    monkeypatch.setenv("EASY_PERSONAL_LOCK_TIMEOUT", "0.1")
    calls = []
    monkeypatch.setattr(api, "serve", lambda: calls.append("serve"))
    with lock.ingest_lock(home / "ingest.lock", 1):
        # flock is per open file description, so a second open in this process is blocked
        assert cli.main(["dashboard"]) == 75
    assert calls == []


def test_websocket_connect_is_refused(client):
    from starlette.websockets import WebSocketDisconnect

    for path in ("/ws", "/api/usage/summary"):
        with pytest.raises(WebSocketDisconnect), client.websocket_connect(path, headers=AUTH):
            pass
