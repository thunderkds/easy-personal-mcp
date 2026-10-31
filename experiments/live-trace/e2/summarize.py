"""Aggregate e2/runs/e2-async-*.analysis.json into the numbers for RESULT.md (metadata only)."""
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from run_e2 import BASELINE_MEDIAN_MS, BASELINE_SPREAD_MS, _pct, _stats, analyze, read_events  # noqa: E402


def main():
    from reference import read_reference
    runs = []
    for p in sorted((HERE / "runs").glob("e2-async-?.analysis.json")):
        old = json.loads(p.read_text())
        ev = read_events(HERE / "runs" / f"{old['label']}.events.jsonl")
        an = analyze(ev, read_reference(old["session_id"]), old["session_id"])
        an.update(label=old["label"], session_id=old["session_id"], wall_ms=old["wall_ms"], session_steps=old.get("session_steps", old.get("steps")))
        runs.append(an)
    print("runs:", len(runs))
    for r in runs:
        print(r["label"], r["session_id"][:8], "wall", r["wall_ms"], "pre%", r["all"]["pre_capture_pct"], "post%",
              r["all"]["post_capture_pct"], 
              r["events_by_name"])
    walls = [r["wall_ms"] for r in runs]
    med = statistics.median(walls)
    print("wall median", med, "min", min(walls), "max", max(walls), "overhead ms", round(med - BASELINE_MEDIAN_MS, 1),
          "overhead %", round(100 * (med - BASELINE_MEDIAN_MS) / BASELINE_MEDIAN_MS, 1), "noise", BASELINE_SPREAD_MS)
    tot = {k: sum(r["all"][k] for r in runs) for k in ("ref_ids", "pre_captured", "post_captured")}
    print("capture totals", tot, "pre%", round(100 * tot["pre_captured"] / tot["ref_ids"], 1), "post%", round(100 * tot["post_captured"] / tot["ref_ids"], 1))
    for key in ("pre_latency_values", "post_latency_values"):
        v = [x for r in runs for x in r[key]]
        print(key, _stats(v))
    v = [x for r in runs for x in r["post_latency_values"]]
    for key in ("post_latency_excl_subagent_tool", "post_minus_pre"):
        print(key, "medians per run", [r[key]["median_ms"] for r in runs], "max", [r[key]["max_ms"] for r in runs])


if __name__ == "__main__":
    main()
