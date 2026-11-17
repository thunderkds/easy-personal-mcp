"""AC1 + AC2: two user messages on stdin get two results; a slash command as a user message."""
import json, sys, time
from cc import *

repo = scratch_repo("p1")
s = Session(repo)
line1 = s.send("say ok")
r1 = s.result()
t1 = time.time()
line2 = s.send("say ok again")
r2 = s.result()
print("stdin line shape:", redact(json.loads(line1)))
print("process still alive after 2 results:", s.alive())
print("event kinds:", s.kinds())
for e in s.events:
    if e.get("type") in ("system", "assistant", "result", "rate_limit_event") and not e.get("_shown"):
        e["_shown"] = True
        dump(f"{e.get('type')}/{e.get('subtype')}", redact({k: v for k, v in e.items() if k != "_t"}))
print("session ids:", {e.get("session_id") for e in s.events if e.get("session_id")})
print("results:", [(r.get("subtype"), r.get("is_error"), r.get("num_turns")) for r in (r1, r2)])
s.close()
print("stderr:", s.stderr[:5])
