"""Saved investigate reports (HUN-221, FR-076, FR-078): validate, save + rule snapshot, list, before / now."""

import json
import sqlite3
from datetime import UTC, datetime, timedelta

import pytest

from easy_personal.core import reports, rules, store, workspaces
from easy_personal.sources import claude_code

pytestmark = pytest.mark.usefixtures("real_projects")  # these tests are not about Scratch (FR-069)

NOW = datetime(2026, 10, 21, 12, 0, tzinfo=UTC)
LATER = NOW + timedelta(days=7)  # the week of the data is now the *previous* week: tokens_jump reads clear
DAY = timedelta(days=1)
FIX = {"text": "Start a new session per task", "tool": "session_outliers", "number": "645301"}


@pytest.fixture(autouse=True)
def _home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))


def _msg(n, ago, i, session, cwd="/work/demo"):
    return {
        "type": "assistant",
        "version": "2.1.295",
        "cwd": cwd,
        "sessionId": session,
        "timestamp": (NOW - ago).strftime("%Y-%m-%dT%H:%M:%S.000Z"),
        "message": {
            "id": f"msg{n}",
            "model": "m-a",
            "content": [{"type": "text", "text": "x"}],
            "usage": {"input_tokens": i, "output_tokens": 0, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0},
        },
    }


@pytest.fixture
def loaded(conn, tmp_path):
    """15 days of history; the last week holds 2.5x the tokens of the week before: `tokens_jump` fires at NOW."""
    records = [_msg("anchor", 15 * DAY, 1, "old"), _msg(1, 10 * DAY, 4000, "prev")]
    records += [_msg(10 + k, DAY, 1000, f"now-{k}", cwd="/work/alpha") for k in range(10)]
    root = tmp_path / "projects"
    root.mkdir()
    (root / "s.jsonl").write_text("".join(json.dumps(r) + "\n" for r in records))
    claude_code.ingest(conn, root)
    return conn


def _rows(conn):
    return conn.execute("SELECT COUNT(*) FROM investigate_reports").fetchone()[0]


def _save(conn, **kw):
    kw.setdefault("summary", "Token use rose because of more sessions.\nSecond line.")
    kw.setdefault("fixes", [FIX, {"text": "Compact earlier"}])
    return reports.save(conn, kw.pop("summary"), kw.pop("fixes"), now=kw.pop("now", NOW), **kw)


# AC 2
def test_save_stores_one_row_with_the_rule_snapshot(loaded):
    saved = _save(loaded)
    assert saved == {"id": 1, "saved_utc": "2026-10-21T12:00:00Z"}
    assert _rows(loaded) == 1
    row = loaded.execute("SELECT * FROM investigate_reports").fetchone()
    assert row[1:6] == ("2026-10-21T12:00:00Z", row[2], None, None, "Token use rose because of more sessions.\nSecond line.")
    assert json.loads(row[6]) == [FIX, {"text": "Compact earlier"}]
    snapshot = json.loads(row[7])
    ids = {r.id for r in rules.load_rules()}
    seen = {r["rule_id"] for r in snapshot["checked"]} | {r["rule_id"] for r in snapshot["abstained"]}
    assert seen == ids  # every declared rule: fired, not fired, abstained
    assert snapshot["abstained"] and any(r["fired"] for r in snapshot["checked"]) and any(not r["fired"] for r in snapshot["checked"])
    for item in snapshot["checked"]:
        assert set(item) == {"rule_id", "title", "measured", "threshold", "fired"}
    jump = next(f for f in snapshot["fired"] if f["rule_id"] == "tokens_jump")
    assert jump["causes"] and jump["diagnosis"] and jump["summary"]
    assert [f["rule_id"] for f in snapshot["fired"]] == [r["rule_id"] for r in snapshot["checked"] if r["fired"]]


def test_snapshot_comes_from_the_server_not_from_the_text(loaded):
    _save(loaded, summary="Token use rose 99999% (tokens_jump).", fixes=[{"text": "x", "number": 12345}])
    snapshot = json.loads(loaded.execute("SELECT snapshot_json FROM investigate_reports").fetchone()[0])
    assert "99999" not in json.dumps(snapshot) and "12345" not in json.dumps(snapshot)
    assert next(r for r in snapshot["checked"] if r["rule_id"] == "tokens_jump")["measured"] == 1.5


def test_the_numbers_the_server_stores_equal_the_warnings_output(loaded):
    _save(loaded)
    snapshot = json.loads(loaded.execute("SELECT snapshot_json FROM investigate_reports").fetchone()[0])
    out = rules.evaluate(loaded, NOW)
    assert [f["rule_id"] for f in snapshot["fired"]] == [h["rule_id"] for h in out["hits"]]
    assert {f["rule_id"]: f["causes"] for f in snapshot["fired"]} == {h["rule_id"]: h["causes"] for h in out["hits"]}
    assert snapshot["abstained"] == out["abstained"]


# AC 4
def test_checked_is_opt_in_and_the_rest_of_the_output_is_unchanged(loaded):
    plain = rules.evaluate(loaded, NOW)
    assert "checked" not in plain and set(plain) == {"as_of_utc", "hits", "abstained"}
    with_checked = rules.evaluate(loaded, NOW, checked=True)
    assert {k: v for k, v in with_checked.items() if k != "checked"} == plain
    assert json.dumps(plain, sort_keys=True) == json.dumps({k: v for k, v in with_checked.items() if k != "checked"}, sort_keys=True)
    fired = {c["rule_id"] for c in with_checked["checked"] if c["fired"]}
    assert fired == {h["rule_id"] for h in plain["hits"]}
    assert [c["rule_id"] for c in with_checked["checked"]] == sorted(c["rule_id"] for c in with_checked["checked"])
    assert not {c["rule_id"] for c in with_checked["checked"]} & {a["rule_id"] for a in plain["abstained"]}


# AC 3
LONG_RUN = "a1" * 16  # 32 letters and digits


@pytest.mark.parametrize(
    ("kw", "field"),
    [
        ({"summary": "x " * 1000 + "x"}, "summary"),
        ({"summary": ""}, "summary"),
        ({"summary": 7}, "summary"),
        ({"fixes": [FIX] * 6}, "fixes"),
        ({"fixes": "no"}, "fixes"),
        ({"fixes": [{"text": "y " * 150 + "y"}]}, "fixes[0].text"),
        ({"fixes": [FIX, {"text": "z " * 150 + "z"}]}, "fixes[1].text"),
        ({"fixes": [{"tool": "t"}]}, "fixes[0]"),
        ({"fixes": [{"text": "ok", "extra": "no"}]}, "fixes[0]"),
        ({"fixes": [{"text": "ok", "tool": "t " * 50 + "t"}]}, "fixes[0].tool"),
        ({"fixes": [{"text": "ok", "number": True}]}, "fixes[0].number"),
        ({"fixes": [{"text": "ok", "number": float("nan")}]}, "fixes[0].number"),
        ({"workspace": "nope"}, "workspace"),
        ({"account": "nope"}, "account"),
        ({"summary": "key sk-ant-api03-abcdef used"}, "summary"),
        ({"summary": "key sk-LEAKCANARY-0123abcd used"}, "summary"),
        ({"summary": "token ghp_abcdefghijklmnop"}, "summary"),
        ({"summary": "token xoxb-1234-abcd"}, "summary"),
        ({"summary": "aws AKIAIOSFODNN7EXAMPLE"}, "summary"),
        ({"summary": "-----BEGIN RSA PRIVATE KEY-----"}, "summary"),
        ({"summary": f"run {LONG_RUN} end"}, "summary"),
        ({"fixes": [{"text": "see ghp_abcdefghijklmnop"}]}, "fixes[0].text"),
        ({"fixes": [{"text": "ok", "tool": "sk-abcdefgh"}]}, "fixes[0].tool"),
        ({"fixes": [{"text": "ok", "number": LONG_RUN}]}, "fixes[0].number"),
    ],
)
def test_refusals_name_the_field_and_store_nothing(loaded, kw, field):
    with pytest.raises(ValueError) as caught:
        _save(loaded, **kw)
    assert str(caught.value).startswith(field + ":"), str(caught.value)
    assert "nothing was saved" in str(caught.value)
    assert _rows(loaded) == 0


def test_a_refusal_never_echoes_the_text(loaded):
    for text in ("sk-LEAKCANARY-0123abcd", LONG_RUN):
        with pytest.raises(ValueError) as caught:
            _save(loaded, summary=text)
        assert text not in str(caught.value)
    with pytest.raises(ValueError) as caught:
        _save(loaded, workspace="sk-LEAKCANARY-0123abcd")
    assert "LEAKCANARY" not in str(caught.value) and "LEAKCANARY" not in repr(caught.value.__cause__)


def test_limits_are_inclusive(loaded):
    fix = {"text": "f " * 150, "tool": "t " * 50, "number": "9 " * 50}
    assert (len(fix["text"]), len(fix["tool"]), len(fix["number"])) == (300, 100, 100)
    _save(loaded, summary="s " * 1000, fixes=[fix] * 5)
    assert _rows(loaded) == 1


def test_a_uuid_session_id_and_an_absolute_path_are_accepted(loaded):
    text = "Session 6d1f3c2e-8a4b-4c7d-9e5f-0a1b2c3d4e5f in /home/owner/work/easy-personal-mcp/src/easy_personal/core/rules.py"
    _save(loaded, summary=text, fixes=[{"text": text[:290]}])
    assert loaded.execute("SELECT summary FROM investigate_reports").fetchone()[0] == text


def test_empty_scope_text_means_no_scope_and_two_saves_make_two_rows(loaded):
    _save(loaded, workspace="", account="")
    _save(loaded)
    assert _rows(loaded) == 2
    assert loaded.execute("SELECT DISTINCT workspace, account FROM investigate_reports").fetchall() == [(None, None)]


def test_scope_is_validated_and_stored(loaded):
    workspaces.add(loaded, "acme", repos=["/work"])
    _save(loaded, workspace="acme")
    assert loaded.execute("SELECT workspace, account FROM investigate_reports").fetchone() == ("acme", None)
    snapshot = json.loads(loaded.execute("SELECT snapshot_json FROM investigate_reports").fetchone()[0])
    assert next(r for r in snapshot["checked"] if r["rule_id"] == "tokens_jump")["measured"] == 1.5  # the same projects


# AC 5
def _rule(report, rule_id):
    return next(r for r in report["rules"] if r["rule_id"] == rule_id)


def test_before_now_for_a_rule_that_fired_then_and_is_clear_now(loaded):
    saved = _save(loaded)
    report = reports.get_report(loaded, saved["id"], now=LATER)
    jump = _rule(report, "tokens_jump")
    assert jump["before"]["state"] == "fired" and jump["before"]["measured"] == 1.5 and jump["before"]["fired"] is True
    assert jump["before"]["causes"] and jump["before"]["diagnosis"] and jump["before"]["summary"]
    assert jump["now"]["state"] == "clear" and jump["now"]["fired"] is False
    assert jump["now"]["measured"] == -1.0 and jump["now"]["measured"] != jump["before"]["measured"]
    assert jump["title"]
    assert (report["summary"], report["fixes"]) == ("Token use rose because of more sessions.\nSecond line.", [FIX, {"text": "Compact earlier"}])
    assert report["saved_utc"] == "2026-10-21T12:00:00Z" and report["workspace"] is None and report["server_version"]
    assert report["as_of_utc"] != report["now_as_of_utc"]
    assert [r["rule_id"] for r in report["rules"]] == sorted(r["rule_id"] for r in report["rules"])


def test_now_is_worked_out_on_each_call_and_never_stored(loaded):
    saved = _save(loaded)
    before = loaded.execute("SELECT * FROM investigate_reports").fetchone()
    first = reports.get_report(loaded, saved["id"], now=NOW)
    second = reports.get_report(loaded, saved["id"], now=LATER)
    assert _rule(first, "tokens_jump")["now"]["state"] == "fired"  # same moment as the save
    assert _rule(second, "tokens_jump")["now"]["state"] == "clear"
    assert loaded.execute("SELECT * FROM investigate_reports").fetchone() == before


def test_a_rule_abstaining_now_shows_its_reason(loaded):
    saved = _save(loaded)
    report = reports.get_report(loaded, saved["id"], now=LATER)
    gone = [r for r in report["rules"] if r["now"]["state"] == "abstained"]
    assert gone and all(r["now"]["reason"] for r in gone)
    still = [r for r in report["rules"] if r["before"]["state"] == "abstained"]
    assert still and all(r["before"]["reason"] and "measured" not in r["before"] for r in still)


def test_a_rule_missing_from_the_current_config_says_not_measured_now(loaded, monkeypatch):
    saved = _save(loaded)
    kept = [r for r in rules.load_rules() if r.id != "tokens_jump"]
    monkeypatch.setattr(rules, "load_rules", lambda *_a, **_k: kept)
    jump = _rule(reports.get_report(loaded, saved["id"], now=NOW), "tokens_jump")
    assert jump["now"] == {"state": "not_measured", "reason": "not measured now"}
    assert jump["before"]["state"] == "fired"


def test_a_changed_threshold_shows_each_sides_own_threshold(loaded, tmp_path):
    saved = _save(loaded)
    (tmp_path / "home").mkdir(exist_ok=True)
    (tmp_path / "home" / "config.yaml").write_text("thresholds:\n  tokens_jump: 3\n")
    jump = _rule(reports.get_report(loaded, saved["id"], now=NOW), "tokens_jump")
    assert jump["before"]["threshold"] == 0.5 and jump["now"]["threshold"] == 3
    assert jump["before"]["fired"] is True and jump["now"]["fired"] is False  # 1.5 is under the new threshold


def test_a_workspace_deleted_since_the_save_leaves_the_report_readable(loaded):
    ws = workspaces.add(loaded, "acme", repos=["/work"])
    saved = _save(loaded, workspace="acme")
    workspaces.delete(loaded, ws["id"])
    report = reports.get_report(loaded, saved["id"], now=NOW)
    assert report["workspace"] == "acme" and report["now_as_of_utc"] is None
    assert {r["now"]["reason"] for r in report["rules"]} == {"not measured now"}
    assert _rule(report, "tokens_jump")["before"]["measured"] == 1.5


def test_unknown_id_is_none(loaded):
    assert reports.get_report(loaded, 1) is None
    _save(loaded)
    assert reports.get_report(loaded, 2) is None


# AC 6
def test_list_is_newest_first_capped_at_50_with_the_first_line_and_fired_count(loaded):
    for n in range(52):
        _save(loaded, summary=f"Report {n}\nmore text", now=NOW + timedelta(minutes=n))
    listed = reports.list_reports(loaded)["reports"]
    assert len(listed) == 50 == reports.LIST_LIMIT
    assert [r["id"] for r in listed] == list(range(52, 2, -1))
    first = listed[0]
    assert first == {
        "id": 52,
        "saved_utc": "2026-10-21T12:51:00Z",
        "workspace": None,
        "account": None,
        "fired_count": len(rules.evaluate(loaded, NOW)["hits"]),
        "summary_first_line": "Report 51",
    }
    assert "text" not in json.dumps(listed)  # only the first line of the summary


def test_empty_list(loaded):
    assert reports.list_reports(loaded) == {"reports": []}


# AC 1 companion: the table is plain and the snapshot JSON is stable
def test_json_is_stored_with_sorted_keys(loaded):
    _save(loaded)
    fixes_json, snapshot_json = loaded.execute("SELECT fixes_json, snapshot_json FROM investigate_reports").fetchone()
    assert fixes_json == json.dumps(json.loads(fixes_json), sort_keys=True)
    assert snapshot_json == json.dumps(json.loads(snapshot_json), sort_keys=True)


def test_a_failed_snapshot_stores_no_row(loaded, monkeypatch):
    def boom(*_a, **_k):
        raise sqlite3.OperationalError("boom")

    monkeypatch.setattr(rules, "evaluate", boom)
    with pytest.raises(sqlite3.OperationalError):
        _save(loaded)
    assert _rows(loaded) == 0
    assert store.machine_id(loaded)  # the connection is still usable


def test_empty_optional_fix_fields_count_as_absent():
    """Stage 4 review: a model often sends "" for an optional argument; that must not refuse the whole save."""
    _, fixes = reports.validate("Summary.", [{"text": "Start a new session per task", "tool": "", "number": ""}])
    assert fixes == [{"text": "Start a new session per task"}]
