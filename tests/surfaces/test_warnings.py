import asyncio
import json
from datetime import UTC, datetime, timedelta

import pytest

from easy_personal.surfaces import cli
from easy_personal.surfaces import mcp as mcp_surface


def _cli_json(capsys, argv):
    assert cli.main(argv) == 0
    return json.loads(capsys.readouterr().out)


def _record(n, ago, **usage):
    stamp = (datetime.now(UTC) - ago).strftime("%Y-%m-%dT%H:%M:%S.000Z")
    return {
        "type": "assistant",
        "version": "2.1.295",
        "cwd": "/work/demo-app",
        "sessionId": f"s{n}",
        "timestamp": stamp,
        "message": {
            "id": f"msg{n}",
            "model": "m",
            "content": [{"type": "text", "text": "x"}],
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
                **usage,
            },
        },
    }


@pytest.fixture
def recent(tmp_path, monkeypatch, capsys):
    """Fifteen days of history whose last week has a 0% cache-hit ratio and one 160k session."""
    root = tmp_path / "projects"
    root.mkdir()
    records = [
        _record(1, timedelta(days=15), input_tokens=1),
        _record(2, timedelta(days=1), input_tokens=160_000),
    ]
    (root / "s.jsonl").write_text("".join(json.dumps(r) + "\n" for r in records))
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(root))
    _cli_json(capsys, ["ingest"])


def _without_time(result):
    return {k: v for k, v in result.items() if k != "as_of_utc"}


def test_mcp_and_cli_warnings_identical_and_cite_values(recent, capsys):
    app = mcp_surface.create_app()
    assert {"warnings", "improvement_plan"} <= {t.name for t in asyncio.run(app.list_tools())}
    cli_out = _cli_json(capsys, ["warnings"])
    _, mcp_out = asyncio.run(app.call_tool("warnings", {}))
    assert _without_time(cli_out) == _without_time(mcp_out)
    by_id = {h["rule_id"]: h for h in cli_out["hits"]}
    assert by_id["cache_hit_low"]["measured"] == 0.0
    assert by_id["cache_hit_low"]["threshold"] == 0.5
    assert by_id["cache_hit_low"]["label"] == "warning"
    assert by_id["session_huge_no_compact"]["measured"] == 160_000


def test_mcp_and_cli_plan_identical(recent, capsys):
    app = mcp_surface.create_app()
    cli_out = _cli_json(capsys, ["plan"])
    _, mcp_out = asyncio.run(app.call_tool("improvement_plan", {}))
    assert cli_out == mcp_out
    assert [g["priority"] for g in cli_out["groups"]] == [1, 2]
    assert cli_out["groups"][0]["items"][0]["rule_id"] == "cache_hit_low"
    assert cli_out["groups"][0]["items"][0]["fix"]


def test_config_threshold_changes_cli_output(recent, tmp_path, capsys):
    (tmp_path / "home" / "config.yaml").write_text(
        "thresholds:\n  session_huge_no_compact: 170000\n"
    )
    ids = [h["rule_id"] for h in _cli_json(capsys, ["warnings"])["hits"]]
    assert "session_huge_no_compact" not in ids


def test_empty_store_abstains_without_error(tmp_path, monkeypatch, capsys):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    out = _cli_json(capsys, ["warnings"])
    assert out["hits"] == [] and len(out["abstained"]) == 5
    assert _cli_json(capsys, ["plan"]) == {"groups": []}


@pytest.mark.parametrize("command", ["warnings", "plan"])
@pytest.mark.parametrize("line", ["cache_hit_low: abc", "no_such_rule: 1"])
def test_bad_config_threshold_is_a_clean_cli_error(tmp_path, monkeypatch, capsys, command, line):
    home = tmp_path / "home"
    home.mkdir()
    (home / "config.yaml").write_text(f"thresholds:\n  {line}\n")
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(home))
    assert cli.main([command]) == 2
    err = capsys.readouterr().err.strip()
    assert err.count("\n") == 0 and "config" in err
