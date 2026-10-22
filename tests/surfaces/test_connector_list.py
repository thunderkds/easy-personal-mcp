from easy_personal.connectors import catalog
from easy_personal.surfaces import cli

LINK = "https://claude.ai/settings/connectors"


def run(capsys, *argv):
    rc = cli.main(list(argv))
    return rc, capsys.readouterr().out


def test_list_groups_by_category_with_claude_ai_entries_first(capsys):
    rc, out = run(capsys, "connector", "list")
    assert rc == 0
    pos = {i: out.index(f"{i} ") for i in catalog.ids()}
    assert out.index("Email") < out.index("Messaging")
    assert pos["gmail"] < pos["sendgrid"] < out.index("Messaging")
    assert out.index("Messaging") < pos["slack"] < pos["teams"] < pos["discord"]


def test_list_shows_link_and_field_labels_but_no_values(capsys, monkeypatch):
    monkeypatch.setenv("DISCORD_TOKEN", "PLANTED-secret-value")
    rc, out = run(capsys, "connector", "list")
    assert rc == 0
    assert LINK in out and "Bot token" in out and "API key" in out
    assert "PLANTED-secret-value" not in out
    assert "yes" in out and "no" in out
