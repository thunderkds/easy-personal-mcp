"""Agent accounts (HUN-62): detection on a temp root, confirm / ignore / add, missing folders."""

import sqlite3

import pytest

from easy_personal.core import accounts, config, store
from easy_personal.core.migrations import MIGRATIONS


@pytest.fixture
def root():
    return config.agent_root().resolve()


def make_home(root):
    (root / ".claude").mkdir()
    (root / ".claude-work" / "projects").mkdir(parents=True)
    (root / ".claude.json").write_text("{}")
    (root / ".claude.json.backup").write_text("{}")
    (root / ".codex").mkdir()
    (root / ".codex" / "config.toml").write_text("")
    (root / ".claude-junk").mkdir()


def by_folder(conn):
    return {a["folder"]: a for a in accounts.list_accounts(conn)}


def test_migration_3_is_registered_additive_table():
    assert 3 in [n for n, _ in MIGRATIONS]


def test_v2_store_upgrades_keeping_rows(tmp_path, fixtures_root):
    from easy_personal.sources import claude_code

    path = tmp_path / "store.sqlite"
    conn = store.connect(path)
    claude_code.ingest(conn, fixtures_root)
    rows = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    conn.close()
    raw = sqlite3.connect(path)
    raw.execute("DROP TABLE agent_accounts")
    raw.execute("PRAGMA user_version = 2")
    raw.commit()
    raw.close()
    conn = store.connect(path)
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == rows > 0
    assert conn.execute("SELECT COUNT(*) FROM agent_accounts").fetchone()[0] == 0
    assert list(tmp_path.glob("*.bak-v2"))
    conn.close()


def test_detects_accounts_and_skips_files_and_empty_folders(conn, root):
    make_home(root)
    accounts.detect(conn)
    found = by_folder(conn)
    assert set(found) == {
        str(root / ".claude"), str((root / ".claude-work").resolve()), str(root / ".codex")
    }
    default = found[str(root / ".claude")]
    assert (default["provider"], default["status"], default["label"], default["config_dir_env"]) == (
        "claude", "confirmed", "Claude default", None
    )
    work = found[str((root / ".claude-work").resolve())]
    assert (work["provider"], work["status"], work["label"]) == ("claude", "detected", None)
    assert work["config_dir_env"] == work["folder"]
    codex = found[str(root / ".codex")]
    assert (codex["provider"], codex["status"], codex["config_dir_env"]) == ("codex", "detected", None)


def test_claude_settings_json_counts_and_empty_root_is_fine(conn, root):
    accounts.detect(conn)
    assert accounts.list_accounts(conn) == []
    (root / ".claude-s").mkdir()
    (root / ".claude-s" / "settings.json").write_text("{}")
    (root / ".codex-empty").mkdir()
    accounts.detect(conn)
    assert [a["folder"] for a in accounts.list_accounts(conn)] == [str(root / ".claude-s")]


def test_detect_is_idempotent_and_keeps_owner_choices(conn, root):
    make_home(root)
    accounts.detect(conn)
    work = str((root / ".claude-work").resolve())
    accounts.confirm(conn, work, "Claude work")
    accounts.ignore(conn, str(root / ".codex"))
    accounts.detect(conn)
    found = by_folder(conn)
    assert len(found) == 3
    assert (found[work]["status"], found[work]["label"]) == ("confirmed", "Claude work")
    assert found[str(root / ".codex")]["status"] == "ignored"


def test_ignored_folder_stays_ignored_after_redetection(conn, root):
    make_home(root)
    accounts.detect(conn)
    accounts.ignore(conn, str(root / ".codex"))
    accounts.detect(conn)
    assert by_folder(conn)[str(root / ".codex")]["status"] == "ignored"


def test_confirm_by_id_and_relabel(conn, root):
    make_home(root)
    accounts.detect(conn)
    acc = by_folder(conn)[str(root / ".codex")]
    accounts.confirm(conn, str(acc["id"]), "Codex main")
    accounts.confirm(conn, str(acc["id"]), "Codex renamed")
    assert by_folder(conn)[acc["folder"]]["label"] == "Codex renamed"


@pytest.mark.parametrize("label", ["", "   ", "x" * 61])
def test_bad_label_refused(conn, root, label):
    make_home(root)
    accounts.detect(conn)
    with pytest.raises(accounts.AccountError):
        accounts.confirm(conn, str(root / ".codex"), label)
    assert by_folder(conn)[str(root / ".codex")]["status"] == "detected"


def test_label_of_60_chars_ok(conn, root):
    make_home(root)
    accounts.detect(conn)
    accounts.confirm(conn, str(root / ".codex"), "x" * 60)


def test_unknown_account_is_not_found(conn):
    with pytest.raises(accounts.NotFound):
        accounts.confirm(conn, "/no/such/folder", "L")
    with pytest.raises(accounts.NotFound):
        accounts.ignore(conn, "999")


def test_missing_folder_is_reported_not_deleted(conn, root):
    make_home(root)
    accounts.detect(conn)
    work = (root / ".claude-work").resolve()
    accounts.confirm(conn, str(work), "Claude work")
    (work / "projects").rmdir()
    work.rmdir()
    accounts.detect(conn)
    found = by_folder(conn)
    assert found[str(work)]["status"] == "missing"
    assert conn.execute("SELECT status FROM agent_accounts WHERE folder = ?", (str(work),)).fetchone() == ("confirmed",)


def test_symlinked_folder_resolves_and_same_real_path_is_one_account(conn, root, tmp_path):
    real = tmp_path / "elsewhere"
    (real / "projects").mkdir(parents=True)
    (root / ".claude-a").symlink_to(real)
    (root / ".claude-b").symlink_to(real)
    accounts.detect(conn)
    assert [a["folder"] for a in accounts.list_accounts(conn)] == [str(real.resolve())]


def test_spaces_and_unicode_folder_names(conn, root):
    (root / ".claude-café perso" / "projects").mkdir(parents=True)
    accounts.detect(conn)
    [acc] = accounts.list_accounts(conn)
    assert acc["folder"].endswith(".claude-café perso")
    accounts.confirm(conn, acc["folder"], "Perso é")
    assert accounts.list_accounts(conn)[0]["label"] == "Perso é"


def test_backup_copy_is_detected_and_can_be_ignored(conn, root):
    (root / ".claude-backup" / "projects").mkdir(parents=True)
    accounts.detect(conn)
    [acc] = accounts.list_accounts(conn)
    assert acc["status"] == "detected"
    accounts.ignore(conn, str(acc["id"]))
    assert accounts.list_accounts(conn)[0]["status"] == "ignored"


def test_add_by_path_outside_home_needs_content_check(conn, tmp_path):
    good = tmp_path / "other" / "cfg"
    (good / "projects").mkdir(parents=True)
    bad = tmp_path / "other" / "empty"
    bad.mkdir()
    added = accounts.add(conn, str(good))
    assert (added["provider"], added["status"], added["config_dir_env"]) == ("claude", "detected", str(good))
    for refused in (bad, tmp_path / "nope", tmp_path / "other" / "cfg" / "projects" / "x"):
        with pytest.raises(accounts.AccountError):
            accounts.add(conn, str(refused))
    codex = tmp_path / "other" / "cx"
    codex.mkdir()
    (codex / "config.toml").write_text("")
    assert accounts.add(conn, str(codex))["provider"] == "codex"
    assert accounts.add(conn, str(good))["id"] == added["id"]  # same folder twice: one account


def test_add_refuses_a_file(conn, tmp_path):
    f = tmp_path / "x.json"
    f.write_text("{}")
    with pytest.raises(accounts.AccountError):
        accounts.add(conn, str(f))


def test_group_by_provider(conn, root):
    make_home(root)
    accounts.detect(conn)
    grouped = accounts.group_by_provider(accounts.list_accounts(conn))
    assert {k: len(v) for k, v in grouped.items()} == {"claude": 2, "codex": 1}


def test_table_rejects_bad_rows(conn):
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO agent_accounts (provider, folder, status) VALUES ('gemini', '/x', 'detected')")
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO agent_accounts (provider, folder, status) VALUES ('claude', '/x', 'confirmed')")


def test_default_account_cannot_be_ignored(conn, root):
    make_home(root)
    accounts.detect(conn)
    default = by_folder(conn)[str(root / ".claude")]
    for ref in (str(default["id"]), default["folder"]):
        with pytest.raises(accounts.AccountError, match="default account cannot be ignored"):
            accounts.ignore(conn, ref)
    assert by_folder(conn)[default["folder"]]["status"] == "confirmed"


def test_only_the_default_account_is_marked_is_default(conn, root):
    make_home(root)
    accounts.detect(conn)
    found = by_folder(conn)
    assert found[str(root / ".claude")]["is_default"] is True
    others = [a for f, a in found.items() if f != str(root / ".claude")]
    assert others and all(a["is_default"] is False for a in others)


def test_ignored_account_can_be_confirmed_again(conn, root):
    make_home(root)
    accounts.detect(conn)
    acc = by_folder(conn)[str(root / ".claude-work")]
    accounts.ignore(conn, str(acc["id"]))
    assert accounts.list_accounts(conn)[acc["id"] - 1]["status"] == "ignored"
    back = accounts.confirm(conn, str(acc["id"]), "Claude work")
    assert (back["status"], back["label"], back["is_default"]) == ("confirmed", "Claude work", False)
