"""E2 driver (HUN-167): settings writer, capture/latency analysis, and the lock-wrapped session runs.

Interface for a combined run (HUN-170):
  collector:  python e2/collector.py --port 4390 --out FILE      (or collector.Collector)
  settings:   write_settings(path, url="http://127.0.0.1:4390/hook", async_=True)  -> passed to
              run_session(settings_file=path)
  analysis:   analyze(events, reference) -> dict with capture % and median/p95 latency (ms)
Sessions must hold the shared lock (session_lock()); other experiments run in parallel.
Metadata only: nothing here reads or stores prompt text, tool input or tool output.
"""
import fcntl
import json
import statistics
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "harness"))
sys.path.insert(0, str(HERE))
from reference import read_reference  # noqa: E402
from run_session import run_session  # noqa: E402

LOCK = ("/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/"
        "90deaa04-a1cb-4209-90a2-0a2669cb6205/scratchpad/live-trace-session.lock")
URL = "http://127.0.0.1:4390/hook"
EVENTS = ["SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse", "PostToolUseFailure",
          "SubagentStart", "SubagentStop", "Stop", "SessionEnd"]
BASELINE_MEDIAN_MS = 28264
BASELINE_SPREAD_MS = 5768
RUNS = HERE / "runs"


def write_settings(path, url=URL, async_=True, timeout=10):
    """Write a settings file with one http hook per event (async flag optional)."""
    hook = {"type": "http", "url": url, "timeout": timeout}
    if async_:
        hook["async"] = True
    data = {"hooks": {e: [{"hooks": [dict(hook)]}] for e in EVENTS}}
    Path(path).write_text(json.dumps(data, indent=2) + "\n")
    return path


@contextmanager
def session_lock(path=LOCK):
    with open(path, "a+") as fh:
        fcntl.flock(fh, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(fh, fcntl.LOCK_UN)


def read_events(path):
    out = []
    p = Path(path)
    if p.exists():
        for line in p.read_text().splitlines():
            try:
                out.append(json.loads(line))
            except ValueError:
                pass
    return out


def _iso_ms(iso):
    from reference import to_epoch_ms
    return to_epoch_ms(iso)


def _pct(vals, q):
    s = sorted(vals)
    return s[min(len(s) - 1, round(q * (len(s) - 1)))] if s else None


def _stats(vals):
    return {"n": len(vals), "median_ms": round(statistics.median(vals), 1) if vals else None,
            "p95_ms": round(_pct(vals, 0.95), 1) if vals else None,
            "min_ms": round(min(vals), 1) if vals else None,
            "max_ms": round(max(vals), 1) if vals else None}


def analyze(events, reference, session_id=None, ignore_tools=("SubagentHandback",)):
    """Capture % and latency for one session. reference = {tool_use_id: (name, iso ts)}.

    latency = collector receive time minus the transcript timestamp of the record holding the
    tool_use (same machine clock). Pre = PreToolUse event, Post = PostToolUse or PostToolUseFailure.
    """
    ev = [e for e in events if session_id is None or e.get("session_id") == session_id]
    pre = {e["tool_use_id"]: e["recv_ms"] for e in ev if e.get("hook_event_name") == "PreToolUse" and e.get("tool_use_id")}
    post = {e["tool_use_id"]: e["recv_ms"] for e in ev
            if e.get("hook_event_name") in ("PostToolUse", "PostToolUseFailure") and e.get("tool_use_id")}
    out = {}
    for label, ids in (("all", list(reference)),
                       ("no_handback", [t for t, (n, _) in reference.items() if n not in ignore_tools])):
        ref = {t: reference[t] for t in ids}
        out[label] = {"ref_ids": len(ref),
                      "pre_captured": sum(t in pre for t in ref),
                      "post_captured": sum(t in post for t in ref),
                      "pre_capture_pct": round(100.0 * sum(t in pre for t in ref) / len(ref), 1) if ref else None,
                      "post_capture_pct": round(100.0 * sum(t in post for t in ref) / len(ref), 1) if ref else None}
    pre_lat = [pre[t] - _iso_ms(ts) for t, (_, ts) in reference.items() if t in pre]
    post_lat = [post[t] - _iso_ms(ts) for t, (_, ts) in reference.items() if t in post]
    post_lat_noagent = [post[t] - _iso_ms(ts) for t, (n, ts) in reference.items() if t in post and n not in ("Agent", "Task")]
    out["post_latency_excl_subagent_tool"] = _stats(post_lat_noagent)
    out["post_minus_pre"] = _stats([post[t] - pre[t] for t in post if t in pre])
    out["pre_latency"] = _stats(pre_lat)
    out["post_latency"] = _stats(post_lat)
    out["pre_latency_values"] = [round(v, 1) for v in pre_lat]
    out["post_latency_values"] = [round(v, 1) for v in post_lat]
    out["events_by_name"] = {}
    for e in ev:
        n = e.get("hook_event_name", "?")
        out["events_by_name"][n] = out["events_by_name"].get(n, 0) + 1
    out["subagent_events_with_agent_id"] = sum(1 for e in ev if e.get("agent_id"))
    out["pre_post_extra_ids"] = sorted((set(pre) | set(post)) - set(reference))[:5]
    return out


def start_collector(out_path, port=4390, delay_ms=0):
    p = subprocess.Popen([sys.executable, str(HERE / "collector.py"), "--port", str(port), "--out", str(out_path),
                          "--delay-ms", str(delay_ms)],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(50):
        time.sleep(0.1)
        try:
            import socket
            socket.create_connection(("127.0.0.1", port), timeout=0.5).close()
            return p
        except OSError:
            continue
    p.kill()
    raise RuntimeError("collector did not start")


def stop_collector(p):
    p.send_signal(2)  # SIGINT -> final fields dump
    try:
        p.wait(timeout=5)
    except subprocess.TimeoutExpired:
        p.kill()


def measured_run(label, workdir, async_=True, with_collector=True, config_dir="/home/hungnguyenhuu/.claude-hung2"):
    """One standard session with http hooks in --settings. Returns (record, analysis)."""
    workdir = Path(workdir)
    workdir.mkdir(parents=True, exist_ok=True)
    settings = write_settings(workdir / f"{label}.settings.json", async_=async_)
    out = workdir / f"{label}.events.jsonl"
    cp = start_collector(out) if with_collector else None
    try:
        with session_lock():
            rec = run_session(settings_file=settings, label=label, runs_dir=RUNS, config_dir=config_dir)
        time.sleep(1.5)  # let late async posts land
    finally:
        if cp:
            stop_collector(cp)
    events = read_events(out)
    ref = read_reference(rec["session_id"], config_dir) if rec.get("session_id") else {}
    an = analyze(events, ref, rec.get("session_id"))
    an["wall_ms"] = rec["wall_ms"]
    an["session_steps"] = rec["steps"]
    an["exit_code"] = rec["exit_code"]
    an["label"] = label
    an["session_id"] = rec.get("session_id")
    (RUNS / f"{label}.analysis.json").write_text(json.dumps(an, indent=1) + "\n")
    return rec, an


def two_sessions_run(label, workdir, delay_ms=2000, config_dir="/home/hungnguyenhuu/.claude-hung2"):
    """Two sessions at once (async http hooks, slow collector) under ONE lock acquisition.

    The collector delays every reply by delay_ms: if `async` is honoured for http hooks the
    sessions are not slowed; if ignored, each blocking hook adds about delay_ms.
    """
    import threading
    workdir = Path(workdir)
    settings = write_settings(workdir / f"{label}.settings.json", async_=True)
    out = workdir / f"{label}.events.jsonl"
    cp = start_collector(out, delay_ms=delay_ms)
    recs = {}

    def one(tag):
        recs[tag] = run_session(settings_file=settings, label=f"{label}-{tag}", runs_dir=RUNS, config_dir=config_dir)

    try:
        with session_lock():
            ts = [threading.Thread(target=one, args=(t,)) for t in "ab"]
            [t.start() for t in ts]
            [t.join() for t in ts]
        time.sleep(delay_ms / 1000.0 + 1.5)
    finally:
        stop_collector(cp)
    events = read_events(out)
    res = {}
    for tag, rec in recs.items():
        ref = read_reference(rec["session_id"], config_dir) if rec.get("session_id") else {}
        an = analyze(events, ref, rec.get("session_id"))
        an.update(wall_ms=rec["wall_ms"], session_steps=rec["steps"], exit_code=rec["exit_code"], session_id=rec.get("session_id"))
        res[tag] = an
    sids = {e.get("session_id") for e in events}
    res["distinct_session_ids_in_events"] = len(sids)
    res["events_total"] = len(events)
    (RUNS / f"{label}.analysis.json").write_text(json.dumps(res, indent=1) + "\n")
    return res


if __name__ == "__main__":
    if sys.argv[1] == "two":
        r = two_sessions_run(sys.argv[2], Path(sys.argv[3]))
        print(json.dumps({k: (v if not isinstance(v, dict) else {x: v[x] for x in ("wall_ms", "session_steps", "all", "events_by_name")}) for k, v in r.items()}))
        sys.exit(0)
    mode, label = sys.argv[1], sys.argv[2]
    work = Path(sys.argv[3])
    rec, an = measured_run(label, work, async_=mode in ("async", "down_async"), with_collector=not mode.startswith("down"))
    print(json.dumps({k: an[k] for k in ("label", "wall_ms", "exit_code", "session_steps", "events_by_name")}))
    print(json.dumps({"steps": an["session_steps"], "pre": an["pre_latency"], "post": an["post_latency"], "all": an["all"]}))
