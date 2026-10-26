"""Workspace CRUD and the usage filter (HUN-64)."""

import re

import pytest

from easy_personal.core import metrics, workspaces
from easy_personal.core.workspaces import WorkspaceError

from test_migrations import BASE, _ts, seed  # noqa: F401  (synthetic events helper)


def make(conn, name="acme", **kw):
    return workspaces.add(conn, name, **kw)


def test_add_and_get_roundtrip(conn):
    ws = make(conn, color="#00ff00", icon="A", description="d", email="me@example.com", repos=["/work/a"])
    assert ws["name"] == "acme" and ws["is_orphan"] is False
    assert ws["repos"] == ["/work/a"]
    assert workspaces.get(conn, ws["id"]) == ws
    names = [w["name"] for w in workspaces.list_all(conn)]
    assert names == ["orphan", "acme"]


@pytest.mark.parametrize("name", ["", "Acme", "orphan", "a_b", "x" * 41, None, 5])
def test_add_refuses_bad_names(conn, name):
    with pytest.raises(WorkspaceError):
        workspaces.add(conn, name)
    assert [w["name"] for w in workspaces.list_all(conn)] == ["orphan"]


def test_add_duplicate_is_refused_with_409(conn):
    make(conn)
    with pytest.raises(WorkspaceError) as exc:
        make(conn)
    assert exc.value.status == 409


def test_repo_paths_are_normalised(conn, tmp_path, monkeypatch):
    monkeypatch.setenv("HOME", str(tmp_path))
    real = tmp_path / "work" / "a"
    real.mkdir(parents=True)
    ws = make(conn, repos=["~/work/a/", f"{tmp_path}/work/./a", "/nope/not/yet/", "/"])
    assert ws["repos"] == sorted({str(real), "/nope/not/yet", "/"})


def test_relative_repo_is_refused(conn):
    with pytest.raises(WorkspaceError):
        make(conn, repos=["relative/dir"])
    assert [w["name"] for w in workspaces.list_all(conn)] == ["orphan"]  # nothing half-created


def test_repo_may_belong_to_several_workspaces(conn):
    a, b = make(conn, "a", repos=["/work/x"]), make(conn, "b", repos=["/work/x"])
    assert a["repos"] == b["repos"] == ["/work/x"]


def test_edit_changes_only_given_fields_and_replaces_repos(conn):
    ws = make(conn, color="#00ff00", repos=["/a", "/b"])
    out = workspaces.edit(conn, ws["id"], {"description": "new", "repos": ["/c"]})
    assert out["description"] == "new" and out["color"] == "#00ff00" and out["repos"] == ["/c"]
    assert workspaces.edit(conn, ws["id"], {"color": None})["color"] is None


def test_rename_changes_name_only(conn):
    ws = make(conn, repos=["/a"])
    out = workspaces.edit(conn, ws["id"], {"name": "company"})
    assert out["id"] == ws["id"] and out["name"] == "company" and out["repos"] == ["/a"]
    assert workspaces.by_name(conn, "company")["id"] == ws["id"]


def test_rename_to_existing_name_refused_nothing_changed(conn):
    a, b = make(conn, "a", repos=["/a"]), make(conn, "b")
    with pytest.raises(WorkspaceError) as exc:
        workspaces.edit(conn, a["id"], {"name": "b", "repos": ["/zzz"]})
    assert exc.value.status == 409
    assert workspaces.get(conn, a["id"]) == a
    assert workspaces.get(conn, b["id"]) == b


def test_edit_rejects_unknown_fields_and_bad_values(conn):
    ws = make(conn)
    for changes in ({"id": 7}, {"is_orphan": 1}, {"color": "red"}, {"name": "orphan"}, {"repos": "/a"}, {"email": 5}):
        with pytest.raises(WorkspaceError):
            workspaces.edit(conn, ws["id"], changes)
    assert workspaces.get(conn, ws["id"]) == ws


def test_orphan_cannot_be_edited_renamed_or_deleted(conn):
    orphan = workspaces.by_name(conn, "orphan")
    for call in (
        lambda: workspaces.edit(conn, orphan["id"], {"name": "other"}),
        lambda: workspaces.edit(conn, orphan["id"], {"description": "x"}),
        lambda: workspaces.edit(conn, orphan["id"], {"repos": ["/a"]}),
        lambda: workspaces.delete(conn, orphan["id"]),
    ):
        with pytest.raises(WorkspaceError, match="orphan"):
            call()
    assert workspaces.get(conn, orphan["id"]) == orphan


def test_delete_removes_workspace_and_repos(conn):
    ws = make(conn, repos=["/a"])
    workspaces.delete(conn, ws["id"])
    assert [w["name"] for w in workspaces.list_all(conn)] == ["orphan"]
    assert conn.execute("SELECT COUNT(*) FROM workspace_repos").fetchone()[0] == 0
    with pytest.raises(WorkspaceError) as exc:
        workspaces.delete(conn, ws["id"])
    assert exc.value.status == 404


def test_unknown_id_and_name_are_404(conn):
    for call in (lambda: workspaces.get(conn, 999), lambda: workspaces.by_name(conn, "nope"),
                 lambda: workspaces.edit(conn, 999, {})):
        with pytest.raises(WorkspaceError) as exc:
            call()
        assert exc.value.status == 404


# --- usage filter ------------------------------------------------------------------------------


def add_event(conn, n, project, ts=None):
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc,"
        " session_id, project_path, model, message_id, input_tokens, output_tokens,"
        " cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,0,?,?,?,'model-0',?,10,5,0,0)",
        (n, ts or _ts(BASE + n), f"s-{project}", project, f"msg{n}"),
    )


def build_usage(conn):
    for n, project in enumerate(
        ["/work/a", "/work/a", "/work/b", "/work/bc", "/work/a/pkg", "/other", None, "/work/a2"]
    ):
        add_event(conn, n, project)
    conn.commit()


def count(conn, **kw):
    return metrics.usage_summary(conn, **kw)["messages"]


def test_filter_matches_equal_or_under_but_not_string_prefix(conn):
    build_usage(conn)
    make(conn, "company", repos=["/work/a", "/work/b"])
    # /work/a x2, /work/a/pkg, /work/b -> 4 ; NOT /work/bc, /work/a2, /other, NULL
    assert count(conn, workspace="company") == 4
    assert count(conn) == 8


def test_filter_covers_every_exposed_usage_function(conn):
    build_usage(conn)
    make(conn, "company", repos=["/work/a", "/work/b"])
    total = metrics.usage_breakdown(conn, "project", workspace="company")["total"]["messages"]
    assert total == 4
    keys = {g["key"] for g in metrics.usage_breakdown(conn, "project", workspace="company")["groups"]}
    assert keys == {"a", "b", "pkg"}
    sessions = {s["session_id"] for s in metrics.session_timeline(conn, workspace="company")["sessions"]}
    assert sessions == {"s-/work/a", "s-/work/b", "s-/work/a/pkg"}
    assert metrics.session_outliers(conn, workspace="company")["sessions"] == []


def test_workspace_with_zero_repos_is_empty_not_everything(conn):
    build_usage(conn)
    make(conn, "empty")
    assert count(conn, workspace="empty") == 0
    assert count(conn, workspace="orphan") == 0


def test_repo_that_does_not_exist_matches_nothing(conn):
    build_usage(conn)
    make(conn, "ghost", repos=["/does/not/exist"])
    assert count(conn, workspace="ghost") == 0


def test_two_workspaces_may_count_the_same_events(conn):
    build_usage(conn)
    make(conn, "x", repos=["/work/b"])
    make(conn, "y", repos=["/work/b"])
    assert count(conn, workspace="x") == count(conn, workspace="y") == 1


def test_repo_given_as_subfolder_resolves_to_the_stored_project(conn, tmp_path):
    import subprocess

    repo = tmp_path / "repo"
    (repo / "pkg").mkdir(parents=True)
    subprocess.run(["git", "init", "-q", str(repo)], check=True)
    add_event(conn, 1, str(repo.resolve()))
    conn.commit()
    make(conn, "sub", repos=[str(repo / "pkg")])
    assert count(conn, workspace="sub") == 1


def test_unknown_workspace_is_a_value_error(conn):
    with pytest.raises(ValueError):
        count(conn, workspace="nope")


def test_filter_respects_the_period(conn):
    build_usage(conn)
    make(conn, "company", repos=["/work/a"])
    assert count(conn, workspace="company", since="2025-01-01", until="2025-02-01") == 0
    assert count(conn, workspace="company", since="2026-01-01", until="2026-02-01") == 3


def test_filter_query_never_uses_like_on_events(conn):
    import inspect

    src = inspect.getsource(workspaces) + inspect.getsource(metrics)
    assert not re.search(r"\bLIKE\b", src)  # SQL keyword in upper case


def test_filter_plan_uses_the_time_index(conn):
    since, until = "2026-01-10T00:00:00.000000Z", "2026-01-20T00:00:00.000000Z"
    where, params = metrics._inner_window(since, until)
    plan = " | ".join(
        r[3]
        for r in conn.execute(
            "EXPLAIN QUERY PLAN SELECT MIN(ts_utc) FROM events WHERE abstained = 0 AND "
            f"project_path IN ('/a','/b') AND {where} GROUP BY message_id",
            params,
        )
    )
    assert "events_ts_message" in plan and "SCAN events" not in plan, plan


@pytest.mark.perf
def test_workspace_filter_over_a_million_events_is_under_500ms(tmp_path):
    import time

    from easy_personal.core import store

    conn = store.connect(tmp_path / "big.sqlite")
    seed(conn, 400_000, span_days=365, spread=2)  # 800k, projects /work/app0..4
    conn.executemany(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained,"
        " ts_utc, session_id, project_path, model, message_id, tool_names, input_tokens,"
        " output_tokens, cache_creation_input_tokens, cache_read_input_tokens)"
        " VALUES ('m','claude_code','/x',1,?,0,?,?,?,?,?,NULL,10,5,0,0)",
        ((10**9 + i, _ts(BASE + (i * 365 * 86400) // 200_000), f"s{i % 50}", f"/w/p{i % 40}/sub",
          "model-0", f"x{i}") for i in range(200_000)),
    )
    conn.commit()
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == 1_000_000
    workspaces.add(conn, "company", repos=["/work/app1", "/work/app3", "/w/p7", "/w/p9/sub"])
    timings = []
    for _ in range(3):
        t = time.perf_counter()
        out = metrics.usage_summary(conn, "2026-06-01", "2026-07-01", workspace="company")
        timings.append(time.perf_counter() - t)
    print(f"perf workspace usage_summary 30d over 1M events: {[round(x * 1000) for x in timings]} ms")
    assert out["messages"] > 0
    assert min(timings) < 0.5, timings
    conn.close()
