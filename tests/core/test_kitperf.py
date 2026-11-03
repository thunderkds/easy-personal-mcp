"""HUN-193 (FR-060..FR-062, NFR-014): kit servers compared like with like, session results from git."""

import itertools
import os
import stat
import subprocess
import time
from datetime import UTC, datetime, timedelta

import pytest

from easy_personal.core import kitperf, owners, prices, rules

START = datetime(2026, 1, 15, 10, 0, tzinfo=UTC)
CANARY = "LEAKCANARY-kitperf-0123456789"
_offset = itertools.count(1)


def stamp(moment):
    return moment.strftime("%Y-%m-%dT%H:%M:%S.000000Z")


def insert(conn, session, when, *, rtype, path="/work/p", branch=None, model="m1", tokens=100, tools=None,
           errors=None, sidechain=0, abstained=0, out=10):  # fmt: skip
    """One event; an assistant one carries `tokens` (`out` of them output tokens) and a message id."""
    n = next(_offset)
    assistant = rtype == "assistant"
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type, ts_utc,"
        " session_id, project_path, git_branch, is_sidechain, model, message_id, tool_names, input_tokens,"
        " output_tokens, cache_creation_input_tokens, cache_read_input_tokens, tool_error_count)"
        " VALUES ('m','claude_code','/x',1,?,?,?,?,?,?,?,?,?,?,?,?,?,0,0,?)",
        (n, abstained, rtype, stamp(when), session, path, branch, sidechain, model if assistant else None,
         f"msg{n}" if assistant else None, tools, tokens - out if assistant else None, out if assistant else None,
         errors),
    )  # fmt: skip


def make_session(conn, session, *, minutes=10, start=START, calls=0, failed=0, servers=(), **kw):
    """An assistant event (its tool calls) at `start`, a user event (the results) at `start + minutes`."""
    names = [*(f"mcp__{s}__search" for s in servers), *["Read"] * calls]
    insert(conn, session, start, rtype="assistant", tools=",".join(names) or None, **kw)
    insert(conn, session, start + timedelta(minutes=minutes), rtype="user", errors=failed if names else None,
           path=kw.get("path", "/work/p"), branch=kw.get("branch"))  # fmt: skip
    conn.commit()


def prepared(conn, threshold=0.2, budget=5.0):
    return kitperf._prepare(conn, None, prices.load(), threshold, budget)


# ------------------------------------------------------------------ AC1: which sessions used a server


def test_server_use_is_by_tool_name_prefix_and_sub_agent_records_count(conn):
    make_session(conn, "main", servers=["easy-ui"])
    insert(conn, "sub", START, rtype="assistant", tools="Read")
    insert(conn, "sub", START + timedelta(minutes=1), rtype="assistant", tools="mcp__easy-memory__search_notes", sidechain=1)
    insert(conn, "other", START, rtype="assistant", tools="mcp__claude_ai_easy-memory__x,mcp__slack__post,Bash")
    insert(conn, "text", START, rtype="assistant", tools="my_easy-ui_helper,mcp__easy-uix__a")
    conn.commit()
    rows = prepared(conn)
    assert rows["main"]["servers"] == {"easy-ui"}
    assert rows["sub"]["servers"] == {"easy-memory"}
    assert rows["other"]["servers"] == set()
    assert rows["text"]["servers"] == set()


# ------------------------------------------------------------------------ session rows


def test_row_measures_and_exclusions(conn):
    insert(conn, "a", START, rtype="assistant", model="small", tokens=100, tools="Read,Edit")
    insert(conn, "a", START + timedelta(minutes=1), rtype="assistant", model="big", tokens=500)
    insert(conn, "a", START + timedelta(minutes=30), rtype="user", errors=1)
    insert(conn, "nomodel", START, rtype="assistant", abstained=1)
    insert(conn, "noassistant", START, rtype="user")
    insert(conn, "gone-model", START, rtype="assistant", model=None)
    conn.commit()
    rows = prepared(conn)
    assert set(rows) == {"a"}  # abstained-only, no-assistant and no-model sessions are not compared
    row = rows["a"]
    assert (row["tokens"], row["length"], row["calls"], row["errors"]) == (600, 30.0, 2, 1)
    assert row["model"] == "big"  # equal output tokens: the smaller name wins
    assert row["cost"] is None  # models not in the price table abstain


def test_main_model_is_the_one_with_most_output_tokens(conn):
    for i, (model, out) in enumerate([("a", 5), ("b", 50), ("a", 20)]):
        insert(conn, "s", START + timedelta(minutes=i), rtype="assistant", model=model, tokens=100, out=out)
    conn.commit()
    assert prepared(conn)["s"]["model"] == "b"


def test_usage_is_counted_once_per_message(conn):
    for _ in range(2):  # two records of one API message repeat its usage
        conn.execute(
            "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type, ts_utc,"
            " session_id, model, message_id, input_tokens, output_tokens, cache_creation_input_tokens,"
            " cache_read_input_tokens) VALUES ('m','claude_code','/x',1,?,0,'assistant',?,'s','m1','msg-same',90,10,0,0)",
            (next(_offset), stamp(START)),
        )
    conn.commit()
    assert prepared(conn)["s"]["tokens"] == 100


def test_since_keeps_only_sessions_events_inside(conn):
    make_session(conn, "old", start=START - timedelta(days=10))
    make_session(conn, "new", start=START)
    result = kitperf.kit_findings(conn, since="2026-01-10T00:00:00+00:00", threshold=0.2)
    assert result["sessions"] == 1


# ------------------------------------------------------------------------- AC2 and AC3: results


def git(repo, *args, when=None):
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null"}
    if when:
        env.update(GIT_COMMITTER_DATE=when, GIT_AUTHOR_DATE=when)
    cmd = ["git", "-c", "user.email=t@example.invalid", "-c", "user.name=t", "-C", str(repo), *args]
    subprocess.run(cmd, check=True, capture_output=True, text=True, env=env)


def at(minutes):
    return (START + timedelta(minutes=minutes)).strftime("%Y-%m-%dT%H:%M:%S +0000")


@pytest.fixture
def repo(tmp_path):
    path = tmp_path / "repo"
    path.mkdir()
    git(path, "init", "-q", "-b", "main")
    git(path, "commit", "-q", "--allow-empty", "-m", "init", when="2026-01-01T00:00:00 +0000")
    for branch, minutes in {"in": 5, "before": -120, "after": 41, "edge": 40, "none": None}.items():
        git(path, "switch", "-q", "-c", f"feat/{branch}", "main")
        if minutes is not None:
            git(path, "commit", "-q", "--allow-empty", "-m", "work", when=at(minutes))
    git(path, "switch", "-q", "main")
    # merged and deleted, merge subject in the `Merge <b>: <text>` form; its commit is inside the window
    git(path, "switch", "-q", "-c", "feat/merged", "main")
    git(path, "commit", "-q", "--allow-empty", "-m", "work", when=at(3))
    git(path, "switch", "-q", "main")
    git(path, "merge", "-q", "--no-ff", "-m", "Merge feat/merged: add the thing", "feat/merged", when=at(900))
    git(path, "branch", "-q", "-D", "feat/merged")
    # merged and deleted, but its only commit is long before the session
    git(path, "switch", "-q", "-c", "feat/oldmerge", "main")
    git(path, "commit", "-q", "--allow-empty", "-m", "work", when=at(-5000))
    git(path, "switch", "-q", "main")
    git(path, "merge", "-q", "--no-ff", "-m", "Merge feat/oldmerge: old", "feat/oldmerge", when=at(901))
    git(path, "branch", "-q", "-D", "feat/oldmerge")
    return path


def results(conn, repo, **kw):
    owners.clear_cache()
    return {s: row["good_result"] for s, row in prepared(conn, **kw).items()}


def test_commit_window_decides_known_result(conn, repo):
    for name in ("in", "before", "after", "edge", "none", "merged", "oldmerge", "gone"):
        make_session(conn, name, path=str(repo), branch=f"feat/{name}", calls=10, failed=1)
    got = results(conn, repo)
    assert got["in"] == 1  # a commit inside [first, last + 30 min]
    assert got["before"] == 0  # only before the first event
    assert got["after"] == 0  # 31 min after the last event
    assert got["edge"] == 1  # exactly 30 min after the last event still counts
    assert got["none"] == 0  # no commit on the branch at all
    assert got["merged"] == 1  # ref deleted: commits found through `Merge feat/merged: ...`
    assert got["oldmerge"] == 0  # merge found, its commits are outside the window
    assert got["gone"] is None  # ref gone and no merge names it: unknown, not bad


def test_sessions_without_a_usable_branch_or_repo_are_unknown(conn, repo, tmp_path):
    make_session(conn, "on-main", path=str(repo), branch="main")
    make_session(conn, "on-master", path=str(repo), branch="master")
    make_session(conn, "on-head", path=str(repo), branch="HEAD")
    make_session(conn, "no-branch", path=str(repo), branch=None)
    plain = tmp_path / "plain"
    plain.mkdir()
    make_session(conn, "not-a-repo", path=str(plain), branch="feat/in")
    make_session(conn, "gone-folder", path=str(tmp_path / "nowhere"), branch="feat/unrelated")  # no repo owns it
    got = results(conn, repo)
    assert set(got.values()) == {None}
    assert len(got) == 6


def test_a_branch_text_never_acts_as_a_git_option_or_range(conn, repo):
    for i, branch in enumerate(["--all", "feat/in..main", "feat/in^", "x@{1}", "feat in"]):
        make_session(conn, f"s{i}", path=str(repo), branch=branch)
    assert set(results(conn, repo).values()) == {None}


def test_a_session_on_two_branches_uses_the_one_with_most_events_and_a_tie_is_unknown(conn, repo):
    make_session(conn, "most", path=str(repo), branch="feat/in")
    insert(conn, "most", START + timedelta(minutes=1), rtype="user", path=str(repo), branch="feat/before")
    insert(conn, "most", START + timedelta(minutes=2), rtype="user", path=str(repo), branch="feat/in")
    insert(conn, "tie", START, rtype="assistant", path=str(repo), branch="feat/in")
    insert(conn, "tie", START + timedelta(minutes=1), rtype="user", path=str(repo), branch="feat/before")
    conn.commit()
    got = results(conn, repo)
    assert got["most"] == 1
    assert got["tie"] is None


def test_a_gone_worktree_uses_its_owner_repo(conn, repo, tmp_path):
    gone = repo / ".claude" / "worktrees" / "x"  # not on disk; its parent repo owns it (FR-057)
    make_session(conn, "wt", path=str(gone), branch="feat/in")
    assert results(conn, repo)["wt"] == 1


@pytest.mark.parametrize(("failed", "threshold", "good"), [(1, 0.2, 1), (3, 0.2, 0), (3, 0.3, 1), (2, 0.2, 1)])
def test_error_rate_against_the_threshold(conn, repo, failed, threshold, good):
    make_session(conn, "s", path=str(repo), branch="feat/in", calls=10, failed=failed)
    assert results(conn, repo, threshold=threshold)["s"] == good


def test_a_session_with_tool_calls_and_no_counted_result_is_unknown_never_zero(conn, repo):
    make_session(conn, "old", path=str(repo), branch="feat/in", calls=10, failed=None)  # ingested before v8
    make_session(conn, "nocalls", path=str(repo), branch="feat/in")  # no tool call: rate 0
    got = results(conn, repo)
    assert got["old"] is None
    assert prepared(conn)["old"]["errors"] is None
    assert got["nocalls"] == 1


def test_median_error_rate_ignores_sessions_without_calls_or_counts(conn):
    for i, failed in enumerate([0, 1, 2]):
        make_session(conn, f"s{i}", calls=10, failed=failed)
    make_session(conn, "nocalls")
    make_session(conn, "unknown", calls=5, failed=None)
    assert kitperf.median_error_rate(conn) == pytest.approx(0.1)


# ----------------------------------------------------------------------- AC4: the threshold


def test_kit_error_rate_threshold_from_config(tmp_path):
    config = tmp_path / "config.yaml"
    assert rules.kit_error_rate(config) == rules.KIT_ERROR_RATE_DEFAULT == 0.04  # missing file
    assert rules.kit_error_rate(None) == 0.04
    config.write_text("thresholds:\n  cache_hit_low: 0.7\n")
    assert rules.kit_error_rate(config) == 0.04
    config.write_text("thresholds:\n  kit_error_rate: 0.3\n")
    assert rules.kit_error_rate(config) == 0.3
    assert rules.load_rules(config)  # the rules reader accepts the key too
    assert rules.live_thresholds(config) == {"live_failing_streak": 3, "live_open_call_minutes": 10}


def test_kit_error_rate_refuses_a_non_number_and_unknown_keys(tmp_path):
    config = tmp_path / "config.yaml"
    config.write_text("thresholds:\n  kit_error_rate: low\n")
    with pytest.raises(ValueError, match="config thresholds must be numbers"):
        rules.kit_error_rate(config)
    config.write_text("thresholds:\n  kit_error_rate: 0.3\n  kit_no_such_key: 1\n")
    with pytest.raises(ValueError, match="unknown rules: kit_no_such_key"):
        rules.kit_error_rate(config)


def test_kit_findings_reads_the_threshold_from_the_config_file(conn, tmp_path, monkeypatch):
    seen = []
    monkeypatch.setattr(kitperf, "_prepare", lambda conn, since, prices, threshold, budget: seen.append(threshold) or {})
    kitperf.kit_findings(conn)
    (tmp_path / "config.yaml").write_text("thresholds:\n  kit_error_rate: 0.3\n")
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path))
    kitperf.kit_findings(conn)
    assert seen[-1] == 0.3


# --------------------------------------------------------- AC5: strata and pooling, hand computed


def fake(stratum, used, value):
    return {"stratum": stratum, "servers": {"easy-ui"} if used else set(), "tokens": value}


def test_pooled_delta_is_the_exact_weighted_difference():
    rows = {}
    spec = {  # stratum -> (kit values, other values)
        "s1": ([10, 20], [10, 10, 10]),  # kit mean 15, other mean 10, 2 kit sessions: +10 over 20
        "s2": ([30], [20, 40]),  # equal means: 0 over 30
        "s3": ([5], [10]),  # -5 over 10
        "kit-only": ([100], []),  # one side only: excluded from both counts
        "other-only": ([], [1000]),
    }
    for name, (kit_values, other_values) in spec.items():
        for i, v in enumerate(kit_values):
            rows[f"{name}k{i}"] = fake(name, True, v)
        for i, v in enumerate(other_values):
            rows[f"{name}o{i}"] = fake(name, False, v)
    with_n, without_n, delta = kitperf._pool(rows, "easy-ui", "tokens")
    assert (with_n, without_n) == (4, 6)  # 2+1+1 and 3+2+1
    assert delta == pytest.approx((10 + 0 - 5) / (20 + 30 + 10) * 100)  # 8.3333...


def test_a_stratum_whose_other_mean_is_zero_is_skipped_for_that_measure():
    rows = {"a": fake("z", True, 5), "b": fake("z", False, 0), "c": fake("y", True, 4), "d": fake("y", False, 2)}
    assert kitperf._pool(rows, "easy-ui", "tokens") == (1, 1, pytest.approx(100.0))


def test_unknown_values_are_left_out_of_a_measure():
    rows = {"a": fake("z", True, 4), "b": fake("z", False, 2), "c": fake("z", False, None)}
    assert kitperf._pool(rows, "easy-ui", "tokens") == (1, 1, pytest.approx(100.0))


def test_strata_split_by_owner_project_main_model_and_size_band(conn):
    for tokens in (100, 100, 200, 200, 300, 300):  # three size bands in one project and model
        make_session(conn, f"k{tokens}-{next(_offset)}", tokens=tokens, servers=["easy-ui"])
        make_session(conn, f"o{tokens}-{next(_offset)}", tokens=tokens)
    make_session(conn, "other-model", tokens=100, servers=["easy-ui"], model="m2")
    make_session(conn, "other-project", tokens=100, servers=["easy-ui"], path="/work/q")
    rows = prepared(conn)
    by_tokens = {t: {r["stratum"] for r in rows.values() if r["tokens"] == t and r["model"] == "m1" and r["stratum"][0] == "/work/p"} for t in (100, 200, 300)}
    assert all(len(strata) == 1 for strata in by_tokens.values())  # equal size, project and model: one stratum
    bands = [next(iter(by_tokens[t]))[2] for t in (100, 200, 300)]
    assert bands == sorted(bands) and bands[0] < bands[2]  # the size bands are ordered tertiles
    assert rows["other-model"]["stratum"][:2] == ("/work/p", "m2")
    assert rows["other-project"]["stratum"][:2] == ("/work/q", "m1")
    with_n, without_n, _ = kitperf._pool(rows, "easy-ui", "tokens")
    assert (with_n, without_n) == (6, 6)  # the lone other-model and other-project sessions have no other side


# ------------------------------------------------------- AC6 and AC7: the 10 rule and the wording


def seeded(conn, kit_n, other_n, kit_minutes=10, other_minutes=20, calls=0):
    for i in range(kit_n):
        make_session(conn, f"k{i}", minutes=kit_minutes, servers=["easy-memory"], calls=calls)
    for i in range(other_n):
        make_session(conn, f"o{i}", minutes=other_minutes, calls=calls)


def test_nine_sessions_on_a_side_is_not_enough_and_ten_is(conn):
    seeded(conn, 9, 30)
    result = kitperf.kit_findings(conn, threshold=0.2)
    assert result["findings"] == []
    assert {"server": "easy-memory", "with": 9, "without": 30} in result["not_enough"]
    assert {n["server"] for n in result["not_enough"]} == set(kitperf.SERVERS)  # unused servers: 0 vs 39

    conn.execute("DELETE FROM events")  # the same session ids are reused
    seeded(conn, 10, 10)
    result = kitperf.kit_findings(conn, limit=10, threshold=0.2)
    assert {n["server"] for n in result["not_enough"]} == set(kitperf.SERVERS) - {"easy-memory"}
    assert {f["measure"] for f in result["findings"]} == {"tokens", "length"}  # no cost (unpriced model), no result


def test_a_better_server_gets_its_habit_and_the_text_says_linked_to(conn):
    seeded(conn, 10, 12)
    result = kitperf.kit_findings(conn, limit=10, threshold=0.2)
    length = next(f for f in result["findings"] if f["measure"] == "length")
    assert length["delta_pct"] == -50.0
    assert (length["with"], length["without"]) == (10, 12)
    assert length["text"] == (
        "Sessions that used easy-memory are linked to 50.0% fewer minutes of session time than similar "
        "sessions without it (10 vs 12 sessions)."
    )
    assert length["habit"] == kitperf.HABITS["easy-memory"]
    tokens = next(f for f in result["findings"] if f["measure"] == "tokens")
    assert tokens["delta_pct"] == 0.0


def test_a_worse_server_has_no_habit(conn):
    seeded(conn, 10, 10, kit_minutes=30, other_minutes=20)
    result = kitperf.kit_findings(conn, threshold=0.2)
    finding = next(f for f in result["findings"] if f["measure"] == "length")
    assert finding["delta_pct"] == 50.0 and finding["habit"] is None
    assert "more minutes of session time" in finding["text"]


def test_texts_never_claim_a_cause(conn):
    seeded(conn, 10, 12)
    for finding in kitperf.kit_findings(conn, limit=10, threshold=0.2)["findings"]:
        assert "linked to" in finding["text"]
        assert not {"caused", "because", "cause"} & set(finding["text"].lower().replace(".", " ").split())
    assert all(kitperf.NOTE == "linked to, not caused" and "caused" not in h for h in kitperf.HABITS.values())


def test_good_result_share_is_a_finding_with_its_own_ten_check(conn, repo):
    for i in range(12):  # kit sessions: commit inside the window
        make_session(conn, f"k{i}", path=str(repo), branch="feat/in", servers=["easy-ui"], calls=4)
    for i in range(12):  # non-kit sessions: half on a branch with no commit in the window
        make_session(conn, f"o{i}", path=str(repo), branch="feat/in" if i < 6 else "feat/before", calls=4)
    make_session(conn, "unknown", path=str(repo), branch="main", calls=4)
    owners.clear_cache()
    result = kitperf.kit_findings(conn, limit=10, threshold=0.2)
    share = next(f for f in result["findings"] if f["measure"] == "good_result" and f["server"] == "easy-ui")
    assert share["delta_pct"] == 100.0 and (share["with"], share["without"]) == (12, 12)  # share 1.0 vs 0.5
    assert "more good results" in share["text"] and share["habit"] == kitperf.HABITS["easy-ui"]
    assert result["unknown_results"] == 1


def test_findings_are_ranked_by_absolute_delta_then_server_and_measure_and_limited(conn):
    seeded(conn, 10, 10)
    for i in range(10):
        make_session(conn, f"u{i}", minutes=40, servers=["easy-ui"])  # easy-ui: longer sessions
    full = kitperf.kit_findings(conn, limit=10, threshold=0.2)["findings"]
    keys = [(-abs(f["delta_pct"]), f["server"], f["measure"]) for f in full]
    assert keys == sorted(keys)
    assert [f["server"] for f in kitperf.kit_findings(conn, limit=1, threshold=0.2)["findings"]] == [full[0]["server"]]
    assert len(kitperf.kit_findings(conn, limit=99, threshold=0.2)["findings"]) == len(full)  # capped at 10
    assert len(kitperf.kit_findings(conn, threshold=0.2)["findings"]) == min(3, len(full))


def test_an_empty_store_returns_no_findings_and_no_error(conn):
    result = kitperf.kit_findings(conn, threshold=0.2)
    assert result["sessions"] == 0 and result["findings"] == [] and result["unknown_results"] == 0
    assert [n["with"] for n in result["not_enough"]] == [0, 0, 0, 0]
    assert result["note"] == "linked to, not caused" and result["as_of"].endswith("Z")


def test_the_response_has_exactly_the_contract_keys(conn):
    seeded(conn, 10, 12)
    result = kitperf.kit_findings(conn, threshold=0.2)
    assert set(result) == {"as_of", "sessions", "findings", "not_enough", "unknown_results", "note"}
    assert set(result["findings"][0]) == {"server", "measure", "delta_pct", "with", "without", "text", "habit"}
    assert set(result["not_enough"][0]) == {"server", "with", "without"}


# ------------------------------------------------------------------------ AC9: git budget


def fake_git(tmp_path, monkeypatch, body):
    bin_dir = tmp_path / "fakebin"
    bin_dir.mkdir()
    script = bin_dir / "git"
    script.write_text(f"#!/bin/sh\n{body}\n")
    script.chmod(script.stat().st_mode | stat.S_IEXEC)
    monkeypatch.setenv("PATH", f"{bin_dir}:{os.environ['PATH']}")


def test_a_slow_git_returns_within_the_budget_and_the_slow_branches_are_unknown(conn, repo, tmp_path, monkeypatch):
    for i, branch in enumerate(["in", "before", "after", "edge", "none"]):
        make_session(conn, f"s{i}", path=str(repo), branch=f"feat/{branch}")
    owners.clear_cache()
    fake_git(tmp_path, monkeypatch, "exec sleep 30")
    monkeypatch.setattr(kitperf, "GIT_BUDGET_SECONDS", 0.5)
    started = time.monotonic()
    result = kitperf.kit_findings(conn, threshold=0.2)
    assert time.monotonic() - started < 0.5 + 1
    assert result["sessions"] == 5 and result["unknown_results"] == 5


def test_git_missing_on_the_path_makes_every_result_unknown(conn, repo, tmp_path, monkeypatch):
    make_session(conn, "s", path=str(repo), branch="feat/in")
    owners.clear_cache()
    empty = tmp_path / "empty-bin"
    empty.mkdir()
    monkeypatch.setenv("PATH", str(empty))
    assert kitperf.kit_findings(conn, threshold=0.2)["unknown_results"] == 1


def test_one_git_call_set_serves_every_session_of_a_branch(conn, repo, monkeypatch):
    calls = []
    real = kitperf.discovery.run_git
    monkeypatch.setattr(kitperf.discovery, "run_git", lambda *a, **k: calls.append(a) or real(*a, **k))
    for i in range(5):
        make_session(conn, f"s{i}", path=str(repo), branch="feat/in")
    owners.clear_cache()
    assert set(results(conn, repo).values()) == {1}
    assert len(calls) == 1


# ------------------------------------------------------------------------ AC10: no content


def test_no_text_or_id_from_the_store_reaches_the_response(conn):
    insert(conn, CANARY + "-session", START, rtype="assistant", path="/work/" + CANARY, branch=CANARY + "-branch", tools="Read")
    seeded(conn, 10, 12)
    result = kitperf.kit_findings(conn, limit=10, threshold=0.2)
    assert CANARY not in repr(result)
