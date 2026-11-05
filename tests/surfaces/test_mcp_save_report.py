"""MCP tool `save_report` and the last step of the `investigate` prompt (HUN-221, FR-076, FR-077)."""

import asyncio
import json
import os
import sqlite3
import sys

import pytest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.server.fastmcp.exceptions import ToolError

from easy_personal.core import config, store, workspaces
from easy_personal.surfaces import mcp as mcp_surface

FIXES = [{"text": "Start a new session per task", "tool": "session_outliers", "number": "645301"}, {"text": "Compact earlier"}]
SECRET = "sk-LEAKCANARY-0123456789abcdef"


@pytest.fixture
def app():
    return mcp_surface.create_app()


def call(app, **arguments):
    _, structured = asyncio.run(app.call_tool("save_report", arguments))
    return structured


def rows():
    with store.opened(config.store_path()) as conn:
        return conn.execute("SELECT id, summary, fixes_json, workspace, account FROM investigate_reports").fetchall()


# AC 2
def test_tool_is_listed_with_summary_and_fixes_required_and_a_scope_optional(app):
    tool = next(t for t in asyncio.run(app.list_tools()) if t.name == "save_report")
    assert set(tool.inputSchema["required"]) == {"summary", "fixes"}
    assert set(tool.inputSchema["properties"]) == {"summary", "fixes", "workspace", "account"}
    assert "2,000" in tool.description and "300" in tool.description and "secret" in tool.description


def test_valid_call_stores_one_row_and_returns_id_and_time(app):
    result = call(app, summary="Token use rose.\nBecause of more sessions.", fixes=FIXES)
    assert set(result) == {"id", "saved_utc"} and result["id"] == 1
    assert result["saved_utc"].endswith("Z")
    [(id_, summary, fixes, workspace, account)] = rows()
    assert (id_, summary, workspace, account) == (1, "Token use rose.\nBecause of more sessions.", None, None)
    assert json.loads(fixes) == FIXES


def test_scope_arguments_are_stored(app):
    with store.opened(config.store_path()) as conn:
        workspaces.add(conn, "acme", repos=["/work"])
    call(app, summary="Scoped.", fixes=[], workspace="acme")
    assert rows()[0][3:] == ("acme", None)


# AC 3
@pytest.mark.parametrize(
    ("arguments", "field"),
    [
        ({"summary": "x " * 1000 + "x", "fixes": []}, "summary"),
        ({"summary": "ok", "fixes": [{"text": "a"}] * 6}, "fixes"),
        ({"summary": "ok", "fixes": [{"text": "a"}, {"text": "y " * 150 + "y"}]}, "fixes[1].text"),
        ({"summary": "ok", "fixes": [], "workspace": "nope"}, "workspace"),
        ({"summary": "ok", "fixes": [], "account": "nope"}, "account"),
        ({"summary": f"key {SECRET}", "fixes": []}, "summary"),
        ({"summary": "ok", "fixes": [{"text": f"key {SECRET}"}]}, "fixes[0].text"),
        ({"summary": "ok", "fixes": [{"text": "a", "tool": SECRET}]}, "fixes[0].tool"),
        ({"summary": "ok", "fixes": [{"text": "a", "number": SECRET}]}, "fixes[0].number"),
        ({"summary": "-----BEGIN PRIVATE KEY-----", "fixes": []}, "summary"),
    ],
)
def test_refusal_is_an_error_that_names_the_field_and_stores_nothing(app, arguments, field):
    with pytest.raises(ToolError) as caught:
        call(app, **arguments)
    assert field + ":" in str(caught.value) and "nothing was saved" in str(caught.value)
    assert SECRET not in str(caught.value) and SECRET not in repr(caught.value.__cause__)
    assert rows() == []


def test_a_uuid_and_an_absolute_path_are_accepted(app):
    text = "Session 6d1f3c2e-8a4b-4c7d-9e5f-0a1b2c3d4e5f in /home/owner/work/repo/src/app.py"
    assert call(app, summary=text, fixes=[{"text": text}])["id"] == 1


def test_a_second_save_makes_a_second_report(app):
    assert [call(app, summary="one", fixes=[])["id"], call(app, summary="one", fixes=[])["id"]] == [1, 2]


def test_a_store_older_than_v10_is_migrated_on_the_first_save(app):
    path = config.store_path()
    store.connect(path).close()
    raw = sqlite3.connect(path)
    raw.execute("DROP TABLE investigate_reports")
    raw.execute("PRAGMA user_version = 9")
    raw.commit()
    raw.close()
    assert call(app, summary="after upgrade", fixes=[])["id"] == 1
    assert (path.parent / "store.sqlite.bak-v9").exists()


# real stdio server: tool list, save, refusal, prompt text
def test_stdio_server_saves_refuses_and_serves_the_prompt(tmp_path):
    env = {**os.environ, "EASY_PERSONAL_HOME": str(tmp_path / "stdio-home")}
    params = StdioServerParameters(command=sys.executable, args=["-m", "easy_personal.surfaces.cli", "mcp"], env=env)
    seen = {}

    async def drive(errlog):
        async with stdio_client(params, errlog=errlog) as (read, write), ClientSession(read, write) as session:
            await session.initialize()
            seen["tools"] = [t.name for t in (await session.list_tools()).tools]
            ok = await session.call_tool("save_report", {"summary": "Stdio report.", "fixes": FIXES})
            seen["ok"] = (ok.isError, json.loads(ok.content[0].text))
            bad = await session.call_tool("save_report", {"summary": f"key {SECRET}", "fixes": []})
            seen["bad"] = (bad.isError, bad.content[0].text)
            seen["prompt"] = (await session.get_prompt("investigate", {})).messages[0].content.text

    with open(tmp_path / "stderr.log", "w") as errlog:
        asyncio.run(asyncio.wait_for(drive(errlog), 60))
    assert "save_report" in seen["tools"]
    assert seen["ok"][0] is False and seen["ok"][1]["id"] == 1
    assert seen["bad"][0] is True and "summary:" in seen["bad"][1] and SECRET not in seen["bad"][1]
    assert seen["prompt"].rstrip().endswith("do not retry.")
    raw = sqlite3.connect(tmp_path / "stdio-home" / "store.sqlite")
    assert raw.execute("SELECT COUNT(*) FROM investigate_reports").fetchone()[0] == 1
    raw.close()
    assert SECRET not in (tmp_path / "stderr.log").read_text()


# AC 7
def prompt(app, **arguments):
    return asyncio.run(app.get_prompt("investigate", arguments or None)).messages[0].content.text


def test_prompt_ends_with_the_save_step(app):
    text = prompt(app)
    last = text.rstrip().splitlines()[-1]
    assert last.startswith("7. ")
    assert text.endswith(mcp_surface.SAVE_REPORT_STEP)
    assert "`save_report` once" in last
    assert "report id" in last
    assert "at most 5" in last and "tool and number it cites" in last
    assert "same scope" in last
    assert "not available" in last and "reconnect" in last and "/mcp" in last


def test_prompt_with_a_scope_still_ends_with_the_step_and_passes_the_scope_to_save_report(app):
    text = prompt(app, workspace="Acme", account="work")
    assert text.endswith(mcp_surface.SAVE_REPORT_STEP)
    scope_line = next(line for line in text.splitlines() if line.startswith("Scope:"))
    assert '"Acme"' in scope_line and '"work"' in scope_line and "`save_report`" in scope_line
    assert text.index(scope_line) < text.index("7. ")


def test_old_steps_are_unchanged(app):
    text = prompt(app)
    for n in range(1, 7):
        assert f"\n{n}. " in text
    assert text.startswith(mcp_surface.INVESTIGATE_PROMPT)
