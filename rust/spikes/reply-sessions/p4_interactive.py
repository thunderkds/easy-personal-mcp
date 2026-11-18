"""AC4 / AC5: interactive `claude` under a PTY (a private tmux server `tmux -L t110`), Stop hook that sleeps.
usage: p4_interactive.py <scenario> [sleep_s] [hook_timeout_s]
  type   : hold 20 s; type text (no Enter) at +4 s, press Enter at +8 s; watch what the screen shows and what happens at the end
  esc    : hold 20 s; press Escape at +5 s
  ctrlc  : hold 20 s; press Ctrl+C at +5 s (then, if the screen asks, nothing more)
  esc_text  : type text at +4 s (no Enter), Escape at +9 s
  esc_queued: type text + Enter (queued) at +4/+6 s, Escape at +9 s
  block  : hook holds sleep_s (arg, default 20) then blocks with a reason; touch nothing
  block_queued: as block, and the owner types 'alpha-typed' + Enter at +4/+6 s (queued) while the hook holds, before it blocks
  plain  : hold 20 s; touch nothing (the baseline screen)
  timeout: hook sleeps 60 s with timeout 30: touch nothing, watch to +75 s
Prints, per second when it changed, the last non-empty screen lines (only my test words and the model's one-word answers appear)."""
import json, os, re, subprocess, sys, time
from cc import *

scn = sys.argv[1]
sleep_s = int(sys.argv[2]) if len(sys.argv) > 2 else (60 if scn == "timeout" else 20)
hto = int(sys.argv[3]) if len(sys.argv) > 3 else (30 if scn == "timeout" else None)
repo = scratch_repo("p4"); work = f"{BASE}/p4_cfg_{scn}"
st = write_settings(work, timeout=hto)
env = hook_env(work, [f"delay_block:{sleep_s}"], reason="Reply with only the single word banana.") if scn in ("block", "block_queued") else hook_env(work, [f"sleep:{sleep_s}"])
SES = "t110p4"; TMUX = ["tmux", "-L", "t110"]
subprocess.run(TMUX + ["kill-session", "-t", SES], capture_output=True)
envargs = ["env"] + [x for k in DROP for x in ("-u", k)] + [f"{k}={v}" for k, v in env.items()]
subprocess.run(TMUX + ["new-session", "-d", "-s", SES, "-x", "150", "-y", "40", "-c", repo, "--"] + envargs +
               ["claude", "--setting-sources", "project", "--settings", st, "--model", "haiku"], check=True)

def pane(): return subprocess.run(TMUX + ["capture-pane", "-p", "-t", SES], capture_output=True, text=True).stdout
def keys(*k, lit=False): subprocess.run(TMUX + ["send-keys", "-t", SES] + (["-l"] if lit else []) + list(k), check=True)
def tail(p, n=9):
    ls = [l.rstrip()[:110] for l in p.splitlines() if l.strip() and not l.startswith("─") and "tmux focus-events" not in l and "/effort" not in l]
    return ls[-n:]
def norm(t): return re.sub(r"[✻✢✶✳·]|\b\d+s\b|\d+ tokens", "", json.dumps(t))
def wait_for(pred, timeout=60, step=0.5):
    end = time.time() + timeout
    while time.time() < end:
        p = pane()
        if pred(p): return p
        time.sleep(step)
    return pane()

p = wait_for(lambda p: "trust" in p.lower() or "Try \"" in p, 40)
if "trust" in p.lower():
    keys("Down"); time.sleep(0.5); keys("Enter")
    wait_for(lambda p: "trust" not in p.lower() and "Try \"" in p, 40)
time.sleep(2)
print("ready screen tail:", tail(pane(), 4))
os.environ["X"] = "1"
keys("say ok", lit=True); time.sleep(0.5); keys("Enter")
# wait for the hook to start (its 'sleeping' record)
end = time.time() + 60
while time.time() < end and not read_log(work): time.sleep(0.2)
L = read_log(work)
if not L: print("HOOK NEVER STARTED"); print(tail(pane(), 15)); subprocess.run(TMUX + ["kill-session", "-t", SES]); sys.exit(1)
t0 = L[0]["t_start"]; pid = L[0]["pid"]
print(f"hook started; pid alive check follows; t0 set (hook t_start)")
def alive():
    try: os.kill(pid, 0); return True
    except OSError: return False
last = None; did = set(); horizon = sleep_s + 20 if scn != "timeout" else 80
while time.time() - t0 < horizon:
    t = time.time() - t0
    if scn in ("type", "block_queued") and t > 4 and "t4" not in did: keys("alpha-typed", lit=True); did.add("t4"); print(f"[+{t:.1f}] SENT text 'alpha-typed' (no Enter)")
    if scn in ("type", "block_queued") and t > 6 and "t8" not in did: keys("Enter"); did.add("t8"); print(f"[+{t:.1f}] SENT Enter")
    if scn in ("esc_text", "esc_queued") and t > 4 and "t4" not in did: keys("alpha-typed", lit=True); did.add("t4"); print(f"[+{t:.1f}] SENT text 'alpha-typed' (no Enter)")
    if scn == "esc_queued" and t > 6 and "t6" not in did: keys("Enter"); did.add("t6"); print(f"[+{t:.1f}] SENT Enter (queues it)")
    if scn in ("esc_text", "esc_queued") and t > 9 and "e9" not in did: keys("Escape"); did.add("e9"); print(f"[+{t:.1f}] SENT Escape")
    if scn == "esc" and t > 5 and "e" not in did: keys("Escape"); did.add("e"); print(f"[+{t:.1f}] SENT Escape")
    if scn == "ctrlc" and t > 5 and "c" not in did: keys("C-c"); did.add("c"); print(f"[+{t:.1f}] SENT Ctrl+C")
    cur = (tail(pane(), 7), alive())
    if last is None or norm(cur) != norm(last):
        print(f"[+{t:.1f}] hook_alive={cur[1]} screen_tail={json.dumps(cur[0], ensure_ascii=False)}")
        last = cur
    time.sleep(1)
print("hook log:", [{k: v for k, v in r.items() if k in ("call", "mode", "t_end", "slept_to_end", "note", "stop_hook_active")} for r in read_log(work)])
print("hook t_end - t_start:", [None if r.get("t_end") is None else round(r["t_end"] - r["t_start"], 1) for r in read_log(work)])
if "Teach auto mode" in pane(): keys("Escape"); time.sleep(1); print("(dismissed the one-time auto-mode dialog with Esc)")
print("final screen:"); print("\n".join(tail(pane(), 14)))
subprocess.run(TMUX + ["kill-session", "-t", SES], capture_output=True)
