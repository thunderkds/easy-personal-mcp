"""AC7: --resume <id> --fork-session gives a new session id and leaves the original transcript unchanged;
plain --resume keeps the id and appends to the same transcript (the case that loses messages if a terminal has it open)."""
import hashlib, os, re
from cc import *

repo = scratch_repo("p7")
cfgdir = os.environ.get("CLAUDE_CONFIG_DIR", os.path.expanduser("~/.claude"))
proj = os.path.join(cfgdir, "projects", re.sub(r"[^A-Za-z0-9]", "-", os.path.realpath(repo)))


def snap(sid):
    f = os.path.join(proj, sid + ".jsonl")
    if not os.path.exists(f):
        return None
    b = open(f, "rb").read()
    return {"lines": b.count(b"\n"), "sha256": hashlib.sha256(b).hexdigest()[:16]}


def one(extra, text):
    s = Session(repo, extra)
    s.send(text)
    r = s.result(90)
    sid = [e["session_id"] for e in s.events if e.get("subtype") == "init"][0]
    s.close()
    return sid, r


a, _ = one([], "say ok")
sa = snap(a)
print("A:", sa, "transcript exists in the config dir's projects folder:", sa is not None)
b, rb = one(["--resume", a, "--fork-session"], "say ok again")
print("fork -> new id differs from A:", b != a, "| fork result:", (rb or {}).get("subtype"))
print("A unchanged after the fork:", snap(a) == sa, "| fork transcript:", snap(b))
c, rc = one(["--resume", a], "say ok a third time")
print("plain resume -> same id as A:", c == a, "| result:", (rc or {}).get("subtype"))
print("A after plain resume (appended):", snap(a), "(was", sa, ")")
