"""Move / alias / workspace rename / workspace delete (HUN-66, FR-030, FR-031, FR-033).

Each operation = DB change + the sync plan for the affected accounts, shown and confirmed first.
`claude` is the fake with a per-account registry; the keychain is a recording fake whose call log
must stay empty: these operations never read, write or delete a secret (NFR-002).
"""

import keyring
import pytest
from fake_backend import MemoryKeyring

from easy_personal.connectors import sync, writer
from easy_personal.core import accounts, config, store, workspaces

SLACK, GMAIL = "c0000aaaa", "c0000bbbb"


class RecordingKeyring(MemoryKeyring):
    def __init__(self):
        super().__init__()
        self.calls = []

    def get_password(self, service, username):
        self.calls.append(("get", service))
        return super().get_password(service, username)

    def set_password(self, service, username, password):
        self.calls.append(("set", service))
        super().set_password(service, username, password)

    def delete_password(self, service, username):
        self.calls.append(("delete", service))
        super().delete_password(service, username)


@pytest.fixture
def kr():
    old = keyring.get_keyring()
    backend = RecordingKeyring()
    keyring.set_keyring(backend)
    yield backend
    keyring.set_keyring(old)


@pytest.fixture
def world(tmp_path, monkeypatch, claude_state, launcher, kr):
    """company(slack) + personal(gmail) + side(empty); default picks company, work picks company+side.
    Everything is already synced when a test starts."""
    root = tmp_path / "agents"
    (root / ".claude").mkdir(parents=True)
    (root / ".claude-work" / "projects").mkdir(parents=True)
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(root))
    conn = store.connect(config.store_path())
    accounts.detect(conn)
    ids = {a["folder"].rsplit("/", 1)[1]: a["id"] for a in accounts.list_accounts(conn)}
    accounts.confirm(conn, str(ids[".claude-work"]), "Claude work")
    ws = {n: workspaces.add(conn, n, repos=[f"/work/{n}"])["id"] for n in ("company", "personal", "side")}
    with conn:
        conn.execute("INSERT INTO connectors VALUES (?, 'company-slack', 0, 'slack', ?)", (SLACK, ws["company"]))
        conn.execute("INSERT INTO connectors VALUES (?, 'personal-gmail', 0, 'gmail', ?)", (GMAIL, ws["personal"]))
    accounts.use(conn, str(ids[".claude"]), ["company"])
    accounts.use(conn, str(ids[".claude-work"]), ["company", "side"])
    assert sync.apply(conn, sync.plan_all(conn), lambda _: None) == 0
    claude_state.log.unlink()  # forget the setup calls
    yield {"conn": conn, "ids": ids, "ws": ws, "fake": claude_state, "kr": kr, "root": root,
           "work_dir": str(root.resolve() / ".claude-work")}  # fmt: skip
    conn.close()


def yes():
    return True


def no():
    return False


def run(op, *args, confirm=yes, assume_yes=False):
    out = []
    rc = op(*args, confirm, out.append, assume_yes)
    return rc, out


def row(conn, cid):
    return conn.execute("SELECT alias, alias_custom, workspace_id FROM connectors WHERE id = ?", (cid,)).fetchone()


def regs(conn):
    return sorted(conn.execute("SELECT a.label, r.alias, r.connector_id FROM account_registrations r "
                               "JOIN agent_accounts a ON a.id = r.account_id"))  # fmt: skip


def untouched(w):
    assert w["kr"].calls == []  # the keychain was never read, written or deleted


# ---- move -------------------------------------------------------------------------------------


def test_move_swaps_the_alias_in_every_affected_account_only(world):
    conn, fake = world["conn"], world["fake"]
    rc, out = run(sync.move_connector, conn, SLACK, "side")
    assert rc == 0
    assert row(conn, SLACK) == ("side-slack", 0, world["ws"]["side"])
    writes = fake.write_records()
    assert [(cfg, argv[1], argv[2]) for cfg, argv in writes] == [
        ("<unset>", "remove", "company-slack"),
        (world["work_dir"], "remove", "company-slack"),
        (world["work_dir"], "add-json", "side-slack"),
    ]
    assert f'"args": ["exec", "{SLACK}"]' in writes[2][1][3]  # same id: the launcher key never changes
    assert regs(conn) == [("Claude work", "side-slack", SLACK)]
    assert any("remove company-slack" in ln for ln in out) and any("add side-slack" in ln for ln in out)
    untouched(world)


def test_move_not_confirmed_changes_nothing(world):
    conn, fake = world["conn"], world["fake"]
    rc, out = run(sync.move_connector, conn, SLACK, "side", confirm=no)
    assert rc == 1 and any("aborted" in ln for ln in out)
    assert row(conn, SLACK) == ("company-slack", 0, world["ws"]["company"])
    assert fake.write_records() == [] and len(regs(conn)) == 2
    assert not conn.in_transaction
    untouched(world)


def test_move_assume_yes_skips_the_question(world):
    rc, _ = run(sync.move_connector, world["conn"], SLACK, "side", confirm=no, assume_yes=True)
    assert rc == 0 and row(world["conn"], SLACK)[0] == "side-slack"


def test_move_into_a_workspace_with_the_provider_is_refused(world):
    conn, fake = world["conn"], world["fake"]
    with conn:
        conn.execute("INSERT INTO connectors VALUES ('c0000cccc', 'old-slack', 0, 'slack', 1)")  # 1 = orphan
    before = row(conn, "c0000cccc")
    with pytest.raises(writer.ConnectorError, match="already has a slack connector"):
        run(sync.move_connector, conn, "c0000cccc", "company")
    assert row(conn, "c0000cccc") == before and fake.write_records() == []
    assert not conn.in_transaction
    untouched(world)


def test_move_keeps_a_custom_alias_and_refuses_an_alias_clash(world):
    conn = world["conn"]
    with conn:
        conn.execute("UPDATE connectors SET alias = 'mine', alias_custom = 1 WHERE id = ?", (SLACK,))
    sync.apply(conn, sync.plan_all(conn), lambda _: None)
    rc, _ = run(sync.move_connector, conn, SLACK, "side")
    assert rc == 0 and row(conn, SLACK) == ("mine", 1, world["ws"]["side"])
    assert ("Claude work", "mine", SLACK) in regs(conn)
    with conn:  # an orphan connector whose custom alias is taken by a connector outside orphan
        conn.execute("INSERT INTO connectors VALUES ('c0000dddd', 'personal-gmail', 1, 'slack', 1)")
    with pytest.raises(writer.ConnectorError, match="personal-gmail"):
        run(sync.move_connector, conn, "c0000dddd", "company")
    assert row(conn, "c0000dddd")[2] == 1 and not conn.in_transaction


def test_move_unknown_connector_or_workspace_and_same_workspace(world):
    conn = world["conn"]
    with pytest.raises(writer.ConnectorError, match="no such connector"):
        run(sync.move_connector, conn, "nope", "side")
    with pytest.raises(workspaces.WorkspaceError):
        run(sync.move_connector, conn, SLACK, "missing")
    with pytest.raises(writer.ConnectorError, match="already in"):
        run(sync.move_connector, conn, SLACK, "company")
    assert world["fake"].write_records() == []


def test_move_with_a_missing_account_folder_keeps_the_db_change_and_reports_pending(world):
    conn, fake = world["conn"], world["fake"]
    import shutil

    shutil.rmtree(world["work_dir"])
    rc, out = run(sync.move_connector, conn, SLACK, "side")
    assert rc == 0 and row(conn, SLACK)[0] == "side-slack"
    assert any("skipped" in ln and "folder missing" in ln for ln in out)
    assert any("connector sync" in ln for ln in out)
    assert [r[1][:3] for r in fake.write_records()] == [["mcp", "remove", "company-slack"]]  # default account only
    assert sync.plan_all(conn)  # work account still owes its steps
    untouched(world)


# ---- alias ------------------------------------------------------------------------------------


def test_alias_set_custom_then_reset_to_default(world):
    conn, fake = world["conn"], world["fake"]
    rc, _ = run(sync.set_alias, conn, SLACK, "team-chat")
    assert rc == 0 and row(conn, SLACK)[:2] == ("team-chat", 1)
    assert regs(conn) == [("Claude default", "team-chat", SLACK), ("Claude work", "team-chat", SLACK)]
    assert [(cfg, argv[1], argv[2]) for cfg, argv in fake.write_records()] == [
        ("<unset>", "remove", "company-slack"), ("<unset>", "add-json", "team-chat"),
        (world["work_dir"], "remove", "company-slack"), (world["work_dir"], "add-json", "team-chat"),
    ]
    rc, _ = run(sync.set_alias, conn, SLACK, None)  # reset
    assert rc == 0 and row(conn, SLACK)[:2] == ("company-slack", 0)
    assert regs(conn) == [("Claude default", "company-slack", SLACK), ("Claude work", "company-slack", SLACK)]
    untouched(world)


def test_alias_refusals_change_nothing(world):
    conn = world["conn"]
    with pytest.raises(writer.ConnectorError, match="personal-gmail"):  # names the clashing connector
        run(sync.set_alias, conn, SLACK, "personal-gmail")
    with pytest.raises(writer.ConnectorError, match="too long or has invalid"):
        run(sync.set_alias, conn, SLACK, "Bad Alias")
    with pytest.raises(writer.ConnectorError, match="no such connector"):
        run(sync.set_alias, conn, "nope", "x")
    with conn:
        conn.execute("UPDATE connectors SET workspace_id = 1 WHERE id = ?", (SLACK,))
    with pytest.raises(writer.ConnectorError, match="orphan"):
        run(sync.set_alias, conn, SLACK, None)  # an orphan connector has no default alias
    assert row(conn, SLACK)[0] == "company-slack" and world["fake"].write_records() == []
    untouched(world)


def test_alias_not_confirmed_rolls_back(world):
    conn = world["conn"]
    rc, _ = run(sync.set_alias, conn, SLACK, "team-chat", confirm=no)
    assert rc == 1 and row(conn, SLACK)[:2] == ("company-slack", 0) and not conn.in_transaction


# ---- workspace rename -------------------------------------------------------------------------


def test_rename_moves_default_aliases_and_keeps_custom_ones(world):
    conn, fake = world["conn"], world["fake"]
    with conn:
        conn.execute("INSERT INTO connectors VALUES ('c0000eee1', 'my-mail', 1, 'gmail', ?)", (world["ws"]["company"],))
    sync.apply(conn, sync.plan_all(conn), lambda _: None)
    fake.log.unlink()
    rc, out = run(sync.rename_workspace, conn, world["ws"]["company"], {"name": "firm"})
    assert rc == 0
    assert workspaces.get(conn, world["ws"]["company"])["name"] == "firm"
    assert row(conn, SLACK)[:3] == ("firm-slack", 0, world["ws"]["company"])
    assert row(conn, "c0000eee1")[:2] == ("my-mail", 1)
    assert ("Claude work", "firm-slack", SLACK) in regs(conn) and ("Claude default", "firm-slack", SLACK) in regs(conn)
    assert {r[1][2] for r in fake.write_records() if r[1][1] == "add-json"} == {"firm-slack"}
    assert any("remove company-slack" in ln for ln in out)
    untouched(world)


def test_rename_also_applies_the_other_fields_and_rolls_back_when_not_confirmed(world):
    conn = world["conn"]
    rc, _ = run(sync.rename_workspace, conn, world["ws"]["company"], {"name": "firm", "color": "#112233"}, confirm=no)
    ws = workspaces.get(conn, world["ws"]["company"])
    assert rc == 1 and ws["name"] == "company" and ws["color"] is None
    assert row(conn, SLACK)[0] == "company-slack" and not conn.in_transaction


def test_rename_refused_when_an_alias_would_clash_or_be_too_long(world):
    conn = world["conn"]
    with conn:
        conn.execute("UPDATE connectors SET alias = 'firm-slack', alias_custom = 1 WHERE id = ?", (GMAIL,))
        conn.execute("UPDATE connectors SET workspace_id = ? WHERE id = ?", (world["ws"]["company"], GMAIL))
        conn.execute("UPDATE connectors SET alias = 'company-slack' WHERE id = ?", (SLACK,))
    # company now holds slack (default alias) and gmail (custom alias firm-slack): renaming to `firm` clashes
    with pytest.raises(writer.ConnectorError, match="firm-slack"):
        run(sync.rename_workspace, conn, world["ws"]["company"], {"name": "firm"})
    assert workspaces.get(conn, world["ws"]["company"])["name"] == "company" and row(conn, SLACK)[0] == "company-slack"
    long_name = "a" * 40
    with pytest.raises(writer.ConnectorError, match="too long"):
        # 40 + len("-slack") = 46 is fine; push over 64 with a long provider id
        with conn:
            conn.execute("INSERT INTO connectors VALUES ('c0000eee2', 'company-" + "p" * 30 + "', 0, '" + "p" * 30 + "', ?)", (world["ws"]["company"],))
        run(sync.rename_workspace, conn, world["ws"]["company"], {"name": long_name})
    assert workspaces.get(conn, world["ws"]["company"])["name"] == "company"


def test_rename_of_a_workspace_without_connectors_needs_no_plan(world):
    conn = world["conn"]
    rc, out = run(sync.rename_workspace, conn, world["ws"]["side"], {"name": "extra"}, confirm=no)
    assert rc == 0 and workspaces.get(conn, world["ws"]["side"])["name"] == "extra" and world["fake"].write_records() == []


# ---- workspace delete -------------------------------------------------------------------------


def test_delete_moves_connectors_to_orphan_and_removes_them_everywhere(world):
    conn, fake = world["conn"], world["fake"]
    with conn:
        conn.execute("INSERT INTO connectors VALUES ('c0000eee3', 'company-gmail', 0, 'gmail', ?)", (world["ws"]["company"],))
    sync.apply(conn, sync.plan_all(conn), lambda _: None)
    fake.log.unlink()
    rc, out = run(sync.delete_workspace, conn, world["ws"]["company"])
    assert rc == 0
    orphan = conn.execute("SELECT id FROM workspaces WHERE is_orphan = 1").fetchone()[0]
    assert row(conn, SLACK) == ("company-slack", 0, orphan) and row(conn, "c0000eee3") == ("company-gmail", 0, orphan)
    assert conn.execute("SELECT COUNT(*) FROM workspaces WHERE id = ?", (world["ws"]["company"],)).fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM workspace_repos WHERE workspace_id = ?", (world["ws"]["company"],)).fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM account_workspaces WHERE workspace_id = ?", (world["ws"]["company"],)).fetchone()[0] == 0
    assert regs(conn) == [] and fake.state()["<unset>"] == {}  # gone from every account
    assert sorted(r[1][2] for r in fake.write_records()) == ["company-gmail", "company-gmail", "company-slack", "company-slack"]
    assert all(r[1][1] == "remove" for r in fake.write_records())
    untouched(world)


def test_delete_not_confirmed_changes_nothing(world):
    conn = world["conn"]
    rc, out = run(sync.delete_workspace, conn, world["ws"]["company"], confirm=no)
    assert rc == 1 and workspaces.get(conn, world["ws"]["company"])["name"] == "company"
    assert row(conn, SLACK)[2] == world["ws"]["company"] and len(regs(conn)) == 2
    assert world["fake"].write_records() == [] and not conn.in_transaction


def test_delete_of_a_workspace_nobody_picks_makes_no_claude_call(world):
    conn, fake = world["conn"], world["fake"]
    with conn:
        conn.execute("INSERT INTO connectors VALUES ('c0000eee4', 'personal-slack', 0, 'slack', ?)", (world["ws"]["personal"],))
    rc, _ = run(sync.delete_workspace, conn, world["ws"]["personal"])
    assert rc == 0 and fake.calls() == []
    assert row(conn, "c0000eee4")[2] == conn.execute("SELECT id FROM workspaces WHERE is_orphan = 1").fetchone()[0]
    untouched(world)


def test_delete_refuses_orphan_and_unknown(world):
    conn = world["conn"]
    with pytest.raises(workspaces.WorkspaceError) as exc:
        run(sync.delete_workspace, conn, 1)
    assert exc.value.status == 403
    with pytest.raises(workspaces.WorkspaceError) as exc:
        run(sync.delete_workspace, conn, 999)
    assert exc.value.status == 404


def test_failure_after_the_db_change_is_repaired_by_sync(world, monkeypatch):
    conn, fake = world["conn"], world["fake"]
    monkeypatch.setenv("FAKE_CLAUDE_FAIL", "remove company-slack")
    rc, out = run(sync.delete_workspace, conn, world["ws"]["company"])
    assert rc == 1 and any("failed" in ln for ln in out)
    assert conn.execute("SELECT COUNT(*) FROM workspaces WHERE id = ?", (world["ws"]["company"],)).fetchone()[0] == 0
    monkeypatch.delenv("FAKE_CLAUDE_FAIL")
    assert sync.apply(conn, sync.plan_all(conn), lambda _: None) == 0
    assert regs(conn) == [] and sync.plan_all(conn) == []
    untouched(world)


# ---- assign out of orphan ---------------------------------------------------------------------


def test_orphan_holds_duplicates_and_assigning_out_is_a_move(world):
    conn, fake = world["conn"], world["fake"]
    run(sync.delete_workspace, conn, world["ws"]["company"])
    with conn:  # a second orphan slack with the very same alias is allowed only in orphan
        conn.execute("INSERT INTO connectors VALUES ('c0000eee5', 'company-slack', 0, 'slack', 1)")
    fake.log.unlink()
    rc, out = run(sync.move_connector, conn, SLACK, "side")
    assert rc == 0 and row(conn, SLACK)[:3] == ("side-slack", 0, world["ws"]["side"])
    assert regs(conn) == [("Claude work", "side-slack", SLACK)]
    with pytest.raises(writer.ConnectorError, match="already has a slack connector"):
        run(sync.move_connector, conn, "c0000eee5", "side")
    untouched(world)
