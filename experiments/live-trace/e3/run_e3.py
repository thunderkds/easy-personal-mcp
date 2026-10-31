"""Run one traced session against the local OTLP receiver (HUN-168, E3).

Usage: python run_e3.py CONFIG [--n N] [--grace-quiet S] [--grace-max S]
CONFIG: default | lowered | down | traces
  default : default export intervals (logs 5 s, metrics 60 s)
  lowered : OTEL_LOGS_EXPORT_INTERVAL=1000, OTEL_METRIC_EXPORT_INTERVAL=5000
  down    : lowered intervals, endpoint set but NO receiver listening
  traces  : lowered intervals + CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1, OTEL_TRACES_EXPORTER=otlp

Telemetry env vars go ONLY into the child process env (run_session extra_env). Content-logging
vars (OTEL_LOG_*) are never set; run_session refuses them. Every session holds an exclusive flock
on the shared lock file for its whole run (other experiments run in parallel).
Output (metadata only): e3/runs/<config>-<stamp>-<session8>.json (sanitized records + numbers).
"""
import argparse
import fcntl
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "harness"))
from analyze import analyze_run  # noqa: E402
from receiver import OtlpReceiver  # noqa: E402
from run_session import DEFAULT_CONFIG_DIR, run_session  # noqa: E402

RUNS = HERE / "runs"
LOCK = ("/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/"
        "90deaa04-a1cb-4209-90a2-0a2669cb6205/scratchpad/live-trace-session.lock")
PORT = 4318
BASE_ENV = {
    "CLAUDE_CODE_ENABLE_TELEMETRY": "1",
    "OTEL_LOGS_EXPORTER": "otlp",
    "OTEL_METRICS_EXPORTER": "otlp",
    "OTEL_EXPORTER_OTLP_PROTOCOL": "http/json",
    "OTEL_EXPORTER_OTLP_ENDPOINT": f"http://127.0.0.1:{PORT}",
}
LOWERED = {"OTEL_LOGS_EXPORT_INTERVAL": "1000", "OTEL_METRIC_EXPORT_INTERVAL": "5000"}
CONFIGS = {
    "default": {},
    "lowered": LOWERED,
    "down": LOWERED,
    "traces": {**LOWERED, "CLAUDE_CODE_ENHANCED_TELEMETRY_BETA": "1", "OTEL_TRACES_EXPORTER": "otlp",
               "OTEL_TRACES_EXPORT_INTERVAL": "1000"},
}


def run_once(config: str, grace_quiet: float = 8.0, grace_max: float = 30.0, lock_path: str = LOCK,
             config_dir: str = DEFAULT_CONFIG_DIR, runs_dir=RUNS, port: int = PORT,
             claude_bin: str = "claude") -> dict:
    """One session under CONFIG with the lock held; returns the (sanitized) result record."""
    env = {**BASE_ENV, **CONFIGS[config], "OTEL_EXPORTER_OTLP_ENDPOINT": f"http://127.0.0.1:{port}"}
    with open(lock_path, "a+") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)  # held for the whole run, released on close
        rx = None if config == "down" else OtlpReceiver(port=port).start()
        try:
            rec = run_session(extra_env=env, label=f"e3-{config}", config_dir=config_dir,
                              runs_dir=Path(runs_dir) / "sessions", claude_bin=claude_bin)
            waited = rx.wait_quiet(quiet_s=grace_quiet, max_s=grace_max) if rx else 0.0
            snap = rx.snapshot() if rx else {"events": [], "metrics": [], "spans": [], "posts": []}
        finally:
            if rx:
                rx.stop()
    result = {
        "config": config, "env_names": sorted(env), "session": {
            k: rec.get(k) for k in ("session_id", "start_utc", "end_utc", "start_epoch_ms",
                                    "end_epoch_ms", "wall_ms", "exit_code", "timed_out", "steps",
                                    "total_cost_usd", "tool_use_by_name", "subagent_tool_use_by_name")},
        "grace_waited_s": round(waited, 2), "receiver": snap,
        "analysis": analyze_run(snap, rec.get("session_id"), config_dir, rec.get("start_epoch_ms"))
        if rx else None,
    }
    stamp = rec["start_utc"].replace(":", "").replace("-", "")[:15]
    out = Path(runs_dir) / f"{config}-{stamp}-{(rec.get('session_id') or 'nosession')[:8]}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=1) + "\n")
    result["out_file"] = str(out)
    return result


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("config", choices=sorted(CONFIGS))
    ap.add_argument("--n", type=int, default=1)
    ap.add_argument("--grace-quiet", type=float, default=8.0)
    ap.add_argument("--grace-max", type=float, default=30.0)
    args = ap.parse_args()
    for _ in range(args.n):
        r = run_once(args.config, args.grace_quiet, args.grace_max)
        s, a = r["session"], r["analysis"] or {}
        print(json.dumps({"out": r["out_file"], "wall_ms": s["wall_ms"], "exit": s["exit_code"],
                          "steps": s["steps"], "posts": a.get("posts"),
                          "events": a.get("event_counts"),
                          "latency": (a.get("latency") or {}).get("latency")}, default=str))


if __name__ == "__main__":
    main()
