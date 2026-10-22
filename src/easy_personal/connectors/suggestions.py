"""Read-only connector views for the MCP tools: catalog, configured yes/no, usage-based suggestions.

Nothing here writes config or the keychain, and nothing returns a secret value. Suggestions come
only from counters over tool names, WebFetch hosts and Bash first words (metadata, never content).
"""
import shutil
import sqlite3

from easy_personal.connectors import catalog, writer

# Registrable domain -> provider id. A host matches the domain itself or any subdomain of it.
DOMAINS = {
    "slack.com": "slack",
    "discord.com": "discord",
    "discord.gg": "discord",
    "sendgrid.com": "sendgrid",
    "mail.google.com": "gmail",
    "gmail.googleapis.com": "gmail",
    "teams.microsoft.com": "teams",
    "graph.microsoft.com": "teams",
}
NOTE = (
    "Counts cover sessions ingested since this version; run `easy-personal ingest` to refresh. "
    "Bash first words are counted but map to no provider yet."
)


def catalog_view() -> dict:
    """Every catalog entry: ids, provider, category, claude.ai flag and link, field labels."""
    entries = [catalog.load(i) for i in catalog.ids()]
    return {
        "connectors": [
            {
                "id": e.id,
                "provider": e.provider,
                "category": e.category,
                "claude_ai_connector": e.claude_ai_connector,
                "claude_ai_link": e.claude_ai_link,
                "fields": [{"label": f.label, "secret": f.secret} for f in e.fields],
            }
            for e in entries
        ]
    }


def _configured(name: str, has_claude: bool) -> bool:
    """Keychain item present AND Claude Code reports the entry. Any failure means not configured."""
    try:
        return has_claude and writer._keychain_item(name) and writer._entry_exists(name) is not None
    except Exception:  # noqa: BLE001 - locked or missing keychain, odd claude output: not configured
        return False


def configured_ids() -> list[str]:
    has_claude = shutil.which("claude") is not None
    return [i for i in catalog.ids() if _configured(i, has_claude)]


def status() -> dict:
    done = set(configured_ids())
    return {"connectors": [{"id": i, "configured": i in done} for i in catalog.ids()]}


def _provider_of_host(host: str) -> str | None:
    for domain, provider in DOMAINS.items():
        if host == domain or host.endswith("." + domain):
            return provider
    return None


def suggestions(conn: sqlite3.Connection, configured: list[str] | None = None) -> dict:
    """Providers seen in use but not yet configured, with the call counts that point to them."""
    skip = set(configured_ids() if configured is None else configured)
    ids = catalog.ids()
    domain_calls: dict[str, int] = {}
    for host, n in conn.execute(
        "SELECT value, COUNT(*) FROM tool_signals WHERE kind = 'domain' GROUP BY value"
    ):
        if (provider := _provider_of_host(host)) is not None:
            domain_calls[provider] = domain_calls.get(provider, 0) + n
    mcp_calls: dict[str, int] = {}
    for (names,) in conn.execute("SELECT tool_names FROM events WHERE tool_names LIKE '%mcp\\_\\_%' ESCAPE '\\'"):
        for tool in names.split(","):
            parts = tool.split("__")
            if parts[0] == "mcp" and len(parts) >= 3:
                server = parts[1].lower()
                for provider in ids:
                    if provider in server:
                        mcp_calls[provider] = mcp_calls.get(provider, 0) + 1
    found = [
        {
            "id": i,
            "provider": catalog.load(i).provider,
            "webfetch_calls": domain_calls.get(i, 0),
            "mcp_calls": mcp_calls.get(i, 0),
        }
        for i in ids
        if i not in skip and (domain_calls.get(i) or mcp_calls.get(i))
    ]
    found.sort(key=lambda s: -(s["webfetch_calls"] + s["mcp_calls"]))
    return {"suggestions": found, "note": NOTE}
