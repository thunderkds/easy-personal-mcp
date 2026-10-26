"""HUN-72: D1 remove only what we own, D2 doctor after adoption, D3 default account env, D5 EOF.

Fake `claude` with per-account state and an in-memory keyring; the real ones are never used.
CLAUDE_CONFIG_DIR is set or removed by each test, never inherited.
"""
import builtins
import getpass
import json

import pytest

from easy_personal.core import config, store, workspaces
from easy_personal.surfaces import cli

LAUNCH = {"command": "/opt/fake/bin/easy-personal-launch", "args": ["exec", "slack"]}
HAND = {"command": "/usr/bin/some-server", "args": ["--serve"]}
SECRET = "xoxb-PLANTED-secret-123"


@pytest.fixture
def world(tmp_path, monkeypatch, claude_state, launcher, mem_keyring):
    monkeypatch.delenv("CLAUDE_CONFIG_DIR", raising=False)
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(tmp_path / "agents"))
    monkeypatch.setattr(builtins, "input", lambda prompt="": "y")
    return {"fake": claude_state, "kr": mem_keyring, "state": tmp_path / "claude-state.json"}


def seed(world, entries, keychain=()):
    world["state"].write_text(json.dumps({"<unset>": entries}))
    for name in keychain:
        world["kr"].items[(f"easy-personal/{name}", "connector")] = json.dumps({"env": {"T": SECRET}})


def default_entries(world):
    return world["fake"].state().get("<unset>", {})


def run(capsys, *argv):
    rc = cli.main([str(a) for a in argv])
    out = capsys.readouterr()
    return rc, out.out, out.err


# --- D1 ------------------------------------------------------------------------------------------


def test_remove_of_a_name_that_is_not_a_catalog_id_changes_nothing(world, capsys):
    seed(world, {"notes": HAND})
    rc, out, err = run(capsys, "connector", "remove", "notes", "--yes")
    assert rc != 0 and len((out + err).strip().splitlines()) == 1 and SECRET not in out + err
    assert default_entries(world) == {"notes": HAND} and world["fake"].writes() == []


def test_remove_of_a_catalog_id_with_a_foreign_entry_leaves_it_and_the_keychain(world, capsys):
    seed(world, {"slack": HAND}, keychain=["slack"])
    rc, *_ = run(capsys, "connector", "remove", "slack", "--yes")
    assert rc != 0
    assert default_entries(world) == {"slack": HAND} and len(world["kr"].items) == 1
    assert world["fake"].writes() == []


def test_remove_of_our_legacy_entry_still_works(world, capsys):
    seed(world, {"slack": LAUNCH}, keychain=["slack"])
    rc, *_ = run(capsys, "connector", "remove", "slack", "--yes")
    assert rc == 0 and default_entries(world) == {} and world["kr"].items == {}


def test_remove_of_a_keychain_only_legacy_id_makes_no_claude_write(world, capsys):
    seed(world, {}, keychain=["slack"])
    rc, *_ = run(capsys, "connector", "remove", "slack", "--yes")
    assert rc == 0 and world["kr"].items == {} and world["fake"].writes() == []


# --- D3 ------------------------------------------------------------------------------------------


def test_default_account_calls_unset_an_inherited_config_dir(world, capsys, monkeypatch):
    seed(world, {"slack": LAUNCH}, keychain=["slack"])
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", "/some/other")
    assert run(capsys, "doctor")[0] in (0, 1)
    assert run(capsys, "connector", "remove", "slack", "--yes")[0] == 0
    records = world["fake"].records()
    assert any(r["argv"][:2] == ["mcp", "remove"] for r in records)
    assert {r["config_dir"] for r in records} == {"<unset>"}, records


def test_configured_checks_unset_an_inherited_config_dir(world, monkeypatch):
    from easy_personal.connectors import suggestions
    from easy_personal.surfaces import api

    seed(world, {"slack": LAUNCH}, keychain=["slack"])
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", "/some/other")
    assert api._configured("slack") is True
    assert suggestions._configured("slack", True) is True
    records = world["fake"].records()
    assert len(records) == 2 and {r["config_dir"] for r in records} == {"<unset>"}, records


# --- D2 ------------------------------------------------------------------------------------------


def test_doctor_skips_ids_that_are_stored_connectors(world, capsys):
    seed(world, {}, keychain=["slack"])  # adopted: keychain item, no default-account entry any more
    with store.opened(config.store_path()) as conn:
        orphan = workspaces.by_name(conn, "orphan")["id"]
        with conn:
            conn.execute("INSERT INTO connectors VALUES ('slack', 'slack', 0, 'slack', ?)", (orphan,))
    _rc, out, err = run(capsys, "doctor")
    assert "FAIL slack" not in out + err, out
    assert "FAIL discord" not in out + err


def test_doctor_still_fails_a_stored_connector_without_a_keychain_item(world, capsys):
    seed(world, {})
    with store.opened(config.store_path()) as conn:
        orphan = workspaces.by_name(conn, "orphan")["id"]
        with conn:
            conn.execute("INSERT INTO connectors VALUES ('slack', 'slack', 0, 'slack', ?)", (orphan,))
    rc, out, _ = run(capsys, "doctor")
    assert rc == 1 and "FAIL slack" in out


# --- D5 ------------------------------------------------------------------------------------------


@pytest.mark.parametrize("provider", ["slack", "gmail"])
def test_connector_add_prompt_at_eof_is_one_line(world, capsys, monkeypatch, provider):
    def eof(prompt=""):
        raise EOFError

    monkeypatch.setattr(builtins, "input", eof)
    monkeypatch.setattr(getpass, "getpass", eof)
    with store.opened(config.store_path()) as conn:
        workspaces.add(conn, "acme")
    rc, out, err = run(capsys, "connector", "add", provider, "--workspace", "acme", "--yes")
    assert rc != 0 and "Traceback" not in out + err
    assert len((out + err).strip().splitlines()) <= 2  # at most the prompt-line break and one message
    assert world["kr"].items == {}
    with store.opened(config.store_path()) as conn:
        assert conn.execute("SELECT COUNT(*) FROM connectors").fetchone()[0] == 0
