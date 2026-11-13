"""Run every surface against a canary-planted store and collect what each one emitted.

Registries (`MCP_CALLS`, `CLI_CALLS`) list every current MCP tool and CLI command with the
arguments to call it by. A meta-test fails when the product has a tool or command that is not
registered, so the suite cannot silently fall behind. See tests/leak/README.md.
"""

import asyncio
import contextlib
import io
import json
import logging
import os
import sqlite3
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Self

import pytest

from easy_personal.surfaces import cli
from easy_personal.surfaces import mcp as mcp_surface

SRC = str(Path(__file__).resolve().parents[2] / "src")

# MCP tool name -> list of argument dicts (the first is the default call, no arguments).
MCP_CALLS: dict[str, list[dict]] = {
    "ping": [{}],
    "usage_summary": [{}, {"since": "2026-01-01"}, {"until": "2027-01-01"}],
    "usage_breakdown": [
        {"by": "day"},
        {"by": "model"},
        {"by": "project"},
        {"by": "tool"},
        {"by": "day", "since": "2026-01-01"},
    ],
    "session_outliers": [{}, {"since": "2026-01-01"}],
    "warnings": [{}, {"workspace": "acme"}, {"account": "1"}, {"workspace": "acme", "account": "1"}],
    "improvement_plan": [{}, {"workspace": "acme"}, {"account": "1"}],
    "kit_summary": [{}, {"workspace": "acme"}, {"account": "1"}, {"workspace": "acme", "account": "1"}],
    "kit_findings": [{}, {"limit": 10}, {"limit": 99, "since": "2026-01-01"}],
    "connector_catalog": [{}],
    "connector_status": [{}],
    "connector_suggestions": [{}],
    # HUN-221: the only MCP tool that writes. Valid calls store a report (its text is the DDR-0003 exception); the
    # refused ones must store nothing. A planted secret sent to it is in tests/leak/test_report_leak.py.
    "save_report": [
        {"summary": "Leak suite report.\nSecond line.", "fixes": [{"text": "Start a new session per task", "tool": "session_outliers", "number": "645301"}]},
        {"summary": "Scoped report.", "fixes": [], "workspace": "acme"},
        {"summary": "Scoped report.", "fixes": [], "account": "1"},
        {"summary": "Unknown scope.", "fixes": [], "workspace": "nope"},
        {"summary": "x" * 2001, "fixes": []},
        {"summary": "Fix too long.", "fixes": [{"text": "y" * 301}]},
    ],
}
# CLI command -> list of argv tails. `{projects}` is the planted transcript root.
CLI_CALLS: dict[str, list[list[str]]] = {
    "ping": [[]],
    "connector": [["list"]],  # reads the catalog only; add/remove/adopt-legacy/exec/headers use the keychain (tests/leak/test_connector_leak.py)
    "ingest": [[], ["--source", "{projects}"], ["--retention-days", "3650"]],
    "schedule": [["install", "--dry-run"], ["remove", "--dry-run"]],
    # sequence matters: `acme` is created here and filtered on by `usage --workspace` below (HUN-64)
    "workspace": [
        ["list"],
        ["add", "acme", "--color", "#336699", "--repo", "{projects}", "--repo", "/work/leak-demo"],
        ["add", "scratch"],
        ["edit", "acme", "--description", "company work", "--add-repo", "/work/other"],
        ["edit", "scratch", "--rename", "scratch2"],
        ["delete", "scratch2", "--yes"],
        ["list"],
    ],
    "usage": [
        [],
        ["--workspace", "acme"],
        ["--workspace", "acme", "--by", "project"],
        ["--account", "1"],
        ["--account", "1", "--by", "day"],
        ["--account", "1", "--workspace", "acme"],
        ["--since", "2026-01-01"],
        ["--until", "2027-01-01"],
        ["--by", "day"],
        ["--by", "model"],
        ["--by", "project"],
        ["--by", "tool"],
    ],
    "outliers": [[], ["--since", "2026-01-01"], ["--workspace", "acme"], ["--account", "1"]],
    "warnings": [[], ["--workspace", "acme"], ["--account", "1"], ["--account", "1", "--workspace", "acme"]],
    "plan": [[], ["--workspace", "acme"], ["--account", "1"]],
    "kit": [[], ["--workspace", "acme"], ["--account", "1"], ["--account", "1", "--workspace", "acme"]],
    "kit-findings": [[], ["--json"], ["--json", "--limit", "10", "--since", "2026-01-01"]],
    "store-check": [[], ["--json"]],  # HUN-244: read-only; counts, versions, sizes, times, backup names (planted names: test_store_check_leak.py)
    "account": [
        ["list"], ["list", "--by", "provider"], ["list", "--by", "account"],
        # HUN-65: id 1 is the fake agent root's `.claude-work`; `acme` comes from "workspace" above.
        # `connector sync` is not run here (real PATH): tests/leak/test_connector_leak.py covers it.
        ["confirm", "1", "--label", "Claude work"], ["use", "1", "acme"], ["use", "1"], ["use", "1", "acme"],
    ],
}
# Commands that start a long-running server; covered by the stdio test instead of a one-shot call.
# `dashboard` is covered through API_CALLS (in-process TestClient) instead.
CLI_SERVER_COMMANDS = {"mcp", "dashboard"}
# Dashboard JSON API route path -> list of query strings (HUN-49).
API_CALLS: dict[str, list[str]] = {
    "/api/usage/summary": ["", "?since=2026-01-01", "?until=2027-01-01", "?workspace=acme", "?workspace=nope",
        "?account=1", "?account=nope", "?account=1&workspace=acme",
    ],
    "/api/usage/breakdown": [
        "?by=day", "?by=model", "?by=project", "?by=tool", "?by=bad", "?by=project&workspace=acme",
        "?by=day&account=1", "?by=day&account=nope",
    ],
    "/api/usage/outliers": ["", "?since=2026-01-01", "?workspace=acme", "?account=1", "?account=nope"],
    "/api/usage/sessions": ["", "?since=2026-01-01", "?until=2027-01-01", "?workspace=acme", "?account=1", "?account=nope"],
    "/api/workspaces": [""],
    "/api/workspaces/unassigned": [""],  # HUN-206: repo paths, counts and cost only
    "/api/reports": [""],  # HUN-221: the newest saved reports (the one table with model-written text); each id is fetched via API_WRITE_CALLS below
    "/api/warnings": ["", "?workspace=acme", "?account=1", "?account=nope", "?workspace=nope"],
    "/api/plan": ["", "?workspace=acme", "?account=1", "?account=nope"],
    "/api/kit": ["", "?workspace=acme", "?account=1", "?account=nope", "?workspace=nope"],
    "/api/kit/workspaces": ["", "?since=2026-01-01", "?until=2027-01-01", "?account=1", "?account=nope"],  # HUN-238: folder names, scores and counts only
    "/api/kit/reports": [""],  # HUN-240: repo folder name, report folder name, counts and a time only (planted names: test_kit_reports_leak.py)
    "/api/kit/report-files": ["?workspace=acme&repo_no=0&folder=reports", "?workspace=nope&repo_no=0&folder=reports"],  # HUN-242: relative paths inside one report folder; planted names: test_kit_report_files_leak.py
    "/api/kit/report-file": ["?workspace=acme&repo_no=0&folder=reports&path=a.html", "?workspace=acme&repo_no=0&folder=reports&path=..%2Fx"],  # HUN-242: bytes of one file, fixed 404 bodies
    "/api/accounts": ["", "?by=provider", "?by=account", "?by=bad"],
    "/api/board": ["", "?workspace=acme", "?account=1", "?account=nope", "?workspace=nope"],  # HUN-162: titles are shown, never stored
    "/api/jobs": [""],  # HUN-163: fake systemctl/crontab with a planted canary in arguments (see leak conftest)
    "/api/memory-graph": ["", "?project=%2Fnope", "?project="],  # HUN-164; each listed project is also called by collect_api
    "/api/live": ["", "?token=leak-test-token"],  # HUN-177: SSE stream, read by `read_live_stream` over a tail that has read the planted transcripts
    "/api/dirs": ["", "?path=%2F", "?path=%2Fnope", "?path=relative"],  # HUN-203: folder names only; planted trees are listed in collect_api
    "/api/store/health": [""],  # HUN-244: read-only store check; no row content, no absolute path (planted names: test_store_check_leak.py)
    "/api/first-run": [""],  # HUN-160: four done / not-done flags and fixed hints
    "/api/connectors/legacy": [""],  # HUN-67: the failing keyring answers 503 with the guidance text
}
# Dashboard write routes (HUN-52): route path -> [(method, concrete path, body)]. `{secret}` in a
# body becomes the planted API-key canary. Run against an in-memory keyring and a fake `claude`.
API_WRITE_CALLS: dict[str, list[tuple[str, str, dict]]] = {
    "/api/connectors": [
        ("POST", "/api/connectors", {"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": "{secret}"}, "confirm": False}),
        ("POST", "/api/connectors", {"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": "{secret}"}, "confirm": True}),
        ("POST", "/api/connectors", {"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": "{secret}"}, "confirm": True}),
        ("POST", "/api/connectors", {"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": "{secret}"}, "confirm": "{secret}"}),
        ("POST", "/api/connectors", {"provider": "slack", "values": {"SLACK_MCP_XOXB_TOKEN": "{secret}"}, "confirm": True}),
        ("POST", "/api/connectors", {"provider": "gmail", "workspace": "{secret}", "values": {}, "confirm": True}),
        ("POST", "/api/connectors", {"provider": "gmail", "workspace": "acme", "alias": "{secret}", "values": {}, "confirm": True}),
        ("POST", "/api/connectors", {"provider": "{secret}", "workspace": "acme", "values": {}, "confirm": True}),
        ("GET", "/api/connectors", {}),
    ],
    # HUN-67: in-memory keyring, fake `claude` (get -> not found): nothing legacy, so adoption is a no-op
    "/api/connectors/legacy": [
        ("GET", "/api/connectors/legacy", {}),
        ("POST", "/api/connectors/legacy", {"confirm": False}),
        ("POST", "/api/connectors/legacy", {"confirm": True}),
        ("POST", "/api/connectors/legacy", {"confirm": "{secret}"}),
    ],
    "/api/accounts": [
        ("POST", "/api/accounts", {"path": "/no/such/{secret}"}),
        ("POST", "/api/accounts", {"path": "{secret}"}),
        ("POST", "/api/accounts", {"path": 7}),
    ],
    "/api/workspaces": [
        ("POST", "/api/workspaces", {"name": "api-ws", "repos": ["/work/leak-demo"], "email": "owner@example.test"}),
        ("POST", "/api/workspaces", {"name": "api-ws"}),
        ("POST", "/api/workspaces", {"name": "{secret}", "description": "{secret}", "repos": ["{secret}"]}),
        ("POST", "/api/workspaces", {"name": "orphan"}),
        ("GET", "/api/workspaces", {}),
    ],
    "/api/workspaces/{id}": [
        ("PATCH", "/api/workspaces/3", {"name": "api-ws2", "color": "#010203"}),
        ("PATCH", "/api/workspaces/3", {"name": "{secret}", "repos": ["{secret}"]}),
        ("PATCH", "/api/workspaces/1", {"name": "x"}),
        ("PATCH", "/api/workspaces/3", {"name": "api-ws3", "confirm": "{secret}"}),
        ("DELETE", "/api/workspaces/1", {}),
        ("DELETE", "/api/workspaces/3", {"confirm": False}),
        ("DELETE", "/api/workspaces/3", {"confirm": True}),
        ("DELETE", "/api/workspaces/{secret}", {}),
    ],
    # HUN-66: no stored connector id is known up front, so these answer 400 (refused) without echoing input;
    # the successful move / alias / rename / delete runs are scanned in tests/leak/test_connector_leak.py.
    "/api/connectors/{id}/move": [
        ("POST", "/api/connectors/nope/move", {"workspace": "acme", "confirm": True}),
        ("POST", "/api/connectors/{secret}/move", {"workspace": "{secret}", "confirm": True}),
        ("POST", "/api/connectors/nope/move", {"workspace": 7}),
        ("POST", "/api/connectors/nope/move", {"confirm": "{secret}"}),
    ],
    "/api/connectors/{id}/alias": [
        ("PUT", "/api/connectors/nope/alias", {"alias": "{secret}", "confirm": True}),
        ("PUT", "/api/connectors/nope/alias", {"alias": None, "confirm": False}),
        ("PUT", "/api/connectors/nope/alias", {"alias": 7}),
    ],
    "/api/connectors/{name}": [
        ("DELETE", "/api/connectors/slack", {"confirm": False}),
        ("DELETE", "/api/connectors/slack", {"confirm": True}),
        ("DELETE", "/api/connectors/{secret}", {"confirm": True}),
    ],
    # HUN-62: ids 1/2 are the fake agent root's `.claude-work` and `.codex` (see leak_run); labels and
    # paths carrying the secret must be refused without echo.
    "/api/accounts/{id}/confirm": [
        ("POST", "/api/accounts/1/confirm", {"label": "Claude work"}),
        ("POST", "/api/accounts/1/confirm", {"label": "{secret}{secret}{secret}"}),
        ("POST", "/api/accounts/999/confirm", {"label": "{secret}"}),
    ],
    "/api/accounts/{id}/ignore": [
        ("POST", "/api/accounts/2/ignore", {}),
        ("POST", "/api/accounts/{secret}/ignore", {}),
    ],
    # HUN-65: account 1 picks `acme`, which holds the slack connector added above; sync writes through
    # the fake `claude` only. Codex (2), orphan and planted names are refused without echo.
    "/api/accounts/{id}/workspaces": [
        ("PUT", "/api/accounts/1/workspaces", {"workspaces": ["acme"]}),
        ("PUT", "/api/accounts/1/workspaces", {"workspaces": ["{secret}"]}),
        ("PUT", "/api/accounts/1/workspaces", {"workspaces": ["orphan"]}),
        ("PUT", "/api/accounts/2/workspaces", {"workspaces": ["acme"]}),
        ("PUT", "/api/accounts/{secret}/workspaces", {"workspaces": []}),
    ],
    "/api/sync": [
        ("GET", "/api/sync", {}),
        ("POST", "/api/sync", {"confirm": False}),
        ("POST", "/api/sync", {"confirm": True}),
        ("POST", "/api/sync", {"account": "{secret}", "confirm": True}),
        ("POST", "/api/sync", {"account": 1, "confirm": "{secret}"}),
        ("GET", "/api/sync", {}),
    ],
}  # fmt: skip
API_WRITE_CALLS["/api/reports/{id}"] = [  # HUN-221: GETs; ids 1..3 are saved by the `save_report` calls above
    ("GET", "/api/reports/1", {}),
    ("GET", "/api/reports/2", {}),
    ("GET", "/api/reports/3", {}),
    ("GET", "/api/reports/99999", {}),
    ("GET", "/api/reports/{secret}", {}),
]
_FAKE_CLAUDE = """#!{python}
import json, os, sys
open(os.environ["FAKE_CLAUDE_LOG"], "a").write(json.dumps(sys.argv[1:]) + "\\n")
if sys.argv[2:3] == ["get"]:
    sys.exit(int(os.environ.get("FAKE_CLAUDE_GET_RC", "1")))
sys.exit(0)
"""
# Keychain launchers and connector commands: tested with an in-memory backend in tests/connectors (never the real keychain).
CLI_KEYCHAIN_COMMANDS = {"exec", "headers", "connector", "doctor"}


@dataclass
class Collected:
    """Everything the surfaces emitted: label -> text/object."""

    outputs: dict[str, object] = field(default_factory=dict)

    def add(self, label: str, blob) -> None:
        key, n = label, 2
        while key in self.outputs:
            key, n = f"{label}#{n}", n + 1
        self.outputs[key] = blob


def cli_commands() -> set[str]:
    sub = next(a for a in cli.build_parser()._actions if a.dest == "command")
    return set(sub.choices)


def unregistered_tools(app) -> set[str]:
    names = {t.name for t in asyncio.run(app.list_tools())}
    return names - set(MCP_CALLS)


def unregistered_commands() -> set[str]:
    return cli_commands() - set(CLI_CALLS) - CLI_SERVER_COMMANDS - CLI_KEYCHAIN_COMMANDS


def child_env(home: Path, projects: Path, ui_reports: Path | None = None) -> dict[str, str]:
    env = dict(os.environ)
    env.update(
        EASY_PERSONAL_HOME=str(home),
        EASY_MEMORY_HOME=str(home.parent / "easy-memory"),  # never the owner's ~/.easy-memory
        EASY_PERSONAL_CLAUDE_PROJECTS=str(projects),
        PYTHONPATH=SRC,
        EASY_PERSONAL_VERIFIER_REPORTS=str(home / "no-verifier-container"),  # never the owner's
        EASY_PERSONAL_AGENT_ROOT=str(home.parent / "agent-root"),  # never the owner's ~/.claude*
        PYTHON_KEYRING_BACKEND="keyring.backends.fail.Keyring",  # never the owner's keychain
    )
    if ui_reports is not None:
        env["EASY_PERSONAL_UI_REPORTS"] = str(ui_reports)
    return env


def run_cli(env: dict, argv: list[str]) -> dict:
    """Run the real CLI in a subprocess; capture exactly what a user would see."""
    done = subprocess.run(
        [sys.executable, "-m", "easy_personal.surfaces.cli", *argv],
        env=env, capture_output=True, timeout=120, check=False,
    )  # fmt: skip
    return {"argv": argv, "rc": done.returncode, "stdout": done.stdout, "stderr": done.stderr}


def collect_cli(env: dict, projects: Path, out: Collected) -> None:
    for command, variants in CLI_CALLS.items():
        for tail in variants:
            argv = [command, *(a.format(projects=projects) for a in tail)]
            result = run_cli(env, argv)
            assert result["rc"] == 0, result
            out.add("cli " + " ".join(argv), result)


def collect_mcp(app, out: Collected, logs: "LogCapture") -> None:
    """Call every registered tool in-process; keep the full result object, stdout and stderr."""
    for tool, variants in MCP_CALLS.items():
        for args in variants:
            sink_out, sink_err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(sink_out), contextlib.redirect_stderr(sink_err):
                try:
                    result = asyncio.run(app.call_tool(tool, args))
                except Exception as exc:  # noqa: BLE001 - e.g. a scope argument naming a workspace this store lacks; scan the error text too
                    result = f"tool error: {exc!r} cause: {exc.__cause__!r}"
            out.add(f"mcp {tool} {args}", result)
            out.add(f"mcp {tool} {args} stdout", sink_out.getvalue())
            out.add(f"mcp {tool} {args} stderr", sink_err.getvalue())
    out.add("log records (in-process)", logs.text())


def _collect_live(out: Collected, app, token: str) -> None:
    """HUN-177: a tail that read the planted transcripts from a fresh store, streamed (plain, by query token, resumed)."""
    import tempfile

    from easy_personal.core import live
    from easy_personal.surfaces import api

    mp = pytest.MonkeyPatch()
    with tempfile.TemporaryDirectory() as home:
        mp.setenv("EASY_PERSONAL_HOME", home)  # empty offsets: the tail sees every planted line as new
        mp.setattr(live, "FORGET_AFTER_S", 10**9)  # the planted transcripts are old: keep their sessions ...
        tail = live.LiveTail()
        tail.poll_once()
        mp.undo()
    mp.setattr(live, "_current", tail)
    mp.setattr(live, "ACTIVE_WINDOW_S", 10**9)  # ... and count them as active, so the snapshot lists sessions
    mp.setattr(api, "PING_S", 0.05)  # a short buffer still reaches the chunk count: pings fill it
    try:
        for query in API_CALLS["/api/live"]:
            out.add(f"api /api/live{query}", read_live_stream(app, "/api/live" + query, token, chunks=6))
        out.add("api /api/live resumed", read_live_stream(app, "/api/live", token, resume="0", chunks=8))
    finally:
        mp.undo()
    out.add("api /api/live tail buffer", repr((tail.events_since(0), tail.snapshot())))


def api_routes(app) -> set[str]:
    return {r.path for r in app.routes if r.path.startswith("/api/")}


def unregistered_api_routes(app) -> set[str]:
    return api_routes(app) - set(API_CALLS) - set(API_WRITE_CALLS)


def read_live_stream(app, path: str, token: str, resume: str = "", chunks: int = 3) -> str:
    """GET the never-ending `/api/live` stream in-process, drop the connection after `chunks` messages; return the text."""
    route, _, query = path.partition("?")
    headers = [(b"host", b"127.0.0.1:4321"), (b"x-dashboard-token", token.encode())]
    if resume:
        headers.append((b"last-event-id", resume.encode()))
    scope = {"type": "http", "asgi": {"version": "3.0", "spec_version": "2.3"}, "http_version": "1.1", "method": "GET",
             "scheme": "http", "path": route, "raw_path": route.encode(), "query_string": query.encode(),
             "headers": headers, "client": ("127.0.0.1", 1), "server": ("127.0.0.1", 4321)}  # fmt: skip
    text = []

    async def run() -> None:
        gone, first = asyncio.Event(), True

        async def receive():
            nonlocal first
            if first:
                first = False
                return {"type": "http.request", "body": b"", "more_body": False}
            await gone.wait()
            return {"type": "http.disconnect"}

        async def send(message):
            if message["type"] == "http.response.start":
                text.append(f"{message['status']} {message['headers']}\n")
            elif message["type"] == "http.response.body":
                text.append(message.get("body", b"").decode())
                if "".join(text).count("\n\n") >= chunks:
                    gone.set()

        await app(scope, receive, send)

    asyncio.run(asyncio.wait_for(run(), 10))
    return "".join(text)


def collect_api(out: Collected, logs: "LogCapture", secret: str = "", workdir: Path | None = None) -> None:
    """Call every dashboard API route in-process with a valid token; keep status, headers, body.

    Write routes run against an in-memory keyring and a fake `claude` (never the real ones);
    the claude argv log is collected too. They run only when `secret` and `workdir` are given.
    """
    import keyring
    from starlette.testclient import TestClient

    from connectors.fake_backend import MemoryKeyring
    from easy_personal.connectors import writer
    from easy_personal.surfaces import api

    token, port = "leak-test-token", 4321
    client = TestClient(api.create_app(token, port), base_url=f"http://127.0.0.1:{port}")
    headers = {api.TOKEN_HEADER: token}
    for path, queries in API_CALLS.items():
        if path == "/api/live":
            continue  # a stream: read below
        for query in queries:
            res = client.get(path + query, headers=headers)
            out.add(f"api {path}{query}", f"{res.status_code} {dict(res.headers)} {res.text}")
    _collect_live(out, client.app, token)
    if workdir is not None:  # HUN-203: list the planted trees (transcripts, repo with memory notes, easy-memory store): their file names and bodies hold canaries
        for target in (workdir.parent, workdir.parent / "projects", workdir.parent / "repo", workdir.parent / "repo" / "memory", workdir.parent / "easy-memory"):
            res = client.get("/api/dirs", params={"path": str(target)}, headers=headers)
            out.add(f"api /api/dirs planted {target.name}", f"{res.status_code} {dict(res.headers)} {res.text}")
    listed = client.get("/api/memory-graph", headers=headers).json().get("projects", [])
    for project in listed:  # HUN-164: the graph of every project with memory notes
        res = client.get("/api/memory-graph", params={"project": project["project"]}, headers=headers)
        out.add(f"api /api/memory-graph project={project['name']}", f"{res.status_code} {dict(res.headers)} {res.text}")
    if secret and workdir is not None:
        bindir = workdir / "fake-bin"
        bindir.mkdir(parents=True, exist_ok=True)
        exe = bindir / "claude"
        exe.write_text(_FAKE_CLAUDE.format(python=sys.executable))
        exe.chmod(0o755)
        log = workdir / "claude-argv.log"
        mp = pytest.MonkeyPatch()
        mp.setenv("PATH", str(bindir))
        mp.setenv("FAKE_CLAUDE_LOG", str(log))
        mp.setattr(writer, "launcher_path", lambda: "/opt/fake/easy-personal-launch")
        old = keyring.get_keyring()
        keyring.set_keyring(MemoryKeyring())
        try:
            for route, calls in API_WRITE_CALLS.items():
                for method, path, body in calls:
                    text = json.dumps(body).replace("{secret}", secret)
                    res = client.request(method, path.replace("{secret}", "x"), headers={**headers, "content-type": "application/json"}, content=text)
                    out.add(f"api {method} {route}", f"{res.status_code} {dict(res.headers)} {res.text}")
        finally:
            keyring.set_keyring(old)
            mp.undo()
        out.add("api fake claude argv", log.read_text() if log.exists() else "")
    out.add("log records (api)", logs.text())


def collect_mcp_stdio(env: dict, errlog_path: Path, out: Collected) -> None:
    """Drive the real stdio server end to end; scan every response and the server's stderr."""
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.stdio import stdio_client

    params = StdioServerParameters(
        command=sys.executable, args=["-m", "easy_personal.surfaces.cli", "mcp"], env=env
    )

    async def drive(errlog) -> None:
        async with (
            stdio_client(params, errlog=errlog) as (read, write),
            ClientSession(read, write) as session,
        ):
            await session.initialize()
            out.add("mcp-stdio list_tools", (await session.list_tools()).model_dump_json())
            for tool, variants in MCP_CALLS.items():
                for args in variants:
                    res = await session.call_tool(tool, args)
                    out.add(f"mcp-stdio {tool} {args}", res.model_dump_json())

    with open(errlog_path, "w") as errlog:
        asyncio.run(asyncio.wait_for(drive(errlog), 120))
    out.add("mcp-stdio server stderr", errlog_path.read_bytes())


def store_blobs(home: Path) -> dict[str, object]:
    """The store as `sqlite3 .dump` plus the raw bytes of the db file and its WAL/SHM files."""
    db = home / "store.sqlite"
    conn = sqlite3.connect(db)
    try:
        dump = "\n".join(conn.iterdump())
    finally:
        conn.close()
    blobs: dict[str, object] = {"store .dump": dump}
    for path in sorted(home.glob("store.sqlite*")):
        blobs[f"store file {path.name}"] = path.read_bytes()
    return blobs


class LogCapture(logging.Handler):
    """Root-logger handler (all levels) for the in-process runs."""

    def __init__(self) -> None:
        super().__init__(logging.DEBUG)
        self.lines: list[str] = []
        self._root = logging.getLogger()
        self._level = self._root.level

    def emit(self, record: logging.LogRecord) -> None:
        self.lines.append(f"{record.name} {record.getMessage()} {record.args!r} {record.exc_text}")

    def text(self) -> str:
        return "\n".join(self.lines)

    def __enter__(self) -> Self:
        self._root.addHandler(self)
        self._root.setLevel(logging.DEBUG)
        return self

    def __exit__(self, *exc) -> None:
        self._root.removeHandler(self)
        self._root.setLevel(self._level)


def build_app():
    return mcp_surface.create_app()
