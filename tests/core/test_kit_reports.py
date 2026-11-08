"""HUN-240 (FR-089): `kit.report_folders` = the report folders of each workspace's repos, for reference only.

A report folder sits directly inside a workspace repo (depth 1, no symlink followed) and has `report` in
its lowercase name. Files are counted by name and `stat` only: none is opened, nothing is written.
"""

import builtins
import hashlib
import os
import statistics
import time
from pathlib import Path

import pytest

from easy_personal.core import kit, workspaces

NEWEST = 1_767_268_800  # 2026-01-01T12:00:00Z
OLDER = NEWEST - 86_400


@pytest.fixture(autouse=True)
def _home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))


def _repo(tmp_path, name, *, where="repos"):
    path = tmp_path.resolve() / where / name
    path.mkdir(parents=True)
    return path


def _files(folder: Path, *names, mtime=OLDER):
    folder.mkdir(parents=True, exist_ok=True)
    for name in names:
        file = folder / name
        file.write_text("x")
        os.utime(file, (mtime, mtime))


def _folders(out, workspace):
    return {(f["repo"], f["folder"]): f for w in out["workspaces"] if w["name"] == workspace for f in w["folders"]}


def test_ac1_one_entry_per_report_folder_with_counts_and_newest_time(conn, tmp_path):
    app, lib = _repo(tmp_path, "app"), _repo(tmp_path, "lib")
    _files(app / "reports", "a.html", "session-1.json", "session-2.json", mtime=OLDER)
    os.utime(app / "reports" / "a.html", (NEWEST, NEWEST))
    _files(app / "smoke-reports", "s.txt")
    _files(lib / "src", "m.py")
    workspaces.add(conn, "ws-a", repos=[str(app), str(lib)])
    out = kit.report_folders(conn)
    assert [w["name"] for w in out["workspaces"]] == ["ws-a"] and set(out["workspaces"][0]) == {"name", "folders"}
    found = _folders(out, "ws-a")
    assert set(found) == {("app", "reports"), ("app", "smoke-reports")}  # lib adds nothing
    assert found[("app", "reports")] == {
        "repo": "app", "repo_no": 0, "folder": "reports", "files": 3, "files_capped": False,
        "easy_ui_sessions": 2, "newest_utc": "2026-01-01T12:00:00Z",
    }  # fmt: skip
    assert found[("app", "smoke-reports")]["files"] == 1 and found[("app", "smoke-reports")]["easy_ui_sessions"] == 0
    assert found[("app", "smoke-reports")]["newest_utc"] == "2025-12-31T12:00:00Z"


def test_ac2_name_match_is_a_lowercase_report_substring_at_depth_one(conn, tmp_path):
    app = _repo(tmp_path, "app")
    for name in ("Reports", "report", "test-reports", "reporting-tools", "src", "docs"):
        _files(app / name, "f.txt")
    _files(app / "docs" / "reports", "deep.txt")  # depth 2: not a candidate
    (app / "Report-file").write_text("a plain file, not a folder")  # a file is not a folder
    (app / "reports.json").write_text("x")
    workspaces.add(conn, "w", repos=[str(app)])
    names = {f["folder"] for f in kit.report_folders(conn)["workspaces"][0]["folders"]}
    assert names == {"Reports", "report", "test-reports", "reporting-tools"}


def test_ac3_a_symlinked_folder_or_file_is_never_listed_walked_or_counted(conn, tmp_path):
    app = _repo(tmp_path, "app")
    outside = _repo(tmp_path, "outside", where="elsewhere")
    _files(outside, "secret-1.txt", "secret-2.txt", "session-9.json")
    (app / "reports").symlink_to(outside, target_is_directory=True)
    _files(app / "smoke-reports", "real.txt")
    (app / "smoke-reports" / "link.txt").symlink_to(outside / "secret-1.txt")
    (app / "smoke-reports" / "linkdir").symlink_to(outside, target_is_directory=True)
    workspaces.add(conn, "w", repos=[str(app)])
    found = _folders(kit.report_folders(conn), "w")
    assert set(found) == {("app", "smoke-reports")}
    assert found[("app", "smoke-reports")]["files"] == 1 and found[("app", "smoke-reports")]["easy_ui_sessions"] == 0


def test_ac4_no_repo_gone_repo_unreadable_folder_and_orphan_cause_no_error(conn, tmp_path):
    app = _repo(tmp_path, "app")
    _files(app / "reports", "a.txt")
    _files(app / "locked-reports", "b.txt")
    gone = tmp_path.resolve() / "repos" / "gone"
    workspaces.add(conn, "w", repos=[str(app), str(gone)])
    workspaces.add(conn, "none", repos=[])
    (app / "locked-reports").chmod(0)
    try:
        out = kit.report_folders(conn)
    finally:
        (app / "locked-reports").chmod(0o700)
    assert [w["name"] for w in out["workspaces"]] == ["none", "w"]  # orphan is left out; list_all order
    assert out["workspaces"][0]["folders"] == []
    assert set(_folders(out, "w")) == {("app", "reports")}  # gone repo and unreadable folder skipped


def test_ac5_cap_empty_folder_and_depth(conn, tmp_path):
    app = _repo(tmp_path, "app")
    _files(app / "big-reports", *(f"{n}.txt" for n in range(2001)))
    (app / "report").mkdir()  # empty
    _files(app / "exact-reports", *(f"{n}.txt" for n in range(2000)))
    _files(app / "reports" / "d2", "two.txt")
    _files(app / "reports" / "d2" / "d3", "three.txt")
    _files(app / "reports" / "d2" / "d3" / "d4", "four.txt")  # a file at depth 4
    _files(app / "reports", "one.txt")
    workspaces.add(conn, "w", repos=[str(app)])
    found = _folders(kit.report_folders(conn), "w")
    assert (found[("app", "big-reports")]["files"], found[("app", "big-reports")]["files_capped"]) == (2000, True)
    assert (found[("app", "exact-reports")]["files"], found[("app", "exact-reports")]["files_capped"]) == (2000, False)
    assert found[("app", "report")] == {
        "repo": "app", "repo_no": 0, "folder": "report", "files": 0, "files_capped": False, "easy_ui_sessions": 0, "newest_utc": None,
    }  # fmt: skip
    assert found[("app", "reports")]["files"] == 3  # one + two + three; four sits at depth 4


def test_many_subfolders_never_exhaust_file_descriptors(conn, tmp_path):
    """Review finding: a report folder with many sub-folders must not hold one open handle per sub-folder."""
    import resource

    app = _repo(tmp_path, "app")
    for n in range(300):
        _files(app / "reports" / f"d{n}", "a.txt")
    workspaces.add(conn, "w", repos=[str(app)])
    soft, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
    resource.setrlimit(resource.RLIMIT_NOFILE, (min(soft, 120), hard))
    try:
        found = _folders(kit.report_folders(conn), "w")
    finally:
        resource.setrlimit(resource.RLIMIT_NOFILE, (soft, hard))
    assert found[("app", "reports")]["files"] == 300


def test_ac6_a_repo_in_two_workspaces_is_in_both_and_at_most_eight_folders_sorted(conn, tmp_path):
    app, other = _repo(tmp_path, "app"), _repo(tmp_path, "other")
    for n in range(10):
        _files(app / f"r{n}-reports", "f.txt")
    _files(other / "reports", "f.txt")
    workspaces.add(conn, "one", repos=[str(app)])
    workspaces.add(conn, "two", repos=[str(app), str(other)])
    out = kit.report_folders(conn)
    one, two = ({w["name"]: w for w in out["workspaces"]}[n]["folders"] for n in ("one", "two"))
    assert [f["folder"] for f in one] == [f"r{n}-reports" for n in range(8)]  # the first 8 by name
    assert [(f["repo"], f["folder"]) for f in two] == [("app", f"r{n}-reports") for n in range(8)] + [("other", "reports")]


def test_two_repos_with_one_folder_name_stay_two_entries(conn, tmp_path):
    first, second = _repo(tmp_path, "app", where="a"), _repo(tmp_path, "app", where="b")
    _files(first / "reports", "x.txt")
    _files(second / "reports", "x.txt", "y.txt")
    workspaces.add(conn, "w", repos=[str(first), str(second)])
    folders = kit.report_folders(conn)["workspaces"][0]["folders"]
    assert sorted(f["files"] for f in folders) == [1, 2] and {f["repo"] for f in folders} == {"app"}


def test_ac9_160_repos_answer_in_under_two_seconds(conn, tmp_path):
    timings = []
    for number in range(10):
        repos = []
        for index in range(16):
            repo = _repo(tmp_path, f"repo-{number}-{index}")
            _files(repo / "reports", *(f"f{n}.txt" for n in range(50)))
            _files(repo / "smoke-reports", *(f"session-{n}.json" for n in range(50)))
            _files(repo / "src", "m.py")
            repos.append(str(repo))
        workspaces.add(conn, f"ws-{number}", repos=repos)
    for _ in range(3):
        start = time.perf_counter()
        out = kit.report_folders(conn)
        timings.append(time.perf_counter() - start)
    assert sum(len(w["folders"]) for w in out["workspaces"]) == 320
    print(f"report_folders on 160 repos / 10 workspaces / 2 folders x 50 files: {timings} median {statistics.median(timings):.4f}s")
    assert statistics.median(timings) < 2


def test_ac10_no_file_is_opened_and_nothing_is_written(conn, tmp_path, monkeypatch):
    app = _repo(tmp_path, "app")
    _files(app / "reports", "a.html", "session-1.json")
    workspaces.add(conn, "w", repos=[str(app)])
    conn.commit()

    def tree():
        return sorted((str(p), p.stat().st_mtime_ns, p.stat().st_size) for p in tmp_path.rglob("*") if not p.name.startswith("store.sqlite"))

    def store_hash():
        return hashlib.sha256((tmp_path / "store.sqlite").read_bytes()).hexdigest()

    before_tree, before_hash = tree(), store_hash()

    def refuse(*args, **kwargs):
        raise AssertionError("a file was opened")

    monkeypatch.setattr(builtins, "open", refuse)
    monkeypatch.setattr(Path, "read_text", refuse)
    monkeypatch.setattr(Path, "read_bytes", refuse)
    monkeypatch.setattr(Path, "open", refuse)
    monkeypatch.setattr(os, "open", refuse)
    out = kit.report_folders(conn)
    monkeypatch.undo()
    assert out["workspaces"][0]["folders"][0]["files"] == 2
    assert tree() == before_tree and store_hash() == before_hash


def test_the_result_holds_only_names_numbers_and_times(conn, tmp_path):
    app = _repo(tmp_path, "app")
    _files(app / "reports" / "CANARY-sub", "CANARY-name.txt")
    workspaces.add(conn, "w", repos=[str(app)])
    text = str(kit.report_folders(conn))
    assert "CANARY" not in text and str(tmp_path) not in text


# --- an mtime outside what `datetime` can represent must not break the folder list ---

UNREADABLE = 10**17  # far past year 9999: `datetime.fromtimestamp` raises on it


def _make_unreadable(file):
    """Give `file` an mtime `datetime` cannot represent, or skip if the filesystem will not keep it."""
    try:
        os.utime(file, (UNREADABLE, UNREADABLE))
    except (OSError, OverflowError):
        pytest.skip("the filesystem will not store an out-of-range mtime")
    if os.lstat(file).st_mtime < UNREADABLE / 2:
        pytest.skip("the filesystem clamped the out-of-range mtime")


def test_unreadable_mtime_keeps_the_folder_listed_with_the_newest_readable_time(conn, tmp_path):
    app = _repo(tmp_path, "app")
    _files(app / "reports", "a.html", mtime=NEWEST)
    _files(app / "reports", "bad.txt", mtime=OLDER)
    _make_unreadable(app / "reports" / "bad.txt")
    workspaces.add(conn, "ws-a", repos=[str(app)])

    found = _folders(kit.report_folders(conn), "ws-a")  # must not raise

    assert found[("app", "reports")]["files"] == 2  # the bad file is still counted
    assert found[("app", "reports")]["newest_utc"] == "2026-01-01T12:00:00Z"  # the newest readable one


def test_newest_utc_is_null_when_no_file_has_a_readable_time(conn, tmp_path):
    app = _repo(tmp_path, "app")
    _files(app / "reports", "only.txt", mtime=OLDER)
    _make_unreadable(app / "reports" / "only.txt")
    workspaces.add(conn, "ws-a", repos=[str(app)])

    found = _folders(kit.report_folders(conn), "ws-a")  # must not raise

    assert found[("app", "reports")]["files"] == 1
    assert found[("app", "reports")]["newest_utc"] is None
