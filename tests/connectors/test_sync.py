"""Per-account reconcile (HUN-65, FR-033): plan = desired vs registered; apply via `claude mcp` with the
account's CLAUDE_CONFIG_DIR (unset for the default account); resumable; never touches unregistered names.

`claude` is the fake on PATH with a per-account registry (`claude_state`); the keychain is never used here.
"""

import json
import shutil

import pytest

from easy_personal.connectors import sync
from easy_personal.core import accounts, config, store, workspaces

LAUNCHER = "/opt/fake/bin/easy-personal-launch"
SLACK, GMAIL = "c0000aaaa", "c0000bbbb"


@pytest.fixture
def world(tmp_path, monkeypatch, claude_state, launcher):
    """Workspaces company (slack), personal (gmail); accounts default -> personal, work -> company+personal."""
    root = tmp_path / "agents"
    (root / ".claude").mkdir(parents=True)
    (root / ".claude-work" / "projects").mkdir(parents=True)
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(root))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", "/the/runner/own/dir")  # must never leak into a call
    conn = store.connect(config.store_path())
    accounts.detect(conn)
    ids = {a["folder"].rsplit("/", 1)[1]: a["id"] for a in accounts.list_accounts(conn)}
    accounts.confirm(conn, str(ids[".claude-work"]), "Claude work")
    company = workspaces.add(conn, "company")["id"]
    personal = workspaces.add(conn, "personal")["id"]
    with conn:
        conn.execute("INSERT INTO connectors VALUES (?, 'company-slack', 0, 'slack', ?)", (SLACK, company))
        conn.execute("INSERT INTO connectors VALUES (?, 'personal-gmail', 0, 'gmail', ?)", (GMAIL, personal))
    accounts.use(conn, str(ids[".claude"]), ["personal"])
    accounts.use(conn, str(ids[".claude-work"]), ["company", "personal"])
    yield {"conn": conn, "ids": ids, "work_dir": str(root.resolve() / ".claude-work"), "fake": claude_state}
    conn.close()


def lines(steps):
    return [s.line() for s in steps]


def registered(conn):
    return sorted(conn.execute("SELECT account_id, alias, connector_id FROM account_registrations"))


def test_plan_for_two_accounts(world):
    conn, ids = world["conn"], world["ids"]
    assert lines(sync.plan(conn, ids[".claude"])) == [f"Claude default: add personal-gmail -> exec {GMAIL}"]
    assert lines(sync.plan(conn, ids[".claude-work"])) == [
        f"Claude work: add company-slack -> exec {SLACK}",
        f"Claude work: add personal-gmail -> exec {GMAIL}",
    ]
    assert world["fake"].writes() == []  # planning never writes


def test_apply_writes_each_account_with_its_own_config_dir(world):
    conn, fake = world["conn"], world["fake"]
    out = []
    assert sync.apply(conn, sync.plan_all(conn), out.append) == 0
    writes = fake.write_records()
    expected_json = lambda cid: json.dumps({"type": "stdio", "command": LAUNCHER, "args": ["exec", cid]})
    assert writes == [
        ("<unset>", ["mcp", "add-json", "personal-gmail", expected_json(GMAIL), "--scope", "user"]),
        (world["work_dir"], ["mcp", "add-json", "company-slack", expected_json(SLACK), "--scope", "user"]),
        (world["work_dir"], ["mcp", "add-json", "personal-gmail", expected_json(GMAIL), "--scope", "user"]),
    ]
    assert all(r["config_dir"] != "/the/runner/own/dir" for r in fake.records())
    ids = world["ids"]
    assert registered(conn) == sorted([
        (ids[".claude"], "personal-gmail", GMAIL),
        (ids[".claude-work"], "company-slack", SLACK),
        (ids[".claude-work"], "personal-gmail", GMAIL),
    ])
    assert sync.plan_all(conn) == []  # idempotent: nothing left to do
    assert sync.apply(conn, sync.plan_all(conn), out.append) == 0
    assert len(fake.write_records()) == 3


def test_failure_mid_plan_keeps_what_succeeded_and_rerun_finishes(world, monkeypatch):
    conn, fake, ids = world["conn"], world["fake"], world["ids"]
    monkeypatch.setenv("FAKE_CLAUDE_FAIL", "add-json company-slack")  # the 2nd add of the run
    out = []
    assert sync.apply(conn, sync.plan_all(conn), out.append) == 1
    assert registered(conn) == [(ids[".claude"], "personal-gmail", GMAIL)]
    assert any("failed" in ln and "company-slack" in ln for ln in out)
    monkeypatch.delenv("FAKE_CLAUDE_FAIL")
    rest = sync.plan_all(conn)
    assert lines(rest) == [
        f"Claude work: add company-slack -> exec {SLACK}",
        f"Claude work: add personal-gmail -> exec {GMAIL}",
    ]
    assert sync.apply(conn, rest, out.append) == 0
    assert len(registered(conn)) == 3
    adds = [argv[2] for _, argv in fake.write_records() if argv[1] == "add-json"]
    assert adds.count("personal-gmail") == 2 and adds.count("company-slack") == 2  # 1 failed + 1 ok


def test_unpick_removes_only_from_that_account(world):
    conn, fake, ids = world["conn"], world["fake"], world["ids"]
    sync.apply(conn, sync.plan_all(conn), lambda _: None)
    accounts.use(conn, str(ids[".claude-work"]), ["company"])
    steps = sync.plan_all(conn)
    assert lines(steps) == [f"Claude work: remove personal-gmail (exec {GMAIL})"]
    assert sync.apply(conn, steps, lambda _: None) == 0
    assert fake.write_records()[-1] == (world["work_dir"], ["mcp", "remove", "personal-gmail", "--scope", "user"])
    state = fake.state()
    assert "personal-gmail" in state["<unset>"] and "personal-gmail" not in state[world["work_dir"]]


def test_hand_made_entry_with_our_alias_is_refused_never_overwritten(world):
    conn, fake = world["conn"], world["fake"]
    hand = {"type": "stdio", "command": "npx", "args": ["my-own-server"]}
    fake.log.parent.joinpath("claude-state.json").write_text(json.dumps({"<unset>": {"personal-gmail": hand}}))
    steps = sync.plan(conn, world["ids"][".claude"])
    assert [s.action for s in steps] == ["refused"]
    assert "personal-gmail" in steps[0].line() and "did not add" in steps[0].line()
    assert sync.apply(conn, steps, lambda _: None) == 0
    assert fake.state()["<unset>"]["personal-gmail"] == hand and fake.write_records() == []
    assert registered(conn) == []


def test_unregistered_names_are_never_removed(world):
    conn, fake = world["conn"], world["fake"]
    other = {"type": "stdio", "command": "npx", "args": ["x"]}
    fake.log.parent.joinpath("claude-state.json").write_text(json.dumps({world["work_dir"]: {"github": other}}))
    sync.apply(conn, sync.plan_all(conn), lambda _: None)
    accounts.use(conn, str(world["ids"][".claude-work"]), [])
    sync.apply(conn, sync.plan_all(conn), lambda _: None)
    assert fake.state()[world["work_dir"]] == {"github": other}


def test_entry_already_ours_is_recorded_without_a_write(world):
    """A crash after `claude mcp add-json` but before the registration row: the next run records it."""
    conn, fake = world["conn"], world["fake"]
    ours = {"type": "stdio", "command": LAUNCHER, "args": ["exec", GMAIL]}
    fake.log.parent.joinpath("claude-state.json").write_text(json.dumps({"<unset>": {"personal-gmail": ours}}))
    steps = sync.plan(conn, world["ids"][".claude"])
    assert [s.action for s in steps] == ["record"]
    assert sync.apply(conn, steps, lambda _: None) == 0
    assert fake.write_records() == [] and len(registered(conn)) == 1


def test_drift_missing_entry_is_re_added_and_gone_entry_is_forgotten(world):
    conn, fake, ids = world["conn"], world["fake"], world["ids"]
    sync.apply(conn, sync.plan_all(conn), lambda _: None)
    path = fake.log.parent / "claude-state.json"
    state = json.loads(path.read_text())
    del state["<unset>"]["personal-gmail"]  # removed by hand
    del state[world["work_dir"]]["personal-gmail"]
    path.write_text(json.dumps(state))
    steps = sync.plan(conn, ids[".claude"])
    assert [s.action for s in steps] == ["add"] and "missing" in steps[0].line()
    accounts.use(conn, str(ids[".claude-work"]), ["company"])
    steps = sync.plan(conn, ids[".claude-work"])
    assert [s.action for s in steps] == ["forget"]
    assert sync.apply(conn, sync.plan_all(conn), lambda _: None) == 0
    assert "personal-gmail" in fake.state()["<unset>"]
    assert (ids[".claude-work"], "personal-gmail", GMAIL) not in registered(conn)


def test_drift_entry_pointing_elsewhere_is_replaced(world):
    conn, fake = world["conn"], world["fake"]
    sync.apply(conn, sync.plan_all(conn), lambda _: None)
    path = fake.log.parent / "claude-state.json"
    state = json.loads(path.read_text())
    state["<unset>"]["personal-gmail"] = {"type": "stdio", "command": "npx", "args": ["x"]}
    path.write_text(json.dumps(state))
    steps = sync.plan(conn, world["ids"][".claude"])
    assert [s.action for s in steps] == ["remove", "add"]
    assert sync.apply(conn, steps, lambda _: None) == 0
    assert fake.state()["<unset>"]["personal-gmail"]["command"] == LAUNCHER


def test_missing_account_folder_is_skipped_with_a_note(world):
    conn = world["conn"]
    shutil.rmtree(world["work_dir"])
    steps = sync.plan(conn, world["ids"][".claude-work"])
    assert [s.action for s in steps] == ["skipped"] and "folder missing" in steps[0].line()
    assert sync.apply(conn, steps, lambda _: None) == 0 and world["fake"].calls() == []


def test_claude_missing_prints_snippets_and_keeps_registrations(world, monkeypatch, tmp_path):
    conn = world["conn"]
    empty = tmp_path / "empty"
    empty.mkdir()
    monkeypatch.setenv("PATH", str(empty))
    steps = sync.plan_all(conn)
    out = []
    assert sync.apply(conn, steps, out.append) == 0
    text = "\n".join(out)
    assert "not on PATH" in text
    assert "env -u CLAUDE_CONFIG_DIR claude mcp add-json personal-gmail" in text
    assert f"CLAUDE_CONFIG_DIR={world['work_dir']} claude mcp add-json company-slack" in text
    assert registered(conn) == []


def test_codex_and_unconfirmed_accounts_get_no_plan(world, tmp_path):
    conn = world["conn"]
    (tmp_path / "agents" / ".codex").mkdir()
    (tmp_path / "agents" / ".codex" / "config.toml").write_text("")
    accounts.detect(conn)
    codex = next(a["id"] for a in accounts.list_accounts(conn) if a["provider"] == "codex")
    assert [s.action for s in sync.plan(conn, codex)] == ["skipped"]
    assert {s.account_id for s in sync.plan_all(conn)} == set(world["ids"].values())


def test_plan_lines_never_hold_a_secret(world, mem_keyring):
    mem_keyring.items[(f"easy-personal/{SLACK}", "connector")] = '{"env": {"T": "PLANTED-SECRET"}}'
    out = []
    steps = sync.plan_all(world["conn"])
    sync.apply(world["conn"], steps, out.append)
    assert "PLANTED" not in "\n".join(lines(steps) + out)
    assert "PLANTED" not in world["fake"].log.read_text()


def test_ignored_account_gets_its_registered_aliases_removed(world):
    """Supervisor decision 2026-10-26: ignoring an account cleans up the entries we own there."""
    conn, fake, ids = world["conn"], world["fake"], world["ids"]
    sync.apply(conn, sync.plan_all(conn), lambda _: None)
    accounts.ignore(conn, str(ids[".claude-work"]))  # its picks stay in the table but are ignored
    steps = sync.plan(conn, ids[".claude-work"])
    assert lines(steps) == [
        f"Claude work: remove company-slack (exec {SLACK})",
        f"Claude work: remove personal-gmail (exec {GMAIL})",
    ]
    assert sync.plan_all(conn) == steps  # included in a full sync
    assert sync.apply(conn, steps, lambda _: None) == 0
    assert fake.write_records()[-2:] == [
        (world["work_dir"], ["mcp", "remove", "company-slack", "--scope", "user"]),
        (world["work_dir"], ["mcp", "remove", "personal-gmail", "--scope", "user"]),
    ]
    assert registered(conn) == [(ids[".claude"], "personal-gmail", GMAIL)]
    assert fake.state()[world["work_dir"]] == {}
    assert sync.plan(conn, ids[".claude-work"]) == []  # nothing left: no skipped note either
