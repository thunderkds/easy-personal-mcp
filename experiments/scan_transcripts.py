"""Throwaway read-only scan for HUN-32. Prints AGGREGATE counts only.

Never prints a transcript line, text, path, or session id. Session ids are
written (if --ids-out given) to a file OUTSIDE the repo for the leak check.
"""
import json, os, re, sys, time, argparse
from collections import Counter, defaultdict

SAFE = re.compile(r"^[A-Za-z0-9_\-\.]{1,40}$")
def safe(s):
    return s if isinstance(s, str) and SAFE.match(s) else "<unsafe-or-nonstr>"

ap = argparse.ArgumentParser()
ap.add_argument("--root", default=os.path.expanduser("~/.claude/projects"))
ap.add_argument("--days", type=int, default=30)
ap.add_argument("--ids-out")
a = ap.parse_args()

cutoff = time.time() - a.days * 86400
files = []
for d, _, fs in os.walk(a.root):
    for f in fs:
        if f.endswith(".jsonl"):
            files.append(os.path.join(d, f))

tot = Counter()
vol = Counter()
tv = Counter()                      # (version,type) -> n
keys = defaultdict(Counter)         # (version,type) -> key -> n
nested = defaultdict(Counter)       # (version,type,'path') -> key -> n
ids = set()
for p in files:
    st = os.stat(p)
    all_bytes = st.st_size
    tot["files_all"] += 1; tot["bytes_all"] += all_bytes
    if st.st_mtime < cutoff:
        continue
    rel = os.path.relpath(p, a.root).split(os.sep)
    vol["files"] += 1; vol["bytes"] += all_bytes
    if len(rel) > 2: vol["files_nested"] += 1
    if len(rel) > 2 and "subagents" in rel: vol["files_in_subagents_dir"] += 1
    with open(p, "rb") as fh:
        data = fh.read()
    lines = data.split(b"\n")
    ends_nl = data.endswith(b"\n")
    if not ends_nl: vol["files_no_trailing_newline"] += 1
    for i, ln in enumerate(lines):
        if not ln.strip(): continue
        vol["lines"] += 1
        try:
            r = json.loads(ln)
        except Exception:
            vol["lines_unparseable"] += 1
            if i == len(lines) - 1 and not ends_nl: vol["truncated_last_line"] += 1
            continue
        if not isinstance(r, dict):
            vol["lines_nondict"] += 1; continue
        v = safe(r.get("version")) if "version" in r else "<no-version>"
        t = safe(r.get("type")) if "type" in r else "<no-type>"
        k = (v, t)
        tv[k] += 1
        for key in r:
            keys[k][safe(key)] += 1
        sid = r.get("sessionId")
        if isinstance(sid, str): ids.add(sid)
        if r.get("isSidechain") is True: nested[k]["isSidechain=true"] += 1
        m = r.get("message")
        if isinstance(m, dict):
            for key in m: nested[k]["message." + safe(key)] += 1
            u = m.get("usage")
            if isinstance(u, dict):
                for key in u: nested[k]["usage." + safe(key)] += 1
            c = m.get("content")
            if isinstance(c, list):
                for b in c:
                    if isinstance(b, dict): nested[k]["block:" + safe(b.get("type"))] += 1
            elif isinstance(c, str):
                nested[k]["content:str"] += 1
        # content-less record variants
        for key in ("subtype", "level"):
            if key in r: nested[k][f"{key}:{safe(r[key])}"] += 1

print("== ALL-TIME (all files) ==", dict(tot))
print(f"== LAST {a.days} DAYS ==", dict(vol))
print("distinct sessionIds (30d):", len(ids))
print("top-level project dirs:", len({os.path.relpath(p, a.root).split(os.sep)[0] for p in files}))
print("\n== version x type counts ==")
for (v, t), n in sorted(tv.items()):
    print(f"{v}\t{t}\t{n}")
print("\n== field presence % per (version,type) (top-level keys) ==")
for k in sorted(tv):
    n = tv[k]
    print(f"[{k[0]} | {k[1]}] n={n}")
    for key, c in sorted(keys[k].items()):
        print(f"   {key}: {100*c/n:.0f}%")
    for key, c in sorted(nested[k].items()):
        print(f"   ~{key}: {100*c/n:.0f}%")
if a.ids_out:
    with open(a.ids_out, "w") as f:
        f.write("\n".join(sorted(ids)))
