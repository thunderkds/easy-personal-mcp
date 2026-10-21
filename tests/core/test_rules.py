import json
from datetime import UTC, datetime, timedelta

import pytest

from easy_personal.core import plan, rules
from easy_personal.sources import claude_code

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
    ("cr", "expected"), [(49, "fired"), (50, "ok"), (51, "ok")]  # 50/100 is not "< 50%"
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
        {**_msg(k, DAY), "type": "user", "message": None, "version": "9.9.9"} for k in range(100 - bad)
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
    }
    assert all(a["reason"] for a in out["abstained"])


def test_empty_store_abstains_everything(conn):
    out = rules.evaluate(conn, now=NOW)
    assert out["hits"] == []
    assert len(out["abstained"]) == 5


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


def test_missing_config_uses_defaults(tmp_path):
    defaults = rules.load_rules(tmp_path / "absent.yaml")
    assert {r.id: r.threshold for r in defaults} == {
        "cache_hit_low": 0.5,
        "session_huge_no_compact": 150_000,
        "tokens_jump": 0.5,
        "project_dominates": 0.6,
        "records_unreadable": 0.02,
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
