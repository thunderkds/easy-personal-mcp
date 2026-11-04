"""`workspace` / `account` scope on warnings, plan and kit: core, API, MCP and CLI (HUN-117)."""

import asyncio
import json
from datetime import datetime
from pathlib import Path

import pytest
from scope_fixture import NOW, seed
from starlette.testclient import TestClient

from easy_personal.core import kit, plan, rules, store
from easy_personal.surfaces import api, cli
from easy_personal.surfaces import mcp as mcp_surface

pytestmark = pytest.mark.usefixtures("real_projects")  # these tests are not about Scratch (FR-069)

GOLDEN = json.loads((Path(__file__).parent / "scope_golden.json").read_text())
TOKEN, PORT = "tok-117", 5555
AUTH = {api.TOKEN_HEADER: TOKEN}


class FrozenDatetime(datetime):
    @classmethod
    def now(cls, tz=None):
        return NOW


@pytest.fixture
def conn(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path))
    monkeypatch.setattr(rules, "datetime", FrozenDatetime)  # surfaces call evaluate() with the wall clock
    seed(tmp_path)
    with store.opened(tmp_path / "store.sqlite") as c:
        yield c


def hits(result):
    return {h["rule_id"]: h for h in result["hits"]}


def abstained(result):
    return {a["rule_id"]: a["reason"] for a in result["abstained"]}


def projects(result):
    return [p["project"] for p in result["projects"]]


# AC 1: no scope = today's output, byte for byte (golden captured before the change)
def test_no_scope_equals_golden(conn):
    assert rules.evaluate(conn, NOW) == GOLDEN["warnings"]
    assert plan.improvement_plan(conn, NOW) == GOLDEN["plan"]
    assert kit.kit_summary(conn) == GOLDEN["kit"]
    assert rules.evaluate(conn, NOW, workspace=None, account=None) == GOLDEN["warnings"]


# AC 2 / success 1: only account 2 reads no cache
def test_account_scopes_usage_rules(conn):
    assert "cache_hit_low" not in hits(rules.evaluate(conn, NOW))
    assert "cache_hit_low" not in hits(rules.evaluate(conn, NOW, account="Claude default"))
    low = hits(rules.evaluate(conn, NOW, account="Claude work"))["cache_hit_low"]
    assert low["measured"] == 0.0
    assert hits(rules.evaluate(conn, NOW, account="2"))["cache_hit_low"]["measured"] == 0.0
    items = plan.improvement_plan(conn, NOW, account="Claude work")["groups"][0]["items"]
    assert items[0]["rule_id"] == "cache_hit_low"


# AC 3: workspace scopes usage rules
def test_workspace_scopes_usage_rules(conn):
    # wa = /work/a only: all account-1 cache reads -> no hit; one project -> project_dominates abstains
    result = rules.evaluate(conn, NOW, workspace="wa")
    assert "cache_hit_low" not in hits(result)
    assert abstained(result)["project_dominates"] == "fewer than two projects with tokens in the window"
    # wc = /work/c + /work/b; ANDed with an account only that account's events in those projects count
    work = rules.evaluate(conn, NOW, workspace="wc", account="Claude work")
    assert hits(work)["cache_hit_low"]["measured"] == 0.0  # /work/c and /work/b, account 2: no cache reads
    default = rules.evaluate(conn, NOW, workspace="wc", account="Claude default")
    assert "cache_hit_low" not in hits(default)  # only account 1's /work/b events, all cached


# edge: tokens_jump uses the same scope for both weeks
def test_tokens_jump_scopes_both_weeks(conn):
    assert hits(rules.evaluate(conn, NOW))["tokens_jump"]["measured"] == 102.0
    assert "no tokens in the previous window" not in abstained(rules.evaluate(conn, NOW, account="2")).values()
    # account 2: previous week 2 messages x 15 tokens = 30; current 4 x (15 + 0 cache) = 60 -> +100%
    assert hits(rules.evaluate(conn, NOW, account="2"))["tokens_jump"]["measured"] == 1.0
    # account 1: previous week 1 message x 15 = 15; current 5 x (15 + 900) -> huge
    assert hits(rules.evaluate(conn, NOW, account="1"))["tokens_jump"]["measured"] > 100


# edge: tool_errors_high filters kit traces by the scope's projects
def test_kernel_error_rate_follows_scope_projects(conn):
    assert hits(rules.evaluate(conn, NOW))["tool_errors_high"]["measured"] == 0.25
    assert "tool_errors_high" not in hits(rules.evaluate(conn, NOW, workspace="wa"))  # 0 of 10
    assert hits(rules.evaluate(conn, NOW, workspace="wc"))["tool_errors_high"]["measured"] == 0.5
    # account 2 used /work/b and /work/c; account 1 used /work/a and /work/b
    assert hits(rules.evaluate(conn, NOW, account="2"))["tool_errors_high"]["measured"] == 0.5


def test_workspace_without_matching_projects_selects_nothing(conn):
    from easy_personal.core import workspaces

    workspaces.add(conn, "empty", repos=["/work/none"])
    result = rules.evaluate(conn, NOW, workspace="empty")
    assert result["hits"] == []
    assert abstained(result)["tool_errors_high"] == "no kernel-trace tool calls in the window"
    assert kit.kit_summary(conn, workspace="empty")["projects"] == []


# AC 3 / success 2 and AC 4: kit lists only the scope's projects
def test_kit_summary_filters_projects(conn):
    assert projects(kit.kit_summary(conn)) == ["a", "b", "c"]
    assert projects(kit.kit_summary(conn, workspace="wa")) == ["a"]
    assert projects(kit.kit_summary(conn, workspace="wc")) == ["b", "c"]
    assert projects(kit.kit_summary(conn, account="Claude work")) == ["b", "c"]
    assert projects(kit.kit_summary(conn, account="1")) == ["a", "b"]
    assert projects(kit.kit_summary(conn, workspace="wc", account="Claude default")) == ["b"]
    assert projects(kit.kit_summary(conn, workspace="wa", account="Claude work")) == []


# AC 5: unknown workspace / account raise the same errors as the usage metrics
def test_unknown_scope_raises_even_on_an_empty_store(tmp_path, monkeypatch):
    from easy_personal.core import accounts, workspaces

    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path))
    with store.opened(tmp_path / "store.sqlite") as c:
        with pytest.raises(workspaces.WorkspaceError):
            rules.evaluate(c, NOW, workspace="nope")
        with pytest.raises(accounts.AccountError):
            plan.improvement_plan(c, NOW, account="nope")
        with pytest.raises(workspaces.WorkspaceError):
            kit.kit_summary(c, workspace="nope")
        with pytest.raises(accounts.AccountError):
            kit.kit_summary(c, account="nope")


@pytest.fixture
def client(conn, monkeypatch):
    monkeypatch.setattr(kit, "split_gone", lambda paths: (paths, 0))  # the /work/* folders are fake: keep them, this tests scope
    return TestClient(api.create_app(TOKEN, PORT), base_url=f"http://127.0.0.1:{PORT}")


def get(client, path, query=""):
    return client.get(path + query, headers=AUTH)


def test_api_routes_read_scope_from_the_query(client):
    kit_names = lambda q: [p["project"] for p in get(client, "/api/kit", q).json()["projects"]]
    assert kit_names("") == ["a", "b", "c"]
    assert kit_names("?workspace=wa") == ["a"]
    assert kit_names("?workspace=") == ["a", "b", "c"]  # empty = no filter, like the usage routes
    assert kit_names("?account=Claude%20work") == ["b", "c"]
    assert kit_names("?workspace=wc&account=1") == ["b"]
    work = get(client, "/api/warnings", "?account=Claude%20work").json()
    assert "cache_hit_low" in hits(work)
    assert "cache_hit_low" not in hits(get(client, "/api/warnings", "?account=1").json())
    assert get(client, "/api/plan", "?account=2").json()["groups"][0]["items"][0]["rule_id"] == "cache_hit_low"
    wa_plan = get(client, "/api/plan", "?workspace=wa").json()["groups"]
    assert [i["rule_id"] for g in wa_plan for i in g["items"]] == ["tokens_jump"]  # no tool errors in wa


def test_api_without_scope_is_the_golden_shape(client):
    assert get(client, "/api/warnings").json() == GOLDEN["warnings"]
    assert get(client, "/api/plan").json() == GOLDEN["plan"]
    assert get(client, "/api/kit").json() == {**GOLDEN["kit"], "gone": 0}


@pytest.mark.parametrize("path", ["/api/warnings", "/api/plan", "/api/kit"])
def test_api_unknown_scope_is_400_like_the_usage_routes(client, path):
    usage = get(client, "/api/usage/summary", "?workspace=nope")
    res = get(client, path, "?workspace=nope")
    assert res.status_code == usage.status_code == 400
    assert res.json() == usage.json()
    usage = get(client, "/api/usage/summary", "?account=nope-secret")
    res = get(client, path, "?account=nope-secret")
    assert res.status_code == usage.status_code == 400
    assert res.json() == usage.json() and "nope-secret" not in res.text


def run(capsys, *argv, code=0):
    rc = cli.main(list(argv))
    out = capsys.readouterr()
    assert rc == code, out
    return out


def test_cli_scope_flags(conn, capsys):
    kit_out = json.loads(run(capsys, "kit", "--workspace", "wa").out)
    assert projects(kit_out) == ["a"]
    assert projects(json.loads(run(capsys, "kit", "--account", "Claude work").out)) == ["b", "c"]
    warn = json.loads(run(capsys, "warnings", "--account", "2").out)
    assert "cache_hit_low" in hits(warn)
    groups = json.loads(run(capsys, "plan", "--account", "2").out)["groups"]
    assert groups[0]["items"][0]["rule_id"] == "cache_hit_low"
    warn = json.loads(run(capsys, "warnings", "--workspace", "wa", "--account", "1").out)
    assert "cache_hit_low" not in hits(warn)


@pytest.mark.parametrize("command", ["warnings", "plan", "kit"])
def test_cli_unknown_scope_exits_2(conn, capsys, command):
    out = run(capsys, command, "--account", "nope-secret", code=2)
    assert "Claude work" in out.err and "nope-secret" not in out.err + out.out and out.out == ""
    out = run(capsys, command, "--workspace", "nope", code=2)
    assert out.out == "" and out.err.startswith("easy-personal:")


def test_mcp_tools_accept_scope_and_match_cli(conn, capsys):
    app = mcp_surface.create_app()
    schemas = {t.name: t.inputSchema["properties"] for t in asyncio.run(app.list_tools())}
    for name in ("warnings", "improvement_plan", "kit_summary"):
        assert set(schemas[name]) == {"workspace", "account"}, name
    _, got = asyncio.run(app.call_tool("kit_summary", {"workspace": "wa"}))
    assert got == json.loads(run(capsys, "kit", "--workspace", "wa").out)
    _, got = asyncio.run(app.call_tool("warnings", {"account": "Claude work"}))
    want = json.loads(run(capsys, "warnings", "--account", "Claude work").out)
    assert got == want
    _, got = asyncio.run(app.call_tool("improvement_plan", {"account": "2"}))
    assert got == json.loads(run(capsys, "plan", "--account", "2").out)
    _, noargs = asyncio.run(app.call_tool("kit_summary", {}))
    assert projects(noargs) == ["a", "b", "c"]
