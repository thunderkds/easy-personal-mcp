"""AC4: --permission-prompt-tool with a stdio MCP server. usage: p4_permission_tool.py <scenario>

scenarios: bash-allow | bash-allow-noinput | bash-deny | bash-badshape | ask | plan
"""
import json, os, sys, time
from cc import *

scn = sys.argv[1]
here = os.path.dirname(os.path.abspath(__file__))
repo = scratch_repo("p4_" + scn)
work = os.path.join(BASE, "p4_" + scn + "_cfg")
os.makedirs(work, exist_ok=True)
log, ans = f"{work}/perm.log", f"{work}/answers.json"
open(log, "w").close()
cfg = {"mcpServers": {"perm": {"command": "python3", "args": [f"{here}/perm_server.py"], "env": {"PERM_LOG": log, "PERM_ANSWERS": ans}}}}
json.dump(cfg, open(f"{work}/mcp.json", "w"))

prompt = "Use the Bash tool to run exactly: touch spike-marker.txt . Then say done."
mode = []
if scn == "bash-allow":
    answers = {"*": {"behavior": "allow", "_echo_input": True}}
elif scn == "bash-allow-noinput":
    answers = {"*": {"behavior": "allow"}}
elif scn == "bash-deny":
    answers = {"*": {"behavior": "deny", "message": "spike-denied-by-owner"}}
elif scn == "bash-badshape":
    answers = {"*": {"_raw": "not json at all"}}
elif scn == "ask":
    prompt = "Use the AskUserQuestion tool to ask me which colour I prefer, options Red and Blue. Then tell me the answer you got in one short sentence."
    answers = {"AskUserQuestion": {"_ask": True}}
elif scn == "ask-other":
    prompt = "Use the AskUserQuestion tool to ask me which colour I prefer, options Red and Blue. Then tell me the answer you got in one short sentence."
    answers = {"AskUserQuestion": {"_ask": True, "_label": "Green-typed-by-owner"}}
elif scn == "plan-deny":
    prompt = "Make a one-step plan to print hello and call ExitPlanMode with it. Do not run anything."
    mode = ["--permission-mode", "plan"]
    answers = {"*": {"behavior": "deny", "message": "keep-planning-spike-note"}}
elif scn == "plan":
    prompt = "Make a one-step plan to print hello and call ExitPlanMode with it. Do not run anything."
    mode = ["--permission-mode", "plan"]
    answers = {"*": {"behavior": "allow", "_echo_input": True}}
json.dump(answers, open(ans, "w"))

s = Session(repo, ["--mcp-config", f"{work}/mcp.json", "--strict-mcp-config",
                   "--permission-prompt-tool", "mcp__perm__permission_prompt"] + mode)
s.send(prompt)
r = s.result(120)
print("result:", None if r is None else {k: ("<text>" if k == "result" else r.get(k)) for k in ("subtype", "is_error", "num_turns", "terminal_reason", "permission_denials")})
if r:
    print("result text:", str(r.get("result"))[:300].replace("\n", " "))
print("kinds:", s.kinds())
for e in s.events:
    if e.get("type") == "system" and e.get("subtype") == "init":
        print("mcp_servers in init:", e.get("mcp_servers"), "tools has perm:", [t for t in e.get("tools", []) if "perm" in t])
        break
for e in s.events:
    if e.get("type") == "user":
        for b in e["message"]["content"] if isinstance(e["message"]["content"], list) else []:
            if b.get("type") == "tool_result":
                print("tool_result is_error=", b.get("is_error"), "content=", str(b.get("content"))[:300].replace("\n", " "))
print("--- perm server log")
for l in open(log):
    o = json.loads(l)
    if "call" in o:
        print(json.dumps({"call": o["call"], "meta_keys": o["meta_keys"], "arguments": o["arguments"]})[:1500])
    elif "rpc" in o:
        print("rpc", o["rpc"])
    else:
        print("note:", {k: v for k, v in o.items() if k != "t"})
s.close()
print("stderr:", s.stderr[:5])
