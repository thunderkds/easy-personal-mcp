"""E4: stream-json capture from a headless `claude -p` run (HUN-169).

`capture_session()` runs the standard session (or a resume / partial-messages variant), reads the
stdout stream line by line, stamps each line with its local receive time, and reduces it at once to
metadata: line type/subtype, tool_use ids and names, usage / cost FIELD NAMES and numbers, stream
event type names. Prompt text, assistant text, tool input and tool output are dropped at parse
time (PRD FR-002 / NFR-002): only numbers, booleans, ids, names and a small allow-list of enum
strings survive.

`evaluate()` compares a capture with the transcript reference (harness/reference.py): capture % of
reference tool_use ids and per-id latency = stream receive time minus transcript timestamp.

Interface for the combined run (HUN-170):
    from capture import capture_session, evaluate, summarize_latency, DEFAULT_LOCK
    cap = capture_session(label="e4", scratch=None, partial_messages=False, resume=None,
                          lock_path=DEFAULT_LOCK, runs_dir=...)   # holds the lock for the run
    ev  = evaluate(cap, config_dir)       # {"capture_pct", "latency_ms": [...], ...}

CLI: python capture.py --label NAME [--partial] [--resume SESSION --scratch DIR] [--lock FILE]
"""
import argparse
import fcntl
import json
import os
import subprocess
import sys
import threading
import time
from datetime import UTC, datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "harness"))
from make_scratch import make_scratch  # noqa: E402
from reference import read_reference, to_epoch_ms  # noqa: E402
from run_session import (  # noqa: E402
    ALLOWED_TOOLS,
    DEFAULT_CONFIG_DIR,
    FORBIDDEN_ENV,
    STANDARD_PROMPT,
    parse_stream,
    steps_done,
    summarize,
)

RUNS_DIR = HERE / "runs"
DEFAULT_LOCK = os.environ.get("LIVE_TRACE_LOCK") or ("/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/"
                "90deaa04-a1cb-4209-90a2-0a2669cb6205/scratchpad/live-trace-session.lock")
RESUME_PROMPT = "Reply with the single word: ok."

# String values that are safe to keep (short enums and ids). Every other string is dropped and
# only its type name is kept. Keys are matched by exact name at any depth.
SAFE_STRING_KEYS = {
    "type", "subtype", "session_id", "uuid", "stop_reason", "terminal_reason", "kind", "subkind",
    "model", "id", "name", "role", "parent_tool_use_id", "tool_use_id", "service_tier",
    "speed", "fast_mode_state", "resume_reason", "api_error_status", "permissionMode",
    "output_style", "apiKeySource", "claude_code_version", "status",
}
MAX_DEPTH = 4


def reduce_value(value, key=None, depth=0):
    """Metadata-only copy of a JSON value: numbers, bools, null, allow-listed strings, shapes.

    dict -> dict of reduced values; list -> {"len": n} (numbers-only lists keep nothing else);
    str -> the string only when `key` is in SAFE_STRING_KEYS, else "<str>".
    """
    if value is None or isinstance(value, (bool, int, float)):
        return value
    if isinstance(value, str):
        return value if key in SAFE_STRING_KEYS and len(value) <= 80 else "<str>"
    if isinstance(value, list):
        return {"len": len(value)}
    if isinstance(value, dict):
        if depth >= MAX_DEPTH:
            return {"keys": len(value)}
        return {k: reduce_value(v, k, depth + 1) for k, v in value.items() if isinstance(k, str)}
    return "<other>"


def _num_fields(d):
    """Only the numeric (non-bool) fields of a dict, nested dicts reduced the same way."""
    out = {}
    if isinstance(d, dict):
        for k, v in d.items():
            if isinstance(v, bool):
                continue
            if isinstance(v, (int, float)):
                out[k] = v
            elif isinstance(v, dict):
                sub = _num_fields(v)
                if sub:
                    out[k] = sub
    return out


def parse_line(raw: str, recv_ms: float, seq: int) -> dict:
    """One stdout line -> one metadata record. Never raises; a bad line is recorded as such."""
    rec = {"seq": seq, "recv_ms": round(recv_ms, 1), "bytes": len(raw.encode("utf-8", "replace"))}
    try:
        ev = json.loads(raw)
    except ValueError:
        rec["type"] = "unparsed"
        return rec
    if not isinstance(ev, dict):
        rec["type"] = "unparsed"
        return rec
    etype = ev.get("type") if isinstance(ev.get("type"), str) else None
    rec["type"] = etype
    if isinstance(ev.get("subtype"), str):
        rec["subtype"] = ev["subtype"][:60]
    rec["keys"] = sorted(k for k in ev if isinstance(k, str))
    if isinstance(ev.get("session_id"), str):
        rec["session_id"] = ev["session_id"]
    parent = ev.get("parent_tool_use_id")
    rec["parent_tool_use_id"] = parent if isinstance(parent, str) else None
    if etype in ("assistant", "user"):
        msg = ev.get("message")
        if isinstance(msg, dict):
            if isinstance(msg.get("id"), str):
                rec["message_id"] = msg["id"]
            if isinstance(msg.get("model"), str):
                rec["model"] = msg["model"][:60]
            usage = _num_fields(msg.get("usage"))
            if usage:
                rec["usage"] = usage
            content = msg.get("content")
            if isinstance(content, list):
                rec["block_types"] = [b.get("type") for b in content
                                      if isinstance(b, dict) and isinstance(b.get("type"), str)]
                uses, results = [], []
                for b in content:
                    if not isinstance(b, dict):
                        continue
                    if b.get("type") == "tool_use" and isinstance(b.get("id"), str):
                        uses.append({"id": b["id"],
                                     "name": b["name"] if isinstance(b.get("name"), str) else None})
                    elif b.get("type") == "tool_result" and isinstance(b.get("tool_use_id"), str):
                        results.append({"tool_use_id": b["tool_use_id"],
                                        "is_error": b.get("is_error") is True})
                if uses:
                    rec["tool_uses"] = uses
                if results:
                    rec["tool_results"] = results
    elif etype == "stream_event":
        inner = ev.get("event")
        if isinstance(inner, dict):
            if isinstance(inner.get("type"), str):
                rec["event_type"] = inner["type"]
            delta = inner.get("delta")
            if isinstance(delta, dict) and isinstance(delta.get("type"), str):
                rec["delta_type"] = delta["type"]
            if isinstance(inner.get("index"), int):
                rec["index"] = inner["index"]
            usage = _num_fields(inner.get("usage"))
            if usage:
                rec["usage"] = usage
    elif etype == "result":
        # Whole line reduced: field names, numbers, enums. The `result` text becomes "<str>".
        rec["fields"] = {k: reduce_value(v, k) for k, v in ev.items() if isinstance(k, str)}
    elif etype == "system":
        rec["fields"] = {k: reduce_value(v, k) for k, v in ev.items()
                         if isinstance(k, str) and k in ("subtype", "session_id", "model",
                                                         "permissionMode", "claude_code_version",
                                                         "output_style", "apiKeySource")}
    return rec


def build_cmd(claude_bin, prompt, model, partial_messages, resume, settings_file=None):
    cmd = [claude_bin, "-p", prompt, "--model", model, "--output-format", "stream-json",
           "--verbose", "--allowed-tools", *ALLOWED_TOOLS]
    if partial_messages:
        cmd.append("--include-partial-messages")
    if resume:
        cmd += ["--resume", resume]
    if settings_file:
        cmd += ["--settings", str(settings_file)]
    return cmd


def capture_session(label="e4", scratch=None, extra_env=None, partial_messages=False, resume=None,
                    prompt=None, model="haiku", config_dir=DEFAULT_CONFIG_DIR, runs_dir=RUNS_DIR,
                    timeout=300, claude_bin="claude", lock_path=DEFAULT_LOCK, write=True) -> dict:
    """Run one headless session under an exclusive lock and write the metadata-only capture.

    resume: session id to continue (the same scratch folder must be passed). partial_messages adds
    --include-partial-messages. Returns the capture record (also written to runs_dir/<label>-*.json
    when `write`). Keys: label, session_id, start/end epoch ms, wall_ms, lock_wait_ms, exit_code,
    timed_out, lines (per-line records), line_count, result_lines, has_result, last_line_partial,
    steps, tool_use_by_name, subagent_tool_use_by_name, config{partial_messages, resume}.
    """
    extra_env = dict(extra_env or {})
    bad = [k for k in extra_env if k in FORBIDDEN_ENV]
    if bad:
        raise ValueError(f"forbidden content-logging env var(s): {bad}")
    if prompt is None:
        prompt = RESUME_PROMPT if resume else STANDARD_PROMPT
    scratch = Path(scratch) if scratch else make_scratch()
    env = {**os.environ, "CLAUDE_CONFIG_DIR": str(config_dir), **extra_env}
    for k in FORBIDDEN_ENV:
        env.pop(k, None)
    cmd = build_cmd(claude_bin, prompt, model, partial_messages, resume)

    lock_fd = None
    lock_t0 = time.monotonic()
    if lock_path:
        Path(lock_path).parent.mkdir(parents=True, exist_ok=True)
        lock_fd = os.open(str(lock_path), os.O_CREAT | os.O_RDWR, 0o600)
        fcntl.flock(lock_fd, fcntl.LOCK_EX)  # blocks until other experiments finish
    lock_wait_ms = (time.monotonic() - lock_t0) * 1000.0
    try:
        start_ms = time.time() * 1000.0
        t0 = time.monotonic()
        proc = subprocess.Popen(cmd, cwd=scratch, env=env, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
                                start_new_session=True)  # own process group: kill reaches children
        timed_out = []
        timer = threading.Timer(timeout, lambda: (timed_out.append(1), _kill_group(proc)))
        timer.start()
        raw_lines, lines = [], []
        try:
            for raw in iter(proc.stdout.readline, ""):
                recv = time.time() * 1000.0
                raw_lines.append(raw)
                lines.append(parse_line(raw.rstrip("\n"), recv, len(lines)))
            exit_code = proc.wait()
        finally:
            timer.cancel()
        wall_ms = (time.monotonic() - t0) * 1000.0
        end_ms = time.time() * 1000.0
    finally:
        if lock_fd is not None:
            fcntl.flock(lock_fd, fcntl.LOCK_UN)
            os.close(lock_fd)

    last_partial = bool(raw_lines) and (not raw_lines[-1].endswith("\n")
                                        or lines[-1].get("type") == "unparsed")
    meta = parse_stream(raw_lines)  # harness parser: ids, names, counts; keeps no text
    del raw_lines
    results = [ln for ln in lines if ln.get("type") == "result"]

    def iso(ms):
        return datetime.fromtimestamp(ms / 1000.0, tz=UTC).isoformat(timespec="milliseconds")

    rec = {
        "label": label, "model": model, "scratch": str(scratch),
        "config": {"partial_messages": bool(partial_messages), "resume": resume},
        "start_utc": iso(start_ms), "end_utc": iso(end_ms),
        "start_epoch_ms": round(start_ms, 1), "end_epoch_ms": round(end_ms, 1),
        "wall_ms": round(wall_ms, 1), "lock_wait_ms": round(lock_wait_ms, 1),
        "exit_code": exit_code, "timed_out": bool(timed_out),
        "session_id": meta["session_id"],
        "tool_use_by_name": meta["tool_use_by_name"],
        "subagent_tool_use_by_name": meta["subagent_tool_use_by_name"],
        "bash_failed": meta["bash_failed"], "unparsed_lines": meta["unparsed_lines"],
        "line_count": len(lines), "result_lines": len(results), "has_result": bool(results),
        "last_line_partial": last_partial,
        "steps": steps_done({**meta, **_last_result_meta(results)}, exit_code),
        "lines": lines,
    }
    if write:
        runs_dir = Path(runs_dir)
        runs_dir.mkdir(parents=True, exist_ok=True)
        stamp = datetime.fromtimestamp(start_ms / 1000.0, tz=UTC).strftime("%Y%m%dT%H%M%S")
        path = runs_dir / f"{label}-{stamp}-{(meta['session_id'] or 'nosession')[:8]}.json"
        rec["run_file"] = str(path)
        path.write_text(json.dumps(rec, indent=1) + "\n")
    return rec


def _kill_group(proc):
    try:
        os.killpg(proc.pid, 9)
    except OSError:
        proc.kill()


def _last_result_meta(results):
    """The result_* keys steps_done needs, from the capture's own per-line result records."""
    for r in reversed(results):
        f = r.get("fields", {})
        if f.get("subtype") == "success" and f.get("is_error") is False and f.get("num_turns", 0) > 0:
            return {"result_subtype": "success", "result_is_error": False}
    f = results[-1].get("fields", {}) if results else {}
    return {"result_subtype": f.get("subtype"), "result_is_error": f.get("is_error")}


def stream_tool_uses(cap) -> dict:
    """{tool_use_id: (name, first_recv_ms, parent_tool_use_id)} from a capture record."""
    out = {}
    for ln in cap["lines"]:
        for tu in ln.get("tool_uses", []):
            out.setdefault(tu["id"], (tu["name"], ln["recv_ms"], ln.get("parent_tool_use_id")))
    return out


def evaluate(cap, config_dir=DEFAULT_CONFIG_DIR, reference=None) -> dict:
    """Completeness and latency of a capture vs the transcript reference.

    capture % = reference tool_use ids that also appear in the stream / reference ids.
    latency (ms) per id = stream first-receive time - transcript timestamp (same machine clock).
    """
    ref = reference if reference is not None else read_reference(cap["session_id"], config_dir)
    seen = stream_tool_uses(cap)
    in_ref = [tid for tid in ref if tid in seen]
    missing = {tid: ref[tid][0] for tid in ref if tid not in seen}
    extra = [tid for tid in seen if tid not in ref]
    lat = {tid: round(seen[tid][1] - to_epoch_ms(ref[tid][1]), 1) for tid in in_ref}
    by_name_ref, by_name_seen = {}, {}
    for tid, (name, _ts) in ref.items():
        by_name_ref[name] = by_name_ref.get(name, 0) + 1
        if tid in seen:
            by_name_seen[name] = by_name_seen.get(name, 0) + 1
    sub_ids = [tid for tid in ref if seen.get(tid, (0, 0, None))[2]]
    return {
        "reference_ids": len(ref), "captured_ids": len(in_ref),
        "capture_pct": round(100.0 * len(in_ref) / len(ref), 1) if ref else None,
        "missing_by_name": _count(missing.values()),
        "stream_ids_not_in_reference": len(extra),
        "reference_by_name": by_name_ref, "captured_by_name": by_name_seen,
        "subagent_ids_in_stream": len(sub_ids),
        "latency_ms": lat,
        "latency_summary": summarize_latency(list(lat.values())),
    }


def _count(names):
    out = {}
    for n in names:
        out[n] = out.get(n, 0) + 1
    return out


def summarize_latency(values) -> dict:
    """n, min, median, p95, max (ms). p95 uses the nearest-rank index as the harness does."""
    if not values:
        return {"n": 0, "min": None, "median": None, "p95": None, "max": None}
    s = summarize([{"wall_ms": v} for v in values])
    ordered = s["wall_ms"]
    return {"n": s["n"], "min": ordered[0], "median": s["median_ms"], "p95": s["p95_ms"],
            "max": ordered[-1]}


def stream_event_summary(cap) -> dict:
    """Counts for a partial-messages run: stream_event lines, by event type and delta type."""
    ev, delta = {}, {}
    n = 0
    for ln in cap["lines"]:
        if ln.get("type") == "stream_event":
            n += 1
            ev[ln.get("event_type")] = ev.get(ln.get("event_type"), 0) + 1
            if ln.get("delta_type"):
                delta[ln["delta_type"]] = delta.get(ln["delta_type"], 0) + 1
    return {"stream_event_lines": n, "by_event_type": ev, "by_delta_type": delta,
            "bytes_total": sum(ln.get("bytes", 0) for ln in cap["lines"]),
            "bytes_stream_events": sum(ln.get("bytes", 0) for ln in cap["lines"]
                                       if ln.get("type") == "stream_event")}


def type_counts(cap) -> dict:
    out = {}
    for ln in cap["lines"]:
        k = ln.get("type") if not ln.get("subtype") else f"{ln.get('type')}/{ln['subtype']}"
        out[k] = out.get(k, 0) + 1
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--label", default="e4")
    ap.add_argument("--scratch")
    ap.add_argument("--partial", action="store_true", help="add --include-partial-messages")
    ap.add_argument("--resume", metavar="SESSION_ID")
    ap.add_argument("--lock", default=DEFAULT_LOCK)
    ap.add_argument("--config-dir", default=DEFAULT_CONFIG_DIR)
    ap.add_argument("--runs-dir", default=str(RUNS_DIR))
    ap.add_argument("--timeout", type=int, default=300)
    a = ap.parse_args()
    cap = capture_session(a.label, a.scratch, None, a.partial, a.resume, config_dir=a.config_dir,
                          runs_dir=a.runs_dir, timeout=a.timeout, lock_path=a.lock)
    print(cap["run_file"])


if __name__ == "__main__":
    main()
