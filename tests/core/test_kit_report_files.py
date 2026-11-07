"""HUN-242 (FR-090): `kit.report_files` and `kit.report_file` = list and read the files of a report folder, read-only.

The server finds the repo and the folder itself; every miss is `kit.ReportNotFound`, a file over 10 MB is
`kit.ReportTooLarge`. No symlink is followed at any level; nothing is written.
"""

import hashlib
import os

import pytest

from easy_personal.core import kit, workspaces

NEWEST = 1_767_268_800
OLDER = NEWEST - 86_400
CAP = 10 * 1024 * 1024


@pytest.fixture(autouse=True)
def _home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))


@pytest.fixture
def repo(conn, tmp_path):
    path = tmp_path.resolve() / "repos" / "app"
    (path / "reports").mkdir(parents=True)
    workspaces.add(conn, "acme", repos=[str(path)])
    return path


def _put(folder, name, data=b"x", mtime=OLDER):
    file = folder / name
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_bytes(data)
    os.utime(file, (mtime, mtime))
    return file


def _list(conn, folder="reports", repo_no=0, workspace="acme"):
    return kit.report_files(conn, workspace, repo_no, folder)


def _read(conn, path, folder="reports", repo_no=0, workspace="acme"):
    return kit.report_file(conn, workspace, repo_no, folder, path)


def test_ac1_repo_no_tells_two_repos_with_one_folder_name_apart(conn, tmp_path):
    one, two = (tmp_path.resolve() / "repos" / n for n in ("one", "two"))
    for r in (one, two):
        (r / "reports").mkdir(parents=True)
    workspaces.add(conn, "w", repos=[str(one), str(two)])
    (folders,) = [w["folders"] for w in kit.report_folders(conn)["workspaces"] if w["name"] == "w"]
    assert [(f["repo"], f["repo_no"]) for f in folders] == [("one", 0), ("two", 1)]


def test_ac2_list_newest_first_relative_paths_openable_depth_and_cap(conn, repo):
    reports = repo / "reports"
    _put(reports, "a.html", mtime=NEWEST)
    _put(reports, "sub/b.md", mtime=OLDER)
    _put(reports, "sub/deep/c.json", mtime=OLDER - 10)
    _put(reports, "x.bin", mtime=OLDER - 20)
    _put(reports, "sub/deep/more/d.txt")  # depth 4
    out = _list(conn)
    assert out["truncated"] is False
    assert [(f["path"], f["openable"]) for f in out["files"]] == [
        ("a.html", True), ("sub/b.md", True), ("sub/deep/c.json", True), ("x.bin", False),
    ]  # fmt: skip
    assert set(out["files"][0]) == {"path", "size", "mtime_utc", "openable"}
    assert out["files"][0]["mtime_utc"] == "2026-01-01T12:00:00Z" and out["files"][0]["size"] == 1


def test_ac2_more_than_200_files_are_cut_and_truncated(conn, repo):
    for n in range(201):
        _put(repo / "reports", f"{n}.txt", mtime=OLDER + n)
    out = _list(conn)
    assert len(out["files"]) == 200 and out["truncated"] is True and out["files"][0]["path"] == "200.txt"
    (repo / "reports" / "200.txt").unlink()
    assert _list(conn)["truncated"] is False


def test_ac2_a_file_over_the_cap_is_listed_but_not_openable(conn, repo):
    big = _put(repo / "reports", "big.html")
    os.truncate(big, CAP + 1)
    _put(repo / "reports", "edge.html")
    os.truncate(repo / "reports" / "edge.html", CAP)
    got = {f["path"]: f["openable"] for f in _list(conn)["files"]}
    assert got == {"big.html": False, "edge.html": True}


def test_ac3_symlinks_and_special_files_are_not_listed(conn, repo, tmp_path):
    reports = repo / "reports"
    outside = tmp_path / "outside"
    _put(outside, "secret.txt")
    _put(reports, "real.txt")
    (reports / "link.txt").symlink_to(outside / "secret.txt")
    (reports / "linkdir").symlink_to(outside, target_is_directory=True)
    os.mkfifo(reports / "pipe.txt")
    assert [f["path"] for f in _list(conn)["files"]] == ["real.txt"]


def test_ac3_an_empty_folder_lists_nothing(conn, repo):
    assert _list(conn) == {"files": [], "truncated": False}


@pytest.mark.parametrize(
    ("name", "ctype"),
    [
        ("a.html", "text/html; charset=utf-8"), ("a.htm", "text/html; charset=utf-8"),
        ("a.md", "text/markdown; charset=utf-8"), ("a.txt", "text/plain; charset=utf-8"),
        ("a.json", "application/json"), ("a.png", "image/png"), ("a.jpg", "image/jpeg"),
        ("a.jpeg", "image/jpeg"), ("a.pdf", "application/pdf"), ("A.HTML", "text/html; charset=utf-8"),
    ],
)  # fmt: skip
def test_ac4_bytes_and_content_type_come_from_the_extension(conn, repo, name, ctype):
    data = b"<script>1</script>\x00\xff"
    _put(repo / "reports", name, data)
    assert _read(conn, name) == (ctype, data)


def test_ac4_odd_names_and_sub_folders_are_read(conn, repo):
    for name in ("a b#c?d.txt", "é ü.txt", "sub dir/x y.md"):
        _put(repo / "reports", name, name.encode())
        assert _read(conn, name)[1] == name.encode()


@pytest.mark.parametrize(
    "path",
    ["../x.txt", "a/../../x.txt", "/etc/passwd", "a\\b.txt", "a\0.txt", "", ".", "..", "a//b.txt", "./a.txt",
     "/".join(["d"] * 8) + "/x.txt", "d/" * 9 + "x.txt", "a" * 600 + ".txt", "d1/d2/d3/x.txt", 5, None],
)  # fmt: skip
def test_ac5_path_attacks_are_not_found(conn, repo, path):
    _put(repo / "reports", "a.txt")
    _put(repo, "x.txt")
    _put(repo / "reports", "d1/d2/d3/x.txt")  # depth 4: past the limit
    with pytest.raises(kit.ReportNotFound):
        _read(conn, path)


def test_ac5_symlinks_and_a_fifo_are_not_found(conn, repo, tmp_path):
    reports = repo / "reports"
    outside = tmp_path / "outside"
    _put(outside, "secret.txt", b"SECRET")
    _put(outside, "sub/in.txt", b"SECRET")
    (reports / "link.txt").symlink_to(outside / "secret.txt")
    (reports / "linkdir").symlink_to(outside, target_is_directory=True)
    os.mkfifo(reports / "pipe.txt")
    for path in ("link.txt", "linkdir/sub/in.txt", "pipe.txt"):
        with pytest.raises(kit.ReportNotFound):
            _read(conn, path)


def test_ac5_a_directory_is_not_found(conn, repo):
    (repo / "reports" / "dir.txt").mkdir()
    with pytest.raises(kit.ReportNotFound):
        _read(conn, "dir.txt")


def test_ac6_resolution_misses_are_not_found(conn, repo, tmp_path):
    _put(repo / "reports", "a.txt")
    _put(repo / "src", "a.txt")
    elsewhere = tmp_path.resolve() / "elsewhere"
    _put(elsewhere, "a.txt")
    (repo / "linked-reports").symlink_to(elsewhere, target_is_directory=True)
    workspaces.add(conn, "gone", repos=[str(tmp_path.resolve() / "repos" / "nope")])
    cases = [
        ("nope", 0, "reports"), ("orphan", 0, "reports"), ("acme", -1, "reports"), ("acme", 99, "reports"),
        ("acme", "abc", "reports"), ("acme", None, "reports"), ("acme", True, "reports"),
        ("acme", 0, "src"), ("acme", 0, "reports/.."), ("acme", 0, ".."), ("acme", 0, "../src"), ("acme", 0, ""),
        ("acme", 0, "linked-reports"), ("gone", 0, "reports"), ("acme", 0, "nothing-reports"), (None, 0, "reports"),
    ]  # fmt: skip
    for workspace, repo_no, folder in cases:
        with pytest.raises(kit.ReportNotFound):
            kit.report_files(conn, workspace, repo_no, folder)
        with pytest.raises(kit.ReportNotFound):
            kit.report_file(conn, workspace, repo_no, folder, "a.txt")
    assert _read(conn, "a.txt")[1] == b"x"  # the same call with valid input works


def test_ac6_only_the_first_eight_report_folders_of_a_repo_resolve(conn, repo):
    for n in range(9):  # sorted: r0..r8 then "reports"; the first eight are r0..r7
        _put(repo / f"r{n}-reports", "a.txt")
    assert _read(conn, "a.txt", folder="r7-reports")[1] == b"x"
    with pytest.raises(kit.ReportNotFound):
        _read(conn, "a.txt", folder="r8-reports")


@pytest.mark.parametrize("name", ["a.svg", "a.py", "noext", "a.html.bak", ".html", "a."])
def test_ac7_extensions_outside_the_allow_list_are_not_found(conn, repo, name):
    _put(repo / "reports", name)
    with pytest.raises(kit.ReportNotFound):
        _read(conn, name)


def test_ac7_size_cap_413_and_unreadable_404(conn, repo):
    reports = repo / "reports"
    over = _put(reports, "over.txt")
    os.truncate(over, CAP + 1)
    exact = _put(reports, "exact.txt")
    os.truncate(exact, CAP)
    with pytest.raises(kit.ReportTooLarge):
        _read(conn, "over.txt")
    assert len(_read(conn, "exact.txt")[1]) == CAP
    locked = _put(reports, "locked.txt")
    locked.chmod(0)
    try:
        if os.access(locked, os.R_OK):
            pytest.skip("running as a user that ignores file modes")
        with pytest.raises(kit.ReportNotFound):
            _read(conn, "locked.txt")
    finally:
        locked.chmod(0o600)


def test_the_error_text_never_echoes_input(conn, repo):
    for call in (lambda: _read(conn, "../SECRET-NAME.txt"), lambda: _list(conn, folder="SECRET-FOLDER")):
        with pytest.raises(kit.ReportNotFound) as err:
            call()
        assert "SECRET" not in str(err.value) and str(repo) not in str(err.value)


def _snapshot(root, store_file):
    tree = sorted((str(p), p.lstat().st_mtime_ns, p.lstat().st_size) for p in root.rglob("*"))
    return tree, hashlib.sha256(store_file.read_bytes()).hexdigest()


def test_ac10_nothing_is_written(conn, repo, tmp_path):
    _put(repo / "reports", "a.html")
    _put(repo / "reports", "sub/b.md")
    store_file = tmp_path / "store.sqlite"
    conn.commit()
    before = _snapshot(repo, store_file)
    _list(conn)
    _read(conn, "a.html")
    with pytest.raises(kit.ReportNotFound):
        _read(conn, "../x")
    assert _snapshot(repo, store_file) == before
