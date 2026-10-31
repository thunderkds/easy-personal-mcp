"""Combined run (HUN-170): E1 tail + E2 http hooks + E3 OTel + E4 stream-json, all on, one session.

Usage:
  python combined.py run --label combined-1        # one standard session, all four sources on
  python combined.py aggregate                      # pool report/runs/combined-*.json -> summary

One `run` starts the E2 collector (127.0.0.1:4390), the E3 OTLP receiver (127.0.0.1:4318) and the E1
tail (100 ms poll) in this process, runs ONE standard session through the E4 capture (which holds the
shared session lock for the whole run), then joins the four sources to the transcript reference BY
tool_use_id. Latency per source = its local receive time minus the transcript `timestamp` of the record
holding the tool_use (same machine clock). Raw source files go to a mktemp folder OUTSIDE the repo; only
the reduced per-id rows (ids, tool names, ms) are written to report/runs/<label>.json.
Metadata only (PRD FR-002 / NFR-002): no prompt text, assistant text, tool input or output is read or kept.
Hooks and OTel env reach only the child process (`--settings FILE`, child env); no ~/.claude* file changes.
"""
import argparse
import json
import statistics
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
LT = HERE.parent
for _sub in ("harness", "e1", "e2", "e3", "e4"):
    sys.path.insert(0, str(LT / _sub))

import analyze as e3_analyze  # noqa: E402
import capture  # noqa: E402
import collector  # noqa: E402
import receiver  # noqa: E402
import run_e2  # noqa: E402
import run_e3  # noqa: E402
import tail as e1_tail  # noqa: E402
from reference import read_reference, to_epoch_ms  # noqa: E402

RUNS = HERE / "runs"
BASELINE_MEDIAN_MS = 28264
BASELINE_NOISE_MS = 5768
LIVE_MS = 2000  # the "live" bar of the report: under 2 s behind the transcript timestamp
# source key -> label. The four headline sources use the first latency of each; e2_post and
# e3_decision are extra rows (Post includes the tool's run time; decision is E3's earlier event).
SOURCES = {
    "e1_tail": "E1 transcript tail (tool_use line)",
    "e2_pre": "E2 http hook PreToolUse",
    "e2_post": "E2 http hook PostToolUse/Failure (includes tool run time)",
    "e3_result": "E3 OTel tool_result event (logs 1 s)",
    "e3_decision": "E3 OTel tool_decision event (logs 1 s)",
    "e4_stream": "E4 stream-json tool_use line",
}
HEADLINE = ("e1_tail", "e2_pre", "e3_result", "e4_stream")


def join_rows(ref, e1_events, e2_events, e3_events, e4_latency):
    """One row per reference tool_use id: tool name and, per source, latency ms or None (missed).

    ref: {tool_use_id: (tool_name, iso_ts)}. Joined by tool_use_id only. e1/e2/e3 events are the
    sanitized dicts of those sources (recv_ms epoch ms); e4_latency: {tool_use_id: ms} from
    capture.evaluate. First event per id wins for each source.
    """
    first = {k: {} for k in ("e1_tail", "e2_pre", "e2_post", "e3_result", "e3_decision")}
    for e in e1_events:
        if e.get("kind") == "tool_use":
            first["e1_tail"].setdefault(e["tool_use_id"], e["recv_ms"])
    for e in e2_events:
        key = {"PreToolUse": "e2_pre", "PostToolUse": "e2_post", "PostToolUseFailure": "e2_post"}.get(
            e.get("hook_event_name"))
        if key and e.get("tool_use_id"):
            first[key].setdefault(e["tool_use_id"], e["recv_ms"])
    for e in e3_events:
        name = (e.get("event_name") or "").replace("claude_code.", "")
        tid = (e.get("attrs") or {}).get("tool_use_id")
        key = {"tool_result": "e3_result", "tool_decision": "e3_decision"}.get(name)
        if key and tid:
            first[key].setdefault(tid, e["recv_ms"])
    rows = []
    for tid, (name, ts) in ref.items():
        ref_ms = to_epoch_ms(ts)
        row = {"tool_use_id": tid, "tool": name}
        for key, seen in first.items():
            row[key] = round(seen[tid] - ref_ms, 1) if tid in seen else None
        row["e4_stream"] = round(e4_latency[tid], 1) if tid in e4_latency else None
        rows.append(row)
    return rows


def _pstats(values):
    s = capture.summarize_latency(values)
    return {"median_ms": s["median"], "p95_ms": s["p95"], "max_ms": s["max"]}


def summarize_rows(rows):
    """Per source: captured n of N, capture %, median / p95 / max ms, share under LIVE_MS of ALL N.

    `common`: the same statistics for the 4 headline sources over only the ids ALL four captured
    (the identical-events comparison). A missed event counts against capture and live share; it is
    never dropped from N.
    """
    total = len(rows)
    out = {"total": total, "sources": {}, "common": {}}
    for key in SOURCES:
        vals = [r[key] for r in rows if r.get(key) is not None]
        out["sources"][key] = {
            "captured": len(vals), "total": total,
            "capture_pct": round(100.0 * len(vals) / total, 1) if total else None,
            "live_under_2s": sum(v < LIVE_MS for v in vals),
            "live_pct_of_all": round(100.0 * sum(v < LIVE_MS for v in vals) / total, 1) if total else None,
            **(_pstats(vals) if vals else {"median_ms": None, "p95_ms": None, "max_ms": None}),
        }
    common = [r for r in rows if all(r.get(k) is not None for k in HEADLINE)]
    out["common"]["n"] = len(common)
    for key in SOURCES:
        vals = [r[key] for r in common if r.get(key) is not None]
        out["common"][key] = _pstats(vals) if vals else {"median_ms": None, "p95_ms": None, "max_ms": None}
    return out


def result_index0_cost(cap):
    """total_cost_usd of the first result line (result_index 0), never the last (HUN-169)."""
    results = [ln.get("fields", {}) for ln in cap["lines"] if ln.get("type") == "result"]
    for f in results:
        if f.get("result_index") == 0:
            return f.get("total_cost_usd")
    return results[0].get("total_cost_usd") if results else None


def run_one(label, config_dir=run_e3.DEFAULT_CONFIG_DIR, runs_dir=RUNS, lock_path=capture.DEFAULT_LOCK,
            tmp=None, claude_bin="claude", e1_poll_ms=100.0, e2_port=4390, e3_port=4318):
    """Start the three local sources, run one session with E4 capture, join all, write the record."""
    tmp = Path(tmp) if tmp else Path(tempfile.mkdtemp(prefix="live-trace-combined-"))
    settings = run_e2.write_settings(tmp / "hooks.settings.json", async_=True)
    env = {**run_e3.BASE_ENV, **run_e3.CONFIGS["lowered"],
           "OTEL_EXPORTER_OTLP_ENDPOINT": f"http://127.0.0.1:{e3_port}"}
    orig_build = capture.build_cmd
    # capture_session has no settings argument; build_cmd does: add --settings for this one call.
    capture.build_cmd = lambda *a, **k: orig_build(*a, **{**k, "settings_file": settings})
    try:
        with collector.Collector(tmp / "e2.events.jsonl", port=e2_port), \
                receiver.OtlpReceiver(port=e3_port) as rx:
            handle = e1_tail.start_tail(Path(config_dir) / "projects", tmp / "e1.events.jsonl",
                                        poll_ms=e1_poll_ms)
            try:
                cap = capture.capture_session(label=label, extra_env=env, config_dir=config_dir,
                                              runs_dir=tmp / "e4", lock_path=lock_path,
                                              claude_bin=claude_bin)
                time.sleep(1.5)  # late hook posts
                rx.wait_quiet(quiet_s=3, max_s=10)
            finally:
                tail_stats = handle.stop()
            snap = rx.snapshot()
            rx.dump(tmp / "e3.snapshot.json")
    finally:
        capture.build_cmd = orig_build
    sid = cap["session_id"]
    ref = read_reference(sid, config_dir) if sid else {}
    e4 = capture.evaluate(cap, config_dir, reference=ref)
    rows = join_rows(ref, e1_tail.read_events(tmp / "e1.events.jsonl"),
                     run_e2.read_events(tmp / "e2.events.jsonl"), snap["events"], e4["latency_ms"])
    e3n = e3_analyze.analyze_run(snap, sid, config_dir, cap["start_epoch_ms"]) if sid else {}
    rec = {
        "label": label, "session_id": sid, "wall_ms": cap["wall_ms"],
        "overhead_vs_baseline_ms": round(cap["wall_ms"] - BASELINE_MEDIAN_MS, 1),
        "exit_code": cap["exit_code"], "steps": cap["steps"], "result_lines": cap["result_lines"],
        "e4_capture_pct": e4["capture_pct"], "reference_ids": len(ref),
        "rows": rows, "summary": summarize_rows(rows),
        "e1_tail_stats": {k: v for k, v in tail_stats.items() if isinstance(v, (int, float))},
        "e3": {k: e3n.get(k) for k in ("events_for_session", "token_vs_transcript_events",
                                      "token_vs_transcript_metric", "cost", "subagent_session_ids")},
        "cost_usd": {"e3_api_request_sum": (e3n.get("cost") or {}).get("api_request_sum_usd"),
                     "e4_result_index0_total": result_index0_cost(cap)},
        "raw_dir_outside_repo": str(tmp),
    }
    Path(runs_dir).mkdir(parents=True, exist_ok=True)
    path = Path(runs_dir) / f"{label}.json"
    path.write_text(json.dumps(rec, indent=1) + "\n")
    rec["run_file"] = str(path)
    return rec


def aggregate(run_files):
    """Pool the rows of several run records; add per-run capture and the overhead numbers."""
    recs = [json.loads(Path(p).read_text()) for p in sorted(run_files)]
    rows = [r for rec in recs for r in rec["rows"]]
    walls = sorted(rec["wall_ms"] for rec in recs)
    med = statistics.median(walls) if walls else None
    return {
        "runs": len(recs),
        "wall_ms": walls, "median_wall_ms": med,
        "overhead_vs_baseline_ms": round(med - BASELINE_MEDIAN_MS, 1) if med is not None else None,
        "inside_noise": abs(med - BASELINE_MEDIAN_MS) < BASELINE_NOISE_MS if med is not None else None,
        "all_steps_done": all(all(rec["steps"].values()) for rec in recs),
        "per_run_capture": [{"label": rec["label"], **{k: v["captured"] for k, v in rec["summary"]["sources"].items()},
                             "total": rec["summary"]["total"]} for rec in recs],
        "pooled": summarize_rows(rows),
        "by_tool": _by_tool(rows),
        "cost_usd": [rec["cost_usd"] for rec in recs],
    }


def _by_tool(rows):
    out = {}
    for tool in sorted({r["tool"] for r in rows}):
        sub = [r for r in rows if r["tool"] == tool]
        out[tool] = {"n": len(sub), **{k: summarize_rows(sub)["sources"][k]["median_ms"] for k in HEADLINE}}
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cmd", choices=("run", "aggregate"))
    ap.add_argument("--label", default="combined-1")
    ap.add_argument("--runs-dir", default=str(RUNS))
    args = ap.parse_args()
    if args.cmd == "run":
        rec = run_one(args.label, runs_dir=args.runs_dir)
        print(json.dumps({"label": rec["label"], "wall_ms": rec["wall_ms"], "steps": rec["steps"],
                          "summary": rec["summary"], "cost_usd": rec["cost_usd"],
                          "run_file": rec["run_file"]}, indent=1))
    else:
        files = sorted(Path(args.runs_dir).glob("combined-*.json"))
        out = aggregate(files)
        (Path(args.runs_dir) / "summary.json").write_text(json.dumps(out, indent=1) + "\n")
        print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
