"""HUN-186 (FR-054, NFR-013): faster dashboard reads that show the same numbers, and no gone projects in three lists.

The golden file was captured from the code before the change (see `hun186_fixture.py`).
"""

import json
import re
import time
from pathlib import Path

import pytest
from hun186_fixture import SINCE, UNTIL, normalized, seed, snapshot
from starlette.testclient import TestClient

from easy_personal.core import board, config, kit, memgraph, metrics, store, workspaces
from easy_personal.surfaces import api
from tests_helpers import add_event

pytestmark = pytest.mark.usefixtures("real_projects")  # these tests are not about Scratch (FR-069)

GOLDEN = json.loads((Path(__file__).parent / "hun186_golden.json").read_text())
TOKEN, PORT = "tok-186", 5586
AUTH = {api.TOKEN_HEADER: TOKEN}
UNCHANGED = (
    ["summary", "summary_all", "outliers", "sessions", "sessions_all", "warnings", "plan"]
    + [f"breakdown_{by}" for by in metrics.DIMENSIONS]
)  # fmt: skip


@pytest.fixture(autouse=True)
def _utc(monkeypatch):
    """The golden groups days by local day: pin the zone it was captured in."""
    monkeypatch.setenv("TZ", "UTC")
    time.tzset()
    yield
    monkeypatch.undo()
    time.tzset()


@pytest.fixture
def world(tmp_path):
    root = tmp_path / "ws"
    paths = seed(root, config.data_dir())
    return {"root": root, "paths": paths}


@pytest.fixture
def client(tmp_path):
    return TestClient(api.create_app(TOKEN, PORT, tmp_path / "absent"), base_url=f"http://127.0.0.1:{PORT}")


def _alive(items):
    return [p for p in items if not p.get("missing_on_disk")]


# AC 4: golden equality, old output vs new output
def test_unchanged_reads_equal_the_golden(conn, world):
    with store.opened(config.store_path()) as c:
        now = normalized(snapshot(c), world["root"])
    for key in UNCHANGED:
        assert now[key] == GOLDEN[key], key


def test_mcp_kit_summary_keeps_the_missing_rows(world):
    with store.opened(config.store_path()) as c:
        now = normalized(snapshot(c), world["root"])
    assert now["kit"] == GOLDEN["kit"]  # AC 5: MCP / CLI output unchanged, `missing_on_disk` rows included
    assert "gone" not in now["kit"]
    assert [p["project"] for p in now["kit"]["projects"]] == ["alive-a", "alive-b", "gone-x", "gone-y"]


# AC 5: gone projects leave the three lists and are counted
def test_kit_for_the_dashboard_drops_gone_and_counts_them(world):
    with store.opened(config.store_path()) as c:
        got = normalized(kit.kit_summary(c, hide_gone=True), world["root"])
    expected = dict(GOLDEN["kit"], gone=2, projects=_alive(GOLDEN["kit"]["projects"]))
    assert got == expected


def test_board_drops_gone_and_counts_them(world):
    with store.opened(config.store_path()) as c:
        got = normalized(board.board(c), world["root"])
    names = {p["project"] for p in GOLDEN["board"]["projects"]}
    assert names == {"alive-a", "alive-b", "gone-x", "gone-y"}  # before: every stored project
    assert got == {"projects": [p for p in GOLDEN["board"]["projects"] if p["project"].startswith("alive")], "gone": 2}


def test_memory_list_drops_gone_and_counts_only_listed_ones(world):
    gone_note = Path(world["paths"]["gone-x"]) / "memory"
    gone_note.mkdir(parents=True)  # the folder exists again for a moment: list it, then delete it
    (gone_note / "MEMORY.md").write_text("---\nname: x\n---\n")
    with store.opened(config.store_path()) as c:
        assert [p["name"] for p in memgraph.project_list(c)["projects"]] == ["alive-a", "gone-x"]
    (gone_note / "MEMORY.md").unlink()
    gone_note.rmdir()
    gone_note.parent.rmdir()
    with store.opened(config.store_path()) as c:
        got = normalized(memgraph.project_list(c), world["root"])
    assert got == dict(GOLDEN["memory_list"], gone=0)


def test_memory_list_hides_a_claude_note_of_a_gone_project_but_keeps_claude_only_keys(world):
    from easy_personal.core import accounts

    def encoded(path):  # Claude Code's folder name for a cwd
        return re.sub(r"[^A-Za-z0-9]", "-", path)

    claude = config.agent_root() / ".claude"
    for key in (world["paths"]["gone-x"], "/nowhere/unmapped-folder"):
        mem = claude / "projects" / encoded(key) / "memory"
        mem.mkdir(parents=True)
        (mem / "MEMORY.md").write_text("---\nname: n\n---\n")
    with store.opened(config.store_path()) as c:
        accounts.detect(c)
        accounts.confirm(c, str(claude), "Claude")
        got = memgraph.project_list(c)
    assert got["gone"] == 1  # gone-x
    assert sorted(p["name"] for p in got["projects"]) == sorted(["alive-a", encoded("/nowhere/unmapped-folder")])


def test_routes_carry_the_gone_count_and_no_gone_rows(client, world):
    kit_out = client.get("/api/kit", headers=AUTH).json()
    board_out = client.get("/api/board", headers=AUTH).json()
    memory_out = client.get("/api/memory-graph", headers=AUTH).json()
    assert (kit_out["gone"], board_out["gone"], memory_out["gone"]) == (2, 2, 0)
    assert [p["project"] for p in kit_out["projects"]] == ["alive-a", "alive-b"]
    assert [p["project"] for p in board_out["projects"]] == ["alive-a", "alive-b"]


def test_empty_store_has_gone_zero(client):
    for route in ("/api/kit", "/api/board", "/api/memory-graph"):
        body = client.get(route, headers=AUTH).json()
        assert body["gone"] == 0 and body["projects"] == []


def test_a_folder_deleted_between_two_requests_is_gone_on_the_next(client, world):
    assert client.get("/api/board", headers=AUTH).json()["gone"] == 2
    (Path(world["paths"]["alive-b"])).rmdir()
    body = client.get("/api/board", headers=AUTH).json()
    assert body["gone"] == 3 and [p["project"] for p in body["projects"]] == ["alive-a"]


def test_gone_counts_only_projects_in_scope(client, world):
    with store.opened(config.store_path()) as c:
        workspaces.add(c, "acme", repos=[world["paths"]["alive-a"], world["paths"]["gone-x"]])
    body = client.get("/api/kit?workspace=acme", headers=AUTH).json()
    assert body["gone"] == 1 and [p["project"] for p in body["projects"]] == ["alive-a"]
    assert client.get("/api/board?workspace=acme", headers=AUTH).json()["gone"] == 1


# AC 6: a gone project's events still count
def test_gone_project_history_still_counts(world):
    with store.opened(config.store_path()) as c:
        summary = metrics.usage_summary(c, SINCE, UNTIL)
        by_project = {g["key"]: g["messages"] for g in metrics.usage_breakdown(c, "project", SINCE, UNTIL)["groups"]}
        sessions = {s["session_id"] for s in metrics.session_timeline(c, SINCE, UNTIL)["sessions"]}
    assert by_project["gone-x"] == 2 and by_project["gone-y"] == 1
    assert {"gx1", "gy1"} <= sessions
    assert summary["messages"] == sum(by_project.values())
    assert [h["rule_id"] for h in GOLDEN["warnings"]["hits"]] == [
        "cache_hit_low", "records_unreadable", "tool_errors_high", "project_dominates", "tokens_jump",
    ]  # fmt: skip


# AC 3: the sessions payload
def _insert_sessions(c, n_sessions: int, points: int) -> None:
    """`n_sessions` sessions of `points` one-message events each (distinct start times, ids s000...)."""
    c.executemany(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,"
        " project_path, model, message_id, input_tokens, output_tokens, cache_creation_input_tokens,"
        " cache_read_input_tokens) VALUES ('m','claude_code','/x',1,?,0,?,?,'/work/p','model-0',?,10,5,0,0)",
        [
            (s * points + k, f"2026-10-15T{s % 24:02d}:{k % 60:02d}:{s // 24 % 60:02d}.000000Z", f"s{s:03d}", f"m{s}-{k}")
            for s in range(n_sessions)
            for k in range(points)
        ],
    )
    c.commit()


def _many_sessions(n_sessions: int, points: int) -> None:
    with store.opened(config.store_path()) as c:
        _insert_sessions(c, n_sessions, points)


def test_only_the_newest_fifty_sessions_carry_points_and_gaps(client):
    _many_sessions(60, 3)
    data = client.get("/api/usage/sessions", headers=AUTH).json()
    assert data["detail_sessions"] == api.TIMELINE_DETAIL_SESSIONS == 50
    sessions = data["sessions"]
    assert len(sessions) == 60
    assert all(len(s["points"]) == s["messages"] == 3 for s in sessions[:50])
    rest = sessions[50:]
    assert all(s["points"] == [] and s["gaps"] == [] and s["messages"] == 3 for s in rest)
    assert all(s["input_tokens"] == 30 and s["models"] == ["model-0"] for s in rest)  # totals and models stay


def test_sessions_payload_is_small(client):
    _many_sessions(300, 40)
    size = len(client.get("/api/usage/sessions", headers=AUTH).content)
    assert size < 1_000_000
    with store.opened(config.store_path()) as c:
        full = len(json.dumps(metrics.session_timeline(c)))
    assert full > 4 * size  # the old shape would have been several times bigger


def test_detail_sessions_none_is_the_full_answer(conn):
    _insert_sessions(conn, 3, 2)
    full = metrics.session_timeline(conn)
    assert "detail_sessions" not in full and all(len(s["points"]) == 2 for s in full["sessions"])


# NFR-013 proxies that do not depend on machine load
def test_kit_does_not_run_git_or_a_trace_scan_per_project(world, monkeypatch):
    with store.opened(config.store_path()) as c:
        for n in range(150):
            add_event(c, 1000 + n, f"/tmp/gone-test-folder-{n}")
        c.commit()
        calls = []
        monkeypatch.setattr("easy_personal.sources.git_local.run_git", lambda *a, **k: calls.append(a) or None)
        statements = []
        c.set_trace_callback(statements.append)
        got = kit.kit_summary(c)
        assert len(got["projects"]) == 154
        assert len(calls) == 2  # the two live folders only; 152 missing folders spawn nothing
        assert sum("FROM kit_trace_events" in q for q in statements) == 1  # was two full scans per project
        assert kit.kit_summary(c, hide_gone=True)["gone"] == 152


def test_light_token_queries_equal_the_full_ones(world):
    """The rules read three figures with lighter queries; each must equal what `usage_summary` / `usage_breakdown` say."""
    with store.opened(config.store_path()) as c:
        for since, until in ((SINCE, UNTIL), (None, None), ("2026-10-16T00:00:00Z", None)):
            summary = metrics.usage_summary(c, since, until)
            assert metrics.cache_hit_ratio(c, since, until) == summary["cache_hit_ratio"]
            groups = metrics.usage_breakdown(c, "project", since, until)["groups"]
            want = {
                g["key"]: g["input_tokens"] + g["output_tokens"] + g["cache_creation_input_tokens"] + g["cache_read_input_tokens"]
                for g in groups
            }  # fmt: skip
            assert metrics.tokens_by_project(c, since, until) == want
            total = metrics.usage_breakdown(c, "day", since, until)["total"]
            assert metrics.token_total(c, since, until) == sum(
                total[k] for k in ("input_tokens", "output_tokens", "cache_creation_input_tokens", "cache_read_input_tokens")
            )
        assert metrics.token_total(c, "2099-01-01T00:00:00Z") == 0
        assert metrics.cache_hit_ratio(c, "2099-01-01T00:00:00Z") is None


def test_stored_times_parse_to_the_same_instant_as_before():
    from datetime import UTC, datetime

    stored = "2026-09-09T03:52:50.097000Z"
    assert metrics._parse_utc(stored) == datetime.strptime(stored, "%Y-%m-%dT%H:%M:%S.%fZ").replace(tzinfo=UTC)
