"""`events.tool_error_count`: failed `tool_result` blocks of a user record (HUN-192, FR-059)."""

import pytest

from easy_personal.sources import claude_code
from tests_helpers import rec, write


def _parse(record):
    return claude_code.parse_record(record, lambda cwd: cwd)


def _user(blocks, n=1, session="s", **kwargs):
    record = rec(n, session, "user", **kwargs)
    record["message"] = {"role": "user", "content": blocks}
    return record


def _result(failed=None, **extra):
    block = {"type": "tool_result", "tool_use_id": "toolu_x", "content": "INVENTED_text"}
    if failed is not None:
        block["is_error"] = failed
    return {**block, **extra}


@pytest.mark.parametrize(
    "blocks, expected",
    [
        ([_result(True), _result(True), _result(False)], 2),  # 3 results, 2 failed
        ([_result(False), _result(), _result(False)], 0),  # results, none failed
        ([_result(True)], 1),
        ([{"type": "text", "text": "hello"}], None),  # text only
        ([], None),
        ([_result("true"), _result(1), _result(None)], 0),  # only the JSON boolean true counts
        ([_result(True), "not a dict", None, 5, {"type": "text"}], 1),  # odd blocks are skipped
        ([_result(True), {"type": "tool_use", "name": "Bash", "is_error": True}], 1),  # only tool_result blocks
    ],
)
def test_user_record_counts_failed_tool_results(blocks, expected):
    assert _parse(_user(blocks))["tool_error_count"] == expected


@pytest.mark.parametrize("content", ["a typed prompt", None, {"type": "tool_result", "is_error": True}, 7])
def test_content_that_is_not_a_list_is_null(content):
    assert _parse(_user(content))["tool_error_count"] is None


def test_user_record_without_a_message_is_null():
    record = rec(1, "s", "user")
    assert _parse(record)["tool_error_count"] is None
    record["message"] = "text"
    assert _parse(record)["tool_error_count"] is None


def test_assistant_and_other_records_are_null():
    assistant = rec(1, "s")
    assistant["message"]["content"] = [_result(True)]
    assert _parse(assistant)["tool_error_count"] is None
    system = rec(2, "s", "system")
    system["message"] = {"content": [_result(True)]}
    assert _parse(system)["tool_error_count"] is None


def test_sidechain_user_record_is_counted_and_stays_a_sidechain():
    event = _parse(_user([_result(True)], sidechain=True))
    assert event["tool_error_count"] == 1 and event["is_sidechain"] == 1


def test_ingest_stores_the_count_and_never_the_result_text(conn, tmp_path):
    root = write(tmp_path / "p", "a.jsonl", [_user([_result(True, content="INVENTED_secret")], 1), _user("typed", 2)])
    claude_code.ingest(conn, root)
    assert conn.execute(
        "SELECT tool_error_count FROM events WHERE record_type = 'user' ORDER BY byte_offset"
    ).fetchall() == [(1,), (None,)]
    assert "INVENTED" not in "\n".join(conn.iterdump())
