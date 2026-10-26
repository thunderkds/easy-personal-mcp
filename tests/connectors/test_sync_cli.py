"""CLI: `account use`, `connector sync [--account] [--yes]`, `connector remove <id>` (HUN-65, FR-033).

Every write shows its plan and needs confirmation. Fake `claude` with per-account state; memory keyring.
"""

import builtins
import getpass
import json
import re

import pytest

from easy_personal.core import accounts, config, store, workspaces
from easy_personal.surfaces import cli

SECRET = "xoxb-PLANTED-secret-123"


@pytest.fixture
def world(tmp_path, monkeypatch, claude_state, launcher, mem_keyring):
    root = tmp_path / "agents"
    (root / ".claude").mkdir(parents=True)
    (root / ".claude-work" / "projects").mkdir(parents=True)
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(root))
    monkeypatch.delenv("CLAUDE_CONFIG_DIR", raising=False)
    state = {"confirm": "y"}
    monkeypatch.setattr(getpass, "getpass", lambda prompt="": SECRET)
    monkeypatch.setattr(builtins, "input", lambda prompt="": state["confirm"])
    conn = store.connect(config.store_path())
    accounts.detect(conn)
    ids = {a["folder"].rsplit("/", 1)[1]: a["id"] for a in accounts.list_accounts(conn)}
    accounts.confirm(conn, str(ids[".claude-work"]), "Claude work")
    workspaces.add(conn, "company")
    conn.close()
    return {"ids": ids, "fake": claude_state, "keyring": mem_keyring, "answers": state,
            "work_dir": str(root.resolve() / ".claude-work")}  # fmt: skip


def run(capsys, *argv):
    rc = cli.main([str(a) for a in argv])
    out = capsys.readouterr()
    return rc, out.out, out.err


def connector_id(capsys):
    rc, out, _ = run(capsys, "connector", "add", "slack", "--workspace", "company", "--yes")
    assert rc == 0
    return re.search(r"\bc[0-9a-f]{8}\b", out)[0]


def registrations():
    with store.opened(config.store_path()) as conn:
        return sorted(conn.execute("SELECT account_id, alias, connector_id FROM account_registrations"))


def test_use_then_sync_shows_plan_asks_and_writes_per_account(world, capsys):
    cid = connector_id(capsys)
    ids = world["ids"]
    rc, out, err = run(capsys, "account", "use", ids[".claude"], "company")
    assert rc == 0 and json.loads(out)["workspaces"] == ["company"] and "connector sync" in err
    run(capsys, "account", "use", ids[".claude-work"], "company")
    world["answers"]["confirm"] = "n"
    rc, out, _ = run(capsys, "connector", "sync")
    assert rc == 1 and "aborted" in out
    assert f"Claude default: add company-slack -> exec {cid}" in out
    assert f"Claude work: add company-slack -> exec {cid}" in out
    assert world["fake"].write_records() == [] and registrations() == []
    world["answers"]["confirm"] = "y"
    rc, out, _ = run(capsys, "connector", "sync")
    assert rc == 0
    assert [d for d, _ in world["fake"].write_records()] == ["<unset>", world["work_dir"]]
    assert len(registrations()) == 2
    rc, out, _ = run(capsys, "connector", "sync", "--yes")
    assert rc == 0 and "nothing to change" in out
    assert SECRET not in out


def test_sync_one_account_only(world, capsys):
    connector_id(capsys)
    for acc in world["ids"].values():
        run(capsys, "account", "use", acc, "company")
    rc, *_ = run(capsys, "connector", "sync", "--account", world["work_dir"], "--yes")
    assert rc == 0 and [d for d, _ in world["fake"].write_records()] == [world["work_dir"]]


def test_sync_unknown_account_is_exit_2(world, capsys):
    rc, _, err = run(capsys, "connector", "sync", "--account", "999", "--yes")
    assert rc == 2 and "no such account" in err


def test_use_codex_or_orphan_is_exit_2(world, capsys):
    rc, _, err = run(capsys, "account", "use", world["ids"][".claude"], "orphan")
    assert rc == 2 and "orphan" in err


def test_remove_unregisters_everywhere_then_deletes_keychain_and_row(world, capsys):
    cid = connector_id(capsys)
    for acc in world["ids"].values():
        run(capsys, "account", "use", acc, "company")
    run(capsys, "connector", "sync", "--yes")
    world["answers"]["confirm"] = "n"
    rc, out, _ = run(capsys, "connector", "remove", cid)
    assert rc == 1 and "aborted" in out and f"remove company-slack (exec {cid})" in out
    assert f"easy-personal/{cid}" in out and len(registrations()) == 2
    world["answers"]["confirm"] = "y"
    rc, out, _ = run(capsys, "connector", "remove", cid)
    assert rc == 0, out
    removes = [(d, a) for d, a in world["fake"].write_records() if a[1] == "remove"]
    assert removes == [("<unset>", ["mcp", "remove", "company-slack", "--scope", "user"]),
                       (world["work_dir"], ["mcp", "remove", "company-slack", "--scope", "user"])]  # fmt: skip
    assert registrations() == [] and world["keyring"].items == {}
    with store.opened(config.store_path()) as conn:
        assert conn.execute("SELECT COUNT(*) FROM connectors").fetchone()[0] == 0


def test_remove_failure_keeps_connector_and_keychain_and_rerun_finishes(world, capsys, monkeypatch):
    cid = connector_id(capsys)
    for acc in world["ids"].values():
        run(capsys, "account", "use", acc, "company")
    run(capsys, "connector", "sync", "--yes")
    state = world["fake"].state()
    del state[world["work_dir"]]  # removed by hand from the work account: the plan will `forget` it
    world["fake"].log.parent.joinpath("claude-state.json").write_text(json.dumps(state))
    monkeypatch.setenv("FAKE_CLAUDE_FAIL", "remove company-slack")  # the default account's remove fails
    rc, out, _ = run(capsys, "connector", "remove", cid, "--yes")
    assert rc == 1 and "failed" in out
    assert len(registrations()) == 2 and len(world["keyring"].items) == 1  # nothing deleted
    monkeypatch.delenv("FAKE_CLAUDE_FAIL")
    rc, out, _ = run(capsys, "connector", "remove", cid, "--yes")
    assert rc == 0 and "forget company-slack" in out
    assert registrations() == [] and world["keyring"].items == {}


def test_remove_unregistered_connector_needs_no_claude(world, capsys, monkeypatch, tmp_path):
    cid = connector_id(capsys)
    empty = tmp_path / "empty"
    empty.mkdir()
    monkeypatch.setenv("PATH", str(empty))
    rc, out, _ = run(capsys, "connector", "remove", cid, "--yes")
    assert "removed" in out
    assert rc == 0 and world["keyring"].items == {}


def test_remove_with_claude_missing_prints_snippets_and_deletes_nothing(world, capsys, monkeypatch, tmp_path):
    cid = connector_id(capsys)
    run(capsys, "account", "use", world["ids"][".claude"], "company")
    run(capsys, "connector", "sync", "--yes")
    empty = tmp_path / "empty"
    empty.mkdir()
    monkeypatch.setenv("PATH", str(empty))
    rc, out, _ = run(capsys, "connector", "remove", cid, "--yes")
    assert rc == 0 and "env -u CLAUDE_CONFIG_DIR claude mcp remove company-slack" in out
    assert len(registrations()) == 1 and len(world["keyring"].items) == 1


def test_remove_pre_workspace_name_keeps_the_old_path(world, capsys):
    world["keyring"].items[("easy-personal/slack", "connector")] = "{}"
    legacy = {"<unset>": {"slack": {"type": "stdio", "command": "x", "args": ["exec", "slack"]}}}
    world["fake"].log.parent.joinpath("claude-state.json").write_text(json.dumps(legacy))
    rc, *_ = run(capsys, "connector", "remove", "slack", "--yes")
    assert rc == 0 and world["keyring"].items == {}
    assert world["fake"].write_records()[-1][1] == ["mcp", "remove", "slack", "--scope", "user"]


def test_doctor_reports_registered_aliases_and_drift_never_removes(world, capsys):
    cid = connector_id(capsys)
    for acc in world["ids"].values():
        run(capsys, "account", "use", acc, "company")
    run(capsys, "connector", "sync", "--yes")
    rc, out, _ = run(capsys, "doctor")
    assert rc == 0, out
    assert f"ok Claude default company-slack: points at the launcher (exec {cid})" in out
    assert f"ok Claude work company-slack: points at the launcher (exec {cid})" in out
    path = world["fake"].log.parent / "claude-state.json"
    state = json.loads(path.read_text())
    del state["<unset>"]["company-slack"]
    state[world["work_dir"]]["company-slack"]["command"] = "npx"
    path.write_text(json.dumps(state))
    before = len(world["fake"].write_records())
    rc, out, _ = run(capsys, "doctor")
    assert rc == 1
    assert "FAIL Claude default company-slack: drift: missing from Claude Code" in out
    assert "FAIL Claude work company-slack: drift: does not point at easy-personal-launch" in out
    assert len(world["fake"].write_records()) == before  # doctor never writes
    assert len(registrations()) == 2
    gets = [r for r in world["fake"].records() if r["argv"][:3] == ["mcp", "get", "company-slack"]]
    assert {r["config_dir"] for r in gets} == {"<unset>", world["work_dir"]}


def test_doctor_flags_a_connector_without_keychain_item(world, capsys):
    cid = connector_id(capsys)
    world["keyring"].items.clear()
    rc, out, _ = run(capsys, "doctor")
    assert rc == 1 and f"FAIL {cid}: no keychain item" in out


def test_remove_works_when_the_holder_account_is_ignored(world, capsys):
    cid = connector_id(capsys)
    for acc in world["ids"].values():
        run(capsys, "account", "use", acc, "company")
    run(capsys, "connector", "sync", "--yes")
    assert run(capsys, "account", "ignore", world["work_dir"])[0] == 0
    rc, out, _ = run(capsys, "connector", "remove", cid, "--yes")
    assert rc == 0, out
    removes = [(d, a[2]) for d, a in world["fake"].write_records() if a[1] == "remove"]
    assert sorted(removes) == sorted([("<unset>", "company-slack"), (world["work_dir"], "company-slack")])
    assert registrations() == [] and world["keyring"].items == {}
