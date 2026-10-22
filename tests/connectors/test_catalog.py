import pytest

from easy_personal.connectors import catalog

GOOD = """\
# comment
category: Messaging
provider: Slack
claude_ai_connector: yes
command: npx -y some-slack-server
field: SLACK_BOT_TOKEN | Bot token | secret
field: SLACK_TEAM_ID | Team ID | plain
"""


def test_slack_entry_is_packaged_and_valid():
    entry = catalog.load("slack")
    assert entry.id == "slack" and entry.provider == "Slack" and entry.category
    assert entry.command[0] == "npx"
    secrets = [f.env for f in entry.fields if f.secret]
    assert secrets and all(f.label for f in entry.fields)
    assert isinstance(entry.claude_ai_connector, bool)
    assert "slack" in catalog.ids()


def test_parse_reads_every_schema_part():
    entry = catalog.parse("slack", GOOD)
    assert (entry.category, entry.provider, entry.claude_ai_connector) == ("Messaging", "Slack", True)
    assert entry.command == ["npx", "-y", "some-slack-server"]
    assert [(f.env, f.label, f.secret) for f in entry.fields] == [
        ("SLACK_BOT_TOKEN", "Bot token", True),
        ("SLACK_TEAM_ID", "Team ID", False),
    ]


@pytest.mark.parametrize(
    "bad",
    [
        GOOD.replace("category: Messaging\n", ""),
        GOOD.replace("command: npx -y some-slack-server\n", ""),
        GOOD + "extra: 1\n",
        GOOD.replace("| secret", "| hidden"),
        GOOD.replace("SLACK_BOT_TOKEN", "slack token"),
        GOOD + "field: SLACK_TEAM_ID | Dup | plain\n",
        GOOD.replace("claude_ai_connector: yes", "claude_ai_connector: maybe"),
        "\n".join(line for line in GOOD.splitlines() if not line.startswith("field")),
        GOOD.replace("Slack", ""),
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
