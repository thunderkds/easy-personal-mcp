import pytest

from easy_personal.connectors import catalog

GOOD = """\
# comment
category: Messaging
provider: Slack
claude_ai_connector: yes
claude_ai_link: https://claude.ai/settings/connectors
command: npx -y some-slack-server@1.2.3
field: SLACK_BOT_TOKEN | Bot token | secret
field: SLACK_TEAM_ID | Team ID | plain
"""


def test_slack_entry_is_packaged_and_valid():
    entry = catalog.load("slack")
    assert entry.id == "slack" and entry.provider == "Slack" and entry.category
    assert entry.command == ["npx", "-y", "slack-mcp-server@1.3.0", "--transport", "stdio"]
    assert [(f.env, f.secret) for f in entry.fields] == [("SLACK_MCP_XOXB_TOKEN", True)]
    secrets = [f.env for f in entry.fields if f.secret]
    assert secrets and all(f.label for f in entry.fields)
    assert entry.claude_ai_connector and entry.claude_ai_link == "https://claude.ai/settings/connectors"
    assert "slack" in catalog.ids()


def test_parse_reads_every_schema_part():
    entry = catalog.parse("slack", GOOD)
    assert (entry.category, entry.provider, entry.claude_ai_connector) == ("Messaging", "Slack", True)
    assert entry.command == ["npx", "-y", "some-slack-server@1.2.3"]
    assert [(f.env, f.label, f.secret) for f in entry.fields] == [
        ("SLACK_BOT_TOKEN", "Bot token", True),
        ("SLACK_TEAM_ID", "Team ID", False),
    ]


@pytest.mark.parametrize(
    "bad",
    [
        GOOD.replace("category: Messaging\n", ""),
        GOOD.replace("command: npx -y some-slack-server@1.2.3\n", ""),
        GOOD + "extra: 1\n",
        GOOD.replace("| secret", "| hidden"),
        GOOD.replace("SLACK_BOT_TOKEN", "slack token"),
        GOOD + "field: SLACK_TEAM_ID | Dup | plain\n",
        GOOD.replace("claude_ai_connector: yes", "claude_ai_connector: maybe"),
        "\n".join(line for line in GOOD.splitlines() if not line.startswith("field")),
        GOOD.replace("Slack", ""),
        GOOD.replace("claude_ai_link: https://claude.ai/settings/connectors\n", ""),
        GOOD.replace("https://claude.ai/settings/connectors", "http://claude.ai/x"),
        GOOD.replace("https://claude.ai/settings/connectors", "javascript:alert(1)"),
        GOOD.replace("claude_ai_connector: yes", "claude_ai_connector: no"),
    ],
)
def test_schema_rejects_bad_entries(bad):
    with pytest.raises(catalog.CatalogError):
        catalog.parse("slack", bad)


def test_entry_id_must_be_a_connector_name():
    with pytest.raises(catalog.CatalogError):
        catalog.parse("../evil", GOOD)


def test_unknown_provider_is_a_catalog_error():
    with pytest.raises(catalog.CatalogError):
        catalog.load("nope")


@pytest.mark.parametrize(
    "command",
    [
        "npx -y some-server",
        "npx -y some-server@latest",
        "npx -y some-server@^1.2.3",
        "npx -y some-server@1.2",
        "npx -y @scope/server",
        "npx -y @scope/server@latest",
        "uvx some-server",
        "pipx run some-server",
    ],
)
def test_unpinned_package_runner_is_refused(command):
    with pytest.raises(catalog.CatalogError, match="pinned"):
        catalog.parse("slack", GOOD.replace("npx -y some-slack-server@1.2.3", command))


@pytest.mark.parametrize(
    "command",
    [
        "npx -y @scope/server@1.2.3 --transport stdio",
        "uvx some-server@0.4.1",
        "pipx run some-server@2.0.0",
        "/usr/bin/some-binary --flag",
    ],
)
def test_pinned_or_non_runner_command_is_accepted(command):
    catalog.parse("slack", GOOD.replace("npx -y some-slack-server@1.2.3", command))


NO_CONNECTOR = GOOD.replace("claude_ai_connector: yes\nclaude_ai_link: https://claude.ai/settings/connectors",
                            "claude_ai_connector: no")


def test_link_is_none_when_no_claude_ai_connector():
    entry = catalog.parse("slack", NO_CONNECTOR)
    assert entry.claude_ai_connector is False and entry.claude_ai_link is None


def test_packaged_catalog_has_five_providers_in_two_categories_all_valid_and_pinned():
    assert catalog.ids() == ["discord", "gmail", "sendgrid", "slack", "teams"]
    entries = [catalog.load(i) for i in catalog.ids()]
    assert {e.category for e in entries} == {"Email", "Messaging"}
    for e in entries:
        assert e.command[0] == "npx" and "@" in e.command[2]  # _check_pinned already enforced
        assert e.claude_ai_connector == (e.claude_ai_link is not None)
        assert e.fields and all(f.label for f in e.fields)


@pytest.mark.parametrize(
    "entry_id, category, connector, command, fields",
    [
        ("gmail", "Email", True, ["npx", "-y", "@gongrzhe/server-gmail-autoauth-mcp@1.1.11"],
         [("GMAIL_OAUTH_PATH", False)]),
        ("sendgrid", "Email", False, ["npx", "-y", "sendgrid-mcp@1.4.0"], [("SENDGRID_API_KEY", True)]),
        ("teams", "Messaging", True, ["npx", "-y", "@softeria/ms-365-mcp-server@0.170.2", "--org-mode"],
         [("MS365_MCP_TENANT_ID", False)]),
        ("discord", "Messaging", False, ["npx", "-y", "@pasympa/discord-mcp@2.2.0"],
         [("DISCORD_TOKEN", True)]),
    ],
)
def test_new_entries_match_the_approved_servers(entry_id, category, connector, command, fields):
    e = catalog.load(entry_id)
    assert (e.category, e.claude_ai_connector, e.command) == (category, connector, command)
    assert [(f.env, f.secret) for f in e.fields] == fields
