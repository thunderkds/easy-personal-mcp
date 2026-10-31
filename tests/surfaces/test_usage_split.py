"""The typed / agent / unknown split is the same through MCP, CLI and API (HUN-161)."""

import asyncio
import json

from starlette.testclient import TestClient

from easy_personal.core import metrics, store
from easy_personal.sources import claude_code
from easy_personal.surfaces import api, cli
from easy_personal.surfaces import mcp as mcp_surface
from tests_helpers import TOKENS, rec, write

TOKEN, PORT = "tok-split", 5556


def _home(tmp_path, monkeypatch):
    root = write(
        tmp_path / "projects",
        "a.jsonl",
        [
            rec(1, "typed", entry="cli"),
            rec(2, "typed", entry="cli", sidechain=True),
            rec(3, "head", entry="sdk-cli", day=16),
            rec(4, "old", entry=None, day=16),
        ],
    )
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(root))
    (tmp_path / "home").mkdir()
    with store.opened(tmp_path / "home" / "store.sqlite") as conn:
        claude_code.ingest(conn, root)
    return tmp_path / "home"


def _cli(capsys, argv):
    assert cli.main(argv) == 0
    return json.loads(capsys.readouterr().out)


def test_mcp_cli_and_api_return_the_same_split(tmp_path, monkeypatch, capsys):
    home = _home(tmp_path, monkeypatch)
    static = tmp_path / "static"
    static.mkdir()
    (static / "index.html").write_text("<html></html>")
    client = TestClient(api.create_app(TOKEN, PORT, static), base_url=f"http://127.0.0.1:{PORT}")
    headers = {api.TOKEN_HEADER: TOKEN}
    app = mcp_surface.create_app()

    _, mcp_summary = asyncio.run(app.call_tool("usage_summary", {}))
    cli_summary = _cli(capsys, ["usage"])
    api_summary = client.get("/api/usage/summary", headers=headers).json()
    expected = {"typed": TOKENS, "agent": 2 * TOKENS, "unknown": TOKENS}
    assert mcp_summary["split"] == cli_summary["split"] == api_summary["split"] == expected
    assert mcp_summary == cli_summary == api_summary

    _, mcp_days = asyncio.run(app.call_tool("usage_breakdown", {"by": "day"}))
    cli_days = _cli(capsys, ["usage", "--by", "day"])
    api_days = client.get("/api/usage/breakdown?by=day", headers=headers).json()
    assert mcp_days == cli_days == api_days
    assert [g["split"] for g in mcp_days["groups"]] == [
        {"typed": TOKENS, "agent": TOKENS, "unknown": 0},
        {"typed": 0, "agent": TOKENS, "unknown": TOKENS},
    ]
    assert mcp_days["total"]["split"] == expected
    with store.opened(home / "store.sqlite") as conn:
        assert metrics.usage_summary(conn) == mcp_summary
