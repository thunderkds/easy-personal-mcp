import json
from importlib.metadata import version
from typing import Any

from mcp.server.fastmcp import FastMCP

from easy_personal import core
from easy_personal.core import config, kit, kitperf, metrics, plan, reports, rules, store

LOOPBACK = "127.0.0.1"

# Fixed text of the `investigate` prompt (FR-075). The client's own model does the reasoning; this server runs none.
INVESTIGATE_PROMPT = """\
Investigate my Claude Code usage with the easy-personal tools and tell me what to change. Use only numbers the tools return.

1. Call `warnings` and `improvement_plan`. Use the same scope for both.
2. Call `usage_breakdown` with by="project" and again with by="model", for the last 7 days (set since to the date 7 days ago).
3. Call `session_outliers` for the last 7 days.
4. For each warning that fired, explain in plain words what caused it. Read the hit's causes when it has them. Tie it to the projects, models or sessions you found in steps 2 and 3.
5. Suggest at most 5 fixes, ranked by expected saving (largest first). Cite one number from a tool result for each fix, and name the tool it came from.
6. Say plainly when a change looks intended rather than wasteful (for example, one big project that I am working on right now). Do not suggest a fix for it.

Do not guess. If a tool returns no data or abstains, say so. Do not change any file.
"""

# Last step of the prompt (FR-077). Kept apart so the scope line comes before it and the prompt still ends with this step.
SAVE_REPORT_STEP = """\
7. Save the report: call `save_report` once, with a summary of what you found (at most 2,000 characters), the fixes from step 5 (at most 5, each with its text and the tool and number it cites) and the same scope you used above. Do not put keys, tokens or message text in it; the tool refuses them. Then tell me the report id it returns. If `save_report` is not available, tell me to reconnect the easy-personal server in /mcp, and do not retry.
"""


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
        equals usage_summary; tool groups overlap. Project groups put every scratch folder (temp,
        gone, not in a git repo, or hidden) into one group `Scratch` (`scratch: true`) that lists
        them in `folders`."""
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
    def kit_findings(limit: int = kitperf.DEFAULT_LIMIT, since: str | None = None) -> dict[str, Any]:
        """Which Easy Kit MCP servers (easy-ui, easy-verifier, easy-context, easy-memory) are linked to
        cheaper or better sessions, comparing sessions with and without each server inside the same
        project, main model and tool-call band (tokens and cost per tool call, share of good results).
        `findings` = at most `limit` (default 3, max 10) comparisons where the server side is better by at least 5 %, each
        with numbers, group sizes and one fixed habit; `other` = every other comparison that has at least 10
        sessions on each side (no habit); `not_enough` lists servers with fewer. Worded "linked to", never
        "caused". `since` is an ISO date or datetime (without an offset: local time); omitted means all
        stored history."""
        with store.opened(config.store_path()) as conn:
            return kitperf.kit_findings(conn, limit, since)

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

    @app.tool()
    def save_report(
        summary: str, fixes: list[dict[str, Any]], workspace: str | None = None, account: str | None = None
    ) -> dict[str, Any]:
        """Save an `investigate` report on the dashboard's Warnings & plan page. `summary` is at most 2,000
        characters; `fixes` is at most 5 items, each `text` (at most 300 characters) with an optional `tool`
        and `number` it cites. Pass the same `workspace` (name) and `account` (id or label) you used for
        `warnings`. The server adds its own numbers for every rule at this moment (the rule snapshot), so
        the page can later show before / now per warning. Refuses the whole call, naming the field, when a
        limit is passed, the scope is unknown or any text looks like a secret; nothing is stored then.
        Returns `{id, saved_utc}`. The only tool that writes, and only to the store."""
        with store.opened(config.store_path()) as conn:
            return reports.save(conn, summary, fixes, workspace, account)

    @app.prompt(
        name="investigate",
        description="Investigate your Claude Code usage: causes of each warning and up to 5 ranked fixes.",
    )
    def investigate(workspace: str | None = None, account: str | None = None) -> str:
        scope = [f"{k}={json.dumps(v, ensure_ascii=False)}" for k, v in (("workspace", workspace), ("account", account)) if v]
        text = INVESTIGATE_PROMPT
        if scope:
            text += f"\nScope: pass {' and '.join(scope)} to every tool call that takes a scope (`warnings`, `improvement_plan`, `save_report`).\n"
        return text + "\n" + SAVE_REPORT_STEP

    return app


def serve(transport: str = "stdio", host: str = LOOPBACK, port: int = 8765) -> None:
    app = create_app(host, port)
    app.run(transport="streamable-http" if transport == "http" else "stdio")
