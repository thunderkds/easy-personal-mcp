"""HUN-170 report tests: join / summary maths on synthetic data, and the integrity check."""
import json
import sys
from pathlib import Path

import pytest

LT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LT / "report"))

import check_report as cr  # noqa: E402
import combined  # noqa: E402

T0 = "2026-10-31T10:00:00.000Z"
T0_MS = combined.to_epoch_ms(T0)

GOOD_REPORT = """# Claude Live Trace — Results

## Recommendation
text

## Comparison
| Source | Median latency | Completeness | Setup effort | Overhead | Verdict |
| -- | -- | -- | -- | -- | -- |
| E1 transcript tail | 1 ms | 45/45 | low | none | keep |
| E2 hooks | 1 ms | 45/45 | low | none | keep |
| E3 OTel | 1 ms | 45/45 | low | none | keep |
| E4 stream-json | 1 ms | 45/45 | low | none | keep |
| E5 usage APIs | n/a | n/a | n/a | n/a | docs only |

## Account usage findings
x
## Risks and open questions
x
## v2 ideas
x
"""
GOOD_RESULT = "".join(f"## {s}\ntext\n" for s in cr.RESULT_SECTIONS)


def make_rows():
    ref = {"t1": ("Read", T0), "t2": ("Bash", T0), "t3": ("Edit", T0)}
    e1 = [{"kind": "tool_use", "tool_use_id": t, "recv_ms": T0_MS + 200} for t in ("t1", "t2", "t3")]
    e1.append({"kind": "tool_use", "tool_use_id": "t1", "recv_ms": T0_MS + 999})  # dup: first wins
    e2 = [{"hook_event_name": "PreToolUse", "tool_use_id": t, "recv_ms": T0_MS + 6} for t in ("t1", "t2")]
    e2.append({"hook_event_name": "PostToolUseFailure", "tool_use_id": "t2", "recv_ms": T0_MS + 900})
    e3 = [{"event_name": "tool_result", "attrs": {"tool_use_id": "t1"}, "recv_ms": T0_MS + 1000},
          {"event_name": "claude_code.tool_result", "attrs": {"tool_use_id": "t2"}, "recv_ms": T0_MS + 3000},
          {"event_name": "tool_decision", "attrs": {"tool_use_id": "t3"}, "recv_ms": T0_MS + 500}]
    return combined.join_rows(ref, e1, e2, e3, {"t1": 2.0, "t2": 3.0, "t3": 2.5})


def test_report_join_by_tool_use_id_first_event_wins_and_missing_is_none():
    rows = {r["tool_use_id"]: r for r in make_rows()}
    assert rows["t1"]["e1_tail"] == 200.0
    assert rows["t1"]["e2_pre"] == 6.0 and rows["t3"]["e2_pre"] is None
    assert rows["t2"]["e2_post"] == 900.0
    assert rows["t1"]["e3_result"] == 1000.0 and rows["t2"]["e3_result"] == 3000.0
    assert rows["t3"]["e3_result"] is None and rows["t3"]["e3_decision"] == 500.0
    assert rows["t3"]["e4_stream"] == 2.5


def test_report_summary_shows_n_of_N_and_never_hides_a_miss():
    s = combined.summarize_rows(make_rows())
    assert s["total"] == 3
    assert s["sources"]["e2_pre"]["captured"] == 2 and s["sources"]["e2_pre"]["total"] == 3
    assert s["sources"]["e2_pre"]["capture_pct"] == 66.7
    assert s["sources"]["e1_tail"]["capture_pct"] == 100.0
    # a missed event is not live: 2 of 3 E3 results captured, 1 of those under 2 s
    assert s["sources"]["e3_result"]["live_under_2s"] == 1
    assert s["sources"]["e3_result"]["live_pct_of_all"] == 33.3
    # identical-events comparison: t1 and t2 were captured by all four headline sources, t3 was not
    assert s["common"]["n"] == 2
    assert s["common"]["e3_result"]["median_ms"] == 2000.0


def test_report_summary_empty_source_has_none_stats():
    s = combined.summarize_rows([{"tool_use_id": "a", "tool": "Read", "e1_tail": 5.0}])
    assert s["sources"]["e4_stream"]["captured"] == 0
    assert s["sources"]["e4_stream"]["median_ms"] is None
    assert s["common"]["n"] == 0


def test_report_aggregate_pools_rows_and_overhead(tmp_path):
    rows = make_rows()
    for i, wall in enumerate((27000.0, 29000.0, 31000.0), 1):
        rec = {"label": f"combined-{i}", "wall_ms": wall, "steps": {"a": True}, "rows": rows,
               "summary": combined.summarize_rows(rows), "cost_usd": {"e3_api_request_sum": 0.01,
                                                                      "e4_result_index0_total": 0.01}}
        (tmp_path / f"combined-{i}.json").write_text(json.dumps(rec))
    out = combined.aggregate(sorted(tmp_path.glob("combined-*.json")))
    assert out["runs"] == 3 and out["median_wall_ms"] == 29000.0
    assert out["overhead_vs_baseline_ms"] == 29000.0 - combined.BASELINE_MEDIAN_MS
    assert out["inside_noise"] is True
    assert out["pooled"]["total"] == 9
    assert out["pooled"]["sources"]["e2_pre"]["captured"] == 6


def test_report_result_index0_cost_not_last():
    cap = {"lines": [{"type": "result", "fields": {"result_index": 0, "total_cost_usd": 0.5}},
                     {"type": "result", "fields": {"result_index": 2, "total_cost_usd": 0.9}}]}
    assert combined.result_index0_cost(cap) == 0.5


def test_report_check_passes_on_good_files(tmp_path):
    (tmp_path / "REPORT.md").write_text(GOOD_REPORT)
    (tmp_path / "RESULT.md").write_text(GOOD_RESULT)
    (tmp_path / "e5.md").write_text("Docs only, not tested: no Admin key.")
    assert cr.check_report(tmp_path / "REPORT.md") == []
    assert cr.check_result(tmp_path / "RESULT.md") == []
    assert cr.check_e5(tmp_path / "e5.md") == []


@pytest.mark.parametrize("section", cr.RESULT_SECTIONS)
def test_report_check_fails_when_result_lacks_a_section(tmp_path, section):
    (tmp_path / "RESULT.md").write_text(GOOD_RESULT.replace(f"## {section}\n", ""))
    probs = cr.check_result(tmp_path / "RESULT.md")
    assert any(section in p for p in probs)


@pytest.mark.parametrize("source", cr.REPORT_SOURCES)
def test_report_check_fails_when_comparison_lacks_a_source_row(tmp_path, source):
    text = "\n".join(ln for ln in GOOD_REPORT.splitlines() if not ln.startswith(f"| {source} "))
    (tmp_path / "REPORT.md").write_text(text)
    probs = cr.check_report(tmp_path / "REPORT.md")
    assert any(f"row for {source}" in p for p in probs)


def test_report_check_fails_when_report_lacks_section_or_missing_file(tmp_path):
    (tmp_path / "REPORT.md").write_text(GOOD_REPORT.replace("## v2 ideas\n", ""))
    assert any("v2 ideas" in p for p in cr.check_report(tmp_path / "REPORT.md"))
    assert cr.check_report(tmp_path / "nope.md") and cr.check_result(tmp_path / "nope.md")


def test_report_check_fails_when_e5_lacks_docs_only_marker(tmp_path):
    (tmp_path / "e5.md").write_text("Endpoints and freshness.")
    assert cr.check_e5(tmp_path / "e5.md")


def test_report_secret_grep_catches_a_leak_but_skips_tests_folder(tmp_path):
    (tmp_path / "tests").mkdir()
    (tmp_path / "tests" / "t.py").write_text("X = '" + cr.PLANTED[0] + "'")
    assert cr.check_secrets(tmp_path) == []
    (tmp_path / "note.md").write_text("leak " + cr.PLANTED[0] + "-123")
    assert cr.check_secrets(tmp_path)


def test_report_real_files_pass_integrity_check():
    """The real RESULT.md files, REPORT.md and report/e5.md, plus a planted-secret grep of everything stored."""
    assert cr.check_all(LT) == []


def test_report_planted_secret_absent_from_every_stored_file():
    assert cr.check_secrets(LT) == []
