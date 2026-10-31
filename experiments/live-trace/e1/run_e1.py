"""E1 experiment driver (HUN-166): run live sessions with the tail attached; metadata only.

Every claude session holds an exclusive flock on LOCK for its whole run (other live-trace
experiments run in parallel; timing must not overlap). Writes e1/runs/<label>.json (score and
stats) and e1/runs/<label>.events.jsonl (tail events of the spike's scratch sessions only).

  python run_e1.py measured N           # N standard sessions, tail attached
  python run_e1.py resume_big SESSION_ID SCRATCH  # two sessions at once: --resume + 1.5 MB tool output
  python run_e1.py kill                 # session killed mid-tool (Bash sleep)
"""
import fcntl
import json
import os
import stat
import sys
import tempfile
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "harness"))
import tail  # noqa: E402
from make_scratch import make_scratch  # noqa: E402
from reference import read_file, read_reference, to_epoch_ms, transcript_files  # noqa: E402
from run_session import DEFAULT_CONFIG_DIR, run_session  # noqa: E402

LOCK = ("/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/"
        "90deaa04-a1cb-4209-90a2-0a2669cb6205/scratchpad/live-trace-session.lock")
RUNS = HERE / "runs"
PROJECTS = Path(DEFAULT_CONFIG_DIR) / "projects"
BASELINE_MEDIAN_MS = 28264
BIG_PROMPT = ("Use the Bash tool to run exactly: head -c 1500000 /dev/urandom | base64 -w0 ; echo\n"
              "Then finish with the single word: done.")
KILL_PROMPT = ('Use the Bash tool to run exactly: python3 -c "import time; time.sleep(100)"\n'
               "Do nothing else.")
RESUME_PROMPT = "Use the Read tool to read notes_c.txt. Then finish with the single word: done."


def keep_scratch(ev):
    return (ev.get("project") or "").startswith("/tmp/live-trace-")


def max_line_len(session_id):
    """Longest transcript line (bytes) over the session's files: length only, text dropped."""
    best = 0
    for p in transcript_files(session_id, DEFAULT_CONFIG_DIR):
        with open(p, "rb") as fh:
            for ln in fh:
                best = max(best, len(ln))
    return best


def summarize(rec, ref, events):
    sc = tail.score([e for e in events if e["session_id"] == rec["session_id"]], ref,
                    to_epoch_ms)
    seen = {}
    for e in events:
        if e["session_id"] == rec["session_id"] and e["kind"] == "tool_use":
            seen.setdefault(e["tool_use_id"], e)
    import statistics as st
    tail_lag = [e["recv_ms"] - e["mtime_ms"] for e in seen.values() if e.get("mtime_ms")]
    write_lag = [e["mtime_ms"] - to_epoch_ms(ref[t][1]) for t, e in seen.items()
                 if e.get("mtime_ms") and t in ref]
    diag = {"tail_lag_median_ms": round(st.median(tail_lag), 1) if tail_lag else None,
            "tail_lag_max_ms": round(max(tail_lag), 1) if tail_lag else None,
            "write_lag_median_ms": round(st.median(write_lag), 1) if write_lag else None,
            "write_lag_max_ms": round(max(write_lag), 1) if write_lag else None}
    return {**diag, "session_id": rec["session_id"], "wall_ms": rec["wall_ms"], "exit_code": rec["exit_code"],
            "timed_out": rec["timed_out"], "steps": rec["steps"], "reference_total": sc["reference_total"],
            "captured": sc["captured"], "missed": sc["missed"], "extra": sc["extra"],
            "dup_tool_use_events": sc["dup_tool_use_events"],
            "median_ms": sc["median_ms"], "p95_ms": sc["p95_ms"], "min_ms": sc["min_ms"],
            "max_ms": sc["max_ms"], "latency_ms": sc["latency_ms"],
            "tool_results_seen": sum(1 for e in events if e["session_id"] == rec["session_id"]
                                     and e["kind"] == "tool_result"),
            "usage_events_seen": sum(1 for e in events if e["session_id"] == rec["session_id"]
                                     and e["kind"] == "usage"),
            "tool_use_by_name": rec["tool_use_by_name"]}


def locked_run(label, jobs):
    """jobs: list of dicts of run_session kwargs. One lock around all of them (run in parallel)."""
    RUNS.mkdir(parents=True, exist_ok=True)
    events_path = RUNS / f"{label}.events.jsonl"
    events_path.write_text("")
    with open(LOCK, "w") as lk:
        fcntl.flock(lk, fcntl.LOCK_EX)
        h = tail.start_tail(PROJECTS, events_path, poll_ms=100, keep=keep_scratch)
        cpu0 = time.process_time()
        results = [None] * len(jobs)

        def go(i, kw):
            results[i] = run_session(label=f"{label}-{i}", runs_dir=RUNS / "sessions", **kw)

        threads = [threading.Thread(target=go, args=(i, kw)) for i, kw in enumerate(jobs)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        cpu_s = time.process_time() - cpu0
        time.sleep(1.0)  # let the last transcript writes land before the final poll
        stats = h.stop()
        fcntl.flock(lk, fcntl.LOCK_UN)
    events = tail.read_events(events_path)
    out = {"label": label, "tail_stats": stats, "tail_cpu_s": round(cpu_s, 3), "sessions": []}
    for rec in results:
        ref = read_reference(rec["session_id"], DEFAULT_CONFIG_DIR) if rec["session_id"] else {}
        s = summarize(rec, ref, events)
        s["max_transcript_line_bytes"] = max_line_len(rec["session_id"]) if rec["session_id"] else None
        s["scratch"] = rec["scratch"]
        out["sessions"].append(s)
    (RUNS / f"{label}.json").write_text(json.dumps(out, indent=1) + "\n")
    return out


def brief(out):
    for s in out["sessions"]:
        print(f"{out['label']} sid={str(s['session_id'])[:8]} wall={s['wall_ms']} exit={s['exit_code']} "
              f"timed_out={s['timed_out']} captured={s['captured']}/{s['reference_total']} "
              f"median={s['median_ms']} p95={s['p95_ms']} max={s['max_ms']} dup={s['dup_tool_use_events']} "
              f"tail_lag={s['tail_lag_median_ms']}/{s['tail_lag_max_ms']} write_lag={s['write_lag_median_ms']}/{s['write_lag_max_ms']} "
              f"results_seen={s['tool_results_seen']} maxline={s['max_transcript_line_bytes']} "
              f"steps_ok={all(s['steps'].values())}")
    st = out["tail_stats"]
    print(f"  tail: polls={st['polls']} records={st['records']} events={st['events']} "
          f"abstained={st['abstained']} {st['abstained_by_type']} bad_json={st['bad_json']} "
          f"sub_files={st['subagent_files_seen']} scan_ms_max={st['scan_ms_max']:.1f} "
          f"scan_ms_mean={st['scan_ms_sum'] / max(1, st['polls']):.1f} final_poll_events="
          f"{st['final_poll_events']} cpu_s={out['tail_cpu_s']}")


def main():
    cmd = sys.argv[1]
    if cmd == "measured":
        for i in range(int(sys.argv[2])):
            label = f"measured-{time.strftime('%Y%m%dT%H%M%S')}-{i + 1}"
            brief(locked_run(label, [{}]))
    elif cmd == "kill":
        brief(locked_run("edge-killed-mid-tool", [{"prompt": KILL_PROMPT, "timeout": 25}]))
    elif cmd == "resume_big":
        sid, scratch = sys.argv[2], sys.argv[3]
        wrapper = Path(tempfile.mkdtemp(prefix="e1-wrap-")) / "claude-resume"
        wrapper.write_text(f'#!/bin/sh\nexec claude --resume {sid} "$@"\n')
        wrapper.chmod(wrapper.stat().st_mode | stat.S_IXUSR)
        brief(locked_run("edge-two-at-once-resume-big", [
            {"prompt": RESUME_PROMPT, "claude_bin": str(wrapper), "scratch": scratch},
            {"prompt": BIG_PROMPT, "extra_env": {"BASH_MAX_OUTPUT_LENGTH": "2000000"}}]))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
