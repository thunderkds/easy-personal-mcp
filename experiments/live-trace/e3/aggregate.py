"""Print the E3 numbers (tables for RESULT.md) from e3/runs/*.json. Metadata only.

Usage: python aggregate.py [--runs DIR]
"""
import argparse
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from analyze import attribute_inventory, percentile, summary  # noqa: E402

BASELINE_MEDIAN_MS = 28264.0
BASELINE_SPREAD_MS = 5768.0


def load(runs: Path) -> dict:
    by: dict = {}
    for f in sorted(runs.glob("*.json")):
        r = json.loads(f.read_text())
        by.setdefault(r["config"], []).append(r)
    return by


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", default=str(HERE / "runs"))
    runs = load(Path(ap.parse_args().runs))
    for cfg, rs in runs.items():
        walls = [r["session"]["wall_ms"] for r in rs]
        print(f"\n## {cfg}: n={len(rs)} wall_ms={walls} median={statistics.median(walls):.0f} "
              f"overhead_vs_baseline={statistics.median(walls) - BASELINE_MEDIAN_MS:+.0f} ms (noise ~{BASELINE_SPREAD_MS:.0f})")
        print("steps all true:", [all(r["session"]["steps"].values()) for r in rs], "exit:", [r["session"]["exit_code"] for r in rs])
        if cfg == "down":
            continue
        lat_all, lag_all, by_tool = [], [], {}
        for r in rs:
            a = r["analysis"]
            lat_all += [x["latency_ms"] for x in a["latency_rows"]]
            lag_all += [x["export_lag_ms"] for x in a["latency_rows"]]
            for x in a["latency_rows"]:
                by_tool.setdefault(x["tool_name"], []).append(x["latency_ms"])
            print(f" run {r['session']['session_id'][:8]}: posts={a['posts']} first_post_ms={a['first_post_ms_after_start']:.0f} "
                  f"tool_result={a['tool_result_count']} (matched {a['latency']['latency']['n']}, unmatched {a['latency']['unmatched']}) "
                  f"dup_ids={a['duplicate_tool_use_ids']} med={a['latency']['latency']['median_ms']} p95={a['latency']['latency']['p95_ms']}")
            tv = a["token_vs_transcript_events"]["all_types"]
            tm = a["token_vs_transcript_metric"]["all_types"]
            print(f"   tokens events {tv['otel']} vs transcript {tv['transcript']} diff {tv['pct_diff']}%; "
                  f"metric {tm['otel']} diff {tm['pct_diff']}%; cost events {a['cost']['api_request_sum_usd']:.6f} "
                  f"metric {a['cost']['metric_sum_usd']:.6f} diff {a['cost']['pct_diff_metric_vs_events']}% result {r['session']['total_cost_usd']}")
        print(" pooled latency vs transcript tool_use:", summary(lat_all))
        print(" pooled export lag (receive - event.timestamp):", summary(lag_all))
        print(" per tool median/p95/n:", {k: (round(statistics.median(v)), round(percentile(v, 95)), len(v)) for k, v in sorted(by_tool.items())})
        metric_lags = []
        for r in rs:
            for m in r["receiver"]["metrics"]:
                if m.get("time_ms"):
                    metric_lags.append(m["recv_ms"] - m["time_ms"])
        print(" metric lag (receive - data point time):", summary(metric_lags))
        print(" span names:", sorted({s["span_name"] for r in rs for s in r["receiver"]["spans"] if s.get("span_name")}))

    inventory(runs)


def inventory(runs: dict) -> None:
    """Attribute names per event name / metric name / span name, pooled over the traced runs."""
    ev, mt, sp = [], [], []
    for cfg, rs in runs.items():
        for r in rs:
            ev += r["receiver"]["events"]
            mt += r["receiver"]["metrics"]
            sp += r["receiver"]["spans"]
    for title, rows, key in (("EVENTS", ev, "event_name"), ("METRICS", mt, "metric"), ("SPANS", sp, "span_name")):
        inv = attribute_inventory(rows, key)
        print(f"\n### {title} ({len(rows)} records)")
        for name, slot in sorted(inv.items()):
            n = slot["records"]
            always = sorted(a for a, c in slot["attrs"].items() if c == n)
            partial = sorted(f"{a}({c}/{n})" for a, c in slot["attrs"].items() if c != n)
            print(f"- {name} x{n}\n    always: {', '.join(always)}\n    sometimes: {', '.join(partial)}")
    res = sorted({a for e in ev for a in e.get("resource_attr_names", [])})
    print("\nresource attribute names:", res)
    tools = [e for e in ev if e.get("event_name") == "tool_result"]
    print("tool_result with tool_use_id:", sum(1 for e in tools if "tool_use_id" in e["attrs"]), "of", len(tools))


if __name__ == "__main__":
    main()
