"""Numbers for E3 from sanitized receiver records (HUN-168). Stdlib only, metadata only.

latency   : per tool_result event, receive time minus the transcript timestamp of the tool_use
            (the plan's reference), plus export lag = receive time minus the event's own timestamp.
tokens    : api_request token sums (and the claude_code.token.usage metric) vs the session's
            transcript `message.usage` sums (de-duplicated by message id; numbers only).
cost      : sum of api_request cost_usd vs the claude_code.cost.usage metric sum (and result cost).
attributes: per event name, the attribute names seen and how many records carried each.
"""
import json
import statistics
import sys
from datetime import datetime
from pathlib import Path

HARNESS = Path(__file__).resolve().parents[1] / "harness"
sys.path.insert(0, str(HARNESS))
from reference import read_reference, to_epoch_ms, transcript_files  # noqa: E402

TOKEN_FIELDS = (("input", "input_tokens"), ("output", "output_tokens"),
                ("cacheRead", "cache_read_tokens"), ("cacheCreation", "cache_creation_tokens"))
USAGE_FIELDS = (("input", "input_tokens"), ("output", "output_tokens"),
                ("cacheRead", "cache_read_input_tokens"), ("cacheCreation", "cache_creation_input_tokens"))


def percentile(values, pct: float):
    v = sorted(values)
    if not v:
        return None
    return v[min(len(v) - 1, round(pct / 100.0 * (len(v) - 1)))]


def summary(values) -> dict:
    v = [x for x in values if x is not None]
    return {"n": len(v), "median_ms": round(statistics.median(v), 1) if v else None,
            "p95_ms": round(percentile(v, 95), 1) if v else None,
            "min_ms": round(min(v), 1) if v else None, "max_ms": round(max(v), 1) if v else None}


def event_ms(ev: dict):
    """The event's own time in epoch ms: attribute event.timestamp, else the record time."""
    ts = ev["attrs"].get("event.timestamp")
    if isinstance(ts, str):
        try:
            return to_epoch_ms(ts.replace("Z", "+00:00"))
        except ValueError:
            pass
    return ev.get("record_ms")


def tool_result_latency(events, ref) -> dict:
    """Latency (ms) of each tool_result event against the transcript reference, matched by id."""
    rows, unmatched = [], 0
    for ev in events:
        if ev.get("event_name") not in ("tool_result", "claude_code.tool_result"):
            continue
        tid = ev["attrs"].get("tool_use_id")
        if tid not in ref:
            unmatched += 1
            continue
        name, ts = ref[tid]
        ref_ms = to_epoch_ms(ts)
        own = event_ms(ev)
        rows.append({
            "tool_use_id": tid, "tool_name": name,
            "latency_ms": ev["recv_ms"] - ref_ms,
            "export_lag_ms": (ev["recv_ms"] - own) if own else None,
            "duration_ms": ev["attrs"].get("duration_ms"),
        })
    return {"rows": rows, "unmatched": unmatched,
            "latency": summary(r["latency_ms"] for r in rows),
            "export_lag": summary(r["export_lag_ms"] for r in rows)}


def duplicate_tool_ids(events) -> int:
    seen: dict = {}
    for ev in events:
        if ev.get("event_name") in ("tool_result", "claude_code.tool_result"):
            tid = ev["attrs"].get("tool_use_id")
            seen[tid] = seen.get(tid, 0) + 1
    return sum(c - 1 for c in seen.values() if c > 1)


def transcript_usage(session_id: str, config_dir) -> dict:
    """Sum message.usage over main + subagent transcripts, one count per message id (numbers only)."""
    per_msg: dict = {}
    for path in transcript_files(session_id, config_dir):
        sub = "subagents" in path.parts
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                try:
                    rec = json.loads(line)
                except ValueError:
                    continue
                msg = rec.get("message") if isinstance(rec, dict) else None
                if not isinstance(msg, dict) or not isinstance(msg.get("usage"), dict):
                    continue
                key = msg.get("id") or rec.get("requestId") or rec.get("uuid")
                u = msg["usage"]
                cur = per_msg.setdefault(key, {"sub": sub, **{n: 0 for n, _ in USAGE_FIELDS}})
                for name, field in USAGE_FIELDS:
                    val = u.get(field)
                    if isinstance(val, int) and val > cur[name]:
                        cur[name] = val
                del rec, msg
    out = {"messages": len(per_msg), "total": {n: 0 for n, _ in USAGE_FIELDS},
           "main": {n: 0 for n, _ in USAGE_FIELDS}, "sub": {n: 0 for n, _ in USAGE_FIELDS}}
    for cur in per_msg.values():
        for name, _ in USAGE_FIELDS:
            out["total"][name] += cur[name]
            out["sub" if cur["sub"] else "main"][name] += cur[name]
    return out


def otel_api_totals(events) -> dict:
    """api_request sums by token type and cost, in total and by query_source."""
    tot = {n: 0 for n, _ in TOKEN_FIELDS}
    tot.update(cost_usd=0.0, requests=0)
    by_src: dict = {}
    seen_req: set = set()
    dup = 0
    for ev in events:
        if ev.get("event_name") not in ("api_request", "claude_code.api_request"):
            continue
        a = ev["attrs"]
        rid = a.get("request_id")
        if rid is not None:
            if rid in seen_req:
                dup += 1
                continue
            seen_req.add(rid)
        src = a.get("query_source") or "unknown"
        b = by_src.setdefault(src, {**{n: 0 for n, _ in TOKEN_FIELDS}, "cost_usd": 0.0, "requests": 0})
        for bucket in (tot, b):
            bucket["requests"] += 1
            bucket["cost_usd"] += a.get("cost_usd") or 0.0
            for name, field in TOKEN_FIELDS:
                bucket[name] += a.get(field) or 0
    tot["duplicate_request_ids"] = dup
    return {"total": tot, "by_query_source": by_src}


def metric_totals(metrics) -> dict:
    """Sum of data point values per metric (delta temporality: summed per export), and token type."""
    out: dict = {"cost_usd": 0.0, "tokens": {n: 0 for n, _ in TOKEN_FIELDS}, "by_metric": {},
                 "temporality": sorted({str(m.get("temporality")) for m in metrics})}
    for m in metrics:
        v = m.get("value") or 0
        out["by_metric"][m["metric"]] = out["by_metric"].get(m["metric"], 0) + v
        if m["metric"] == "claude_code.cost.usage":
            out["cost_usd"] += v
        elif m["metric"] == "claude_code.token.usage":
            t = m["attrs"].get("type")
            if t in out["tokens"]:
                out["tokens"][t] += v
    return out


def pct_diff(a, b):
    """100 * (a - b) / b, rounded; None if b is 0 or missing."""
    if a is None or not b:
        return None
    return round(100.0 * (a - b) / b, 3)


def compare_tokens(otel_tokens: dict, transcript_total: dict) -> dict:
    rows = {n: {"otel": otel_tokens[n], "transcript": transcript_total[n],
                "pct_diff": pct_diff(otel_tokens[n], transcript_total[n])} for n, _ in TOKEN_FIELDS}
    o, t = sum(otel_tokens[n] for n, _ in TOKEN_FIELDS), sum(transcript_total[n] for n, _ in TOKEN_FIELDS)
    rows["all_types"] = {"otel": o, "transcript": t, "pct_diff": pct_diff(o, t)}
    return rows


def attribute_inventory(rows, name_key: str) -> dict:
    """{record name: {"records": n, "attrs": {attr name: count}}} over events, metrics or spans."""
    inv: dict = {}
    for r in rows:
        nm = r.get(name_key) or "(unnamed)"
        slot = inv.setdefault(nm, {"records": 0, "attrs": {}})
        slot["records"] += 1
        for a in r.get("attr_names", []):
            slot["attrs"][a] = slot["attrs"].get(a, 0) + 1
    return inv


def analyze_run(snapshot: dict, session_id: str, config_dir, start_ms=None) -> dict:
    ref = read_reference(session_id, config_dir) if session_id else {}
    events = [e for e in snapshot["events"] if e["attrs"].get("session.id") in (session_id, None)]
    lat = tool_result_latency(events, ref)
    api = otel_api_totals(events)
    mt = metric_totals(snapshot["metrics"])
    tr = transcript_usage(session_id, config_dir) if session_id else None
    return {
        "events_total": len(snapshot["events"]), "events_for_session": len(events),
        "event_counts": _counts(e.get("event_name") for e in events),
        "latency": {k: lat[k] for k in ("latency", "export_lag", "unmatched")},
        "latency_rows": lat["rows"],
        "tool_result_count": len(lat["rows"]) + lat["unmatched"],
        "duplicate_tool_use_ids": duplicate_tool_ids(events),
        "reference_tool_use_count": len(ref),
        "otel_api": api, "metric_totals": mt, "transcript": tr,
        "token_vs_transcript_events": compare_tokens(api["total"], tr["total"]) if tr else None,
        "token_vs_transcript_metric": compare_tokens(mt["tokens"], tr["total"]) if tr else None,
        "cost": {"api_request_sum_usd": api["total"]["cost_usd"], "metric_sum_usd": mt["cost_usd"],
                 "pct_diff_metric_vs_events": pct_diff(mt["cost_usd"], api["total"]["cost_usd"])},
        "posts": _counts(p["path"] for p in snapshot["posts"]),
        "first_post_ms_after_start": (min(p["recv_ms"] for p in snapshot["posts"]) - start_ms) if snapshot["posts"] and start_ms else None,
        "subagent_session_ids": sorted({e["attrs"].get("session.id") for e in snapshot["events"] if e["attrs"].get("agent_id")} - {None}),
    }


def _counts(items) -> dict:
    out: dict = {}
    for i in items:
        out[str(i)] = out.get(str(i), 0) + 1
    return out


if __name__ == "__main__":
    print(datetime.now().isoformat(), "analyze.py is a library; see run_e3.py", file=sys.stderr)
