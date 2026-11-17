"""AC2: slash commands sent as user messages: a project command, a project skill, built-ins (/help, /context)."""
import json, os
from cc import *

repo = scratch_repo("p2")
os.makedirs(f"{repo}/.claude/commands")
open(f"{repo}/.claude/commands/ping.md", "w").write("---\ndescription: spike ping\n---\nReply with exactly the single word PONG and nothing else.\n")
os.makedirs(f"{repo}/.claude/skills/pong")
open(f"{repo}/.claude/skills/pong/SKILL.md", "w").write("---\nname: pong\ndescription: spike skill\n---\nReply with exactly the single word PANG and nothing else.\n")
s = Session(repo)
for msg in ("/ping", "/pong", "/help", "/context", "/nosuchcommand"):
    n = len(s.events)
    s.send(msg)
    r = s.result(60)
    new = s.events[n:]
    print(f"== {msg}: kinds={[(e.get('type'), e.get('subtype')) for e in new]}")
    if r:
        print("   result:", {k: ("<text>" if k == "result" else r.get(k)) for k in ("subtype", "is_error", "num_turns", "total_cost_usd", "terminal_reason")}, "result_len=", len(str(r.get("result", ""))))
        if msg in ("/ping", "/pong"):
            print("   result text is the command output (PONG/PANG)?:", str(r.get("result")).strip())
    for e in new:
        if e.get("type") == "assistant":
            print("   assistant blocks:", [b.get("type") for b in e["message"]["content"]])
        if e.get("type") == "user":
            c = e["message"]["content"]
            print("   user event:", "<text:%d chars>" % len(json.dumps(c)))
print("init slash_commands has ping/pong:", [c for c in s.events[0].get("slash_commands", []) if c in ("ping", "pong")], "skills:", [c for c in s.events[0].get("skills", []) if c in ("ping", "pong")])
s.close()
print("stderr:", s.stderr[:3])
