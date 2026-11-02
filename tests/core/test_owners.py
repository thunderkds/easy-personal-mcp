"""HUN-189 (FR-057): a gone worktree is counted under its owner project, at read time, store unchanged."""

import os
import subprocess
from pathlib import Path

import pytest

from easy_personal.core import board, discovery, kit, memgraph, metrics, owners, rules, workspaces

TOKENS = 15  # every event below: input 10 + output 5
CANARY = "LEAKCANARY-owner-0123456789"


def git(repo, *args):
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null"}
    cmd = ["git", "-c", "user.email=t@example.invalid", "-c", "user.name=t", "-C", str(repo), *args]
    subprocess.run(cmd, check=True, capture_output=True, text=True, env=env)


def make_repo(path):
    path.mkdir(parents=True)
    git(path, "init", "-q", "-b", "main")
    git(path, "commit", "-q", "--allow-empty", "-m", "init")
    return path


def merged_branch(repo, branch, subject=None):
    """`branch` merged into main with a merge commit, then deleted: only the merge subject names it."""
    git(repo, "switch", "-q", "-c", branch)
    git(repo, "commit", "-q", "--allow-empty", "-m", "work")
    git(repo, "switch", "-q", "main")
    git(repo, "merge", "-q", "--no-ff", "-m", subject or f"Merge branch '{branch}'", branch)
    git(repo, "branch", "-q", "-d", branch)


def add(conn, n, path, branch=None, model="model-0"):
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc,"
        " session_id, project_path, git_branch, model, message_id, input_tokens, output_tokens,"
        " cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,0,?,?,?,?,?,?,10,5,0,0)",
        (n, f"2026-01-01T00:00:{n:02d}.000000Z", f"s{n}", str(path), branch, model, f"msg{n}"),
    )
    conn.commit()


@pytest.fixture(autouse=True)
def _fresh_cache():
    owners.clear_cache()
    yield
    owners.clear_cache()


@pytest.fixture
def world(tmp_path):
    """Two repos and gone paths: under repo A, sibling worktrees with a branch ref / merge subject / ambiguous name."""
    root = tmp_path.resolve()
    a, b = make_repo(root / "repoA"), make_repo(root / "repoB")
    (a / ".claude" / "worktrees").mkdir(parents=True)
    git(a, "branch", "feat/x")  # a ref in A only
    merged_branch(b, "feat/m")  # a merge subject in B only
    merged_branch(b, "pr-12", "Merge pull request #12 from owner/feat/pr")
    merged_branch(b, "feat/colon", "Merge feat/colon: what it does")
    git(a, "branch", "fix/typo")
    git(b, "branch", "fix/typo")  # the same name in two repos
    return {
        "root": root, "a": a, "b": b,
        "wt_under_a": a / ".claude" / "worktrees" / "x",
        "sub_of_a": a / "sub" / "dir",
        "sib_ref": root / "wt-ref", "sib_merge": root / "wt-merge", "sib_pr": root / "wt-pr", "sib_colon": root / "wt-colon",
        "sib_two": root / "wt-two", "sib_none": root / "wt-none", "sib_main": root / "wt-main",
        "sib_head": root / "wt-head", "sib_empty": root / "wt-empty", "elsewhere": root / "no" / "where" / "wt",
    }  # fmt: skip


def seed(conn, world):
    rows = [
        ("a", None), ("b", None), ("wt_under_a", "feat/zzz"), ("sub_of_a", None),
        ("sib_ref", "feat/x"), ("sib_merge", "feat/m"), ("sib_pr", "feat/pr"), ("sib_colon", "feat/colon"),
        ("sib_two", "fix/typo"), ("sib_none", "feat/nowhere"), ("sib_main", "main"),
        ("sib_head", "HEAD"), ("sib_empty", None), ("elsewhere", "feat/x"),
    ]  # fmt: skip
    for n, (key, branch) in enumerate(rows):
        add(conn, n, world[key], branch)
    return world


def test_rule_1_gone_paths_inside_a_repo_belong_to_it(conn, world):  # AC 1
    seed(conn, world)
    got = owners.owner_map(conn)
    assert got[str(world["wt_under_a"])] == str(world["a"])  # even with a branch that names nothing
    assert got[str(world["sub_of_a"])] == str(world["a"])


def test_rule_2_branch_is_a_ref_or_a_merge_subject_of_one_repo(conn, world):  # AC 2
    seed(conn, world)
    got = owners.owner_map(conn)
    assert got[str(world["sib_ref"])] == str(world["a"])
    assert got[str(world["sib_merge"])] == str(world["b"])
    assert got[str(world["sib_pr"])] == str(world["b"])
    assert got[str(world["sib_colon"])] == str(world["b"])  # the kit's own subject form


def test_rule_3_everything_else_is_unchanged(conn, world):  # AC 3
    seed(conn, world)
    got = owners.owner_map(conn)
    for key in ("sib_two", "sib_none", "sib_main", "sib_head", "sib_empty"):
        assert str(world[key]) not in got, key  # two repos / none / ignored names / empty
    assert str(world["a"]) not in got and str(world["b"]) not in got  # existing folders


def test_rule_2_needs_no_repo_around_the_path(conn, world):
    add(conn, 0, world["elsewhere"], "feat/x")  # its nearest existing parent is no repo
    add(conn, 1, world["a"])
    assert owners.owner_map(conn) == {str(world["elsewhere"]): str(world["a"])}


def test_an_unmatched_tmp_path_is_unchanged(conn, tmp_path):
    add(conn, 0, "/tmp/definitely-gone-hun189", "feat/none")
    assert owners.owner_map(conn) == {}


def test_a_parent_that_is_not_a_repo_gives_no_rule_1(conn, world):
    add(conn, 0, world["sib_none"])  # nearest existing parent is the temp root, no repo
    assert owners.owner_map(conn) == {}


def test_an_existing_folder_is_unchanged_and_runs_no_git(conn, world, monkeypatch):  # AC 3
    add(conn, 0, world["a"], "feat/x")
    add(conn, 1, world["b"], "fix/typo")
    calls = []
    monkeypatch.setattr(discovery, "run_git", lambda *a, **k: calls.append(a))
    assert owners.owner_map(conn) == {}
    assert calls == []


def test_branch_text_is_matched_exactly_never_as_a_pattern_or_a_shell_word(conn, world):
    git(world["a"], "branch", "feat/a.c")
    add(conn, 7, world["a"])
    for n, branch in enumerate(["feat/a.*", "feat/a.c';touch x #", "feat/abc", "$(touch x)"]):
        add(conn, n, world["root"] / f"wt-{n}", branch)
    assert owners.owner_map(conn) == {}
    add(conn, 9, world["root"] / "wt-ok", "feat/a.c")
    assert owners.owner_map(conn) == {str(world["root"] / "wt-ok"): str(world["a"])}


def test_a_repo_that_breaks_git_gives_no_match_and_no_error(conn, world, monkeypatch):
    add(conn, 0, world["sib_ref"], "feat/x")
    add(conn, 1, world["a"])

    def timeout(*a, **k):
        raise subprocess.TimeoutExpired("git", 5)

    monkeypatch.setattr(discovery, "run_git", timeout)
    assert owners.owner_map(conn) == {}


def test_a_plain_folder_with_no_git_dir_is_not_a_candidate(conn, world, monkeypatch):
    plain = world["root"] / "plain"
    plain.mkdir()
    add(conn, 0, plain)
    add(conn, 1, world["sib_ref"], "feat/x")
    seen = []
    real = discovery.run_git
    monkeypatch.setattr(discovery, "run_git", lambda cwd, *a, **k: seen.append(cwd) or real(cwd, *a, **k))
    assert owners.owner_map(conn) == {}
    assert str(plain) not in seen


def test_never_maps_to_a_gone_path_or_through_a_chain(conn, world):
    add(conn, 0, world["wt_under_a"], "feat/x")
    add(conn, 1, world["wt_under_a"] / "deeper", "feat/x")  # parent chain is gone too: still the repo
    gone_repo = world["root"] / "goneRepo"
    add(conn, 2, gone_repo, "feat/x")
    got = owners.owner_map(conn)
    assert got[str(world["wt_under_a"] / "deeper")] == str(world["a"])
    assert set(got.values()) == {str(world["a"])}
    assert all(Path(v).is_dir() for v in got.values())


def test_git_runs_only_for_gone_paths_and_at_most_twice_per_repo(conn, world, monkeypatch):  # AC 8
    for n, key in enumerate(["sib_ref", "sib_merge", "sib_pr", "sib_two", "sib_none"]):
        add(conn, n, world[key], {"sib_ref": "feat/x", "sib_merge": "feat/m", "sib_pr": "feat/pr"}.get(key, "fix/typo"))
    add(conn, 10, world["a"])
    add(conn, 11, world["b"])
    calls = []
    real = discovery.run_git
    monkeypatch.setattr(discovery, "run_git", lambda cwd, *a, **k: calls.append((cwd, a[0])) or real(cwd, *a, **k))
    first = owners.owner_map(conn)
    per_repo = {r: [c for c in calls if c[0] == str(world[r])] for r in "ab"}
    assert all(len(v) <= 2 for v in per_repo.values()), per_repo
    calls.clear()
    assert owners.owner_map(conn) == first
    assert calls == []  # cached


def test_the_store_is_untouched(conn, world):  # AC 9
    seed(conn, world)
    path = conn.execute("PRAGMA database_list").fetchone()[2]
    files = [path, path + "-wal"]
    before = (conn.execute("SELECT COUNT(*) FROM events").fetchone(), [os.stat(f).st_mtime_ns for f in files])
    owners.owner_map(conn)
    metrics.usage_breakdown(conn, "project")
    kit.scoped_projects(conn)
    assert (conn.execute("SELECT COUNT(*) FROM events").fetchone(), [os.stat(f).st_mtime_ns for f in files]) == before


def test_usage_by_project_merges_the_owner_rows_and_keeps_the_total(conn, world):  # AC 4
    seed(conn, world)
    plain = metrics.usage_breakdown(conn, "model")["total"]
    got = metrics.usage_breakdown(conn, "project")
    by = {g["key"]: g["input_tokens"] + g["output_tokens"] for g in got["groups"]}
    assert "wt-ref" not in by and "wt-merge" not in by and "wt-pr" not in by and "x" not in by and "dir" not in by
    assert by["repoA"] == TOKENS * 5  # itself, .claude/worktrees/x, sub/dir, wt-ref, elsewhere
    assert by["repoB"] == TOKENS * 4
    assert sum(by.values()) == plain["input_tokens"] + plain["output_tokens"] == TOKENS * 14
    assert got["total"] == plain
    assert by["wt-two"] == TOKENS and by["wt-main"] == TOKENS  # unchanged keys stay


def test_tokens_by_project_and_the_top_project_warning_use_owners(conn, world):  # AC 5
    for n in range(3):
        add(conn, n, world["a"])
    add(conn, 3, world["sib_ref"], "feat/x")
    add(conn, 4, world["sib_ref"], "feat/x")
    for n in range(5, 9):
        add(conn, n, world["b"])
    totals = metrics.tokens_by_project(conn)
    assert totals == {"repoA": TOKENS * 5, "repoB": TOKENS * 4}
    value, detail = rules._top_project_share(conn, "2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z")
    assert value == round(5 / 9, 4) and detail == {"project": "repoA"}


def test_scoped_projects_lists_each_owner_once_and_no_mapped_gone_path(conn, world):  # AC 6
    seed(conn, world)
    got = kit.scoped_projects(conn)
    mapped = set(owners.owner_map(conn))
    assert not mapped & set(got)
    assert got == sorted(set(got))
    assert got.count(str(world["a"])) == 1
    _, gone_before = kit.split_gone(sorted(mapped | set(got)))
    _, gone_after = kit.split_gone(got)
    assert gone_before - gone_after == len(mapped)
    assert kit.kit_summary(conn, hide_gone=True)["gone"] == gone_after
    assert board.board(conn)["gone"] == gone_after
    assert memgraph.project_list(conn)["gone"] <= gone_after


def test_workspace_with_repo_a_includes_the_mapped_gone_worktree_events(conn, world):  # AC 7
    seed(conn, world)
    ws = workspaces.add(conn, "acme", repos=[str(world["a"])])
    got = metrics.usage_breakdown(conn, "project", workspace=ws["name"])
    by = {g["key"]: g["input_tokens"] + g["output_tokens"] for g in got["groups"]}
    assert by == {"repoA": TOKENS * 5}  # itself, .claude/worktrees/x, sub/dir, wt-ref (ref), elsewhere (ref)
    assert metrics.token_total(conn, workspace=ws["name"]) == TOKENS * 5


def test_workspace_without_the_repo_does_not_get_its_worktrees(conn, world):
    seed(conn, world)
    ws = workspaces.add(conn, "other", repos=[str(world["b"])])
    by = {g["key"] for g in metrics.usage_breakdown(conn, "project", workspace=ws["name"])["groups"]}
    assert by == {"repoB"}


def test_no_planted_text_reaches_the_map(conn, world):
    add(conn, 0, world["sib_ref"], "feat/x")
    git(world["a"], "commit", "-q", "--allow-empty", "-m", CANARY)
    assert CANARY not in repr(owners.owner_map(conn))


def _tie(conn, root, names):
    """Repos at `names` (relative to root) that all have branch feat/tie; one gone sibling that used it."""
    paths = [make_repo(root / name) for name in names]
    for repo in paths:
        git(repo, "branch", "feat/tie")
        add(conn, len(conn.execute("SELECT 1 FROM events").fetchall()), repo)
    gone = root / "wt-tie"
    add(conn, 50, gone, "feat/tie")
    return paths, str(gone)


def test_a_tie_between_a_hidden_and_a_visible_repo_goes_to_the_visible_one(conn, tmp_path):
    (_, visible), gone = _tie(conn, tmp_path.resolve(), [".spawn/x/repo", "visible"])
    assert owners.owner_map(conn) == {gone: str(visible)}


def test_a_tie_between_two_visible_repos_is_unchanged(conn, tmp_path):
    _tie(conn, tmp_path.resolve(), ["one", "two"])
    assert owners.owner_map(conn) == {}


def test_a_tie_between_two_hidden_repos_is_unchanged(conn, tmp_path):
    _tie(conn, tmp_path.resolve(), [".a/repo", ".b/repo"])
    assert owners.owner_map(conn) == {}


def test_a_single_match_inside_a_hidden_folder_is_still_used(conn, tmp_path):
    (hidden,), gone = _tie(conn, tmp_path.resolve(), [".spawn/x/repo"])
    assert owners.owner_map(conn) == {gone: str(hidden)}
