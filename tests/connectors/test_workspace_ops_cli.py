"""CLI `connector move|alias`, `workspace edit --rename|delete` with connectors (HUN-66).

Every operation prints its plan and asks; `--yes` skips the question. Fake `claude`, recording keyring.
"""

import builtins

import pytest

from easy_personal.connectors import sync
from easy_personal.core import accounts, config, store, workspaces
from easy_personal.surfaces import cli

CID = "c0000aaaa"


@pytest.fixture
def world(tmp_path, monkeypatch, claude_state, launcher, mem_keyring):
    root = tmp_path / "agents"
    (root / ".claude").mkdir(parents=True)
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(root))
    monkeypatch.delenv("CLAUDE_CONFIG_DIR", raising=False)
    answer = {"value": "y"}
    monkeypatch.setattr(builtins, "input", lambda prompt="": answer["value"])
    with store.opened(config.store_path()) as conn:
        accounts.detect(conn)
        ws = {n: workspaces.add(conn, n)["id"] for n in ("company", "side")}
        with conn:
            conn.execute("INSERT INTO connectors VALUES (?, 'company-slack', 0, 'slack', ?)", (CID, ws["company"]))
        accounts.use(conn, "1", ["company"])
        assert sync.apply(conn, sync.plan_all(conn), lambda _: None) == 0
    claude_state.log.unlink()
    return {"answer": answer, "fake": claude_state, "keyring": mem_keyring}


def run(capsys, *argv):
    rc = cli.main(list(argv))
    out = capsys.readouterr()
    return rc, out.out, out.err


def stored():
    with store.opened(config.store_path()) as conn:
        return conn.execute("SELECT alias, alias_custom FROM connectors WHERE id = ?", (CID,)).fetchone()


def test_move_shows_the_plan_and_asks(world, capsys):
    world["answer"]["value"] = "n"
    rc, out, _ = run(capsys, "connector", "move", CID, "side")
    assert rc == 1 and "remove company-slack" in out and "aborted" in out
    assert stored() == ("company-slack", 0) and world["fake"].write_records() == []
    world["answer"]["value"] = "y"
    rc, out, _ = run(capsys, "connector", "move", CID, "side")
    assert rc == 0 and stored() == ("side-slack", 0)
    assert [a[1:3] for _, a in world["fake"].write_records()] == [["remove", "company-slack"]]
    assert world["keyring"].items == {}


def test_alias_and_default_and_usage_errors(world, capsys):
    rc, _, _ = run(capsys, "connector", "alias", CID, "team-chat", "--yes")
    assert rc == 0 and stored() == ("team-chat", 1)
    rc, _, _ = run(capsys, "connector", "alias", CID, "--default", "--yes")
    assert rc == 0 and stored() == ("company-slack", 0)
    for argv in (["alias", CID], ["alias", CID, "x", "--default"], ["alias", CID, "Bad Alias", "--yes"],
                 ["move", "nope", "side", "--yes"], ["move", CID, "missing", "--yes"], ["move", CID, "company", "--yes"]):  # fmt: skip
        rc, _, err = run(capsys, "connector", *argv)
        assert rc == 2 and err.startswith("easy-personal: "), argv
    assert stored() == ("company-slack", 0)


def test_rename_and_delete_workspace_with_a_connector(world, capsys):
    rc, out, _ = run(capsys, "workspace", "edit", "company", "--rename", "firm", "--yes")
    assert rc == 0 and "company-slack -> firm-slack" in out and stored() == ("firm-slack", 0)
    world["answer"]["value"] = "n"
    rc, out, _ = run(capsys, "workspace", "delete", "firm")
    assert rc == 1 and any(w["name"] == "firm" for w in _workspaces())
    world["answer"]["value"] = "y"
    rc, out, _ = run(capsys, "workspace", "delete", "firm")
    assert rc == 0 and "connectors to orphan" in out and stored() == ("firm-slack", 0)
    assert [w["name"] for w in _workspaces()] == ["orphan", "side"]
    assert world["fake"].state()["<unset>"] == {} and world["keyring"].items == {}
    rc, out, _ = run(capsys, "connector", "move", CID, "side", "--yes")  # assign out of orphan
    assert rc == 0 and stored() == ("side-slack", 0)


def _workspaces():
    with store.opened(config.store_path()) as conn:
        return workspaces.list_all(conn)
