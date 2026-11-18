"""AC1, AC2, AC7 (+ transcript shape): headless `claude -p` with the Stop hook."""
import json, os, sys
from cc import *

repo = scratch_repo("p1"); work = f"{BASE}/p1_cfg"
st = write_settings(work)
out = {}
env = hook_env(work, ["block_if_inactive", "block_if_inactive", "empty"], reason="Reply with only the single word banana.")
ev, wall, rc, err = run_p(repo, work, "say ok", st, env)
res = [e for e in ev if e.get("type") == "result"]
asst = [e for e in ev if e.get("type") == "assistant"]
sid = res[-1]["session_id"] if res else None
out["ac1_7"] = {"rc": rc, "wall_s": round(wall, 1), "event_types": [(e.get("type"), e.get("subtype")) for e in ev],
                "n_result": len(res), "num_turns": [r.get("num_turns") for r in res], "result_text_contains_banana": [("banana" in (r.get("result") or "").lower()) for r in res],
                "assistant_texts_contain_banana": ["banana" in json.dumps(a).lower() for a in asst], "stderr_len": len(err)}
log = read_log(work)
out["ac2"] = {"calls": [{k: c.get(k) for k in ("call", "mode", "stop_hook_active", "blocked")} for c in log], "keys_first": log[0]["keys"] if log else None,
              "types": log[0]["types"] if log else None, "n_stop_calls": len(log)}
tp = transcript(sid) if sid else None
out["transcript"] = {"path_found": bool(tp)}
if tp:
    recs = [json.loads(l) for l in open(tp)]
    out["transcript"]["types"] = [(r.get("type"), (r.get("message") or {}).get("role"), r.get("subtype"), (r.get("attachment") or {}).get("type")) for r in recs]
    out["transcript"]["shapes"] = [shape(r) for r in recs][:14]
    out["transcript"]["raw_has_Stop_hook_feedback"] = any("Stop hook feedback" in l for l in open(tp))
print(json.dumps(out, indent=1, default=str)[:12000])
json.dump([shape(e) for e in ev if e.get("type") in ("user", "system")][:12], open(f"{work}/ev_shape.json", "w"), indent=1)
