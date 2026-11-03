"""HUN-193 (FR-063, FR-008): MCP `kit_findings` and CLI `kit-findings` call the same core function."""

import asyncio
import json
from datetime import UTC, datetime, timedelta

import pytest

from easy_personal.core import kitperf, store
from easy_personal.surfaces import cli
from easy_personal.surfaces import mcp as mcp_surface

START = datetime(2026, 1, 15, 10, 0, tzinfo=UTC)


def seed(path):
    """One project and model, one tool call each: 10 sessions with easy-memory (100 tokens, better), 10 with
    easy-ui (300 tokens, worse) and 12 without a kit server (200 tokens)."""
    groups = [("mcp__easy-memory__search_notes", 100)] * 10 + [("mcp__easy-ui__check", 300)] * 10 + [("Read", 200)] * 12
    with store.opened(path / "store.sqlite") as conn:
        n = 0
        for i, (tool, tokens) in enumerate(groups):
            for kind, minutes in (("assistant", 0), ("user", 10)):
                n += 1
                assistant = kind == "assistant"
                conn.execute(
                    "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type, ts_utc,"
                    " session_id, project_path, model, message_id, tool_names, input_tokens, output_tokens,"
                    " cache_creation_input_tokens, cache_read_input_tokens, tool_error_count)"
                    " VALUES ('m','claude_code','/x',1,?,0,?,?,?,'/work/p',?,?,?,?,?,0,0,?)",
                    (n, kind, (START + timedelta(minutes=minutes)).strftime("%Y-%m-%dT%H:%M:%S.000000Z"), f"s{i}",
                     "m1" if assistant else None, f"msg{n}" if assistant else None, tool if assistant else None,
                     tokens - 10 if assistant else None, 10 if assistant else None, None if assistant else 0),
                )  # fmt: skip
        conn.commit()


@pytest.fixture
def home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path))
    seed(tmp_path)
    return tmp_path


def call_mcp(**args):
    app = mcp_surface.create_app()
    _, result = asyncio.run(app.call_tool("kit_findings", args))
    return result


def run_cli(capsys, *argv):
    rc = cli.main(["kit-findings", *argv])
    out = capsys.readouterr()
    return rc, out.out, out.err


def test_the_tool_is_registered_with_limit_and_since():
    tools = {t.name: t for t in asyncio.run(mcp_surface.create_app().list_tools())}
    assert set(tools["kit_findings"].inputSchema["properties"]) == {"limit", "since"}
    assert "linked to" in tools["kit_findings"].description


def test_mcp_and_cli_json_return_the_same_dict(home, capsys):
    from_mcp = call_mcp()
    rc, out, _ = run_cli(capsys, "--json")
    from_cli = json.loads(out)
    assert rc == 0
    assert from_mcp.pop("as_of") and from_cli.pop("as_of")  # second-resolution clock
    assert from_mcp == from_cli
    assert from_mcp["sessions"] == 32
    assert [f["server"] for f in from_mcp["findings"]] == ["easy-memory"]
    assert [f["server"] for f in from_mcp["other"]] == ["easy-ui"]


def test_limit_is_passed_and_capped_at_ten(home, capsys):
    assert len(call_mcp(limit=1)["findings"]) == 1
    assert len(call_mcp(limit=99)["findings"]) == len(call_mcp(limit=10)["findings"]) <= kitperf.MAX_LIMIT
    _, out, _ = run_cli(capsys, "--json", "--limit", "1")
    assert len(json.loads(out)["findings"]) == 1


def test_since_filters_the_sessions(home, capsys):
    assert call_mcp(since="2026-02-01")["sessions"] == 0
    _, out, _ = run_cli(capsys, "--json", "--since", "2026-02-01")
    assert json.loads(out)["sessions"] == 0


def test_cli_without_json_prints_findings_first_then_other_under_a_heading(home, capsys):
    rc, out, _ = run_cli(capsys)
    lines = out.strip().splitlines()
    assert rc == 0 and len(lines) == 3
    assert lines[0].startswith("Sessions that used easy-memory are linked to") and kitperf.HABITS["easy-memory"] in lines[0]
    assert lines[1] == "Other comparisons (no habit):"
    assert lines[2].startswith("  Sessions that used easy-ui are linked to") and "more tokens per tool call" in lines[2]
    assert kitperf.HABITS["easy-ui"] not in out


def test_cli_without_json_when_only_other_has_comparisons(home, capsys, monkeypatch):
    monkeypatch.setattr(kitperf.rules, "kit_error_rate", lambda _: 0.2)
    with store.opened(home / "store.sqlite") as conn:
        conn.execute("DELETE FROM events WHERE session_id IN ('s0','s1','s2','s3','s4','s5','s6','s7','s8','s9')")  # no easy-memory sessions
        conn.commit()
    rc, out, _ = run_cli(capsys)
    lines = out.strip().splitlines()
    assert rc == 0 and lines[0] == "No kit finding with a habit yet."
    assert lines[1] == "Other comparisons (no habit):" and lines[2].startswith("  Sessions that used easy-ui")


def test_cli_without_json_says_why_when_there_is_no_finding(tmp_path, monkeypatch, capsys):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path))
    rc, out, _ = run_cli(capsys)
    assert rc == 0 and out.startswith("No kit finding yet: easy-ui 0 vs 0")


def test_a_bad_threshold_in_the_config_is_refused_like_the_other_commands(home, capsys):
    (home / "config.yaml").write_text("thresholds:\n  kit_error_rate: soon\n")
    rc, out, err = run_cli(capsys, "--json")
    assert rc == 2 and out == "" and "config thresholds must be numbers" in err
