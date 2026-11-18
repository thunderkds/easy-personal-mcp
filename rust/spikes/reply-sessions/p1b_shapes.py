"""Transcript shapes of the blocked-Stop records, from the latest p1 run (text replaced by <text>)."""
import glob, json, os, re
from cc import *
work = f"{BASE}/p1_cfg"
sid = None
ev = json.load(open(f"{work}/ev_shape.json"))
root = os.environ.get("CLAUDE_CONFIG_DIR") or os.path.expanduser("~/.claude")
tp = max(glob.glob(f"{root}/projects/-tmp-t110-p1/*.jsonl"), key=os.path.getmtime)
recs = [json.loads(l) for l in open(tp)]
i0 = max(i for i, r in enumerate(recs) if r.get("type") == "assistant")
first_a = min(i for i, r in enumerate(recs) if r.get("type") == "assistant")
for r in recs[first_a:]:
    t = r.get("type")
    if t in ("api-request-blob", "api-request", "last-prompt", "session-owner", "atis-latch", "cost-state", "queue-operation"): continue
    s = shape(r)
    if t == "assistant":
        s = {"type": "assistant", "message": {"role": "assistant", "content": [{"type": c.get("type")} for c in r["message"]["content"]], "stop_reason": r["message"].get("stop_reason")}, "contains_banana": "banana" in json.dumps(r).lower()}
    if t == "attachment" and r["attachment"]["type"] == "prompt_snapshot": continue
    print(json.dumps(s)[:900]); print()
print("raw 'Stop hook feedback' prefix check in user content:",
      [ (r["message"]["content"][:25] if isinstance(r["message"]["content"], str) else "list") for r in recs[first_a:] if r.get("type") == "user"])
print("--- stream-json events (user, system) shapes")
for e in ev: print(json.dumps(e)[:500])
