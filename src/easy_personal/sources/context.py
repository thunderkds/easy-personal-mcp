"""context source (FR-012, FR-025): easy-context index freshness, read through its MCP service.

A read-only client of `http://127.0.0.1:8767/mcp` (DDR-0002). It can send only the tools in
`ALLOWED_TOOLS`; any other tool name, and any URL that is not `127.0.0.1`, is refused before a
network call. Every failure (service down, slow, bad reply) is "no data": it never fails ingest.
Only a project's root, `last_index_at` and `root_exists` are kept; no index content is read.
"""

import asyncio
import json
import os
import sqlite3
import subprocess
from datetime import UTC, datetime
from urllib.parse import urlparse

import httpx
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client

from easy_personal.core.discovery import run_git

ALLOWED_TOOLS = frozenset({"list_indexes", "get_stats"})
URL_ENV = "EASY_PERSONAL_CONTEXT_URL"
DEFAULT_URL = "http://127.0.0.1:8767/mcp"
TIMEOUT_S = 3.0
ROOT_TRIES = 3
SKEW_S = 60  # an index this close to a commit, or ahead of our clock, is not called stale


class Refused(Exception):
    """The call is not allowed (tool not in the allow-list, or host not 127.0.0.1)."""


class ContextUnavailable(Exception):
    """The service is down, slow or answered something unreadable."""


class ToolError(ContextUnavailable):
    """The service answered, but the tool call failed (for example, a root outside its workspace)."""


def check_url(url: str) -> str:
    try:
        parts = urlparse(url)
        host = parts.hostname
        parts.port  # noqa: B018  (raises ValueError on a bad port)
    except ValueError:
        raise Refused("bad easy-context URL") from None
    if parts.scheme != "http" or host != "127.0.0.1" or parts.username or parts.password:
        raise Refused("easy-context URL must be http://127.0.0.1:<port>/...")
    return url


def _decode(result):
    if result.isError:
        raise ToolError("tool error")
    data = result.structuredContent
    if data is None:
        try:
            data = json.loads(result.content[0].text)
        except (IndexError, AttributeError, ValueError):
            raise ContextUnavailable("unreadable reply") from None
    if isinstance(data, dict) and set(data) == {"result"}:
        data = data["result"]
    return data


async def _call(url: str, tool: str, arguments: dict, root: str | None):
    headers = {"X-Easy-Context-Root": root} if root else None
    async with (
        httpx.AsyncClient(headers=headers, timeout=TIMEOUT_S) as http,
        streamable_http_client(url, http_client=http) as (read, write, _),
        ClientSession(read, write) as session,
    ):
        await session.initialize()
        return await session.call_tool(tool, arguments)


def call_tool(tool: str, arguments: dict | None = None, *, url: str | None = None,
              root: str | None = None):  # fmt: skip
    """Call one allow-listed tool. Raises Refused (local) or ContextUnavailable (any failure)."""
    if tool not in ALLOWED_TOOLS:
        raise Refused(f"tool not allowed: {tool}")
    url = check_url(url or os.environ.get(URL_ENV) or DEFAULT_URL)
    try:
        result = asyncio.run(
            asyncio.wait_for(_call(url, tool, arguments or {}, root), TIMEOUT_S * 2)
        )
    except BaseException as exc:  # SDK wraps transport errors in ExceptionGroup
        if isinstance(exc, KeyboardInterrupt | SystemExit):
            raise
        raise ContextUnavailable(type(exc).__name__) from None
    return _decode(result)


def _epoch(value) -> float | None:
    """`last_index_at` as epoch seconds: ISO text or a number; None when unreadable or empty."""
    if isinstance(value, int | float) and not isinstance(value, bool):
        return float(value)
    if isinstance(value, str) and value:
        try:
            moment = datetime.fromisoformat(value)
        except ValueError:
            return None
        return (moment if moment.tzinfo else moment.replace(tzinfo=UTC)).timestamp()
    return None


def _iso(epoch: float) -> str:
    return datetime.fromtimestamp(epoch, UTC).isoformat()


def ingest(conn: sqlite3.Connection, url: str | None = None) -> dict:
    """Store `last_index_at` for each known project. Never raises for a service problem.

    AC5 spike (real service): `list_indexes` lists every index on the machine in one reply, so
    one call is enough, not one per repo. The service still wants a root header on every call
    (even this one), so we send an existing project's root; if the service refuses that root
    (outside its workspace), the next project is tried, at most `ROOT_TRIES` times.
    A failed call clears the table: freshness is then "no data", not stale data.
    """
    projects = {
        p
        for (p,) in conn.execute(
            "SELECT DISTINCT project_path FROM events "
            "WHERE source = 'claude_code' AND project_path IS NOT NULL ORDER BY 1"
        )
    }
    reply = None
    for root in [p for p in sorted(projects) if os.path.isdir(p)][:ROOT_TRIES]:
        try:
            reply = call_tool("list_indexes", url=url, root=root)
            break
        except ToolError:
            continue
        except (Refused, ContextUnavailable):
            break
    listed = reply.get("indexes") if isinstance(reply, dict) else None
    if not isinstance(listed, list):
        with conn:
            conn.execute("DELETE FROM context_index")
        return {"status": "no data"}
    rows = []
    for item in listed:
        if not isinstance(item, dict) or item.get("root") not in projects:
            continue
        if item.get("root_exists") is False:  # index kept by easy-context for a deleted folder
            continue
        epoch = _epoch(item.get("last_index_at"))
        rows.append((item["root"], _iso(epoch) if epoch is not None else None))
    with conn:
        conn.execute("DELETE FROM context_index")
        conn.executemany(
            "INSERT INTO context_index (project_path, last_index_at_utc) VALUES (?, ?)", rows
        )
    return {"status": "ok", "indexes": len(rows)}


def latest_commit(project: str) -> float | None:
    """Epoch of the newest commit on any local ref; None for no repo or no commits."""
    out = run_git(project, "log", "--branches", "--tags", "--remotes", "-1", "--format=%ct",
                  timeout=10)  # fmt: skip
    if not out:
        return None
    return float(out.split()[0])


def freshness(conn: sqlite3.Connection, project: str, now: float | None = None) -> dict | None:
    """None = no data. Else {last_index_at, latest_commit_utc, stale}; stale is None if unjudgeable."""
    row = conn.execute(
        "SELECT last_index_at_utc FROM context_index WHERE project_path = ?", (project,)
    ).fetchone()
    if row is None:
        return None
    indexed = _epoch(row[0])
    try:
        commit = latest_commit(project)
    except subprocess.TimeoutExpired:
        commit = None
    now = datetime.now(UTC).timestamp() if now is None else now
    stale = None
    if indexed is not None and commit is not None and indexed <= now + SKEW_S:
        stale = commit - indexed > SKEW_S
    return {
        "last_index_at": row[0],
        "latest_commit_utc": _iso(commit) if commit is not None else None,
        "stale": stale,
    }


def context_part(conn: sqlite3.Connection, project: str) -> dict:
    found = freshness(conn, project)
    return {"status": "no data"} if found is None else {"status": "ok", **found}


def stale_projects(conn: sqlite3.Connection) -> tuple[int, list[str]] | None:
    """(count, folder names) of projects with a stale index; None when no context data is stored."""
    from pathlib import Path

    paths = [p for (p,) in conn.execute("SELECT project_path FROM context_index ORDER BY 1")]
    if not paths:
        return None
    stale = [Path(p).name or p for p in paths if (freshness(conn, p) or {}).get("stale")]
    return len(stale), stale
