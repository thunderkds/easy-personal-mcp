"""HUN-206 (FR-069, FR-070): Scratch group in project lists and the unassigned-repos read, at read time."""

import os
import subprocess

import pytest

from easy_personal.core import discovery, metrics, owners, workspaces
from easy_personal.core import prices as price_table


def git(repo, *args):
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null"}
    cmd = ["git", "-c", "user.email=t@example.invalid", "-c", "user.name=t", "-C", str(repo), *args]
    subprocess.run(cmd, check=True, capture_output=True, text=True, env=env)


def make_repo(path):
    path.mkdir(parents=True)
    git(path, "init", "-q", "-b", "main")
    git(path, "commit", "-q", "--allow-empty", "-m", "init")
    return path


def add(conn, n, path, session=None, model="model-0", ts=None):
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc,"
        " session_id, project_path, model, message_id, input_tokens, output_tokens,"
        " cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,0,?,?,?,?,?,10,5,0,0)",
        (n, ts or f"2026-01-01T00:00:{n:02d}.000000Z", session or f"s{n}", path if path is None else str(path), model, f"msg{n}"),
    )
    conn.commit()


@pytest.fixture(autouse=True)
def _fresh_cache():
    owners.clear_cache()
    yield
    owners.clear_cache()


@pytest.fixture
def world(tmp_path, monkeypatch):
    """The 'system temp folder' is `root/systmp`; everything else under `root` is outside it."""
    root = tmp_path.resolve()
    (root / "systmp").mkdir()
    monkeypatch.setattr(owners, "_tmp_roots", lambda: (str(root / "systmp"),))
    repo = make_repo(root / "repo")
    (root / "plain").mkdir()
    return {
        "root": root, "repo": repo,
        "wt": repo / ".claude" / "worktrees" / "x",  # gone, inside the repo: FR-057 owner
        "tmp": root / "systmp" / "box", "gone": root / "gone-dir", "plain": root / "plain",
        "hidden": make_repo(root / ".hid" / "hrepo"),
    }  # fmt: skip


def seed(conn, w):
    (w["tmp"]).mkdir()
    for n, key in enumerate(("repo", "wt", "tmp", "gone", "plain", "hidden")):
        add(conn, n, w[key])


def sums(group):
    return group["input_tokens"] + group["output_tokens"]


def test_ac1_two_groups_repo_with_worktree_and_scratch_with_four_folders(conn, world):
    seed(conn, world)
    out = metrics.usage_breakdown(conn, "project")
    assert [g["key"] for g in out["groups"]] == ["Scratch", "repo"]  # 4 folders x 15 > 2 x 15
    scratch, repo = out["groups"]
    assert sums(repo) == 30 and "scratch" not in repo and "folders" not in repo
    assert scratch["scratch"] is True and sums(scratch) == 60 and scratch["messages"] == 4
    assert [f["key"] for f in scratch["folders"]] == sorted(["box", "gone-dir", "hrepo", "plain"])  # ties by key
    for folder in scratch["folders"]:
        assert sums(folder) == 15 and folder["messages"] == 1
        assert {"split", "api_equivalent_cost"} <= set(folder)


def test_ac2_groups_sum_to_total_and_summary_is_unchanged(conn, world):
    seed(conn, world)
    out = metrics.usage_breakdown(conn, "project")
    assert sum(sums(g) for g in out["groups"]) == sums(out["total"]) == 90
    assert sum(sums(f) for f in out["groups"][0]["folders"]) == sums(out["groups"][0])
    assert sum(g["messages"] for g in out["groups"]) == out["total"]["messages"] == 6
    summary = metrics.usage_summary(conn)
    assert summary["messages"] == 6 and summary["input_tokens"] == 60 and summary["output_tokens"] == 30


def test_ac3_tokens_by_project_uses_the_same_keys(conn, world):
    seed(conn, world)
    assert metrics.tokens_by_project(conn) == {"repo": 30, "Scratch": 60}


def test_ac4_a_folder_under_the_temp_folder_runs_no_git(conn, world, monkeypatch):
    world["tmp"].mkdir()
    add(conn, 0, world["tmp"])
    calls = []
    monkeypatch.setattr(discovery, "run_git", lambda cwd, *a, **k: calls.append(cwd))
    assert owners.project_map(conn) == {str(world["tmp"]): owners.SCRATCH}
    assert calls == []


def test_the_real_temp_folder_rule_matches_tempfile_and_a_symlinked_tmp(conn, tmp_path, monkeypatch):
    real = (tmp_path / "real-tmp").resolve()
    real.mkdir()
    link = tmp_path / "link-tmp"
    link.symlink_to(real)
    monkeypatch.setattr("tempfile.gettempdir", lambda: str(link))  # gettempdir() is a symlink
    (real / "box").mkdir()
    add(conn, 0, real / "box")
    add(conn, 1, link / "box2")
    assert set(owners.project_map(conn).values()) == {owners.SCRATCH}


def test_ac5_other_dimensions_have_no_scratch_fields(conn, world):
    seed(conn, world)
    for by in ("day", "model", "tool"):
        out = metrics.usage_breakdown(conn, by)
        assert all("scratch" not in g and "folders" not in g for g in out["groups"]), by
    assert [g["key"] for g in metrics.usage_breakdown(conn, "model")["groups"]] == ["model-0"]


def test_a_gone_path_with_an_owner_is_never_scratch(conn, world):
    add(conn, 0, world["wt"])
    add(conn, 1, world["repo"])
    assert owners.project_map(conn) == {str(world["wt"]): str(world["repo"]), str(world["repo"]): str(world["repo"])}


def test_home_and_dot_config_and_hidden_repo_are_scratch(conn, world, monkeypatch):
    home = world["root"] / "home"
    cfg = home / ".config" / "herdr"
    cfg.mkdir(parents=True)
    for n, p in enumerate((home, cfg, world["hidden"])):
        add(conn, n, p)
    assert set(owners.project_map(conn).values()) == {owners.SCRATCH}


def test_relative_and_empty_stored_paths_are_scratch_not_a_crash(conn, world):
    add(conn, 0, "relative/dir")
    add(conn, 1, "")
    add(conn, 2, None)
    out = metrics.usage_breakdown(conn, "project")
    keys = {g["key"]: g for g in out["groups"]}
    assert set(keys) == {"Scratch", "(unknown)"}
    assert sums(keys["Scratch"]) == 30 and len(keys["Scratch"]["folders"]) == 2
    assert metrics.tokens_by_project(conn) == {"Scratch": 30, "(unknown)": 15}


def test_two_scratch_folders_with_one_name_are_one_folder_line(conn, world):
    for n, parent in enumerate(("a", "b")):
        (world["root"] / parent / "proj").mkdir(parents=True)
        add(conn, n, world["root"] / parent / "proj")
    scratch = metrics.usage_breakdown(conn, "project")["groups"][0]
    assert [(f["key"], f["messages"], sums(f)) for f in scratch["folders"]] == [("proj", 2, 30)]


def test_owner_is_not_hidden_inside_scratch_when_it_is_a_real_repo(conn, world):
    add(conn, 0, world["repo"])
    assert owners.project_map(conn) == {str(world["repo"]): str(world["repo"])}


# --- FR-070: unassigned repos -------------------------------------------------------------------


def make_ws(conn, *repos):
    return workspaces.add(conn, "acme", repos=[str(r) for r in repos])


def test_ac6_only_repos_outside_every_workspace_are_listed_newest_first(conn, world):
    r1 = world["repo"]
    r2, r3 = make_repo(world["root"] / "r2"), make_repo(world["root"] / "r3")
    world["tmp"].mkdir()
    add(conn, 0, r1)
    add(conn, 1, world["wt"])  # gone worktree of r1: counts under r1
    add(conn, 2, r2, session="a", ts="2026-01-02T00:00:00.000000Z")
    add(conn, 3, r2, session="b", ts="2026-01-03T00:00:00.000000Z")
    add(conn, 4, r3, ts="2026-01-04T00:00:00.000000Z")
    add(conn, 5, world["tmp"])  # scratch
    make_ws(conn, str(r1) + "/")  # a trailing slash still covers r1
    rows = workspaces.unassigned(conn, price_table.PriceTable("2026-01-01", {}))
    assert [r["path"] for r in rows] == [str(r3), str(r2)]
    assert rows[1] == {
        "path": str(r2), "name": "r2", "sessions": 2, "last_used_utc": "2026-01-03T00:00:00.000000Z",
        "api_equivalent_cost": {"usd": None, "abstained_models": ["model-0"], "price_table_date": "2026-01-01"},
    }  # fmt: skip


def test_unassigned_cost_last_used_and_a_repo_under_a_workspace_root(conn, world):
    r2 = make_repo(world["root"] / "r2")
    add(conn, 0, r2, ts="2026-01-02T00:00:00.000000Z")
    add(conn, 1, r2, ts="2026-01-05T00:00:00.000000Z")
    prices = price_table.PriceTable("2026-01-01", {"model-0": price_table.Price(1.0, 2.0, 0.0, 0.0)})
    (row,) = workspaces.unassigned(conn, prices)
    assert row["last_used_utc"] == "2026-01-05T00:00:00.000000Z" and row["sessions"] == 2
    assert row["api_equivalent_cost"] == {"usd": round(2 * (10 * 1.0 + 5 * 2.0) / 1e6, 6), "abstained_models": [],
                                          "price_table_date": "2026-01-01"}  # fmt: skip
    make_ws(conn, world["root"])  # r2 sits under the workspace repo
    assert workspaces.unassigned(conn, prices) == []


def test_unassigned_is_empty_with_no_usage_and_when_all_covered(conn, world):
    assert workspaces.unassigned(conn) == []
    add(conn, 0, world["repo"])
    make_ws(conn, world["repo"])
    assert workspaces.unassigned(conn) == []


def test_unassigned_never_lists_scratch_or_a_sibling_prefix(conn, world):
    sibling = make_repo(world["root"] / "repo2")  # `/…/repo` must not cover `/…/repo2`
    add(conn, 0, sibling)
    add(conn, 1, world["plain"])
    make_ws(conn, world["repo"])
    assert [r["path"] for r in workspaces.unassigned(conn)] == [str(sibling)]
