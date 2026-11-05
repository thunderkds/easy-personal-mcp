"""easy-context not in use: its rule and its kit part are left out (HUN-225, FR-080 server).

In use = the scope's stored events hold a tool call named `mcp__easy-context__*` in the 7 days ending at `now`.
"""

from datetime import UTC, datetime, timedelta

import pytest

from easy_personal.core import kit, plan, reports, rules, workspaces
from easy_personal.sources import context
from tests_helpers import add_event

pytestmark = pytest.mark.usefixtures("real_projects")  # these tests are not about Scratch (FR-069)

NOW = datetime(2026, 10, 21, 12, 0, tzinfo=UTC)
DAY = timedelta(days=1)
RULE = "context_index_stale"
CALL = "mcp__easy-context__search_semantic"


@pytest.fixture(autouse=True)
def _home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))


def _ts(ago):
    return (NOW - ago).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def _store(conn, project="/work/demo"):
    """15 days of history for `project` (the rules need a history start) and no tool call."""
    add_event(conn, 0, project, ts=_ts(15 * DAY))
    conn.commit()


def _call(conn, n, ago, tools=CALL, project="/work/demo", account=None):
    """One stored event with the given tool names, `ago` before NOW."""
    add_event(conn, n, project, ts=_ts(ago))
    conn.execute(
        "UPDATE events SET tool_names = ?, agent_account_id = ? WHERE byte_offset = ?", (tools, account, n)
    )
    conn.commit()


def _in_use(conn, now=NOW, **scope):
    return context.in_use(conn, (now - timedelta(days=7)).isoformat(), now.isoformat(), **scope)


def _ids(out):
    return (
        {h["rule_id"] for h in out["hits"]},
        {a["rule_id"] for a in out["abstained"]},
        {c["rule_id"] for c in out.get("checked", [])},
    )


# AC 1, 2, 3 and the edge cases: the function
def test_empty_store_is_not_in_use(conn):
    assert _in_use(conn) is False


def test_a_call_three_days_ago_is_in_use(conn):
    _call(conn, 1, 3 * DAY)
    assert _in_use(conn) is True


def test_a_call_among_other_tools_in_a_comma_list_is_in_use(conn):
    _call(conn, 1, DAY, tools=f"Bash,{CALL},Read")
    assert _in_use(conn) is True


@pytest.mark.parametrize("ago", [8 * DAY, 7 * DAY + timedelta(seconds=1)])
def test_a_call_older_than_the_window_is_not_in_use(conn, ago):
    _call(conn, 1, ago)
    assert _in_use(conn) is False


def test_the_window_ends_at_now(conn):
    _call(conn, 1, -DAY)  # a call after `now` (a later read of the same store)
    assert _in_use(conn) is False


@pytest.mark.parametrize(
    "tools", ["mcp__easy-context-old__search", "mcp__easy-contextual__x", "Bash,mcp__easy-memory__search", "easy-context"]
)
def test_only_the_exact_server_prefix_counts(conn, tools):
    _call(conn, 1, DAY, tools=tools)
    assert _in_use(conn) is False


def test_an_abstained_row_does_not_count(conn):
    _call(conn, 1, DAY)
    conn.execute("UPDATE events SET abstained = 1")
    assert _in_use(conn) is False


# AC 4: scope
def test_a_call_only_in_another_scope_is_not_in_use(conn):
    conn.execute(
        "INSERT INTO agent_accounts (provider, folder, label, status, config_dir_env) VALUES "
        "('claude', '/a/.claude', 'Claude default', 'confirmed', NULL),"
        "('claude', '/a/.claude-work', 'Claude work', 'confirmed', '/a/.claude-work')"
    )
    workspaces.add(conn, "wa", repos=["/work/a"])
    workspaces.add(conn, "wb", repos=["/work/b"])
    _call(conn, 1, DAY, project="/work/b", account=2)
    assert _in_use(conn) is True
    assert _in_use(conn, workspace="wb") is True and _in_use(conn, workspace="wa") is False
    assert _in_use(conn, account="Claude work") is True and _in_use(conn, account="1") is False
    assert _in_use(conn, workspace="wb", account="1") is False


# AC 1, 2: rules and plan
def test_not_in_use_leaves_the_rule_out_everywhere(conn):
    _store(conn)
    out = rules.evaluate(conn, now=NOW, checked=True)
    assert RULE not in set().union(*_ids(out))
    assert RULE not in {i["rule_id"] for g in plan.improvement_plan(conn, now=NOW)["groups"] for i in g["items"]}
    assert len(rules.load_rules()) - 1 == len(out["abstained"]) + len(out["checked"])  # every other rule is still there


def test_an_empty_store_leaves_the_rule_out_too(conn):
    out = rules.evaluate(conn, now=NOW)
    assert RULE not in _ids(out)[0] | _ids(out)[1]
    assert len(out["abstained"]) == len(rules.load_rules()) - 1


def test_in_use_the_rule_behaves_as_before(conn):
    _store(conn)
    _call(conn, 1, 3 * DAY)
    out = rules.evaluate(conn, now=NOW)
    assert [a for a in out["abstained"] if a["rule_id"] == RULE] == [
        {
            "rule_id": RULE,
            "title": "Code index is out of date",
            "reason": "no easy-context data (service down or no index for a known project)",
        }
    ]
    conn.execute("INSERT INTO context_index (project_path, last_index_at_utc) VALUES ('/work/demo', ?)", (_ts(DAY),))
    out = rules.evaluate(conn, now=NOW, checked=True)
    assert [c for c in out["checked"] if c["rule_id"] == RULE] == [
        {"rule_id": RULE, "title": "Code index is out of date", "measured": 0, "threshold": 0, "fired": False}
    ]


def test_a_call_in_another_scope_leaves_the_rule_out_for_this_scope(conn):
    conn.execute(
        "INSERT INTO agent_accounts (provider, folder, label, status, config_dir_env) VALUES "
        "('claude', '/a/.claude', 'Claude default', 'confirmed', NULL),"
        "('claude', '/a/.claude-work', 'Claude work', 'confirmed', '/a/.claude-work')"
    )
    add_event(conn, 0, "/work/demo", ts=_ts(15 * DAY))
    conn.execute("UPDATE events SET agent_account_id = 1")
    _call(conn, 1, DAY, account=2)
    assert RULE in _ids(rules.evaluate(conn, now=NOW, account="Claude work"))[1]
    assert RULE not in set().union(*_ids(rules.evaluate(conn, now=NOW, account="1")))


# AC 5: saved reports
def _measured_then(conn):
    """A store where the rule is measured (and clear) at NOW: a call yesterday and an index row."""
    _store(conn)
    _call(conn, 1, DAY)
    conn.execute("INSERT INTO context_index (project_path, last_index_at_utc) VALUES ('/work/demo', ?)", (_ts(DAY),))
    conn.commit()


def _rule(report, rule_id=RULE):
    return next((r for r in report["rules"] if r["rule_id"] == rule_id), None)


def test_a_rule_measured_then_shows_not_in_use_now(conn):
    _measured_then(conn)
    saved = reports.save(conn, "Summary", [], now=NOW)
    assert _rule(reports.get_report(conn, saved["id"], now=NOW))["now"]["state"] == "clear"
    later = reports.get_report(conn, saved["id"], now=NOW + 7 * DAY)  # the call is now 8 days old
    row = _rule(later)
    assert row["before"]["state"] == "clear"
    assert row["now"] == {"state": "not_in_use", "reason": "not in use"}


def test_a_rule_abstained_then_and_not_in_use_now_is_left_out(conn):
    _store(conn)
    _call(conn, 1, DAY)  # in use at save time, no index data: abstained
    saved = reports.save(conn, "Summary", [], now=NOW)
    assert _rule(reports.get_report(conn, saved["id"], now=NOW))["before"]["state"] == "abstained"
    later = reports.get_report(conn, saved["id"], now=NOW + 7 * DAY)
    assert _rule(later) is None
    assert _rule(later, "cache_hit_low")  # every other rule stays


def test_a_report_saved_while_not_in_use_has_no_such_rule_and_it_returns_with_a_call(conn):
    _store(conn)
    saved = reports.save(conn, "Summary", [], now=NOW)
    assert _rule(reports.get_report(conn, saved["id"], now=NOW)) is None
    _call(conn, 1, DAY)
    assert _rule(reports.get_report(conn, saved["id"], now=NOW)) is None  # it was not in the snapshot
    later_save = reports.save(conn, "Again", [], now=NOW)
    assert _rule(reports.get_report(conn, later_save["id"], now=NOW))["before"]["state"] == "abstained"


# AC 6: kit
def test_kit_context_part_says_not_in_use(conn):
    _store(conn, "/work/a")
    _call(conn, 1, 8 * DAY, project="/work/b")
    parts = [p["parts"]["context"] for p in kit.kit_summary(conn, now=NOW)["projects"]]
    assert len(parts) == 2 and parts == [{"status": "not in use"}] * 2


def test_kit_context_part_is_unchanged_when_in_use(conn):
    _store(conn, "/work/a")
    _call(conn, 1, DAY, project="/work/b")
    conn.execute("INSERT INTO context_index (project_path, last_index_at_utc) VALUES ('/work/a', ?)", (_ts(DAY),))
    parts = {p["project"]: p["parts"]["context"] for p in kit.kit_summary(conn, now=NOW)["projects"]}
    assert parts["b"] == {"status": "no data"}  # a project with no index row: as before
    assert parts["a"]["status"] == "ok" and parts["a"]["last_index_at"] == _ts(DAY)


def test_kit_context_part_follows_the_scope(conn):
    workspaces.add(conn, "wa", repos=["/work/a"])
    workspaces.add(conn, "wb", repos=["/work/b"])
    _store(conn, "/work/a")
    _call(conn, 1, DAY, project="/work/b")
    assert kit.kit_summary(conn, workspace="wa", now=NOW)["projects"][0]["parts"]["context"] == {"status": "not in use"}
    assert kit.kit_summary(conn, workspace="wb", now=NOW)["projects"][0]["parts"]["context"] == {"status": "no data"}
