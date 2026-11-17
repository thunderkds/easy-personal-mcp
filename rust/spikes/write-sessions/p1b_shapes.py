"""AC1 extra: result event shape, init capabilities, second init per message, --replay-user-messages, text-array content."""
import json
from cc import *

repo = scratch_repo("p1b")
s = Session(repo, ["--replay-user-messages"])
s.send(raw=json.dumps({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": "say ok"}]}}))
r = s.result()
s.send("say ok")
s.result()
init = s.events[0]
print("capabilities:", init.get("capabilities"))
print("terminal_slash_commands:", init.get("terminal_slash_commands"))
print("permissionMode:", init.get("permissionMode"), "apiKeySource:", init.get("apiKeySource"))
print("event kinds:", s.kinds())
dump("result", redact({k: v for k, v in r.items() if k not in ("_t", "usage", "modelUsage")}))
for e in s.events:
    if e.get("type") == "user":
        dump("user (replayed)", redact({k: v for k, v in e.items() if k != "_t"}))
        break
print("second init session id equals first:", [e.get("session_id") for e in s.events if e.get("subtype") == "init"][0] == [e.get("session_id") for e in s.events if e.get("subtype") == "init"][1])
s.close()
print("stderr:", s.stderr[:3])
