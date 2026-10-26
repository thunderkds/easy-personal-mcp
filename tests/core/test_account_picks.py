"""`account use` (HUN-65, FR-033): a confirmed Claude account picks workspaces; never orphan; Codex refused."""

import os

import pytest

from easy_personal.core import accounts, workspaces


@pytest.fixture
def world(conn, tmp_path, monkeypatch):
    root = tmp_path / "agents"
    for folder in (".claude", ".claude-work/projects", ".claude-new/projects"):
        (root / folder).mkdir(parents=True)
    (root / ".codex").mkdir()
    (root / ".codex" / "config.toml").write_text("")
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(root))
    accounts.detect(conn)
    ids = {os.path.basename(a["folder"]): a["id"] for a in accounts.list_accounts(conn)}
    accounts.confirm(conn, str(ids[".claude-work"]), "Claude work")
    workspaces.add(conn, "company")
    workspaces.add(conn, "personal")
    return ids


def test_use_replaces_picks_and_lists_them(conn, world):
    got = accounts.use(conn, str(world[".claude-work"]), ["company", "personal"])
    assert got["workspaces"] == ["company", "personal"]
    got = accounts.use(conn, str(world[".claude-work"]), ["personal"])
    assert got["workspaces"] == ["personal"]
    listed = {a["id"]: a["workspaces"] for a in accounts.list_accounts(conn)}
    assert listed[world[".claude-work"]] == ["personal"] and listed[world[".claude"]] == []


def test_use_with_no_workspace_clears_picks(conn, world):
    accounts.use(conn, str(world[".claude"]), ["company"])
    assert accounts.use(conn, str(world[".claude"]), [])["workspaces"] == []


def test_codex_account_is_refused(conn, world):
    with pytest.raises(accounts.AccountError, match="confirmed Claude"):
        accounts.use(conn, str(world[".codex"]), ["company"])


def test_unconfirmed_account_is_refused(conn, world):
    with pytest.raises(accounts.AccountError, match="confirmed Claude"):
        accounts.use(conn, str(world[".claude-new"]), ["company"])


def test_orphan_is_refused_and_nothing_changes(conn, world):
    accounts.use(conn, str(world[".claude"]), ["company"])
    with pytest.raises(accounts.AccountError, match="orphan"):
        accounts.use(conn, str(world[".claude"]), ["personal", "orphan"])
    assert accounts.use(conn, str(world[".claude"]), ["company"])["workspaces"] == ["company"]


def test_unknown_workspace_is_refused_without_echo(conn, world):
    with pytest.raises(accounts.AccountError) as exc:
        accounts.use(conn, str(world[".claude"]), ["no-such-PLANTED"])
    assert "PLANTED" not in str(exc.value)


def test_unknown_account_is_not_found(conn, world):
    with pytest.raises(accounts.NotFound):
        accounts.use(conn, "999", ["company"])
