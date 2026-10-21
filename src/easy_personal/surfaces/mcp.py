from importlib.metadata import version
from typing import Any

from mcp.server.fastmcp import FastMCP

from easy_personal import core
from easy_personal.core import config, metrics, plan, rules, store

LOOPBACK = "127.0.0.1"


def validate_host(host: str) -> str:
    if host != LOOPBACK:
        raise ValueError(f"refusing host {host!r}: HTTP must bind {LOOPBACK} only")
    return host


def create_app(host: str = LOOPBACK, port: int = 8765) -> FastMCP:
    app = FastMCP("easy-personal", host=validate_host(host), port=port)
    # FastMCP has no version argument; without this, initialize reports the MCP SDK version.
    app._mcp_server.version = version("easy-personal-mcp")

    @app.tool()
    def ping() -> str:
        """Placeholder tool: returns the shared core's status string."""
        return core.ping()

    @app.tool()
    def usage_summary(since: str | None = None, until: str | None = None) -> dict[str, Any]:
        """Claude Code usage for [since, until): sessions, messages, input/output and cache
        tokens, cache-hit ratio, and records abstained per version. Bounds are ISO 8601 dates or
        datetimes; without an offset they are local time; omitted means all stored history."""
        with store.opened(config.store_path()) as conn:
            return metrics.usage_summary(conn, since, until)

    @app.tool()
    def usage_breakdown(
        by: str, since: str | None = None, until: str | None = None
    ) -> dict[str, Any]:
        """Tokens grouped by day (OS local time), model, project (folder name) or tool for
        [since, until), with an API-equivalent cost (price-table date shown; a model missing
        from the table abstains). `total` equals usage_summary; tool groups overlap."""
        with store.opened(config.store_path()) as conn:
            return metrics.usage_breakdown(conn, by, since, until)

    @app.tool()
    def session_outliers(since: str | None = None, until: str | None = None) -> dict[str, Any]:
        """Sessions with at least 90 tool calls or a very long context, each with its measured
        values and the thresholds used."""
        with store.opened(config.store_path()) as conn:
            return metrics.session_outliers(conn, since, until)

    @app.tool()
    def warnings() -> dict[str, Any]:
        """Declared usage rules over the last 7 days (or 14 for week-over-week). Each hit has the
        rule id, label (improve/aware/warning), measured value, threshold and fix line. Rules that
        cannot be measured (too little history or data) are listed under `abstained`."""
        with store.opened(config.store_path()) as conn:
            return rules.evaluate(conn)

    @app.tool()
    def improvement_plan() -> dict[str, Any]:
        """The fired rules grouped by priority (1 first), each with its measured value and fix."""
        with store.opened(config.store_path()) as conn:
            return plan.improvement_plan(conn)

    return app


def serve(transport: str = "stdio", host: str = LOOPBACK, port: int = 8765) -> None:
    app = create_app(host, port)
    app.run(transport="streamable-http" if transport == "http" else "stdio")
