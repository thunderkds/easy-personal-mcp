"""Adopt pre-workspace connectors into orphan (HUN-67, FR-034).

Fake `claude` with per-account state and an in-memory keyring; the real ones are never used.
"""

import builtins
import json

import pytest

from easy_personal.connectors import legacy
from easy_personal.core import config, store
from easy_personal.surfaces import cli

LAUNCH = {"command": "/opt/fake/bin/easy-personal-launch", "args": ["exec", "slack"]}


@pytest.fixture
def world(tmp_path, monkeypatch, claude_state, launcher, mem_keyring):
    monkeypatch.delenv("CLAUDE_CONFIG_DIR", raising=False)
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(tmp_path / "agents"))
    answer = {"v": "y"}
    monkeypatch.setattr(builtins, "input", lambda prompt="": answer["v"])
    return {"fake": claude_state, "kr": mem_keyring, "answer": answer, "state": tmp_path / "claude-state.json"}


HAND = {"command": "/usr/bin/some-server", "args": ["--serve"]}


def seed(world, *, keychain=True, claude=True, other=None, hand=False):
    if keychain:
        world["kr"].items[("easy-personal/slack", "connector")] = json.dumps({"env": {"T": "s3cret"}})
    entries = {"slack": HAND if hand else LAUNCH} if claude else {}
    state = {"<unset>": entries}
    if other:
        state["/x/.claude-work"] = {"slack": LAUNCH}
    world["state"].write_text(json.dumps(state))


def run(capsys, *argv):
    rc = cli.main([str(a) for a in argv])
    out = capsys.readouterr()
    return rc, out.out, out.err


def connectors():
    with store.opened(config.store_path()) as conn:
        return conn.execute(
            "SELECT c.id, c.alias, c.provider, w.is_orphan FROM connectors c JOIN workspaces w ON w.id = c.workspace_id"
        ).fetchall()


def test_adopts_into_orphan_removes_from_default_claude_keeps_keychain(world, capsys):
    seed(world)
    rc, out, err = run(capsys, "connector", "adopt-legacy")
    assert rc == 0, out + err
    assert "remove slack" in out and "keychain" in out  # the plan is shown
    assert connectors() == [("slack", "slack", "slack", 1)]
    assert ("easy-personal/slack", "connector") in world["kr"].items
    assert world["fake"].state()["<unset>"] == {}
    assert world["fake"].write_records() == [("<unset>", ["mcp", "remove", "slack", "--scope", "user"])]


def test_second_run_finds_nothing(world, capsys):
    seed(world)
    run(capsys, "connector", "adopt-legacy", "--yes")
    before = len(world["fake"].write_records())
    rc, out, _ = run(capsys, "connector", "adopt-legacy")
    assert rc == 0 and "nothing to adopt" in out
    assert len(world["fake"].write_records()) == before and len(connectors()) == 1


def test_declined_changes_nothing(world, capsys):
    seed(world)
    world["answer"]["v"] = "n"
    rc, out, _ = run(capsys, "connector", "adopt-legacy")
    assert rc == 1 and "aborted" in out
    assert connectors() == [] and world["fake"].write_records() == []
    assert "slack" in world["fake"].state()["<unset>"]


def test_yes_skips_the_question(world, capsys, monkeypatch):
    seed(world)
    monkeypatch.setattr(builtins, "input", lambda prompt="": pytest.fail("asked"))
    assert run(capsys, "connector", "adopt-legacy", "--yes")[0] == 0
    assert len(connectors()) == 1


def test_keychain_only_adopts_without_any_claude_write(world, capsys):
    seed(world, claude=False)
    rc, _, _ = run(capsys, "connector", "adopt-legacy", "--yes")
    assert rc == 0 and len(connectors()) == 1
    assert world["fake"].write_records() == []


def test_claude_only_adopts_and_warns_about_the_missing_keychain_item(world, capsys):
    seed(world, keychain=False)
    rc, out, _ = run(capsys, "connector", "adopt-legacy", "--yes")
    assert rc == 0 and "no keychain item" in out
    assert len(connectors()) == 1 and world["fake"].state()["<unset>"] == {}


def test_remove_failure_keeps_the_orphan_row_and_reports_pending(world, capsys, monkeypatch):
    seed(world)
    monkeypatch.setenv("FAKE_CLAUDE_FAIL", "remove slack")
    rc, out, _ = run(capsys, "connector", "adopt-legacy", "--yes")
    assert rc == 1 and "pending" in out and "claude mcp remove slack --scope user" in out
    assert connectors() == [("slack", "slack", "slack", 1)]
    assert "slack" in world["fake"].state()["<unset>"]
    assert ("easy-personal/slack", "connector") in world["kr"].items


def test_only_the_default_account_is_touched(world, capsys):
    seed(world, other=True)
    run(capsys, "connector", "adopt-legacy", "--yes")
    state = world["fake"].state()
    assert state["<unset>"] == {} and state["/x/.claude-work"] == {"slack": LAUNCH}


def test_only_catalog_ids_count_and_stored_connectors_are_skipped(world, capsys):
    seed(world)
    world["kr"].items[("easy-personal/notacatalogid", "connector")] = "{}"
    run(capsys, "connector", "adopt-legacy", "--yes")
    assert [c[0] for c in connectors()] == ["slack"]
    with store.opened(config.store_path()) as conn:
        assert legacy.detect(conn) == []


def test_missing_keychain_backend_refuses_with_guidance(world, no_keyring, capsys):
    seed(world, keychain=False)
    rc, _, err = run(capsys, "connector", "adopt-legacy", "--yes")
    assert rc == 1 and "keychain" in err and connectors() == []


def test_doctor_mentions_pending_legacy_connectors(world, capsys):
    seed(world)
    _, out, _ = run(capsys, "doctor")
    assert "legacy" in out and "adopt-legacy" in out
    run(capsys, "connector", "adopt-legacy", "--yes")
    _, out, _ = run(capsys, "doctor")
    assert "adopt-legacy" not in out


def test_adopted_connector_can_be_removed_and_keeps_its_old_name_as_keychain_key(world, capsys):
    seed(world, claude=False)
    run(capsys, "connector", "adopt-legacy", "--yes")
    rc, _, _ = run(capsys, "connector", "remove", "slack", "--yes")
    assert rc == 0 and connectors() == [] and world["kr"].items == {}


def test_hand_made_entry_with_keychain_item_is_adopted_but_never_removed(world, capsys):
    seed(world, hand=True)
    rc, out, _ = run(capsys, "connector", "adopt-legacy", "--yes")
    assert rc == 0 and "slack is not an easy-personal entry; left as it is" in out
    assert connectors() == [("slack", "slack", "slack", 1)]
    assert world["fake"].write_records() == [] and world["fake"].state()["<unset>"] == {"slack": HAND}


def test_hand_made_entry_without_keychain_item_is_not_legacy(world, capsys):
    seed(world, keychain=False, hand=True)
    rc, out, _ = run(capsys, "connector", "adopt-legacy", "--yes")
    assert rc == 0 and "nothing to adopt" in out and connectors() == []
    assert world["fake"].write_records() == []
