"""Headless runner for the standard test session (HUN-165).

Runs `claude -p <STANDARD_PROMPT> --model haiku --output-format stream-json --verbose` in a
scratch folder with tools Read, Bash, Edit and the subagent tool allowed (nothing else), parses
the stream for ids / names / types only, and writes one metadata-only JSON per run to
harness/runs/. Prompt text, assistant text, tool input and tool output are never kept or printed.

Usage:
  python run_session.py [--scratch DIR] [--settings FILE] [--env K=V ...] [--label NAME]
                        [--config-dir DIR] [--runs-dir DIR] [--timeout SEC]
Prints the path of the run JSON. E1-E4 import `run_session` and call it from Python.
"""
import argparse
import json
import os
import statistics
import subprocess
import sys
import threading
import time
from datetime import UTC, datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_scratch import make_scratch

HERE = Path(__file__).resolve().parent
RUNS_DIR = HERE / "runs"
DEFAULT_CONFIG_DIR = "/home/hungnguyenhuu/.claude-hung2"
SUBAGENT_TOOLS = ("Agent", "Task")  # the subagent tool is called Task in older versions
ALLOWED_TOOLS = ["Read", "Bash", "Edit", "Agent", "Task"]
# Env vars the spike must never set: they would put content into telemetry (shared spike rules).
FORBIDDEN_ENV = (
    "OTEL_LOG_USER_PROMPTS",
    "OTEL_LOG_TOOL_DETAILS",
    "OTEL_LOG_TOOL_CONTENT",
    "OTEL_LOG_RAW_API_BODIES",
)

STANDARD_PROMPT = """You are in a small scratch git repo. Do these steps in order, one tool call per step unless said otherwise. Keep every reply to one short line.
1. Use the Read tool to read notes_a.txt.
2. Use the Read tool to read notes_b.txt.
3. Use the Read tool to read notes_c.txt.
4. Use the Bash tool to run: sh check.sh
5. Use the Bash tool to run: ls /nonexistent   (it is expected to fail; do not retry it)
6. Use the Edit tool to change the word "alpha" to "ALPHA" on the first line of notes_a.txt.
7. Use the Agent tool (subagent) to start one subagent. Tell it to read notes_b.txt and answer with its first word only.
8. Finish with the single word: done."""


def parse_stream(lines) -> dict:
    """Reduce stream-json lines to metadata: ids, names, counts, types. Keeps no text."""
    session_id = None
    main_tools: dict[str, int] = {}
    sub_tools: dict[str, int] = {}
    bash_ids: set[str] = set()
    tool_names: dict[str, str] = {}
    bash_failed = 0
    unparsed = 0
    result = {}
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line)
        except ValueError:
            unparsed += 1
            continue
        if not isinstance(ev, dict):
            unparsed += 1
            continue
        etype = ev.get("type")
        if etype == "system" and ev.get("subtype") == "init" and session_id is None:
            sid = ev.get("session_id")
            session_id = sid if isinstance(sid, str) else None
        elif etype in ("assistant", "user"):
            is_sub = bool(ev.get("parent_tool_use_id"))
            msg = ev.get("message")
            content = msg.get("content") if isinstance(msg, dict) else None
            if not isinstance(content, list):
                continue
            for block in content:
                if not isinstance(block, dict):
                    continue
                if etype == "assistant" and block.get("type") == "tool_use":
                    name, tid = block.get("name"), block.get("id")
                    if not isinstance(name, str):
                        continue
                    bucket = sub_tools if is_sub else main_tools
                    bucket[name] = bucket.get(name, 0) + 1
                    if isinstance(tid, str):
                        tool_names[tid] = name
                        if name == "Bash":
                            bash_ids.add(tid)
                elif etype == "user" and block.get("type") == "tool_result":
                    if block.get("is_error") is True and block.get("tool_use_id") in bash_ids:
                        bash_failed += 1
        elif etype == "result":
            result = {
                "result_subtype": ev.get("subtype") if isinstance(ev.get("subtype"), str) else None,
                "result_is_error": ev.get("is_error") if isinstance(ev.get("is_error"), bool) else None,
                "num_turns": ev.get("num_turns") if isinstance(ev.get("num_turns"), int) else None,
                "cc_duration_ms": ev.get("duration_ms") if isinstance(ev.get("duration_ms"), (int, float)) else None,
                "total_cost_usd": ev.get("total_cost_usd") if isinstance(ev.get("total_cost_usd"), (int, float)) else None,
            }
            if session_id is None and isinstance(ev.get("session_id"), str):
                session_id = ev["session_id"]
    return {
        "session_id": session_id,
        "tool_use_by_name": main_tools,
        "subagent_tool_use_by_name": sub_tools,
        "tool_use_total": sum(main_tools.values()) + sum(sub_tools.values()),
        "bash_total": main_tools.get("Bash", 0),
        "bash_failed": bash_failed,
        "unparsed_lines": unparsed,
        "result_subtype": None, "result_is_error": None, "num_turns": None,
           "cc_duration_ms": None, "total_cost_usd": None, **result,
    }


def steps_done(meta: dict, exit_code) -> dict:
    """Which standard steps happened. Honest booleans; a skipped step is recorded as False."""
    main = meta["tool_use_by_name"]
    return {
        "read_3": main.get("Read", 0) >= 3,
        "bash_2_one_failing": meta["bash_total"] >= 2 and meta["bash_failed"] >= 1,
        "edit_1": main.get("Edit", 0) >= 1,
        "subagent_1": sum(main.get(n, 0) for n in SUBAGENT_TOOLS) >= 1,
        "normal_end": exit_code == 0 and meta["result_subtype"] == "success"
        and meta["result_is_error"] is False,
    }


def run_session(scratch=None, extra_env=None, settings_file=None, label="run", model="haiku",
                config_dir=DEFAULT_CONFIG_DIR, runs_dir=RUNS_DIR, timeout=300,
                claude_bin="claude", prompt=STANDARD_PROMPT) -> dict:
    """Run the standard session once; write and return the metadata-only run record.

    scratch: existing scratch folder (made by make_scratch if None). extra_env: dict merged into
    the child env. settings_file: passed as `--settings FILE`. Record keys: label, session_id,
    scratch, start_utc, end_utc, start_epoch_ms, end_epoch_ms, wall_ms, exit_code, timed_out,
    tool_use_by_name, subagent_tool_use_by_name, steps, run_file, ... (see parse_stream).
    """
    extra_env = dict(extra_env or {})
    bad = [k for k in extra_env if k in FORBIDDEN_ENV]
    if bad:
        raise ValueError(f"forbidden content-logging env var(s): {bad}")
    scratch = Path(scratch) if scratch else make_scratch()
    env = {**os.environ, "CLAUDE_CONFIG_DIR": str(config_dir), **extra_env}
    for k in FORBIDDEN_ENV:
        env.pop(k, None)
    cmd = [claude_bin, "-p", prompt, "--model", model, "--output-format", "stream-json",
           "--verbose", "--allowed-tools", *ALLOWED_TOOLS]
    if settings_file:
        cmd += ["--settings", str(settings_file)]
    start_ms = time.time() * 1000.0
    t0 = time.monotonic()
    proc = subprocess.Popen(cmd, cwd=scratch, env=env, stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    timed_out = []
    timer = threading.Timer(timeout, lambda: (timed_out.append(1), proc.kill()))
    timer.start()
    try:
        meta = parse_stream(proc.stdout)  # streams line by line; text is dropped
        exit_code = proc.wait()
    finally:
        timer.cancel()
    wall_ms = (time.monotonic() - t0) * 1000.0
    end_ms = time.time() * 1000.0

    def iso(ms):
        return datetime.fromtimestamp(ms / 1000.0, tz=UTC).isoformat(timespec="milliseconds")

    rec = {
        "label": label, "model": model, "scratch": str(scratch),
        "start_utc": iso(start_ms), "end_utc": iso(end_ms),
        "start_epoch_ms": round(start_ms, 1), "end_epoch_ms": round(end_ms, 1),
        "wall_ms": round(wall_ms, 1), "exit_code": exit_code, "timed_out": bool(timed_out),
        "extra_env_names": sorted(extra_env), "settings_file": str(settings_file) if settings_file else None,
        **meta, "steps": steps_done(meta, exit_code),
    }
    runs_dir = Path(runs_dir)
    runs_dir.mkdir(parents=True, exist_ok=True)
    stamp = datetime.fromtimestamp(start_ms / 1000.0, tz=UTC).strftime("%Y%m%dT%H%M%S")
    path = runs_dir / f"{label}-{stamp}-{(meta['session_id'] or 'nosession')[:8]}.json"
    rec["run_file"] = str(path)
    path.write_text(json.dumps(rec, indent=1) + "\n")
    return rec


def summarize(records) -> dict:
    """Per-run wall-clock (ms) and median / p95 for a list of run records."""
    walls = sorted(r["wall_ms"] for r in records)
    p95 = walls[min(len(walls) - 1, round(0.95 * (len(walls) - 1)))] if walls else None
    return {"n": len(walls), "wall_ms": walls, "median_ms": statistics.median(walls) if walls else None,
            "p95_ms": p95}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--scratch", help="existing scratch folder (default: make a new one)")
    ap.add_argument("--settings", help="settings JSON file passed as --settings")
    ap.add_argument("--env", action="append", default=[], metavar="K=V", help="extra child env var")
    ap.add_argument("--label", default="run")
    ap.add_argument("--config-dir", default=DEFAULT_CONFIG_DIR)
    ap.add_argument("--runs-dir", default=str(RUNS_DIR))
    ap.add_argument("--timeout", type=int, default=300)
    args = ap.parse_args()
    env = dict(kv.split("=", 1) for kv in args.env)
    rec = run_session(args.scratch, env, args.settings, args.label, config_dir=args.config_dir,
                      runs_dir=args.runs_dir, timeout=args.timeout)
    print(rec["run_file"])


if __name__ == "__main__":
    main()
