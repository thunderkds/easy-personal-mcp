"""MCP connector tools (HUN-47): read-only, configured yes/no, suggestions from metadata counters."""
import asyncio
import json
import sqlite3

import pytest

from easy_personal.connectors import catalog, suggestions
from easy_personal.core import store
from easy_personal.sources import claude_code
from easy_personal.surfaces import mcp as mcp_surface

LISTED = {"connector_catalog", "connector_status", "connector_suggestions"}
WRITE_WORDS = ("add", "remove", "set", "write", "create", "delete", "store", "update", "install")


@pytest.fixture
def app():
    return mcp_surface.create_app()


def call(app, tool):
    result = asyncio.run(app.call_tool(tool, {}))
    return result[1] if isinstance(result, tuple) else json.loads(result[0].text)


def test_tool_surface_has_no_write_or_secret_tool(app):
    tools = {t.name: t for t in asyncio.run(app.list_tools())}
    assert LISTED <= set(tools)
    for name, tool in tools.items():
        assert not any(w in name.split("_") for w in WRITE_WORDS), name
        if name in LISTED:
            assert not tool.inputSchema.get("properties"), f"{name} takes input"
    assert not any("secret" in n or "keychain" in n or "token" in n for n in tools)


def test_catalog_lists_ids_fields_and_no_values(app, monkeypatch):
    monkeypatch.setenv("SLACK_BOT_TOKEN", "PLANTED-secret-value")
    out = call(app, "connector_catalog")
    assert [c["id"] for c in out["connectors"]] == catalog.ids()
    slack = next(c for c in out["connectors"] if c["id"] == "slack")
    assert slack["provider"] and slack["category"] and isinstance(slack["claude_ai_connector"], bool)
    assert all(set(f) == {"label", "secret"} for f in slack["fields"])
    assert "PLANTED-secret-value" not in json.dumps(out)


def test_status_is_yes_no_only(app, mem_keyring, fake_claude, monkeypatch):
    mem_keyring.set_password("easy-personal/slack", "connector", "PLANTED-secret-value")
    mem_keyring.set_password("easy-personal/gmail", "connector", "PLANTED-secret-value")
    monkeypatch.setenv("FAKE_CLAUDE_GET_ONLY", "slack")  # gmail has a keychain item but no entry
    monkeypatch.setenv("FAKE_CLAUDE_GET_RC", "0")
    out = call(app, "connector_status")
    got = {c["id"]: c for c in out["connectors"]}
    assert set(got) == set(catalog.ids())
    assert all(set(c) == {"id", "configured"} and isinstance(c["configured"], bool) for c in got.values())
    assert got["slack"]["configured"] is True
    assert got["gmail"]["configured"] is False
    assert "PLANTED-secret-value" not in json.dumps(out)
    assert fake_claude.writes() == [] and mem_keyring.items.keys() == {
        ("easy-personal/slack", "connector"), ("easy-personal/gmail", "connector")}


def test_status_without_keychain_or_claude_is_not_configured(app, no_keyring, no_claude):
    assert all(not c["configured"] for c in call(app, "connector_status")["connectors"])


def _assistant(n, *blocks, text=None):
    return {"type": "assistant", "cwd": "/work/demo", "sessionId": "s1", "version": "2.1.295",
            "timestamp": f"2026-02-01T09:0{n}:00.000Z",
            "message": {"id": f"m{n}", "model": "claude-fixture-model-1", "content": list(blocks)}}  # fmt: skip


def _tool(name, **tool_input):
    return {"type": "tool_use", "id": "t", "name": name, "input": tool_input}


def _ingest(tmp_path, *records):
    root = tmp_path / "projects"
    root.mkdir(exist_ok=True)
    (root / "s.jsonl").write_text("".join(json.dumps(r) + "\n" for r in records))
    conn = store.connect(tmp_path / "store.sqlite")
    claude_code.ingest(conn, root)
    return conn


def test_gh_and_webfetch_slack_suggest_slack(tmp_path):
    conn = _ingest(
        tmp_path,
        _assistant(1, _tool("Bash", command="gh pr list --repo secret/thing")),
        _assistant(2, _tool("WebFetch", url="https://slack.com/api/chat.postMessage?token=xoxb-1")),
        _assistant(3, _tool("WebFetch", url="https://docs.slack.com/x")),
    )
    out = suggestions.suggestions(conn, configured=[])
    assert [(s["id"], s["webfetch_calls"], s["mcp_calls"]) for s in out["suggestions"]] == [("slack", 2, 0)]
    assert "ingested since" in out["note"]
    rows = conn.execute("SELECT kind, value FROM tool_signals ORDER BY kind, value").fetchall()
    assert rows == [("bash", "gh"), ("domain", "docs.slack.com"), ("domain", "slack.com")]
    dump = "\n".join(conn.iterdump())
    for leak in ("secret/thing", "xoxb-1", "chat.postMessage", "--repo"):
        assert leak not in dump


def test_mcp_tool_server_name_suggests_and_configured_is_skipped(tmp_path):
    conn = _ingest(tmp_path, _assistant(1, _tool("mcp__gmail_server__search"), _tool("mcp__other__x")))
    assert [s["id"] for s in suggestions.suggestions(conn, configured=[])["suggestions"]] == ["gmail"]
    assert suggestions.suggestions(conn, configured=["gmail"])["suggestions"] == []


def test_text_and_bash_words_alone_suggest_nothing(tmp_path):
    text = {"type": "text", "text": "please use slack.com and discord.com, mcp__slack__x"}
    conn = _ingest(tmp_path, _assistant(1, text, _tool("Bash", command="slack send hello"),
                                        _tool("Bash", command="TOKEN=abc ./run.sh")))
    assert suggestions.suggestions(conn, configured=[])["suggestions"] == []
    assert conn.execute("SELECT value FROM tool_signals").fetchall() == [("slack",)]  # env-assign word dropped


def test_odd_urls_store_nothing_and_reingest_does_not_duplicate(tmp_path):
    rec = [_assistant(1, _tool("WebFetch", url="file:///etc/passwd"), _tool("WebFetch", url="http://[bad"),
                      _tool("WebFetch", url=5), _tool("WebFetch", url="https://slack.com:8443/a"))]
    conn = _ingest(tmp_path, *rec)
    claude_code.ingest(conn, tmp_path / "projects")
    assert conn.execute("SELECT kind, value FROM tool_signals").fetchall() == [("domain", "slack.com")]


def test_additive_schema_applies_to_an_existing_store(tmp_path):
    path = tmp_path / "old.sqlite"
    old = sqlite3.connect(path)
    old.executescript(store.SCHEMA.split("CREATE TABLE IF NOT EXISTS tool_signals")[0])
    old.close()
    conn = store.connect(path)
    assert conn.execute("SELECT COUNT(*) FROM tool_signals").fetchone() == (0,)


def test_claude_ai_connector_servers_count_as_already_available(tmp_path):
    conn = _ingest(tmp_path, _assistant(
        1, _tool("mcp__claude_ai_Gmail__search_threads"), _tool("mcp__slack__post")))
    out = suggestions.suggestions(conn, configured=[])
    assert [s["id"] for s in out["suggestions"]] == ["slack"]
