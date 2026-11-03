"""`GET /api/dirs` (HUN-203, FR-068): folder names for the workspace form's folder browser.

Read-only, behind the token + Host guard. Folder names only: never file names, contents or symlink targets.
"""

import logging
import os
from pathlib import Path

import pytest
from starlette.testclient import TestClient

from easy_personal.surfaces import api

TOKEN, PORT = "tok-123", 5555
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
FILE_CANARY = "SECRET_FILE_NAME_9f3a"
CONTENT_CANARY = "SECRET_CONTENT_77c1"


@pytest.fixture
def client(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://{HOST}")


@pytest.fixture
def tree(tmp_path):
    root = tmp_path / "tree"
    for name in ("a", "B", ".hidden", "r/.git"):
        (root / name).mkdir(parents=True)
    (root / f"{FILE_CANARY}.txt").write_text(CONTENT_CANARY)
    (root / "a" / "inner.txt").write_text(CONTENT_CANARY)
    return root


def ask(client, path=None, **kw):
    return client.get("/api/dirs", params={} if path is None else {"path": str(path)}, headers=AUTH, **kw)


def test_lists_folders_only_case_insensitive_and_marks_git_repos(client, tree):
    res = ask(client, tree)
    assert res.status_code == 200
    body = res.json()
    assert body["path"] == str(tree)
    assert body["dirs"] == [{"name": "a", "repo": False}, {"name": "B", "repo": False}, {"name": "r", "repo": True}]
    assert "truncated" not in body


def test_without_path_it_lists_the_process_working_directory_at_request_time(client, tree, monkeypatch):
    monkeypatch.chdir(tree)
    assert ask(client).json()["path"] == str(tree)
    monkeypatch.chdir(tree / "a")
    assert ask(client).json()["path"] == str(tree / "a")


def test_empty_path_is_the_same_as_no_path(client, tree, monkeypatch):
    monkeypatch.chdir(tree)
    assert client.get("/api/dirs?path=", headers=AUTH).json()["path"] == str(tree)


def test_parent_and_home(client, tree, monkeypatch):
    monkeypatch.setenv("HOME", str(tree))
    body = ask(client, tree / "a").json()
    assert body["parent"] == str(tree)
    assert body["home"] == str(tree)


def test_root_has_no_parent(client):
    body = ask(client, "/").json()
    assert body["path"] == "/" and body["parent"] is None
    assert body["home"] == str(Path.home())


def test_path_with_spaces_and_unicode_round_trips(client, tmp_path):
    odd = tmp_path / "mon dossier é ü 日本"
    (odd / "sous dossier").mkdir(parents=True)
    body = ask(client, odd).json()
    assert body["path"] == str(odd)
    assert body["dirs"] == [{"name": "sous dossier", "repo": False}]


@pytest.mark.parametrize("path", ["relative/dir", "..", "./x", "~"])
def test_relative_path_is_400(client, path):
    res = client.get("/api/dirs", params={"path": path}, headers=AUTH)
    assert res.status_code == 400
    assert "error" in res.json()


def test_missing_path_and_a_file_are_400_and_the_error_does_not_echo_the_path(client, tree):
    for target in (tree / "nope", tree / f"{FILE_CANARY}.txt"):
        res = ask(client, target)
        assert res.status_code == 400
        assert str(target) not in res.text and FILE_CANARY not in res.text


def test_nul_byte_path_is_400(client):
    assert client.get("/api/dirs", params={"path": "/tmp/a\x00b"}, headers=AUTH).status_code == 400


@pytest.mark.skipif(os.geteuid() == 0, reason="root can read any folder")
def test_unreadable_folder_is_403_with_a_fixed_message(client, tree):
    locked = tree / "locked"
    locked.mkdir()
    locked.chmod(0)
    try:
        res = ask(client, locked)
    finally:
        locked.chmod(0o700)
    assert res.status_code == 403
    assert res.json() == {"error": "Cannot read this folder"}


def test_more_than_500_folders_is_cut_to_500_with_truncated(client, tmp_path):
    big = tmp_path / "big"
    big.mkdir()
    for i in range(520):
        (big / f"d{i:04d}").mkdir()
    body = ask(client, big).json()
    assert len(body["dirs"]) == 500
    assert body["truncated"] is True
    assert body["dirs"][0]["name"] == "d0000" and body["dirs"][-1]["name"] == "d0499"


def test_exactly_500_folders_is_not_truncated(client, tmp_path):
    big = tmp_path / "big"
    big.mkdir()
    for i in range(500):
        (big / f"d{i:04d}").mkdir()
    body = ask(client, big).json()
    assert len(body["dirs"]) == 500 and "truncated" not in body


def test_symlink_to_a_folder_is_listed_a_broken_link_and_a_loop_are_skipped_and_no_target_leaks(client, tmp_path):
    root = tmp_path / "links"
    (root / "real").mkdir(parents=True)
    (root / "link").symlink_to(root / "real")
    (root / "broken").symlink_to(root / "missing")
    (root / "loop").symlink_to(root / "loop")
    (root / "file-link").symlink_to(root / "real" / "x.txt")
    res = ask(client, root)
    assert res.status_code == 200
    assert [d["name"] for d in res.json()["dirs"]] == ["link", "real"]
    assert "missing" not in res.text and "x.txt" not in res.text


def test_a_git_file_marks_a_worktree_as_a_repo(client, tmp_path):
    root = tmp_path / "w"
    (root / "wt").mkdir(parents=True)
    (root / "wt" / ".git").write_text("gitdir: /elsewhere")
    assert ask(client, root).json()["dirs"] == [{"name": "wt", "repo": True}]


def test_file_names_and_contents_never_reach_the_response(client, tree):
    for target in (tree, tree / "a", tree / "r"):
        res = ask(client, target)
        assert FILE_CANARY not in res.text and CONTENT_CANARY not in res.text and "inner.txt" not in res.text
        assert set(res.json()) <= {"path", "parent", "home", "dirs", "truncated"}


def test_the_listing_is_read_only_and_leaves_the_tree_unchanged(client, tree):
    before = sorted(p.relative_to(tree).as_posix() for p in tree.rglob("*"))
    ask(client, tree)
    assert sorted(p.relative_to(tree).as_posix() for p in tree.rglob("*")) == before


def test_the_path_is_never_logged(client, tree, caplog):
    caplog.set_level(logging.DEBUG)
    ask(client, tree)
    ask(client, tree / "nope")
    assert str(tree) not in caplog.text


def test_needs_the_token(client, tree):
    assert client.get("/api/dirs", params={"path": str(tree)}).status_code == 403
    assert client.get("/api/dirs", params={"path": str(tree)}, headers={api.TOKEN_HEADER: "wrong"}).status_code == 403


def test_wrong_host_and_foreign_origin_are_refused(client, tree):
    wrong_host = TestClient(api.create_app(TOKEN, PORT, tree), base_url="http://evil.example:5555")
    assert wrong_host.get("/api/dirs", headers=AUTH).status_code == 403
    assert client.get("/api/dirs", headers=AUTH | {"origin": "http://evil.example"}).status_code == 403


def test_the_token_in_the_url_is_not_accepted_here(client, tree):
    assert client.get("/api/dirs", params={"path": str(tree), "token": TOKEN}).status_code == 403
