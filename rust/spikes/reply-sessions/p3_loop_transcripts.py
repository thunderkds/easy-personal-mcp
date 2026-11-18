"""AC2 extra: always-block hook, 4 blocks then empty (does Claude Code cap the loop itself?); also reads the stop_hook_summary
records of the interactive probes' transcripts (timeout / Esc cases). Text replaced; only counts and field names."""
import glob, json, os
from cc import *
repo = scratch_repo("p3"); work = f"{BASE}/p3_cfg"
st = write_settings(work)
env = hook_env(work, ["block", "block", "block", "block", "empty"], reason="Reply with only the single word banana.")
ev, wall, rc, err = run_p(repo, work, "say ok", st, env)
r = [e for e in ev if e["type"] == "result"]
L = read_log(work)
print("always-block loop: result events", len(r), "num_turns", [x["num_turns"] for x in r], "stop calls", [(c["call"], c["stop_hook_active"], c.get("blocked")) for c in L], "wall_s", round(wall, 1))
root = os.environ["CLAUDE_CONFIG_DIR"]
for f in sorted(glob.glob(f"{root}/projects/-tmp-t110-p4/*.jsonl"), key=os.path.getmtime):
    recs = [json.loads(l) for l in open(f)]
    S = [{k: (v if k in ("hookCount", "hookErrors", "failedHookErrorIndexes", "preventedContinuation", "hasOutput", "level", "stopReason") else None) for k, v in x.items() if k in ("hookCount", "hookErrors", "failedHookErrorIndexes", "preventedContinuation", "hasOutput", "level", "stopReason")} | {"hookInfos": [sorted(h.keys()) for h in x["hookInfos"]]} for x in recs if x.get("subtype") == "stop_hook_summary"]
    print(os.path.basename(f)[:8], "n_user", sum(1 for x in recs if x.get("type") == "user"), json.dumps(S)[:420])
