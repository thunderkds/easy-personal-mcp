import json
import time

import pytest

from easy_personal.core import metrics, prices
from easy_personal.sources import claude_code

TABLE = prices.PriceTable(
    as_of="2026-01-01",
    models={"m-a": prices.Price(input=1.0, output=2.0, cache_write=0.0, cache_read=0.0)},
)


def _assistant(
    n, ts, model="m-a", cwd="/work/demo-app", session="s1", tools=(), i=100, o=10, cc=0, cr=0
):
    content = [
        {"type": "tool_use", "id": f"t{n}{k}", "name": t, "input": {}} for k, t in enumerate(tools)
    ] or [{"type": "text", "text": "x"}]
    return {
        "type": "assistant",
        "version": "2.1.295",
        "cwd": cwd,
        "sessionId": session,
        "timestamp": ts,
        "isSidechain": False,
        "message": {
            "id": f"msg{n}",
            "model": model,
            "content": content,
            "usage": {
                "input_tokens": i,
                "output_tokens": o,
                "cache_creation_input_tokens": cc,
                "cache_read_input_tokens": cr,
            },
        },
    }


def _load(conn, tmp_path, records):
    root = tmp_path / "projects"
    root.mkdir(exist_ok=True)
    (root / "s.jsonl").write_text("".join(json.dumps(r) + "\n" for r in records))
    claude_code.ingest(conn, root)


@pytest.fixture
def berlin(monkeypatch):
    monkeypatch.setenv("TZ", "Europe/Berlin")
    time.tzset()
    yield
    monkeypatch.undo()
    time.tzset()


def test_days_follow_local_tz_across_dst(conn, tmp_path, berlin):
    # Berlin DST began 2026-03-29 01:00 UTC. A fixed +01:00 offset would put the last one on 03-29.
    _load(
        conn,
        tmp_path,
        [
            _assistant(1, "2026-03-28T22:30:00.000Z"),  # 23:30 CET  -> 03-28
            _assistant(2, "2026-03-28T23:30:00.000Z"),  # 00:30 CET  -> 03-29
            _assistant(3, "2026-03-29T21:30:00.000Z"),  # 23:30 CEST -> 03-29
            _assistant(4, "2026-03-29T22:30:00.000Z"),  # 00:30 CEST -> 03-30
        ],
    )
    result = metrics.usage_breakdown(conn, "day", prices=TABLE)
    assert [(g["key"], g["messages"]) for g in result["groups"]] == [
        ("2026-03-28", 1),
        ("2026-03-29", 2),
        ("2026-03-30", 1),
    ]


def test_totals_equal_usage_summary_for_every_dimension(conn, tmp_path):
    _load(
        conn,
        tmp_path,
        [
            _assistant(1, "2026-01-15T10:00:00.000Z", tools=("Read", "Bash"), cc=5, cr=50),
            _assistant(2, "2026-01-16T10:00:00.000Z", model="m-b", cwd="/work/other", cr=7),
        ],
    )
    summary = metrics.usage_summary(conn)
    for by in ("day", "model", "project", "tool"):
        total = metrics.usage_breakdown(conn, by, prices=TABLE)["total"]
        assert total["input_tokens"] == summary["input_tokens"], by
        assert total["output_tokens"] == summary["output_tokens"], by
        assert total["cache_creation_input_tokens"] == summary["cache_creation_input_tokens"]
        assert total["cache_read_input_tokens"] == summary["cache_read_input_tokens"]
        assert total["messages"] == summary["messages"], by


def test_group_by_model_and_project_folder_name_only(conn, tmp_path):
    _load(
        conn,
        tmp_path,
        [
            _assistant(1, "2026-01-15T10:00:00.000Z"),
            _assistant(2, "2026-01-15T11:00:00.000Z", model="m-b", cwd="/work/other"),
        ],
    )
    models = metrics.usage_breakdown(conn, "model", prices=TABLE)["groups"]
    assert {g["key"] for g in models} == {"m-a", "m-b"}
    projects = metrics.usage_breakdown(conn, "project", prices=TABLE)["groups"]
    assert {g["key"] for g in projects} == {"demo-app", "other"}
    assert "/work" not in json.dumps(projects)


def test_tool_groups_and_none_group(conn, tmp_path):
    _load(
        conn,
        tmp_path,
        [
            _assistant(1, "2026-01-15T10:00:00.000Z", tools=("Read",)),
            _assistant(2, "2026-01-15T10:01:00.000Z", tools=("Read", "Bash")),
            _assistant(3, "2026-01-15T10:02:00.000Z"),
        ],
    )
    groups = {
        g["key"]: g["messages"]
        for g in metrics.usage_breakdown(conn, "tool", prices=TABLE)["groups"]
    }
    assert groups == {"Read": 2, "Bash": 1, "(none)": 1}


def test_cost_labelled_and_abstains_per_model(conn, tmp_path):
    _load(
        conn,
        tmp_path,
        [
            _assistant(1, "2026-01-15T10:00:00.000Z", i=1_000_000, o=1_000_000),
            _assistant(2, "2026-01-15T11:00:00.000Z", model="m-unknown"),
        ],
    )
    result = metrics.usage_breakdown(conn, "model", prices=TABLE)
    by_key = {g["key"]: g["api_equivalent_cost"] for g in result["groups"]}
    assert by_key["m-a"] == {"usd": 3.0, "price_table_date": "2026-01-01", "abstained_models": []}
    assert by_key["m-unknown"] == {
        "usd": None,
        "price_table_date": "2026-01-01",
        "abstained_models": ["m-unknown"],
    }
    day = metrics.usage_breakdown(conn, "day", prices=TABLE)["groups"][0]
    assert day["api_equivalent_cost"]["usd"] == 3.0
    assert day["api_equivalent_cost"]["abstained_models"] == ["m-unknown"]
    assert result["total"]["api_equivalent_cost"]["price_table_date"] == "2026-01-01"


def test_empty_period_gives_empty_groups(conn):
    result = metrics.usage_breakdown(conn, "day", prices=TABLE)
    assert result["groups"] == []
    assert result["total"]["messages"] == 0


def test_period_filter(conn, tmp_path):
    _load(
        conn,
        tmp_path,
        [
            _assistant(1, "2026-01-15T10:00:00.000Z"),
            _assistant(2, "2026-01-17T10:00:00.000Z"),
        ],
    )
    result = metrics.usage_breakdown(conn, "day", since="2026-01-16T00:00:00Z", prices=TABLE)
    assert [g["key"] for g in result["groups"]] == [result["groups"][0]["key"]]
    assert result["total"]["messages"] == 1


def test_unknown_dimension_rejected(conn):
    with pytest.raises(ValueError):
        metrics.usage_breakdown(conn, "branch", prices=TABLE)


def test_session_outliers_by_tool_calls_and_context(conn, tmp_path):
    many = [
        _assistant(n, "2026-01-15T10:00:00.000Z", session="busy", tools=("Read",))
        for n in range(metrics.OUTLIER_TOOL_CALLS)
    ]
    few = [
        _assistant(1000 + n, "2026-01-15T10:00:00.000Z", session="calm", tools=("Read",))
        for n in range(metrics.OUTLIER_TOOL_CALLS - 1)
    ]
    long_ctx = [
        _assistant(
            2000, "2026-01-15T10:00:00.000Z", session="long", cr=metrics.OUTLIER_CONTEXT_TOKENS
        )
    ]
    _load(conn, tmp_path, many + few + long_ctx)
    result = metrics.session_outliers(conn)
    found = {o["session_id"]: o for o in result["sessions"]}
    assert set(found) == {"busy", "long"}
    assert found["busy"]["tool_calls"] == metrics.OUTLIER_TOOL_CALLS
    assert found["busy"]["project"] == "demo-app"
    assert found["busy"]["reasons"] == ["tool_calls"]
    assert found["long"]["max_context_tokens"] == metrics.OUTLIER_CONTEXT_TOKENS + 100  # + input
    assert found["long"]["reasons"] == ["context_tokens"]
    assert result["thresholds"] == {
        "tool_calls": metrics.OUTLIER_TOOL_CALLS,
        "context_tokens": metrics.OUTLIER_CONTEXT_TOKENS,
    }


def test_outlier_threshold_is_90():
    assert metrics.OUTLIER_TOOL_CALLS == 90
