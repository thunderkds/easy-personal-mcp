"""AC3 (no reply + latency), AC6 (4000-char text with line breaks; leading slash), headless."""
import glob, json, os, statistics
from cc import *

def last_transcript(repo_name):
    root = os.environ.get("CLAUDE_CONFIG_DIR") or os.path.expanduser("~/.claude")
    return max(glob.glob(f"{root}/projects/-tmp-t110-{repo_name}/*.jsonl"), key=os.path.getmtime)

out = {}
# AC3: with an empty-output hook vs no hook, 3 runs each (alternating)
repo = scratch_repo("p2"); work = f"{BASE}/p2_cfg"
st_h = write_settings(work); st_n = write_settings(work + "_n", with_hook=False)
rows = []
for i in range(3):
    env = hook_env(work, ["empty"])
    ev, wall, rc, _ = run_p(repo, work, "say ok", st_h, env)
    r = [e for e in ev if e["type"] == "result"][0]
    l = read_log(work)
    rows.append(("hook", round(wall, 2), r["duration_ms"], r["num_turns"], len(l), round((l[0]["t_end"] - l[0]["t_start"]) * 1000) if l else None))
    ev, wall, rc, _ = run_p(repo, work, "say ok", st_n, {})
    r = [e for e in ev if e["type"] == "result"][0]
    rows.append(("nohook", round(wall, 2), r["duration_ms"], r["num_turns"], 0, None))
out["ac3_rows(kind, wall_s, duration_ms, turns, stop_calls, hook_script_ms)"] = rows
# the hook's cost as Claude Code records it (durationMs of stop_hook_summary) in the last hook run
tp = last_transcript("p2")
# AC6a: 4000 chars with line breaks
lines = ["Do not use tools. Reply with only the last line of this message, copied exactly."] + [f"line {i:02d}: " + "x" * 70 for i in range(1, 50)]
text = "\n".join(lines)
text = text + "\n" + "END-MARKER-7Q"
text = text[:3990] if len(text) > 3990 else text
text = text.rstrip("\n").split("\n")
text = "\n".join(text[:-1] if not text[-1].startswith("END") else text)
pad = 4000 - len(text) - len("\nEND-MARKER-7Q")
reason = (text + "x" * max(pad, 0) + "\nEND-MARKER-7Q") if "END-MARKER-7Q" not in text else text
reason = reason[:3986] + "\nEND-MARKER-7Q" if len(reason) != 4000 else reason
out["ac6a_reason_len"] = len(reason); out["ac6a_newlines"] = reason.count("\n")
env = hook_env(work, ["block_if_inactive"] , reason=reason)
ev, wall, rc, _ = run_p(repo, work, "say ok", st_h, env)
r = [e for e in ev if e["type"] == "result"][0]
tp = last_transcript("p2")
recs = [json.loads(l) for l in open(tp)]
fb = [x["message"]["content"] for x in recs if x.get("type") == "user" and x.get("isMeta")]
out["ac6a"] = {"num_turns": r["num_turns"], "result_contains_marker": "END-MARKER-7Q" in r["result"], "transcript_feedback_len": [len(c) for c in fb],
               "transcript_content_equals_prefix_plus_reason": [c == "Stop hook feedback:\n" + reason for c in fb], "prefix_repr": [c[:22] for c in fb]}
# AC6b: leading slash
env = hook_env(work, ["block_if_inactive"], reason="/help")
ev, wall, rc, _ = run_p(repo, work, "say ok", st_h, env)
r = [e for e in ev if e["type"] == "result"][0]
tp = last_transcript("p2")
recs = [json.loads(l) for l in open(tp)]
fb = [x["message"]["content"] for x in recs if x.get("type") == "user" and x.get("isMeta")]
out["ac6b"] = {"num_turns": r["num_turns"], "feedback_content": fb, "types_after_first_assistant": [(x.get("type"), x.get("subtype"), (x.get("attachment") or {}).get("type")) for x in recs if x.get("type") in ("user", "assistant", "system")],
               "any_local_command_record": any("local-command" in json.dumps(x) or "command-name" in json.dumps(x) for x in recs),
               "assistant_text_chars": [len(c.get("text", "")) for x in recs if x.get("type") == "assistant" for c in x["message"]["content"] if c.get("type") == "text"]}
print(json.dumps(out, indent=1, default=str))
