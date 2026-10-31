from importlib.metadata import version
from typing import Any

from mcp.server.fastmcp import FastMCP

from easy_personal import core
from easy_personal.core import config, kit, metrics, plan, rules, store

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
        tokens, cache-hit ratio, records abstained per version, and `split`: tokens by typed (you),
        agent (sidechain, or an sdk* entrypoint) and unknown (not yet backfilled) work.
        Bounds are ISO 8601 dates or datetimes; without an offset they are local time; omitted means all stored history."""
        with store.opened(config.store_path()) as conn:
            return metrics.usage_summary(conn, since, until)

    @app.tool()
    def usage_breakdown(
        by: str, since: str | None = None, until: str | None = None
    ) -> dict[str, Any]:
        """Tokens grouped by day (OS local time), model, project (folder name) or tool for
        [since, until), with an API-equivalent cost (price-table date shown; a model missing
        from the table abstains). Each group has the same typed / agent / unknown `split`. `total`
        equals usage_summary; tool groups overlap."""
        with store.opened(config.store_path()) as conn:
            return metrics.usage_breakdown(conn, by, since, until)

    @app.tool()
    def session_outliers(since: str | None = None, until: str | None = None) -> dict[str, Any]:
        """Sessions with at least 90 tool calls or a very long context, each with its measured
        values and the thresholds used."""
        with store.opened(config.store_path()) as conn:
            return metrics.session_outliers(conn, since, until)

    @app.tool()
    def warnings(workspace: str | None = None, account: str | None = None) -> dict[str, Any]:
        """Declared usage rules over the last 7 days (or 14 for week-over-week). Each hit has the
        rule id, label (improve/aware/warning), measured value, threshold and fix line. Rules that
        cannot be measured (too little history or data) are listed under `abstained`. Optional
        `workspace` (name) and `account` (id or label) limit every rule to that scope."""
        with store.opened(config.store_path()) as conn:
            return rules.evaluate(conn, workspace=workspace, account=account)

    @app.tool()
    def improvement_plan(workspace: str | None = None, account: str | None = None) -> dict[str, Any]:
        """The fired rules grouped by priority (1 first), each with its measured value and fix.
        Optional `workspace` (name) and `account` (id or label) limit every rule to that scope."""
        with store.opened(config.store_path()) as conn:
            return plan.improvement_plan(conn, workspace=workspace, account=account)

    @app.tool()
    def kit_summary(workspace: str | None = None, account: str | None = None) -> dict[str, Any]:
        """What the Easy Kit did per project (folder name): kernel-trace tool calls, errors, kit
        tasks and last activity. A part with nothing to read says "no data". Optional `workspace`
        (name) and `account` (id or label) keep only that scope's projects."""
        with store.opened(config.store_path()) as conn:
            return kit.kit_summary(conn, workspace=workspace, account=account)

    @app.tool()
    def connector_catalog() -> dict[str, Any]:
        """The provider catalog: id, provider, category, whether Claude.ai has an official connector
        (and its link), and the field labels each needs. Read-only; never a secret value."""
        from easy_personal.connectors import suggestions

        return suggestions.catalog_view()

    @app.tool()
    def connector_status() -> dict[str, Any]:
        """Per catalog id, whether the connector is configured (yes/no only): a keychain item and a
        Claude Code entry both exist. Never returns a secret or any config value."""
        from easy_personal.connectors import suggestions

        return suggestions.status()

    @app.tool()
    def connector_suggestions() -> dict[str, Any]:
        """Providers not yet configured that your usage points to, from counted WebFetch hosts and
        MCP tool names (metadata only), with the call counts behind each suggestion."""
        from easy_personal.connectors import suggestions

        with store.opened(config.store_path()) as conn:
            return suggestions.suggestions(conn)

    return app


def serve(transport: str = "stdio", host: str = LOOPBACK, port: int = 8765) -> None:
    app = create_app(host, port)
    app.run(transport="streamable-http" if transport == "http" else "stdio")
