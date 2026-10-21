"""Run every surface against a canary-planted store and collect what each one emitted.

Registries (`MCP_CALLS`, `CLI_CALLS`) list every current MCP tool and CLI command with the
arguments to call it by. A meta-test fails when the product has a tool or command that is not
registered, so the suite cannot silently fall behind. See tests/leak/README.md.
"""

import asyncio
import contextlib
import io
import logging
import os
import sqlite3
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Self

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
    "warnings": [{}],
    "improvement_plan": [{}],
}
# CLI command -> list of argv tails. `{projects}` is the planted transcript root.
CLI_CALLS: dict[str, list[list[str]]] = {
    "ping": [[]],
    "ingest": [[], ["--source", "{projects}"], ["--retention-days", "3650"]],
    "schedule": [["install", "--dry-run"], ["remove", "--dry-run"]],
    "usage": [
        [],
        ["--since", "2026-01-01"],
        ["--until", "2027-01-01"],
        ["--by", "day"],
        ["--by", "model"],
        ["--by", "project"],
        ["--by", "tool"],
    ],
    "outliers": [[], ["--since", "2026-01-01"]],
    "warnings": [[]],
    "plan": [[]],
}
# Commands that start a long-running server; covered by the stdio test instead of a one-shot call.
# `dashboard` is covered through API_CALLS (in-process TestClient) instead.
CLI_SERVER_COMMANDS = {"mcp", "dashboard"}
# Dashboard JSON API route path -> list of query strings (HUN-49).
API_CALLS: dict[str, list[str]] = {
    "/api/usage/summary": ["", "?since=2026-01-01", "?until=2027-01-01"],
    "/api/usage/breakdown": ["?by=day", "?by=model", "?by=project", "?by=tool", "?by=bad"],
    "/api/usage/outliers": ["", "?since=2026-01-01"],
    "/api/warnings": [""],
    "/api/plan": [""],
}


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
    return cli_commands() - set(CLI_CALLS) - CLI_SERVER_COMMANDS


def child_env(home: Path, projects: Path) -> dict[str, str]:
    env = dict(os.environ)
    env.update(
        EASY_PERSONAL_HOME=str(home),
        EASY_PERSONAL_CLAUDE_PROJECTS=str(projects),
        PYTHONPATH=SRC,
    )
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
                result = asyncio.run(app.call_tool(tool, args))
            out.add(f"mcp {tool} {args}", result)
            out.add(f"mcp {tool} {args} stdout", sink_out.getvalue())
            out.add(f"mcp {tool} {args} stderr", sink_err.getvalue())
    out.add("log records (in-process)", logs.text())


def api_routes(app) -> set[str]:
    return {r.path for r in app.routes if r.path.startswith("/api/")}


def unregistered_api_routes(app) -> set[str]:
    return api_routes(app) - set(API_CALLS)


def collect_api(out: Collected, logs: "LogCapture") -> None:
    """Call every dashboard API route in-process with a valid token; keep status, headers, body."""
    from starlette.testclient import TestClient

    from easy_personal.surfaces import api

    token, port = "leak-test-token", 4321
    client = TestClient(api.create_app(token, port), base_url=f"http://127.0.0.1:{port}")
    headers = {api.TOKEN_HEADER: token}
    for path, queries in API_CALLS.items():
        for query in queries:
            res = client.get(path + query, headers=headers)
            out.add(f"api {path}{query}", f"{res.status_code} {dict(res.headers)} {res.text}")
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
