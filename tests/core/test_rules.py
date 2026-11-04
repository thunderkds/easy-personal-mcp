import json
from datetime import UTC, datetime, timedelta

import pytest

from easy_personal.core import plan, rules, store
from easy_personal.sources import claude_code

pytestmark = pytest.mark.usefixtures("real_projects")  # these tests are not about Scratch (FR-069)


@pytest.fixture(autouse=True)
def _home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))


NOW = datetime(2026, 10, 21, 12, 0, tzinfo=UTC)
DAY = timedelta(days=1)


def _ts(delta):
    return (NOW - delta).strftime("%Y-%m-%dT%H:%M:%S.000Z")


def _msg(n, ago, i=0, o=0, cc=0, cr=0, session="s1", cwd="/work/demo", version="2.1.295"):
    return {
        "type": "assistant",
        "version": version,
        "cwd": cwd,
        "sessionId": session,
        "timestamp": _ts(ago),
        "message": {
            "id": f"msg{n}",
            "model": "m-a",
            "content": [{"type": "text", "text": "x"}],
            "usage": {
                "input_tokens": i,
                "output_tokens": o,
                "cache_creation_input_tokens": cc,
                "cache_read_input_tokens": cr,
            },
        },
    }


def _load(conn, tmp_path, records, anchor=True):
    """Ingest records; an old tiny message gives the store 15 days of history."""
    if anchor:
        records = [_msg("anchor", 15 * DAY, i=1, session="old"), *records]
    root = tmp_path / "projects"
    root.mkdir(exist_ok=True)
    (root / "s.jsonl").write_text("".join(json.dumps(r) + "\n" for r in records))
    claude_code.ingest(conn, root)


def _result(conn, rule_id, **kw):
    out = rules.evaluate(conn, now=NOW, **kw)
    for hit in out["hits"]:
        if hit["rule_id"] == rule_id:
            return "fired", hit
    for item in out["abstained"]:
        if item["rule_id"] == rule_id:
            return "abstained", item
    return "ok", None


@pytest.mark.parametrize(
    ("cr", "expected"),
    [(49, "fired"), (50, "ok"), (51, "ok")],  # 50/100 is not "< 50%"
)
def test_cache_hit_low_boundary(conn, tmp_path, cr, expected):
    _load(conn, tmp_path, [_msg(1, DAY, i=100 - cr, cr=cr)])
    status, hit = _result(conn, "cache_hit_low")
    assert status == expected
    if hit:
        assert hit["measured"] == 0.49
        assert hit["threshold"] == 0.5
        assert hit["label"] == "warning"
        assert hit["fix"]


@pytest.mark.parametrize(("context", "expected"), [(150_001, "fired"), (150_000, "ok")])
def test_session_huge_no_compact_boundary(conn, tmp_path, context, expected):
    _load(conn, tmp_path, [_msg(1, DAY, i=context)])
    status, hit = _result(conn, "session_huge_no_compact")
    assert status == expected
    if hit:
        assert hit["measured"] == context
        assert hit["label"] == "improve"


def test_session_that_dropped_context_counts_as_compacted(conn, tmp_path):
    # Metadata only: the context fell below half of its peak, so the session was compacted.
    _load(conn, tmp_path, [_msg(1, 2 * DAY, i=300_000), _msg(2, DAY, i=20_000)])
    assert _result(conn, "session_huge_no_compact")[0] == "ok"


def test_tokens_jump_boundary(conn, tmp_path):
    prev = _msg(1, 10 * DAY, i=1000)
    for this, expected in [(1500, "ok"), (1501, "fired")]:
        conn.execute("DELETE FROM events")
        conn.execute("DELETE FROM source_files")
        conn.commit()
        _load(conn, tmp_path, [prev, _msg(2, DAY, i=this)])
        status, hit = _result(conn, "tokens_jump")
        assert status == expected
    assert hit["measured"] == 0.501
    assert hit["label"] == "aware"


def test_tokens_jump_abstains_without_previous_week(conn, tmp_path):
    _load(conn, tmp_path, [_msg(1, DAY, i=1000)])
    assert _result(conn, "tokens_jump")[0] == "abstained"


@pytest.mark.parametrize(("big", "expected"), [(61, "fired"), (60, "ok")])
def test_project_dominates_boundary(conn, tmp_path, big, expected):
    _load(
        conn,
        tmp_path,
        [_msg(1, DAY, i=big, cwd="/work/big"), _msg(2, DAY, i=100 - big, cwd="/work/small")],
    )
    status, hit = _result(conn, "project_dominates")
    assert status == expected
    if hit:
        assert hit["measured"] == 0.61
        assert hit["detail"] == {"project": "big"}


def test_project_dominates_tie_picks_first_folder_name(conn, tmp_path, monkeypatch):
    _load(conn, tmp_path, [_msg(1, DAY, i=50, cwd="/work/b"), _msg(2, DAY, i=50, cwd="/work/a")])
    custom = {r.id: r for r in rules.load_rules()}
    custom["project_dominates"] = custom["project_dominates"].with_threshold(0.4)
    _, hit = _result(conn, "project_dominates", rules=list(custom.values()))
    assert hit["detail"] == {"project": "a"}


@pytest.mark.parametrize(("bad", "expected"), [(3, "fired"), (2, "ok")])
def test_records_unreadable_boundary(conn, tmp_path, bad, expected):
    good = [
        {**_msg(k, DAY), "type": "user", "message": None, "version": "9.9.9"}
        for k in range(100 - bad)
    ]
    junk = [{"type": "mystery", "version": "9.9.9", "timestamp": _ts(DAY)} for _ in range(bad)]
    _load(conn, tmp_path, good + junk)
    status, hit = _result(conn, "records_unreadable")
    assert status == expected
    if hit:
        assert hit["measured"] == 0.03
        assert hit["detail"] == {"version": "9.9.9"}
        assert hit["label"] == "aware"


def test_too_little_history_abstains(conn, tmp_path):
    # Only 3 days of history: every windowed rule abstains (the readability share needs no span).
    records = [_msg(1, 3 * DAY, i=51, cr=49, cwd="/work/big")]
    _load(conn, tmp_path, records, anchor=False)
    out = rules.evaluate(conn, now=NOW)
    assert out["hits"] == []
    assert {a["rule_id"] for a in out["abstained"]} == {
        r.id for r in rules.load_rules() if r.min_history_days
    } | {"tool_errors_high", "context_index_stale"}  # no kernel-trace calls in this store
    assert all(a["reason"] for a in out["abstained"])


def test_empty_store_abstains_everything(conn):
    out = rules.evaluate(conn, now=NOW)
    assert out["hits"] == []
    assert len(out["abstained"]) == 7


def test_hit_carries_id_label_measured_threshold_fix(conn, tmp_path):
    _load(conn, tmp_path, [_msg(1, DAY, i=90, cr=10)])
    hit = _result(conn, "cache_hit_low")[1]
    assert {"rule_id", "label", "measured", "threshold", "fix"} <= set(hit)


def test_threshold_comes_from_config(conn, tmp_path):
    _load(conn, tmp_path, [_msg(1, DAY, i=40, cr=60)])  # hit ratio 0.6
    assert _result(conn, "cache_hit_low")[0] == "ok"
    config = tmp_path / "config.yaml"
    config.write_text("thresholds:\n  cache_hit_low: 0.7\n")
    custom = rules.load_rules(config)
    status, hit = _result(conn, "cache_hit_low", rules=custom)
    assert status == "fired"
    assert hit["threshold"] == 0.7


def test_unknown_threshold_in_config_is_an_error(tmp_path):
    config = tmp_path / "config.yaml"
    config.write_text("thresholds:\n  no_such_rule: 1\n")
    with pytest.raises(ValueError, match="no_such_rule"):
        rules.load_rules(config)


def test_live_threshold_keys_load_beside_rule_ids_and_default_when_absent(tmp_path):
    config = tmp_path / "config.yaml"
    assert rules.live_thresholds(config) == {"live_failing_streak": 3, "live_open_call_minutes": 10}
    config.write_text("thresholds:\n  cache_hit_low: 0.7\n  live_failing_streak: 5\n  live_open_call_minutes: 1.5\n")
    assert rules.live_thresholds(config) == {"live_failing_streak": 5, "live_open_call_minutes": 1.5}
    assert {r.id: r.threshold for r in rules.load_rules(config)}["cache_hit_low"] == 0.7


@pytest.mark.parametrize("key", ["live_failing_streak", "live_open_call_minutes"])
def test_live_threshold_that_is_not_a_number_is_refused_with_the_existing_message(tmp_path, key):
    config = tmp_path / "config.yaml"
    config.write_text(f"thresholds:\n  {key}: soon\n")
    for load in (rules.load_rules, rules.live_thresholds):
        with pytest.raises(ValueError, match="config thresholds must be numbers"):
            load(config)


def test_an_unknown_threshold_is_still_refused_beside_the_live_keys(tmp_path):
    config = tmp_path / "config.yaml"
    config.write_text("thresholds:\n  live_failing_streak: 5\n  live_no_such_key: 1\n")
    for load in (rules.load_rules, rules.live_thresholds):
        with pytest.raises(ValueError, match="unknown rules: live_no_such_key"):
            load(config)


def test_missing_config_uses_defaults(tmp_path):
    defaults = rules.load_rules(tmp_path / "absent.yaml")
    assert {r.id: r.threshold for r in defaults} == {
        "cache_hit_low": 0.5,
        "session_huge_no_compact": 150_000,
        "tokens_jump": 0.5,
        "project_dominates": 0.6,
        "records_unreadable": 0.02,
        "tool_errors_high": 0.1,
        "context_index_stale": 0,
    }


def test_plan_groups_by_priority_and_is_deterministic(conn, tmp_path):
    # cache hit 0 (priority 1), huge sessions (priority 2), one project at 67% (priority 3)
    _load(
        conn,
        tmp_path,
        [
            _msg(1, DAY, i=400_000, cwd="/work/big"),
            _msg(2, DAY, i=200_000, cwd="/work/small", session="s2"),
        ],
    )
    first = plan.improvement_plan(conn, now=NOW)
    assert first == plan.improvement_plan(conn, now=NOW)
    priorities = [g["priority"] for g in first["groups"]]
    assert priorities == sorted(priorities) and len(priorities) == len(set(priorities))
    items = [i for g in first["groups"] for i in g["items"]]
    ids = [i["rule_id"] for i in items]
    assert {"cache_hit_low", "session_huge_no_compact", "project_dominates"} <= set(ids)
    for item in items:
        assert item["fix"] and item["label"] and "measured" in item
    assert first["groups"][0]["items"][0]["rule_id"] == "cache_hit_low"


def test_plan_is_empty_when_nothing_fires(conn):
    assert plan.improvement_plan(conn, now=NOW) == {"groups": []}


# --- HUN-120: plain title, why, numbered steps, human summary ---------------------------------


def _by_id():
    return {r.id: r for r in rules.load_rules()}


def test_every_rule_has_title_why_and_one_to_three_steps():
    pack = rules.load_rules()
    assert len(pack) == 7
    for r in pack:
        assert r.title and r.why
        assert 1 <= len(r.steps) <= 3
        assert r.fix == " ".join(r.steps)  # old readers keep one fix line


@pytest.mark.parametrize("missing", ["title", "why", "steps"])
def test_load_rules_refuses_a_rule_without_title_why_or_steps(tmp_path, monkeypatch, missing):
    parts = {"title": "title=Low cache", "why": "why=It costs more.", "steps": "steps=Do this|Then that"}
    parts.pop(missing)
    pack = tmp_path / "rules.yaml"
    pack.write_text(
        "rules:\n  cache_hit_low: metric=cache_hit_ratio op=lt threshold=0.5 label=warning priority=1 "
        f"window_days=7 min_history_days=7 {' '.join(parts.values())}\n"
    )
    monkeypatch.setattr(rules, "PACKAGED", pack)
    with pytest.raises(ValueError, match=missing):
        rules.load_rules()


def test_old_fix_only_rule_is_refused(tmp_path, monkeypatch):
    pack = tmp_path / "rules.yaml"
    pack.write_text(
        "rules:\n  cache_hit_low: metric=cache_hit_ratio op=lt threshold=0.5 label=warning priority=1 "
        "window_days=7 min_history_days=7 fix=Keep it stable.\n"
    )
    monkeypatch.setattr(rules, "PACKAGED", pack)
    with pytest.raises(ValueError, match="cache_hit_low"):
        rules.load_rules()


def test_title_and_why_may_contain_equals_signs(tmp_path, monkeypatch):
    pack = tmp_path / "rules.yaml"
    pack.write_text(
        "rules:\n  cache_hit_low: metric=cache_hit_ratio op=lt threshold=0.5 label=warning priority=1 "
        "window_days=7 min_history_days=7 title=Low a=b reuse why=When x=y the bill grows. "
        "steps=Set k=v first|Then retry\n"
    )
    monkeypatch.setattr(rules, "PACKAGED", pack)
    (rule,) = rules.load_rules()
    assert rule.title == "Low a=b reuse"
    assert rule.why == "When x=y the bill grows."
    assert rule.steps == ("Set k=v first", "Then retry")


def test_hit_carries_title_why_steps_summary_and_keeps_old_keys(conn, tmp_path):
    _load(conn, tmp_path, [_msg(1, DAY, i=69, cr=31)])  # cache hit 31%
    hit = _result(conn, "cache_hit_low")[1]
    assert hit["title"] == "Low cache reuse"
    assert hit["why"] and 1 <= len(hit["steps"]) <= 3
    assert hit["summary"] == "31% of prompt tokens came from cache; target 50% or more, last 7 days"
    assert hit["fix"] == " ".join(hit["steps"])
    assert {"rule_id", "label", "priority", "measured", "threshold", "window_days"} <= set(hit)


def test_abstained_entry_carries_title(conn):
    out = rules.evaluate(conn, now=NOW)
    assert out["abstained"] and all(a["title"] for a in out["abstained"])
    assert all(set(a) == {"rule_id", "title", "reason"} for a in out["abstained"])


def test_plan_items_carry_title_why_steps_summary(conn, tmp_path):
    _load(conn, tmp_path, [_msg(1, DAY, i=69, cr=31)])
    item = plan.improvement_plan(conn, now=NOW)["groups"][0]["items"][0]
    assert item["title"] == "Low cache reuse"
    assert item["why"] and item["steps"] and item["summary"].startswith("31% of prompt tokens")
    assert item["fix"]


@pytest.mark.parametrize(
    ("rule_id", "measured", "detail", "expected"),
    [
        ("cache_hit_low", 0.31, None, "31% of prompt tokens came from cache; target 50% or more, last 7 days"),
        (
            "session_huge_no_compact",
            210_500,
            None,
            "The median open session reached 210,500 tokens of context; target 150,000 or fewer, last 7 days",
        ),
        (
            "records_unreadable",
            0.025,
            {"version": "2.1.295"},
            "2.5% of records from Claude Code 2.1.295 could not be read; target 2% or less, last 7 days",
        ),
        ("tokens_jump", 0.62, None, "Token use rose 62% in the last 7 days against the 7 days before; target 50% or less"),
        (
            "project_dominates",
            0.82,
            {"project": "alpha"},
            "One project (alpha) used 82% of tokens; target 60% or less, last 7 days",
        ),
        ("tool_errors_high", 0.105, None, "10.5% of kernel-trace tool calls failed; target 10% or less, last 7 days"),
        (
            "context_index_stale",
            2,
            {"projects": ["a", "b"]},
            "2 projects have a stale easy-context index; target none, last 7 days",
        ),
        (
            "context_index_stale",
            1,
            {"projects": ["a"]},
            "1 project has a stale easy-context index; target none, last 7 days",
        ),
    ],
)
def test_summary_per_metric_kind(rule_id, measured, detail, expected):
    assert rules.summary(_by_id()[rule_id], measured, detail) == expected


def test_summary_uses_the_owner_threshold():
    custom = _by_id()["cache_hit_low"].with_threshold(0.7)
    assert rules.summary(custom, 0.6, None).endswith("target 70% or more, last 7 days")


def test_records_unreadable_ignores_stale_abstained_metadata_rows_after_migration_9(tmp_path):
    # HUN-209: rows stored before `api-request` was a known metadata type kept `abstained = 1` for good.
    path = tmp_path / "stale.sqlite"
    conn = store.connect(path)
    good = [{**_msg(k, DAY), "version": "2.1.312"} for k in range(70)]
    meta = [{"type": "api-request", "version": "2.1.312", "timestamp": _ts(DAY)} for _ in range(30)]
    _load(conn, tmp_path, good + meta)
    conn.execute("UPDATE events SET abstained = 1 WHERE record_type = 'api-request'")
    conn.execute("PRAGMA user_version = 8")
    conn.commit()
    status, hit = _result(conn, "records_unreadable")
    assert status == "fired" and hit["measured"] == 0.3
    conn.close()

    conn = store.connect(path)  # the v8 -> v9 upgrade clears the stale flag
    assert _result(conn, "records_unreadable") == ("ok", None)
    conn.close()
