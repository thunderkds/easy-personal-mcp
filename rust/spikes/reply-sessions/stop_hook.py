#!/usr/bin/env python3
"""T110 spike Stop hook. Reads the hook input on stdin, appends to $HOOK_LOG one JSON line (start/end time, input key list,
stop_hook_active, mode), then acts by the next entry of $HOOK_MODES (JSON list; past the end = "empty"):
  empty              exit 0, no output
  block              print {"decision":"block","reason":<contents of $HOOK_REASON>}
  block_if_inactive  block only when stop_hook_active is false
  sleep:N            sleep N seconds, exit 0, no output
  delay_block:N      sleep N seconds, then block_if_inactive
"""
import json, os, sys, time

t0 = time.time()
inp = json.loads(sys.stdin.read())
log, mf = os.environ["HOOK_LOG"], os.environ["HOOK_MODES"]
cf = mf + ".n"
i = json.load(open(cf)) if os.path.exists(cf) else 0
json.dump(i + 1, open(cf, "w"))
modes = json.load(open(mf))
mode = modes[i] if i < len(modes) else "empty"
rec = {"pid": os.getpid(), "call": i, "mode": mode, "t_start": t0, "keys": sorted(inp.keys()), "stop_hook_active": inp.get("stop_hook_active"),
       "hook_event_name": inp.get("hook_event_name"), "permission_mode": inp.get("permission_mode"),
       "types": {k: type(v).__name__ for k, v in inp.items()}}
def done(extra=None):
    rec.update(extra or {}); rec["t_end"] = time.time()
    open(log, "a").write(json.dumps(rec) + "\n")
if mode.startswith("delay_block:"):  # hold N seconds, then block (only when stop_hook_active is false)
    open(log, "a").write(json.dumps({**rec, "t_end": None, "note": "sleeping"}) + "\n")
    time.sleep(float(mode[12:]))
    mode = "block_if_inactive"
if mode == "block" or (mode == "block_if_inactive" and not inp.get("stop_hook_active")):
    reason = open(os.environ["HOOK_REASON"]).read()
    print(json.dumps({"decision": "block", "reason": reason}))
    done({"blocked": True})
elif mode.startswith("sleep:"):
    open(log, "a").write(json.dumps({**rec, "t_end": None, "note": "sleeping"}) + "\n")
    time.sleep(float(mode[6:]))
    done({"slept_to_end": True})
else:
    done({"blocked": False})
sys.exit(0)
