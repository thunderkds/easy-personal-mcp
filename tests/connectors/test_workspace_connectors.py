"""`connector add --workspace` (HUN-65, FR-029): fixed id + alias, secret only in the keychain, no Claude write."""

import builtins
import getpass
import json
import re

import pytest

from easy_personal.core import config, store, workspaces
from easy_personal.surfaces import cli

SECRET = "xoxb-PLANTED-secret-123"


@pytest.fixture
def answers(monkeypatch):
    state = {"secret": SECRET, "confirm": "y"}
    monkeypatch.setattr(getpass, "getpass", lambda prompt="": state["secret"])
    monkeypatch.setattr(builtins, "input", lambda prompt="": state["confirm"] if "[y/N]" in prompt else "plain")
    return state


@pytest.fixture
def db():
    conn = store.connect(config.store_path())
    workspaces.add(conn, "company")
    workspaces.add(conn, "personal")
    yield conn
    conn.close()


def run(capsys, *argv):
    rc = cli.main(list(argv))
    out = capsys.readouterr()
    return rc, out.out, out.err


def rows(conn):
    return conn.execute(
        "SELECT c.id, c.alias, c.alias_custom, c.provider, w.name FROM connectors c "
        "JOIN workspaces w ON w.id = c.workspace_id ORDER BY c.alias"
    ).fetchall()


def test_add_creates_id_stores_secret_under_id_and_writes_no_claude(db, mem_keyring, fake_claude, launcher, answers, capsys):
    rc, out, err = run(capsys, "connector", "add", "slack", "--workspace", "company")
    assert rc == 0, out + err
    ((id_, alias, custom, provider, ws),) = rows(db)
    assert re.fullmatch(r"c[0-9a-f]{8}", id_)
    assert (alias, custom, provider, ws) == ("company-slack", 0, "slack", "company")
    ((service, user), payload), = mem_keyring.items.items()
    assert (service, user) == (f"easy-personal/{id_}", "connector")
    assert json.loads(payload)["env"] == {"SLACK_MCP_XOXB_TOKEN": SECRET}
    assert fake_claude.writes() == []  # AC2: no Claude write until a sync
    assert id_ in out and "company-slack" in out and "connector sync" in out
    assert "SLACK_MCP_XOXB_TOKEN=***" in out
    assert SECRET not in out + err
    assert SECRET not in "\n".join(db.iterdump())


def test_custom_alias_is_kept_and_marked(db, mem_keyring, fake_claude, launcher, answers, capsys):
    rc, *_ = run(capsys, "connector", "add", "slack", "--workspace", "company", "--alias", "work-chat", "--yes")
    assert rc == 0
    assert rows(db)[0][1:3] == ("work-chat", 1)


def test_two_workspaces_same_provider_get_different_aliases(db, mem_keyring, fake_claude, launcher, answers, capsys):
    assert run(capsys, "connector", "add", "slack", "--workspace", "company", "--yes")[0] == 0
    assert run(capsys, "connector", "add", "slack", "--workspace", "personal", "--yes")[0] == 0
    got = rows(db)
    assert [r[1] for r in got] == ["company-slack", "personal-slack"]
    assert got[0][0] != got[1][0] and len(mem_keyring.items) == 2


def test_second_connector_of_a_provider_in_one_workspace_is_refused(db, mem_keyring, fake_claude, launcher, answers, capsys):
    run(capsys, "connector", "add", "slack", "--workspace", "company", "--yes")
    rc, out, err = run(capsys, "connector", "add", "slack", "--workspace", "company", "--alias", "x", "--yes")
    assert rc == 1 and "already has a slack connector" in out + err
    assert len(rows(db)) == 1 and len(mem_keyring.items) == 1


def test_alias_in_use_is_refused(db, mem_keyring, fake_claude, launcher, answers, capsys):
    run(capsys, "connector", "add", "slack", "--workspace", "company", "--yes")
    rc, out, err = run(capsys, "connector", "add", "gmail", "--workspace", "personal", "--alias", "company-slack", "--yes")
    assert rc == 1 and "alias" in out + err
    assert len(rows(db)) == 1 and len(mem_keyring.items) == 1


@pytest.mark.parametrize("alias", ["Bad", "a" * 65, "-x", "a b"])
def test_bad_alias_is_refused_before_any_prompt(db, mem_keyring, fake_claude, launcher, answers, capsys, alias):
    rc, out, err = run(capsys, "connector", "add", "slack", "--workspace", "company", f"--alias={alias}", "--yes")
    assert rc == 2 and "alias must match" in out + err
    assert alias not in err.replace("alias must match", "")  # never echoes input back
    assert rows(db) == [] and mem_keyring.items == {}


def test_longest_workspace_name_and_alias_fit(mem_keyring, fake_claude, launcher, answers, capsys):
    conn = store.connect(config.store_path())
    workspaces.add(conn, "w" * 40)
    assert run(capsys, "connector", "add", "sendgrid", "--workspace", "w" * 40, "--yes")[0] == 0  # 49 chars
    assert run(capsys, "connector", "add", "slack", "--workspace", "w" * 40, "--alias", "a" * 64, "--yes")[0] == 0
    conn.close()


@pytest.mark.parametrize("ws", ["orphan", "nope"])
def test_unknown_or_orphan_workspace_is_refused(db, mem_keyring, fake_claude, launcher, answers, capsys, ws):
    rc, _, err = run(capsys, "connector", "add", "slack", "--workspace", ws, "--yes")
    assert rc == 2 and "workspace" in err
    assert rows(db) == [] and mem_keyring.items == {}


def test_without_workspace_is_a_clear_error(db, mem_keyring, fake_claude, launcher, answers, capsys):
    with pytest.raises(SystemExit) as exc:
        cli.main(["connector", "add", "slack", "--yes"])
    err = capsys.readouterr().err
    assert exc.value.code == 2 and "required: --workspace" in err
    assert rows(db) == [] and mem_keyring.items == {} and fake_claude.calls() == []


def test_abort_at_confirm_writes_nothing(db, mem_keyring, fake_claude, launcher, answers, capsys):
    answers["confirm"] = "n"
    rc, out, _ = run(capsys, "connector", "add", "slack", "--workspace", "company")
    assert rc == 1 and "aborted" in out
    assert rows(db) == [] and mem_keyring.items == {}


def test_empty_secret_writes_nothing(db, mem_keyring, fake_claude, launcher, answers, capsys):
    answers["secret"] = ""
    rc, *_ = run(capsys, "connector", "add", "slack", "--workspace", "company", "--yes")
    assert rc == 2 and rows(db) == [] and mem_keyring.items == {}


def test_no_keychain_refuses_and_stores_no_row(db, no_keyring, fake_claude, launcher, answers, capsys):
    rc, out, err = run(capsys, "connector", "add", "slack", "--workspace", "company", "--yes")
    assert rc == 1 and "plaintext" in out + err
    assert rows(db) == []


def test_unknown_provider(db, mem_keyring, fake_claude, launcher, answers, capsys):
    assert run(capsys, "connector", "add", "nope", "--workspace", "company")[0] == 2
    assert rows(db) == []


def test_connector_list_shows_stored_connectors_without_secrets(db, mem_keyring, fake_claude, launcher, answers, capsys):
    run(capsys, "connector", "add", "slack", "--workspace", "company", "--yes")
    rc, out, _ = run(capsys, "connector", "list")
    assert rc == 0 and "company-slack" in out and rows(db)[0][0] in out and SECRET not in out
