import asyncio
import json
import time

import pytest

from easy_personal.core import metrics
from easy_personal.sources import claude_code
from easy_personal.surfaces import cli
from easy_personal.surfaces import mcp as mcp_surface

# Oracle written by the Supervisor before code (HUN-34 spawn prompt). Do not derive these values.
ORACLE = {
    "sessions": 4,
    "messages": 3,
    "input_tokens": 15,
    "output_tokens": 8,
    "cache_creation_input_tokens": 250,
    "cache_read_input_tokens": 1000,
    "cache_hit_ratio": 0.7905,
}
ORACLE_ABSTAINED = {"total": 3, "by_version": {"2.1.293": 1, "2.1.295": 1, "2.1.302": 1}}


def test_oracle_totals_over_all_fixtures(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root)
    summary = metrics.usage_summary(conn)
    for key, expected in ORACLE.items():
        assert summary[key] == expected, key
    assert summary["abstained"] == ORACLE_ABSTAINED


def test_second_ingest_adds_zero_rows(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root)
    before = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    second = claude_code.ingest(conn, fixtures_root)
    assert second["new_events"] == 0
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == before


def _ingest_fixture(conn, root, records):
    root.mkdir(exist_ok=True)
    (root / "s.jsonl").write_text("".join(json.dumps(r) + "\n" for r in records))
    claude_code.ingest(conn, root)


def test_period_filters_sessions_messages_abstained(conn, fixtures_root):
    claude_code.ingest(conn, fixtures_root)
    late = metrics.usage_summary(conn, since="2026-01-15T10:10:00Z")
    assert (late["sessions"], late["messages"], late["input_tokens"]) == (1, 1, 5)
    assert late["abstained"]["total"] == 0
    early = metrics.usage_summary(conn, until="2026-01-15T10:10:00Z")
    assert (early["sessions"], early["messages"], early["input_tokens"]) == (4, 2, 10)
    assert early["period"] == {"since_utc": None, "until_utc": "2026-01-15T10:10:00.000000Z"}


def test_session_spanning_midnight_counted_once(conn, tmp_path):
    base = {"type": "user", "version": "2.1.295", "cwd": "/work/demo-app", "sessionId": "s-mid"}
    _ingest_fixture(
        conn,
        tmp_path / "projects",
        [
            {**base, "timestamp": "2026-01-15T23:59:00.000Z"},
            {**base, "timestamp": "2026-01-16T00:01:00.000Z"},
        ],
    )
    day1 = metrics.usage_summary(conn, "2026-01-15T00:00:00Z", "2026-01-16T00:00:00Z")
    day2 = metrics.usage_summary(conn, "2026-01-16T00:00:00Z", "2026-01-17T00:00:00Z")
    both = metrics.usage_summary(conn, "2026-01-15T00:00:00Z", "2026-01-17T00:00:00Z")
    assert (day1["sessions"], day2["sessions"], both["sessions"]) == (1, 1, 1)


def test_naive_bound_is_local_time(monkeypatch):
    monkeypatch.setenv("TZ", "Asia/Ho_Chi_Minh")
    time.tzset()
    try:
        assert metrics._utc_bound("2026-01-16") == "2026-01-15T17:00:00.000000Z"
    finally:
        monkeypatch.undo()
        time.tzset()


def test_empty_store_ratio_is_none(conn):
    summary = metrics.usage_summary(conn)
    assert summary["sessions"] == 0 and summary["cache_hit_ratio"] is None


def _cli_json(capsys, argv):
    assert cli.main(argv) == 0
    return json.loads(capsys.readouterr().out)


def test_mcp_and_cli_return_identical_numbers(tmp_path, monkeypatch, capsys, fixtures_root):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(fixtures_root))
    ingest = _cli_json(capsys, ["ingest"])
    assert ingest["new_events"] > 0
    assert _cli_json(capsys, ["ingest"])["new_events"] == 0

    app = mcp_surface.create_app()
    assert "usage_summary" in {t.name for t in asyncio.run(app.list_tools())}
    for period in ({}, {"since": "2026-01-15T10:10:00Z"}, {"until": "2026-01-15T10:10:00Z"}):
        argv = ["usage"] + [x for k, v in period.items() for x in (f"--{k}", v)]
        cli_summary = _cli_json(capsys, argv)
        _, mcp_summary = asyncio.run(app.call_tool("usage_summary", period))
        assert cli_summary == mcp_summary
    full = _cli_json(capsys, ["usage"])
    for key, expected in ORACLE.items():
        assert full[key] == expected, key
    assert full["abstained"] == ORACLE_ABSTAINED


def test_ingest_source_flag_overrides_env(tmp_path, monkeypatch, capsys, fixtures_root):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(tmp_path / "absent"))
    assert _cli_json(capsys, ["ingest", "--source", str(fixtures_root)])["files"] == 5


def test_cli_usage_rejects_bad_date(tmp_path, monkeypatch, capsys):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    with pytest.raises(SystemExit) as exc:
        cli.main(["usage", "--since", "yesterday"])
    assert exc.value.code == 2
    assert "yesterday" in capsys.readouterr().err
