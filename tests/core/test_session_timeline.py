import json

from easy_personal.core import metrics

A = "2026-10-20T08:{:02d}:00.000000Z"


def _row(
    conn,
    n,
    ts,
    session="A",
    message=None,
    model="m-a",
    project="/work/demo-app",
    abstained=0,
    record_type="assistant",
    i=10,
    o=5,
    cc=1,
    cr=2,
):
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, "
        "record_type, ts_utc, session_id, project_path, model, message_id, input_tokens, "
        "output_tokens, cache_creation_input_tokens, cache_read_input_tokens) "
        "VALUES ('m', 'claude_code', 'f', 1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            n,
            abstained,
            record_type,
            ts,
            session,
            project,
            model,
            message,
            i if message else None,
            o if message else None,
            cc if message else None,
            cr if message else None,
        ),
    )
    conn.commit()


def _seed(conn):
    _row(conn, 1, "2026-10-20T08:00:00.000000Z", message="m1")
    _row(conn, 2, "2026-10-20T08:00:01.000000Z", message="m1")  # same message, 2nd row
    _row(conn, 3, "2026-10-20T08:10:00.000000Z", message="m2")
    _row(conn, 4, "2026-10-20T09:00:00.000000Z", message="m3")
    _row(
        conn,
        5,
        "2026-10-20T07:00:00.000000Z",
        session="B",
        message="b1",
        model="m-b",
        project="/other/shop",
    )


def test_sessions_points_gaps_and_order(conn):
    _seed(conn)
    result = metrics.session_timeline(conn)
    assert result["gap_minutes"] == 30
    assert result["period"] == {"since_utc": None, "until_utc": None}
    a, b = result["sessions"]
    assert (a["session_id"], b["session_id"]) == ("A", "B")  # start descending
    assert a["project"] == "demo-app" and b["project"] == "shop"
    assert a["start_utc"] == "2026-10-20T08:00:00.000000Z"
    assert a["end_utc"] == "2026-10-20T09:00:00.000000Z"
    assert a["models"] == ["m-a"] and b["models"] == ["m-b"]
    assert a["messages"] == 3
    assert [p["ts_utc"] for p in a["points"]] == [
        "2026-10-20T08:00:00.000000Z",
        "2026-10-20T08:10:00.000000Z",
        "2026-10-20T09:00:00.000000Z",
    ]
    assert a["points"][0] == {
        "ts_utc": "2026-10-20T08:00:00.000000Z",
        "model": "m-a",
        "input_tokens": 10,
        "output_tokens": 5,
        "cache_creation_input_tokens": 1,
        "cache_read_input_tokens": 2,
    }
    assert a["input_tokens"] == 30 and a["output_tokens"] == 15
    assert a["cache_creation_input_tokens"] == 3 and a["cache_read_input_tokens"] == 6
    assert a["gaps"] == [
        {
            "start_utc": "2026-10-20T08:10:00.000000Z",
            "end_utc": "2026-10-20T09:00:00.000000Z",
            "minutes": 50.0,
        }
    ]
    assert b["gaps"] == []


def test_gap_threshold(conn):
    _row(conn, 1, "2026-10-20T08:00:00.000000Z", message="m1")
    _row(conn, 2, "2026-10-20T08:29:54.000000Z", message="m2")  # 29.9 min
    assert metrics.session_timeline(conn)["sessions"][0]["gaps"] == []
    _row(conn, 3, "2026-10-20T08:59:54.000000Z", message="m3")  # 30.0 min after m2
    gaps = metrics.session_timeline(conn)["sessions"][0]["gaps"]
    assert [g["minutes"] for g in gaps] == [30.0]


def test_period_since_inclusive_until_exclusive(conn):
    _seed(conn)
    result = metrics.session_timeline(
        conn, "2026-10-20T08:10:00+00:00", "2026-10-20T09:00:00+00:00"
    )
    (a,) = result["sessions"]
    assert a["start_utc"] == a["end_utc"] == "2026-10-20T08:10:00.000000Z"
    assert a["messages"] == 1 and a["gaps"] == []


def test_tool_only_null_model_abstained_and_separate_sessions(conn):
    _row(conn, 1, "2026-10-20T08:00:00.000000Z", record_type="user")  # no message
    _row(conn, 2, "2026-10-20T08:05:00.000000Z", session="X", message="x1", model=None)
    _row(conn, 3, "2026-10-20T08:06:00.000000Z", session="X", abstained=1, message="x2")
    _row(
        conn, 4, "2026-10-20T08:07:00.000000Z", session="Y", message="y1", project="/other/demo-app"
    )
    by_id = {s["session_id"]: s for s in metrics.session_timeline(conn)["sessions"]}
    assert set(by_id) == {"A", "X", "Y"}
    assert by_id["A"]["messages"] == 0 and by_id["A"]["points"] == []
    assert by_id["A"]["start_utc"] == by_id["A"]["end_utc"] == "2026-10-20T08:00:00.000000Z"
    assert by_id["X"]["models"] == [] and by_id["X"]["points"][0]["model"] is None
    assert by_id["X"]["end_utc"] == "2026-10-20T08:05:00.000000Z"  # abstained ignored
    assert by_id["Y"]["project"] == "demo-app"


def test_empty_store_and_determinism(conn):
    assert metrics.session_timeline(conn) == {
        "period": {"since_utc": None, "until_utc": None},
        "gap_minutes": 30,
        "sessions": [],
    }
    _seed(conn)
    assert json.dumps(metrics.session_timeline(conn)) == json.dumps(metrics.session_timeline(conn))
