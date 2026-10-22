import os
import subprocess
import time
from datetime import UTC, datetime, timedelta

import pytest

from easy_personal.core import kit, rules
from easy_personal.sources import context
from fakes.mcp_server import fake_context, index_entry

CANARY = "LEAKCANARY-context-0123456789"
NOW = datetime(2026, 10, 21, 12, 0, tzinfo=UTC)


@pytest.fixture(autouse=True)
def _home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))


def _git_commit(repo, when):
    stamp = f"{int(when.timestamp())} +0000"
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null",
           "GIT_AUTHOR_DATE": stamp, "GIT_COMMITTER_DATE": stamp}  # fmt: skip
    subprocess.run(["git", "-c", "user.email=t@e.invalid", "-c", "user.name=t", "-C", str(repo),
                    "commit", "-q", "--allow-empty", "-m", CANARY], check=True, env=env)  # fmt: skip


@pytest.fixture
def repo(tmp_path):
    path = tmp_path / "repo"
    path.mkdir()
    subprocess.run(["git", "-C", str(path), "init", "-q", "-b", "main"], check=True)
    return path


def _known(conn, *projects):
    for p in projects:
        conn.execute(
            "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, "
            "project_path, ts_utc) VALUES ('m', 'claude_code', 'p', 0, ?, 0, ?, '2026-10-01T00:00:00+00:00')", (abs(hash(str(p))), str(p)),
        )  # fmt: skip
    conn.commit()


def _iso(moment):
    return moment.isoformat()


# AC1: allow-list
def test_only_read_tools_can_be_sent():
    assert context.ALLOWED_TOOLS == {"list_indexes", "get_stats"}


def test_index_path_is_refused_locally_without_a_network_call():
    with fake_context([]) as (url, calls, _roots):
        for tool in ("index_path", "search_semantic", "anything"):
            with pytest.raises(context.Refused):
                context.call_tool(tool, url=url, root="/x")
        assert calls == []  # the fake saw nothing


# AC2: host guard
@pytest.mark.parametrize(
    "url",
    ["http://example.com/mcp", "http://localhost:8767/mcp", "http://0.0.0.0:8767/mcp",
     "https://127.0.0.1:8767/mcp", "http://127.0.0.1@evil.example/mcp",
     "http://evil.example@127.0.0.1:1/mcp", "http://127.0.0.1.evil.example/mcp", "file:///x"],
)  # fmt: skip
def test_non_loopback_urls_are_refused(url):
    with pytest.raises(context.Refused):
        context.call_tool("list_indexes", url=url, root="/x")


# AC3: down / slow -> no data, ingest completes
def test_service_down_is_no_data_and_clears_old_rows(conn, tmp_path):
    _known(conn, tmp_path)
    conn.execute(
        "INSERT INTO context_index VALUES (?, ?)", (str(tmp_path), "2026-01-01T00:00:00+00:00")
    )
    result = context.ingest(conn, url="http://127.0.0.1:9/mcp")  # nothing listens on port 9
    assert result == {"status": "no data"}
    assert conn.execute("SELECT COUNT(*) FROM context_index").fetchone()[0] == 0
    assert context.context_part(conn, str(tmp_path)) == {"status": "no data"}


def test_slow_service_is_bounded(conn, tmp_path, monkeypatch):
    monkeypatch.setattr(context, "TIMEOUT_S", 0.3)
    _known(conn, tmp_path)
    with fake_context([], delay=3) as (url, _calls, _roots):
        started = time.monotonic()
        assert context.ingest(conn, url=url) == {"status": "no data"}
        assert time.monotonic() - started < 2.5


def test_ingest_cli_exits_0_when_service_is_down(tmp_path, monkeypatch, capsys):
    from easy_personal.surfaces import cli

    monkeypatch.setenv(context.URL_ENV, "http://127.0.0.1:9/mcp")
    monkeypatch.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(tmp_path / "none"))
    assert cli.main(["ingest", "--source", str(tmp_path / "none")]) == 0
    assert '"context"' in capsys.readouterr().out


# Success: freshness stored
def test_ingest_stores_last_index_at_for_known_projects_only(conn, repo, tmp_path):
    _known(conn, repo)
    gone = str(tmp_path / "gone")
    _known(conn, gone)
    entries = [
        index_entry(str(repo), "2026-10-21T10:00:00.000+00:00"),
        index_entry(str(tmp_path / "other"), "2026-10-21T10:00:00+00:00"),  # not a known project
        index_entry(
            gone, "2026-10-21T10:00:00+00:00", root_exists=False
        ),  # index of a deleted repo
    ]
    with fake_context(entries) as (url, calls, roots):
        result = context.ingest(conn, url=url)
    assert result == {"status": "ok", "indexes": 1}
    assert calls == ["list_indexes"]  # AC5: one call covers every repo
    assert roots == [str(repo)]  # a root header is sent
    assert conn.execute("SELECT * FROM context_index").fetchall() == [
        (str(repo), "2026-10-21T10:00:00+00:00")
    ]


def test_next_root_is_tried_when_the_service_rejects_one(conn, repo, tmp_path):
    other = tmp_path / "aaa-first"
    other.mkdir()
    _known(conn, other, repo)
    with fake_context([index_entry(str(repo), "2026-10-21T10:00:00+00:00")],
                      reject_roots={str(other)}) as (url, _calls, roots):  # fmt: skip
        assert context.ingest(conn, url=url)["status"] == "ok"
    assert roots == [str(other), str(repo)]


# AC4: stale
def test_stale_index_fires_the_rule_with_the_index_path_fix(conn, repo):
    _git_commit(repo, NOW - timedelta(hours=1))
    _known(conn, repo)
    with fake_context([index_entry(str(repo), _iso(NOW - timedelta(days=2)))]) as (url, *_):
        context.ingest(conn, url=url)
    part = kit.kit_summary(conn)["projects"][0]["parts"]["context"]
    assert part["status"] == "ok" and part["stale"] is True
    assert part["last_index_at"] == _iso(NOW - timedelta(days=2))
    hits = {h["rule_id"]: h for h in rules.evaluate(conn, now=datetime.now(UTC))["hits"]}
    hit = hits["context_index_stale"]
    assert hit["measured"] == 1 and hit["detail"] == {"projects": ["repo"]}
    assert "`index_path`" in hit["fix"]
    assert CANARY not in str(hit) and CANARY not in str(part)


def test_fresh_index_does_not_fire(conn, repo):
    _git_commit(repo, NOW - timedelta(days=2))
    _known(conn, repo)
    with fake_context([index_entry(str(repo), _iso(NOW))]) as (url, *_):
        context.ingest(conn, url=url)
    assert context.context_part(conn, str(repo))["stale"] is False
    out = rules.evaluate(conn, now=datetime.now(UTC))
    assert "context_index_stale" not in {h["rule_id"] for h in out["hits"]}
    assert "context_index_stale" not in {a["rule_id"] for a in out["abstained"]}


def test_rule_abstains_without_context_data(conn, repo):
    _known(conn, repo)
    out = rules.evaluate(conn, now=datetime.now(UTC))
    assert {"rule_id": "context_index_stale",
            "reason": "no easy-context data (service down or no index for a known project)"} in out["abstained"]  # fmt: skip


def test_clock_skew_index_in_the_future_is_not_judged(conn, repo):
    _git_commit(repo, NOW - timedelta(days=1))
    _known(conn, repo)
    future = datetime.now(UTC) + timedelta(hours=3)  # service clock ahead of ours
    with fake_context([index_entry(str(repo), _iso(future))]) as (url, *_):
        context.ingest(conn, url=url)
    assert context.context_part(conn, str(repo))["stale"] is None  # cannot trust that clock


def test_commit_within_skew_window_is_not_stale(conn, repo):
    now = datetime.now(UTC)
    _git_commit(repo, now - timedelta(seconds=10))
    _known(conn, repo)
    with fake_context([index_entry(str(repo), _iso(now - timedelta(seconds=40)))]) as (url, *_):
        context.ingest(conn, url=url)
    assert context.context_part(conn, str(repo))["stale"] is False


def test_repo_without_commits_or_missing_folder_is_not_judged(conn, repo, tmp_path):
    _known(conn, repo)
    with fake_context([index_entry(str(repo), _iso(NOW))]) as (url, *_):
        context.ingest(conn, url=url)
    assert context.context_part(conn, str(repo))["stale"] is None
    assert context.stale_projects(conn) == (0, [])


def test_unreadable_last_index_at_is_stored_as_null_and_not_judged(conn, repo):
    _git_commit(repo, NOW)
    _known(conn, repo)
    with fake_context([index_entry(str(repo), "not a date")]) as (url, *_):
        context.ingest(conn, url=url)
    assert context.context_part(conn, str(repo))["stale"] is None
