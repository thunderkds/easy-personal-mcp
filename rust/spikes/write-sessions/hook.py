#!/usr/bin/env python3
"""Spike hook command (AC6). Reads the hook input on stdin, logs its key names + a timestamp to $HOOK_LOG, then acts by
the next entry of $HOOK_MODES (a JSON file: {"<hook_event_name>": ["answer-slow", "allow", ...]}):
  answer       PreToolUse AskUserQuestion: print hookSpecificOutput permissionDecision allow + updatedInput.answers (2nd option)
  answer-slow  same after 5 s
  allow / deny PermissionRequest: print decision.behavior allow / deny (+ message)
  slow-allow   sleep 5 then allow
  empty        exit 0, no output
  slow         sleep 12 (longer than the hook timeout), no output
"""
import json, os, sys, time

raw = sys.stdin.read()
inp = json.loads(raw)
ev = inp.get("hook_event_name")
log, modes, counters = os.environ["HOOK_LOG"], os.environ["HOOK_MODES"], os.environ["HOOK_MODES"] + ".n"
n = json.load(open(counters)) if os.path.exists(counters) else {}
i = n.get(ev, 0); n[ev] = i + 1; json.dump(n, open(counters, "w"))
mode = (json.load(open(modes)).get(ev) or ["empty"])[i] if i < len(json.load(open(modes)).get(ev, [])) else "empty"
with open(log, "a") as f:
    f.write(json.dumps({"t": time.time(), "event": ev, "mode": mode, "input_keys": sorted(inp.keys()),
                        "tool_name": inp.get("tool_name"), "tool_input_keys": sorted((inp.get("tool_input") or {}).keys()),
                        "extra": {k: (v if isinstance(v, (bool, int)) or k in ("permission_mode", "hook_event_name") else "<text>") for k, v in inp.items() if k not in ("tool_input",)}}) + "\n")
if mode.startswith("answer"):
    if mode == "answer-slow":
        time.sleep(5)
    q = inp["tool_input"]["questions"][0]
    out = {"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow",
           "updatedInput": {**inp["tool_input"], "answers": {q["question"]: q["options"][1]["label"]}}}}
    print(json.dumps(out))
elif mode in ("allow", "slow-allow"):
    if mode == "slow-allow":
        time.sleep(5)
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": {"behavior": "allow"}}}))
elif mode == "deny":
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": {"behavior": "deny", "message": "spike-hook-denied"}}}))
elif mode == "slow":
    time.sleep(12)
sys.exit(0)
