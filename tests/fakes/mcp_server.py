"""Fake easy-context service for tests: an MCP streamable-HTTP server on 127.0.0.1.

Same conventions as the real one (checked in the HUN-41 spike, field names only): every call needs
an `X-Easy-Context-Root` header or a `root` argument; `list_indexes` answers
{project_id, root, indexes: [...]}. Records every tool name it receives in `calls`.
"""

import asyncio
import socket
import threading
import time
from contextlib import contextmanager

import uvicorn
from mcp.server.fastmcp import Context, FastMCP
from mcp.server.fastmcp.exceptions import ToolError
from mcp.server.transport_security import TransportSecuritySettings


def index_entry(root: str, last_index_at: str, root_exists: bool = True) -> dict:
    return {
        "project_id": "id-" + root.strip("/").replace("/", "-"),
        "root": root,
        "root_exists": root_exists,
        "files": 3,
        "slices": 9,
        "last_index_at": last_index_at,
        "status": "done",
    }


@contextmanager
def fake_context(indexes: list[dict], *, delay: float = 0.0, reject_roots: set[str] = frozenset()):
    """Yield (url, calls). `delay` makes every tool slow; `reject_roots` fail like 'outside workspace'."""
    from starlette.requests import Request

    calls: list[str] = []
    roots: list[str | None] = []
    mcp = FastMCP("fake-easy-context", stateless_http=True, json_response=True,
                  transport_security=TransportSecuritySettings(enable_dns_rebinding_protection=False))  # fmt: skip

    def gate(request: Request) -> str:
        root = request.headers.get("x-easy-context-root")
        roots.append(root)
        if not root:
            raise ToolError("no project root: send the X-Easy-Context-Root header")
        if root in reject_roots:
            raise ToolError("outside the workspace")
        return root

    @mcp.tool()
    async def list_indexes(ctx: Context) -> dict:
        calls.append("list_indexes")
        root = gate(ctx.request_context.request)
        if delay:
            await asyncio.sleep(delay)
        return {"project_id": "x", "root": root, "indexes": indexes}

    @mcp.tool()
    async def get_stats(ctx: Context) -> dict:
        calls.append("get_stats")
        gate(ctx.request_context.request)
        return {"indexes": indexes}

    @mcp.tool()
    async def index_path(ctx: Context) -> dict:
        calls.append("index_path")
        return {}

    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    port = sock.getsockname()[1]
    server = uvicorn.Server(uvicorn.Config(mcp.streamable_http_app(), log_level="error"))
    thread = threading.Thread(target=server.run, kwargs={"sockets": [sock]}, daemon=True)
    thread.start()
    while not server.started:
        time.sleep(0.01)
    try:
        yield f"http://127.0.0.1:{port}/mcp", calls, roots
    finally:
        server.should_exit = True
        thread.join(5)
        sock.close()
