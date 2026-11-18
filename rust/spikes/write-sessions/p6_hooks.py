"""AC6: hooks via --settings <tmp file> in an INTERACTIVE session driven through tmux (a PTY).
PreToolUse(AskUserQuestion) answers; PermissionRequest allow / deny / exit-0-empty / slow-allow / timeout.
Records what the terminal shows (tmux capture-pane) while a hook waits. Never touches a real settings file."""
import json, os, subprocess, sys, time
from cc import *

here = os.path.dirname(os.path.abspath(__file__))
repo = scratch_repo("p6")
work = os.path.join(BASE, "p6_cfg"); os.makedirs(work, exist_ok=True)
log, modes = f"{work}/hook.log", f"{work}/modes.json"
for f in (log, modes + ".n"):
    if os.path.exists(f): os.remove(f)
json.dump({"PreToolUse": ["answer-slow"], "PermissionRequest": ["allow", "deny", "empty", "slow-allow", "slow"]}, open(modes, "w"))
cmd = f"python3 {here}/hook.py"
settings = {"hooks": {"PreToolUse": [{"matcher": "AskUserQuestion", "hooks": [{"type": "command", "command": cmd, "timeout": 130}]}],
                      "PermissionRequest": [{"hooks": [{"type": "command", "command": cmd, "timeout": 8}]}]}}
json.dump(settings, open(f"{work}/settings.json", "w"))
SES = "t102p6"
TMUX = ["tmux", "-L", "t102"]  # a private tmux server: never the owner's default one
subprocess.run(TMUX + ["kill-session", "-t", SES], capture_output=True)
envargs = ["env"] + [x for k in DROP for x in ("-u", k)] + [f"HOOK_LOG={log}", f"HOOK_MODES={modes}"]
subprocess.run(TMUX + ["new-session", "-d", "-s", SES, "-x", "170", "-y", "50", "-c", repo, "--"] + envargs +
               ["claude", "--setting-sources", "project", "--settings", f"{work}/settings.json", "--model", "haiku", "--permission-mode", "manual"], check=True)


def pane():
    return subprocess.run(TMUX + ["capture-pane", "-p", "-t", SES], capture_output=True, text=True).stdout


def keys(*k, literal=False):
    subprocess.run(TMUX + ["send-keys", "-t", SES] + (["-l"] if literal else []) + list(k), check=True)


def wait_for(pred, timeout=60, step=0.5):
    end = time.time() + timeout
    while time.time() < end:
        p = pane()
        if pred(p): return p
        time.sleep(step)
    return pane()


def marks(p):
    """What the screen shows, reduced to facts (no model text): booleans, never the lines themselves."""
    return {"question_ui": "Which colour" in p and ("Red" in p) and ("Blue" in p),
            "perm_dialog": "Do you want" in p or "proceed?" in p, "running_hook": "hook" in p.lower()}


def ask(prompt, label):
    keys(prompt, literal=True); time.sleep(0.5); keys("Enter")


p = wait_for(lambda p: "trust" in p.lower() or "❯" in p or "> " in p, 40)
print("startup trust dialog:", "trust" in p.lower())
if "trust" in p.lower():
    keys("Down"); time.sleep(0.5); keys("Enter")  # the dialog starts on "No, exit"; Claude Code records the scratch folder itself
wait_for(lambda p: "trust" not in p.lower() and "Try \"" in p, 40)
time.sleep(2)
res = {}
# turn 1: AskUserQuestion answered by the PreToolUse hook (waits 5 s)
ask("Use the AskUserQuestion tool to ask me which colour I prefer, options Red and Blue. Then tell me the answer you got in one short sentence.", "ask")
seen = []
panes = []
t0 = time.time()
while time.time() - t0 < 60:
    p = pane(); seen.append((round(time.time() - t0, 1), marks(p))); panes.append((round(time.time() - t0, 1), p))
    if len(open(log).read().splitlines() if os.path.exists(log) else []) >= 1 and time.time() - t0 > 14: break
    time.sleep(1)
wait_for(lambda p: "Blue" in p and "answer" in p.lower() and not marks(p)["question_ui"], 40)
if os.environ.get("P6_ONLY_ASK"):  # re-run of turn 1 only: keep the screens (they hold the owner-side test text only) for reading
    with open(f"{work}/panes.txt", "w") as f:
        for t, p in panes:
            f.write(f"===== t={t}\n{p}\n")
    subprocess.run(TMUX + ["kill-session", "-t", SES], capture_output=True)
    sys.exit(0)
print("turn1 marks over time (first 12 samples):", seen[:12])
print("turn1 ever showed question UI:", any(m["question_ui"] for _, m in seen))
print("turn1 final shows answer sentence mentioning Blue:", "Blue" in pane())
# turns 2..: PermissionRequest modes
for name, mode in (("a.txt", "allow"), ("b.txt", "deny"), ("c.txt", "empty"), ("d.txt", "slow-allow"), ("e.txt", "slow")):
    ask(f"Use the Bash tool to run exactly: touch {name} . Then say done.", mode)
    t0 = time.time(); samples = []
    while time.time() - t0 < (30 if mode in ("empty", "slow", "slow-allow") else 25):
        p = pane(); m = marks(p); samples.append((round(time.time() - t0, 1), m["perm_dialog"]))
        if mode in ("allow", "deny") and os.path.exists(log) and sum(1 for _ in open(log)) and ("done" in p.lower() or "denied" in p.lower()) and time.time() - t0 > 12: break
        if mode == "empty" and m["perm_dialog"] and time.time() - t0 > 4: break
        if mode == "slow-allow" and time.time() - t0 > 12: break
        if mode == "slow" and time.time() - t0 > 16: break
        time.sleep(1)
    res[mode] = {"file_created": os.path.exists(f"{repo}/{name}"), "dialog_samples": samples[:20]}
    if mode in ("empty", "slow"):
        keys("Escape"); time.sleep(1); keys("Escape"); time.sleep(2)
        res[mode]["file_created_after_escape"] = os.path.exists(f"{repo}/{name}")
    wait_for(lambda p: not marks(p)["perm_dialog"], 20)
    time.sleep(3)
for k, v in res.items():
    print(f"== PermissionRequest {k}:", json.dumps(v))
print("--- hook log (keys only)")
for l in open(log):
    o = json.loads(l); print({k: v for k, v in o.items() if k != "t"})
subprocess.run(TMUX + ["kill-session", "-t", SES], capture_output=True)
