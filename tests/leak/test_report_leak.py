"""A secret sent to `save_report` is refused and reaches no store row, response, log or stderr (HUN-221, NFR-002, DDR-0003).

`save_report` is the one place that stores model-written text, so the secret check is the guard: every planted
secret form below is refused, then the store, the responses, the logs and the real stdio server's stderr are
scanned for the canary. Report text that is not secret-like is stored on purpose (the DDR-0003 exception).
"""

import asyncio
import sys

import pytest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from leak import harness
from leak.canaries import Canaries

# argument builders: where the canary sits. Each must be refused.
PLANTS = {
    "summary": lambda c: {"summary": f"found {c} in config", "fixes": []},
    "fix text": lambda c: {"summary": "ok", "fixes": [{"text": f"rotate {c}"}]},
    "fix tool": lambda c: {"summary": "ok", "fixes": [{"text": "a", "tool": c}]},
    "fix number": lambda c: {"summary": "ok", "fixes": [{"text": "a", "number": c}]},
    "workspace": lambda c: {"summary": "ok", "fixes": [], "workspace": c},
    "account": lambda c: {"summary": "ok", "fixes": [], "account": c},
}


@pytest.fixture
def world(tmp_path, monkeypatch):
    home = tmp_path / "home"
    projects = tmp_path / "projects"
    projects.mkdir()
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(home))
    return home, projects


def _forms(canaries):
    return {
        "api key": canaries.values["api_key"],  # sk-LEAKCANARY-<hex>
        "mcp env key": canaries.values["claude_json_env_secret"],
        "github token": "ghp_" + canaries.values["api_key"][-16:],
        "slack token": "xoxb-" + canaries.values["api_key"][-16:],
        "aws key": "AKIA" + canaries.values["api_key"][-16:].upper(),
        "private key header": "-----BEGIN PRIVATE KEY-----",
        "long token": canaries.values["api_key"][-16:] * 3,
    }


def test_a_planted_secret_is_refused_everywhere_and_stored_nowhere(world):
    home, _ = world
    canaries = Canaries.new()
    out = harness.Collected()
    app = harness.build_app()
    refused = 0
    with harness.LogCapture() as logs:
        for form, secret in _forms(canaries).items():
            for place, build in PLANTS.items():
                try:
                    asyncio.run(app.call_tool("save_report", build(secret)))
                except Exception as exc:  # noqa: BLE001 - the refusal; scan its text too
                    refused += 1
                    out.add(f"{form} in {place}", f"{exc!r} cause: {exc.__cause__!r} context: {exc.__context__!r}")
                else:
                    pytest.fail(f"{form} in {place} was accepted")
        out.add("log records", logs.text())
    assert refused == len(_forms(canaries)) * len(PLANTS)
    out.outputs.update(harness.store_blobs(home))
    assert "investigate_reports" in out.outputs["store .dump"]  # the table exists and was scanned
    for label, blob in out.outputs.items():
        for name, secret in _forms(canaries).items():
            assert secret.encode() not in (blob if isinstance(blob, bytes) else str(blob).encode()), (label, name)
        canaries.assert_clean(label, blob)
    import sqlite3

    conn = sqlite3.connect(home / "store.sqlite")
    assert conn.execute("SELECT COUNT(*) FROM investigate_reports").fetchone()[0] == 0
    conn.close()


def test_a_planted_secret_never_reaches_stdio_responses_stderr_or_the_store(world, tmp_path):
    home, projects = world
    canaries = Canaries.new()
    env = harness.child_env(home, projects)
    params = StdioServerParameters(command=sys.executable, args=["-m", "easy_personal.surfaces.cli", "mcp"], env=env)
    out = harness.Collected()

    async def drive(errlog):
        async with stdio_client(params, errlog=errlog) as (read, write), ClientSession(read, write) as session:
            await session.initialize()
            for form, secret in _forms(canaries).items():
                for place, build in PLANTS.items():
                    res = await session.call_tool("save_report", build(secret))
                    assert res.isError, (form, place)
                    out.add(f"stdio {form} in {place}", res.model_dump_json())
            res = await session.call_tool("save_report", {"summary": "A clean report.", "fixes": []})
            assert not res.isError  # the server still saves what is not secret-like
            listed = await session.list_tools()
            out.add("tools", listed.model_dump_json())

    with open(tmp_path / "stderr.log", "w") as errlog:
        asyncio.run(asyncio.wait_for(drive(errlog), 120))
    out.add("stderr", (tmp_path / "stderr.log").read_bytes())
    out.outputs.update(harness.store_blobs(home))
    for label, blob in out.outputs.items():
        canaries.assert_clean(label, blob)
        for secret in _forms(canaries).values():
            assert secret.encode() not in (blob if isinstance(blob, bytes) else str(blob).encode()), label
    assert "A clean report." in out.outputs["store .dump"]


def test_non_secret_report_text_is_the_one_text_that_is_stored(world):
    """The DDR-0003 exception, pinned: plain report text is stored; a session id (UUID) and a path are fine."""
    home, _ = world
    app = harness.build_app()
    text = "Session 6d1f3c2e-8a4b-4c7d-9e5f-0a1b2c3d4e5f in /home/owner/work/repo/src/app.py was large."
    asyncio.run(app.call_tool("save_report", {"summary": text, "fixes": [{"text": text[:200]}]}))
    assert text in harness.store_blobs(home)["store .dump"]
