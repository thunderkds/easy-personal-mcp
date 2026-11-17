"""AC3: SIGINT (and the control_request interrupt) during a long answer of a stream-json session."""
import json, os, signal, sys, time
from cc import *

LONG = "Write a 2000-word story about a lighthouse keeper. No preamble."
mode = sys.argv[1]  # sigint | sigint-group | control
repo = scratch_repo("p3_" + mode)
s = Session(repo, ["--include-partial-messages"])
s.send(LONG)
first = s.until(lambda e: e.get("type") == "stream_event" and e.get("event", {}).get("type") == "content_block_delta", 60)
print("streaming started:", bool(first))
time.sleep(1.0)
n = len(s.events)
t0 = time.time()
if mode == "sigint":
    os.kill(s.p.pid, signal.SIGINT)
elif mode == "sigint-group":
    os.killpg(s.p.pid, signal.SIGINT)
else:
    s.send(raw=json.dumps({"type": "control_request", "request_id": "req_1", "request": {"subtype": "interrupt"}}))
r = s.result(30)
print(f"[{mode}] result after {time.time()-t0:.2f}s:", None if r is None else {k: r.get(k) for k in ("subtype", "is_error", "terminal_reason", "num_turns", "stop_reason")}, "result_len", None if r is None else len(str(r.get("result", ""))))
time.sleep(1)
print("process alive:", s.alive(), "returncode:", s.p.poll())
print("kinds after signal:", [(e.get("type"), e.get("subtype") or (e.get("response") or {}).get("subtype")) for e in s.events[n:] if e.get("type") != "stream_event"])
for e in s.events[n:]:
    if e.get("type") == "control_response":
        dump("control_response", redact({k: v for k, v in e.items() if k != "_t"}))
if s.alive():
    s.send("say ok")
    r2 = s.result(45)
    print("next message answered:", None if r2 is None else (r2.get("subtype"), r2.get("is_error"), r2.get("num_turns")))
s.close()
print("returncode after close:", s.p.returncode, "stderr:", s.stderr[:3])
