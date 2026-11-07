"""HUN-238 (FR-087, FR-088): `kit.workspace_rollup` = easy-verifier scores and easy-ui use per workspace.

Read from stored rows only (`verifier_scores`, `events`). The easy-ui check results stay overall.
"""

import statistics
import time

import pytest

from easy_personal.core import accounts, kit, owners, workspaces
from easy_personal.sources.ui import ui_part
from tests_helpers import add_event

UI = "mcp__easy-ui__"
BEFORE, INSIDE, AFTER = "2026-02-01T00:00:00.000000Z", "2026-02-10T12:00:00.000000Z", "2026-03-01T00:00:00.000000Z"


@pytest.fixture(autouse=True)
def _world(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    owners.clear_cache()
    yield
    owners.clear_cache()


def _repo(tmp_path, name):
    path = (tmp_path / "repos" / name).resolve()
    path.mkdir(parents=True)
    return str(path)


def _score(conn, project, value=None, generated="2026-02-01T09:00:00Z", status="ok", abstained=0, reason=None):
    conn.execute(
        "INSERT INTO verifier_scores VALUES (?, ?, 'project', ?, ?, ?, ?)",
        (project, status, None if status == "unknown" else generated, value, abstained, reason),
    )
    conn.commit()


def _call(conn, n, project, session, tools, ts=INSIDE, sidechain=0, account=None, abstained=0):
    add_event(conn, n, project, ts=ts)
    conn.execute(
        "UPDATE events SET session_id = ?, tool_names = ?, is_sidechain = ?, agent_account_id = ?, abstained = ? "
        "WHERE byte_offset = ?",
        (session, tools, sidechain, account, abstained, n),
    )
    conn.commit()


def _by_name(out):
    return {w["name"]: w for w in out["workspaces"]}


@pytest.fixture
def two(conn, tmp_path):
    """ws-a: repos a1 (80), a2 (60), a3 (no row); ws-b: repo b1 (no row)."""
    repos = {n: _repo(tmp_path, n) for n in ("a1", "a2", "a3", "b1")}
    _score(conn, repos["a1"], 80.0, "2026-02-01T09:00:00Z")
    _score(conn, repos["a2"], 60.0, "2026-02-03T09:00:00Z")
    workspaces.add(conn, "ws-a", color="#336699", repos=[repos["a1"], repos["a2"], repos["a3"]])
    workspaces.add(conn, "ws-b", repos=[repos["b1"]])
    return conn, repos


# AC 1
def test_verifier_counts_scored_repos_mean_lowest_and_newest(two):
    conn, _ = two
    out = _by_name(kit.workspace_rollup(conn))
    assert out["ws-a"]["repos"] == 3 and out["ws-a"]["color"] == "#336699"
    assert out["ws-a"]["verifier"] == {
        "status": "ok", "repos_scored": 2, "repos_total": 3, "mean": 70.0,
        "lowest": {"project": "a2", "value": 60.0}, "newest_utc": "2026-02-03T09:00:00Z",
    }  # fmt: skip
    assert out["ws-b"]["verifier"] == {
        "status": "no data", "repos_scored": 0, "repos_total": 1, "mean": None, "lowest": None, "newest_utc": None,
    }  # fmt: skip


def test_workspaces_come_in_list_all_order_and_the_response_has_two_top_keys(two):
    conn, _ = two
    out = kit.workspace_rollup(conn)
    assert set(out) == {"workspaces", "ui_checks"}
    assert [w["name"] for w in out["workspaces"]] == ["ws-a", "ws-b"]
    assert set(out["workspaces"][0]) == {"name", "color", "repos", "verifier", "ui"}


def test_lowest_shows_the_folder_name_never_a_path(two):
    conn, repos = two
    out = kit.workspace_rollup(conn)
    assert "/" not in out["workspaces"][0]["verifier"]["lowest"]["project"]
    assert repos["a1"] not in str(out)


def test_two_repos_with_one_folder_name_stay_two_repos(conn, tmp_path):
    one, two_ = _repo(tmp_path, "x/app"), _repo(tmp_path, "y/app")
    _score(conn, one, 50.0)
    _score(conn, two_, 90.0)
    workspaces.add(conn, "ws", repos=[one, two_])
    verifier = kit.workspace_rollup(conn)["workspaces"][0]["verifier"]
    assert (verifier["repos_scored"], verifier["repos_total"], verifier["mean"]) == (2, 2, 70.0)
    assert verifier["lowest"] == {"project": "app", "value": 50.0}


# AC 2
def test_abstained_and_unknown_schema_rows_are_no_score_never_zero(conn, tmp_path):
    good, bad, odd = (_repo(tmp_path, n) for n in ("good", "bad", "odd"))
    _score(conn, good, 90.0)
    _score(conn, bad, None, abstained=1, reason="no_evidence")
    _score(conn, odd, None, status="unknown")
    workspaces.add(conn, "ws", repos=[good, bad, odd])
    verifier = kit.workspace_rollup(conn)["workspaces"][0]["verifier"]
    assert verifier["repos_scored"] == 1 and verifier["repos_total"] == 3
    assert verifier["mean"] == 90.0 and verifier["lowest"] == {"project": "good", "value": 90.0}


def test_a_workspace_with_only_no_score_rows_says_no_data(conn, tmp_path):
    bad, odd = _repo(tmp_path, "bad"), _repo(tmp_path, "odd")
    _score(conn, bad, None, abstained=1, reason="no_evidence")
    _score(conn, odd, None, status="unknown")
    workspaces.add(conn, "ws", repos=[bad, odd])
    assert kit.workspace_rollup(conn)["workspaces"][0]["verifier"] == {
        "status": "no data", "repos_scored": 0, "repos_total": 2, "mean": None, "lowest": None, "newest_utc": None,
    }  # fmt: skip


def test_a_score_of_zero_is_a_score(conn, tmp_path):
    zero = _repo(tmp_path, "zero")
    _score(conn, zero, 0.0)
    workspaces.add(conn, "ws", repos=[zero])
    verifier = kit.workspace_rollup(conn)["workspaces"][0]["verifier"]
    assert verifier["status"] == "ok" and verifier["mean"] == 0.0 and verifier["lowest"]["value"] == 0.0


# AC 3
def test_orphan_is_not_listed_and_an_empty_workspace_is_an_entry(conn):
    workspaces.add(conn, "empty")
    out = kit.workspace_rollup(conn)
    assert [w["name"] for w in out["workspaces"]] == ["empty"]
    (entry,) = out["workspaces"]
    assert entry["repos"] == 0
    assert entry["verifier"]["status"] == "no data" and entry["verifier"]["repos_total"] == 0
    assert entry["ui"] == {"status": "not used", "sessions": 0, "calls": 0, "last_utc": None}


def test_no_workspace_at_all_gives_an_empty_list_and_the_overall_checks(conn):
    assert kit.workspace_rollup(conn) == {"workspaces": [], "ui_checks": ui_part(conn)}


def test_a_repo_whose_folder_is_gone_is_not_in_repos_total(conn, tmp_path):
    alive, gone = _repo(tmp_path, "alive"), str(tmp_path / "repos" / "gone")
    _score(conn, alive, 70.0)
    _score(conn, gone, 10.0)  # a stale row of a folder deleted since the last ingest
    workspaces.add(conn, "ws", repos=[alive, gone])
    entry = kit.workspace_rollup(conn)["workspaces"][0]
    assert entry["repos"] == 2
    assert (entry["verifier"]["repos_total"], entry["verifier"]["repos_scored"]) == (1, 1)
    assert entry["verifier"]["mean"] == 70.0


# AC 4
def test_a_project_under_a_repo_counts_and_a_sibling_prefix_does_not(conn, tmp_path):
    repo = _repo(tmp_path, "b")
    sub, sibling = _repo(tmp_path, "b/pkg"), _repo(tmp_path, "bc")
    _score(conn, sub, 40.0)
    _score(conn, sibling, 99.0)  # `/…/b` must not match `/…/bc`
    _call(conn, 1, sub, "s1", f"{UI}open")
    _call(conn, 2, sibling, "s2", f"{UI}open")
    workspaces.add(conn, "ws", repos=[repo])
    entry = kit.workspace_rollup(conn)["workspaces"][0]
    assert entry["verifier"]["mean"] == 40.0 and entry["verifier"]["repos_scored"] == 1
    assert entry["ui"]["sessions"] == 1


def test_a_gone_worktree_counts_for_its_owner_repo(conn, tmp_path, monkeypatch):
    repo, other = _repo(tmp_path, "main-repo"), _repo(tmp_path, "other-repo")
    inside, sibling = f"{repo}/.claude/worktrees/x", str(tmp_path / "repos" / "wt-feature")  # both folders are gone
    monkeypatch.setattr(owners, "owner_map", lambda conn: {sibling: repo})  # FR-057 by branch name
    _call(conn, 1, inside, "s1", f"{UI}open")
    _call(conn, 2, sibling, "s2", f"{UI}open,{UI}click")
    workspaces.add(conn, "ws", repos=[repo])
    workspaces.add(conn, "elsewhere", repos=[other])
    out = _by_name(kit.workspace_rollup(conn))
    assert (out["ws"]["ui"]["sessions"], out["ws"]["ui"]["calls"]) == (2, 3)
    assert out["elsewhere"]["ui"]["status"] == "not used"


def test_a_repo_in_two_workspaces_shows_in_both(conn, tmp_path):
    shared = _repo(tmp_path, "shared")
    _score(conn, shared, 75.0)
    _call(conn, 1, shared, "s1", f"{UI}open")
    workspaces.add(conn, "one", repos=[shared])
    workspaces.add(conn, "two", repos=[shared])
    for entry in kit.workspace_rollup(conn)["workspaces"]:
        assert entry["verifier"]["mean"] == 75.0 and entry["ui"]["sessions"] == 1


# AC 5
@pytest.fixture
def ui_world(conn, tmp_path):
    repo, other = _repo(tmp_path, "app"), _repo(tmp_path, "other")
    _call(conn, 1, repo, "s1", f"{UI}open,Read,{UI}click", ts="2026-02-10T12:00:00.000000Z")
    _call(conn, 2, repo, "s1", f"{UI}screenshot", ts="2026-02-10T12:05:00.000000Z")
    _call(conn, 3, repo, "s2", f"{UI}open", ts="2026-02-10T13:00:00.000000Z", sidechain=1)
    _call(conn, 4, repo, "s3", "mcp__easy-verifier__score,mcp__easy-uix__a,mcp__claude_ai_easy-ui__x,Bash")
    _call(conn, 5, other, "s9", f"{UI}open")
    workspaces.add(conn, "ws", repos=[repo])
    workspaces.add(conn, "ws-other", repos=[other])
    return conn


def test_ui_counts_sessions_calls_and_last_time(ui_world):
    out = _by_name(kit.workspace_rollup(ui_world))
    assert out["ws"]["ui"] == {"status": "ok", "sessions": 2, "calls": 4, "last_utc": "2026-02-10T13:00:00.000000Z"}
    assert out["ws-other"]["ui"]["sessions"] == 1


def test_a_workspace_without_a_call_is_not_used(ui_world, tmp_path):
    workspaces.add(ui_world, "idle", repos=[_repo(tmp_path, "idle")])
    assert _by_name(kit.workspace_rollup(ui_world))["idle"]["ui"] == {
        "status": "not used", "sessions": 0, "calls": 0, "last_utc": None,
    }  # fmt: skip


def test_period_is_half_open_and_outside_events_are_not_counted(conn, tmp_path):
    repo = _repo(tmp_path, "app")
    _call(conn, 1, repo, "early", f"{UI}open", ts=BEFORE)
    _call(conn, 2, repo, "edge-in", f"{UI}open", ts="2026-02-05T00:00:00.000000Z")
    _call(conn, 3, repo, "mid", f"{UI}open", ts=INSIDE)
    _call(conn, 4, repo, "edge-out", f"{UI}open", ts="2026-02-20T00:00:00.000000Z")
    _call(conn, 5, repo, "late", f"{UI}open", ts=AFTER)
    workspaces.add(conn, "ws", repos=[repo])
    ui = kit.workspace_rollup(conn, since="2026-02-05T00:00:00Z", until="2026-02-20T00:00:00Z")["workspaces"][0]["ui"]
    assert (ui["sessions"], ui["calls"]) == (2, 2)  # since is included, until is not
    assert kit.workspace_rollup(conn)["workspaces"][0]["ui"]["sessions"] == 5


def test_the_verifier_part_ignores_the_period(two):
    conn, _ = two
    out = kit.workspace_rollup(conn, since="2030-01-01", until="2030-02-01")
    assert out["workspaces"][0]["verifier"]["repos_scored"] == 2


def test_abstained_events_and_null_or_empty_tool_names_do_not_count_or_crash(conn, tmp_path):
    repo = _repo(tmp_path, "app")
    _call(conn, 1, repo, "s1", f"{UI}open", abstained=1)
    _call(conn, 2, repo, "s2", None)
    _call(conn, 3, repo, "s3", "")
    workspaces.add(conn, "ws", repos=[repo])
    assert kit.workspace_rollup(conn)["workspaces"][0]["ui"]["status"] == "not used"


def test_account_limits_the_events_and_an_unknown_account_raises(conn, tmp_path):
    conn.execute(
        "INSERT INTO agent_accounts (provider, folder, label, status, config_dir_env) VALUES "
        "('claude', '/a/.claude', 'Claude default', 'confirmed', NULL),"
        "('claude', '/a/.claude-work', 'Claude work', 'confirmed', '/a/.claude-work')"
    )
    repo = _repo(tmp_path, "app")
    _call(conn, 1, repo, "s1", f"{UI}open", account=1)
    _call(conn, 2, repo, "s2", f"{UI}open,{UI}click", account=2)
    workspaces.add(conn, "ws", repos=[repo])
    assert kit.workspace_rollup(conn, account="Claude work")["workspaces"][0]["ui"]["calls"] == 2
    assert kit.workspace_rollup(conn, account="2")["workspaces"][0]["ui"]["sessions"] == 1
    assert kit.workspace_rollup(conn)["workspaces"][0]["ui"]["sessions"] == 2
    with pytest.raises(accounts.AccountError):
        kit.workspace_rollup(conn, account="nope")


def test_non_claude_code_events_are_not_counted(conn, tmp_path):
    repo = _repo(tmp_path, "app")
    _call(conn, 1, repo, "s1", f"{UI}open")
    conn.execute("UPDATE events SET source = 'other'")
    conn.commit()
    workspaces.add(conn, "ws", repos=[repo])
    assert kit.workspace_rollup(conn)["workspaces"][0]["ui"]["status"] == "not used"


# AC 6
def test_ui_checks_stay_overall_and_equal_ui_part(ui_world):
    ui_world.execute(
        "INSERT INTO ui_sessions VALUES ('u1', 'passed', '2026-02-10T09:00:00.000000Z', '2026-02-10T09:05:00.000000Z', 3)"
    )
    ui_world.commit()
    out = kit.workspace_rollup(ui_world)
    assert out["ui_checks"] == ui_part(ui_world) and out["ui_checks"]["sessions"] == 1
    for entry in out["workspaces"]:
        assert set(entry["ui"]) == {"status", "sessions", "calls", "last_utc"}  # no check result under a workspace
        assert "passed" not in str(entry)


def test_ui_checks_are_the_same_whatever_the_period_or_account(ui_world):
    ui_world.execute("INSERT INTO ui_sessions VALUES ('u1', 'failed', '2026-02-10T09:00:00.000000Z', NULL, 1)")
    ui_world.commit()
    assert kit.workspace_rollup(ui_world, since="2030-01-01")["ui_checks"] == ui_part(ui_world)


def test_ui_sessions_are_read_only_through_ui_part(ui_world, monkeypatch):
    seen = []
    real = kit.ui_part
    monkeypatch.setattr(kit, "ui_part", lambda conn: seen.append(1) or real(conn))
    kit.workspace_rollup(ui_world)
    assert seen == [1]  # once for the whole response, not once per workspace


# AC 9
def test_160_projects_and_10_workspaces_answer_in_under_two_seconds(conn, tmp_path):
    repos = [_repo(tmp_path, f"p{i:03d}") for i in range(160)]
    for i, repo in enumerate(repos):
        if i % 2 == 0:
            _score(conn, repo, float(40 + i % 60))
    for i, repo in enumerate(repos):
        for j in range(40):  # 6400 events; every fourth project uses easy-ui
            tools = f"Read,{UI}open" if i % 4 == 0 and j % 5 == 0 else "Read,Bash"
            add_event(conn, i * 40 + j, repo, ts=f"2026-02-{1 + j % 28:02d}T10:{j:02d}:00.000000Z")
            conn.execute("UPDATE events SET session_id = ?, tool_names = ? WHERE byte_offset = ?", (f"s{i}-{j // 10}", tools, i * 40 + j))
    conn.commit()
    for w in range(10):
        workspaces.add(conn, f"ws-{w}", repos=repos[w * 16 : (w + 1) * 16])
    times = []
    for _ in range(3):
        started = time.perf_counter()
        out = kit.workspace_rollup(conn)
        times.append(time.perf_counter() - started)
    median = statistics.median(times)
    print(f"workspace_rollup, 160 projects, 10 workspaces, 6400 events: runs {[round(t, 3) for t in times]} s, median {median:.3f} s")
    assert len(out["workspaces"]) == 10 and out["workspaces"][0]["verifier"]["repos_scored"] == 8
    assert median < 2.0
