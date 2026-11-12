"""T049 oracle: Python's live tail (`easy_personal.core.live`, read-only) over Python-made inputs.

Writes `golden-live/`:
- `items.json`: lines -> `live._item(record, claude_code.parse_record(record, identity))` (AC 1);
- `labels.json`: inputs (as JSON text: a lone surrogate survives) -> `live._safe`, `live._bash_label`, `live._call_label` (AC 2);
- `scripts.json`: scripted steps (file operations, a frozen `now`) run through `LiveTail.poll_once` over a temp
  `projects/` tree and a temp store; after every step the new buffered events and the snapshot (AC 4-7).

Python runs as shipped (it also stores the lines it reads in its temp store); the Rust engine is publish-only. The
scripts keep to cases where that cannot change an answer (see `rust/parity/README.md`, T049). Every temp folder is
made under `rust/target/` and removed. Never touches a real home: `EASY_PERSONAL_HOME` and
`EASY_PERSONAL_CLAUDE_PROJECTS` point at the temp folder.

Regenerate: `rust/parity/.venv/bin/python rust/parity/gen_live.py` (from the repo root).
"""

import json
import os
import shutil
import sys
import tempfile
from datetime import UTC, datetime, timedelta
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
sys.path.insert(0, str(REPO / "src"))

from easy_personal.core import live, prices, store  # noqa: E402
from easy_personal.sources import claude_code  # noqa: E402

OUT = HERE / "golden-live"
WORK = REPO / "rust" / "target"
T0 = datetime(2026, 1, 20, tzinfo=UTC)  # 1_768_867_200 s
KEYS = ("input_tokens", "output_tokens", "cache_creation_input_tokens", "cache_read_input_tokens")
PRICED = sorted(prices.load().models)[0]
CANARY = "CANARY-7f3a sk-ABCDEFGHIJKLMNOPQRSTUVWX /home/canary/secret.txt"


def ts(sec: float) -> str:
    return (T0 + timedelta(seconds=sec)).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def use(bid, name, inp=None):
    block = {"type": "tool_use", "name": name, "input": {"prompt": CANARY, "description": CANARY} if inp is None else inp}
    if bid is not None:
        block["id"] = bid
    return block


def asst(sid, sec, mid=None, blocks=(), usage=None, model=None, side=False, cwd="/work/demo"):
    msg = {"content": [{"type": "text", "text": CANARY}, {"type": "thinking", "thinking": CANARY}, *blocks]}
    if mid:
        msg["id"] = mid
    if usage:
        msg["usage"] = dict(zip(KEYS, usage, strict=True))
    if model:
        msg["model"] = model
    rec = {"type": "assistant", "sessionId": sid, "timestamp": ts(sec), "cwd": cwd, "version": "2.0.0", "message": msg}
    if side:
        rec["isSidechain"] = True
    return json.dumps(rec) + "\n"


def user(sid, sec, results=(), side=False, cwd="/work/demo"):
    blocks = [{"type": "tool_result", "tool_use_id": b, "is_error": e, "content": CANARY} for b, e in results]
    rec = {"type": "user", "sessionId": sid, "timestamp": ts(sec), "cwd": cwd, "version": "2.0.0",
           "message": {"role": "user", "content": blocks}}  # fmt: skip
    if side:
        rec["isSidechain"] = True
    return json.dumps(rec) + "\n"


# ---- AC 1: items ----------------------------------------------------------------------------------------------------


def item_lines() -> list[str]:
    base = {"sessionId": "s1", "timestamp": "2026-01-20T00:00:01Z", "cwd": "/work/demo", "version": "2.0.0"}
    raw = [
        asst("s1", 1, "m1", [use("t1", "Skill", {"skill": "code-review"}), use("t2", "Bash", {"command": "ls -la"})],
             usage=(10, 2, 3, 4), model=PRICED),  # fmt: skip
        asst("s1", 2, "m2", [use("t3", "mcp__srv__do_it"), use(None, "Read"), use("t4", "bad name"), "notadict", 7]),
        asst("s1", 3, None, [use("t5", "Edit")]),
        asst("s1", 4, "m3", [{"type": "tool_result", "tool_use_id": "x"}]),  # a result in an assistant record
        user("s1", 5, [("t1", True), ("t2", False), ("t" * 65, True)]),
        user("s1", 6, [("t9", "yes")]),
        json.dumps({**base, "type": "user", "message": {"content": [use("t6", "Read")]}}) + "\n",  # call in a user record
        json.dumps({**base, "type": "user"}) + "\n",  # no message
        json.dumps({**base, "type": "user", "message": {"content": "text " + CANARY}}) + "\n",  # content not a list
        json.dumps({**base, "type": "user", "message": "flat"}) + "\n",
        json.dumps({**base, "type": "assistant", "message": {"id": "m4", "content": {"type": "tool_use"}}}) + "\n",
        json.dumps({**base, "type": "assistant", "message": "no dict"}) + "\n",  # abstains
        json.dumps({**base, "type": "system", "subtype": "x", "content": CANARY}) + "\n",
        json.dumps({**base, "type": "attachment", "attachment": CANARY}) + "\n",
        json.dumps({"type": "file-history-snapshot", "sessionId": "s1", "timestamp": "2026-01-20T00:00:02Z"}) + "\n",
        json.dumps({"type": "summary", "summary": CANARY, "sessionId": "s1"}) + "\n",  # unknown type: abstains
        json.dumps({**base, "type": "assistant", "isSidechain": True, "message": {"id": "m5", "content": []}}) + "\n",
        json.dumps({**base, "type": "assistant", "timestamp": None, "message": {"id": "m6"}}) + "\n",  # no time
        json.dumps({k: v for k, v in base.items() if k != "sessionId"} | {"type": "user"}) + "\n",  # no session
        json.dumps({**base, "type": "assistant", "cwd": "", "message": {"id": "m7"}}) + "\n",
        json.dumps({**base, "type": "assistant", "cwd": "/", "message": {"id": "m8"}}) + "\n",
        json.dumps({**base, "type": "assistant", "cwd": "/a/b/..", "message": {"id": "m9"}}) + "\n",
        json.dumps({**base, "type": "assistant", "cwd": "rel/proj/", "message": {"id": "m10"}}) + "\n",
        json.dumps({**base, "type": "assistant", "message": {"id": "m11", "usage": {"input_tokens": None}}}) + "\n",
        json.dumps({**base, "type": "assistant", "isApiErrorMessage": True,
                    "message": {"id": "m12", "model": "nope", "usage": dict(zip(KEYS, (1, 2, 3, 4), strict=True))}})
        + "\n",  # fmt: skip
        json.dumps({**base, "timestamp": "2026-01-20T00:00:03", "type": "user"}) + "\n",  # no `Z`
        json.dumps([1, 2]) + "\n",
        "not json\n",
    ]
    return raw


def items_table() -> list[dict]:
    out = []
    for line in item_lines():
        try:
            record = json.loads(line)
        except ValueError:
            record = None
        parsed = claude_code.parse_record(record, lambda c: c)
        item = live._item(record, parsed) if isinstance(record, dict) else None
        if item and item["usage"] is not None:
            item["usage"] = list(item["usage"])
        out.append({"line": line, "item": item})
    return out


# ---- AC 2: labels ---------------------------------------------------------------------------------------------------


def labels_table() -> list[dict]:
    strings = ["a", "A1", "code-review", "kit:tdd", "x.y_z-1", "_lead", "-lead", ".lead", ":lead", "has space", "",
               "a\n", "naïve", "a" * 64, "a" * 65, "a/b", "a@b", "a+b", "sk-ABCDEF", "sk-", "x" * 32, "ab" * 16 + "!",
               "0123456789abcdef0123456789abcdef", "ghp_abc", "AKIAABCD1234", "-----BEGIN", "\udc80", "é" * 3]  # fmt: skip
    others = [None, 1, 1.5, True, [], {}, ["code-review"]]
    rows = [{"fn": "safe", "input": json.dumps(v), "out": live._safe(v)} for v in strings + others]
    commands = ["git status", "cd x && cmd", 'cd "a b";cmd', "cd 'a b' && cmd --x", "cd a && cd b && make test",
                "cd a; cd b; cd c ; npm run", "  cd   /x   &&   pytest -q", "cd && ls", "cd x &&", "cd x", "cdx && ls",
                "cd x | ls", "cd x & ls", 'cd "a b"x; ls', "cd /x && sk-ABCDEFGHIJKLMNOPQRSTUVWX deploy", "",
                "   ", "/usr/bin/python3 -m x", "VAR=1 cargo test", "sudo rm -rf /", "./run.sh", "echo " + CANARY,
                "a" * 4100, "cd " + "d" * 4090 + " && ls", "x" * 70, "(ls)", "ls -la", "cd　x && ls",
                "python -m easy_personal.cli ingest", "env A=1 B=2 make", "time ls", "\udc80 ls", "cd x\n&& ls",
                "'quoted cmd' arg"]  # fmt: skip
    rows += [{"fn": "bash", "input": json.dumps(c), "out": live._bash_label(c)} for c in commands + [None, 5, ["ls"]]]
    calls = [("Skill", {"skill": "code-review"}), ("Skill", {"skill": "kit:tdd"}), ("Skill", {"skill": 5}),
             ("Skill", {}), ("Skill", None), ("Skill", "code-review"), ("Skill", {"skill": "sk-ABCDEFGH"}),
             ("Agent", {"subagent_type": "Explore", "description": CANARY}), ("Task", {"subagent_type": "general"}),
             ("Agent", {"subagent_type": None}), ("Task", {"prompt": CANARY}), ("Bash", {"command": "cd x && make"}),
             ("Bash", {"command": 5}), ("Bash", {}), ("Read", {"file_path": CANARY}), ("Edit", {}),
             ("mcp__easy-memory__memory_search", {"query": CANARY}), ("mcp__srv__", {}), ("mcp____tool", {}),
             ("mcp__srv", {}), ("mcp__s v__t", {}), ("mcp__srv__tool__more", {}), ("mcp__" + "s" * 65 + "__t", {}),
             ("mcp__srv__sk-ABCDEFGH", {}), ("WebFetch", {"url": "https://x"}), ("Unknown", {"skill": "x"}),
             ("skill", {"skill": "x"}), ("Bash", {"command": "cd x && git push", "description": CANARY})]  # fmt: skip
    rows += [{"fn": "call", "name": n, "input": json.dumps(i), "out": live._call_label(n, i)} for n, i in calls]
    return rows


# ---- AC 4-7: scripts ------------------------------------------------------------------------------------------------


def apply(op: dict, root: Path, conn) -> None:
    path = root / op["path"] if "path" in op else root
    data = bytes.fromhex(op["hex"]) if "hex" in op else op.get("text", "").encode("utf-8", "surrogatepass")
    kind = op["op"]
    if kind in ("write", "append"):
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "wb" if kind == "write" else "ab") as handle:
            handle.write(data)
    elif kind == "replace":
        tmp = path.with_name(path.name + ".tmp")
        tmp.write_bytes(data)
        os.replace(tmp, path)
    elif kind == "delete":
        path.unlink()
    elif kind == "mtime":
        stamp = T0.timestamp() + op["secs"]
        os.utime(path, (stamp, stamp))
    elif kind == "cursor":
        store.save_file(conn, claude_code.SOURCE, str(path), path.stat().st_ino, op["offset"], [], None)
    elif kind == "symlink":
        path.parent.mkdir(parents=True, exist_ok=True)
        os.symlink(root / op["target"], path)
    elif kind == "account":  # T049 fix round: an `agent_accounts` row; `folder` is relative to the work folder
        conn.execute(
            "INSERT INTO agent_accounts (id, provider, folder, label, status, config_dir_env) VALUES (?, ?, ?, ?, ?, ?)",
            (op["id"], "claude", str(root.parent / op["folder"]), op.get("label"), op["status"], op.get("env")),
        )
        conn.commit()
    else:
        raise ValueError(kind)


def run_script(name: str, steps: list[dict]) -> dict:
    work = Path(tempfile.mkdtemp(dir=WORK, prefix="gen-live-"))
    try:
        os.environ["EASY_PERSONAL_HOME"] = str(work / "home")
        os.environ["EASY_PERSONAL_CLAUDE_PROJECTS"] = str(work / "projects")
        root = work / "projects"
        root.mkdir()
        clock = [T0]
        tail = live.LiveTail(clock=lambda: clock[0])
        conn = store.connect(work / "store.sqlite")
        out, last = [], 0
        for step in steps:
            for op in step.get("ops", []):
                apply(op, root, conn)
            clock[0] = T0 + timedelta(seconds=step["now"])
            tail.poll_once(conn)
            events = tail.events_since(last)
            rec = {"now": step["now"], "ops": step.get("ops", []), "snapshot": tail.snapshot(clock[0])}
            if step.get("summary"):
                rec["count"] = len(events)
            else:
                rec["events"] = events
            if "probes" in step:
                rec["probes"] = []
                for k in step["probes"]:
                    got = tail.events_since(k)
                    rec["probes"].append([k, len(got), got[0]["id"] if got else None, got[-1]["id"] if got else None,
                                          tail.rolled_past(k)])  # fmt: skip
            last = tail.last_id
            out.append(rec)
        conn.close()
        return {"name": name, "steps": out}
    finally:
        shutil.rmtree(work)


def scripts() -> list[dict]:
    a, s = "p/a.jsonl", "p/s.jsonl"
    pairing = [
        {"now": 0, "ops": []},
        {"now": 10, "ops": [{"op": "append", "path": s, "text": asst("s1", 1, "m1", [
            use("t1", "Read", {"file_path": CANARY}), use("t2", "Edit"), use("t2", "Edit")], usage=(100, 10, 0, 0),
            model=PRICED)}]},  # fmt: skip
        {"now": 20, "ops": [{"op": "append", "path": s, "text": user("s1", 12, [("t1", False), ("t9", False)])}]},
        {"now": 621},
        {"now": 700, "ops": [{"op": "append", "path": s, "text": asst("s1", 690, "m2", [
            use("t3", "Bash", {"command": "cd /x && git status " + CANARY})])}]},  # fmt: skip
        {"now": 700 + 25 * 3600},
        {"now": 700 + 25 * 3600 + 5, "ops": [{"op": "append", "path": s, "text": user("s1", 700 + 25 * 3600,
                                                                                  [("t3", False)])}]},  # fmt: skip
    ]
    failing = [
        {"now": 0},
        {"now": 5, "ops": [{"op": "append", "path": s, "text": asst("s2", 1, "m1", [
            use("b1", "Bash", {"command": "make"}), use("b2", "Bash", {"command": "make"}), use("b3", "Read"),
            use("b4", "Bash", {"command": "make"})])}]},  # fmt: skip
        {"now": 6, "ops": [{"op": "append", "path": s, "text": user("s2", 2, [("b1", True)])}]},
        {"now": 7, "ops": [{"op": "append", "path": s, "text": user("s2", 3, [("b2", True)])}]},
        {"now": 8, "ops": [{"op": "append", "path": s, "text": user("s2", 4, [("b3", True)])}]},
        {"now": 9, "ops": [{"op": "append", "path": s, "text": user("s2", 5, [("b4", False)])}]},
        {"now": 10, "ops": [{"op": "append", "path": s, "text": user("s2", 6, [("zz", True), ("zy", True)])
                             + user("s2", 7, [("zx", True)])}]},  # fmt: skip
    ]
    waiting = [
        {"now": 0},
        {"now": 5, "ops": [{"op": "append", "path": s, "text": asst("s3", 1, "m1", [
            use("q1", "AskUserQuestion"), use("e1", "Edit"), use("x1", "ExitPlanMode")])}]},  # fmt: skip
        {"now": 6, "ops": [{"op": "append", "path": s, "text": user("s3", 2, [("q1", False)])}]},
        {"now": 2 + 11 * 60},
        {"now": 2 + 11 * 60 + 1, "ops": [{"op": "append", "path": s, "text": user("s3", 700, [("x1", False)])}]},
    ]

    def skill(sec, mid, name):
        return asst("s4", sec, mid, [use("k" + mid, "Skill", {"skill": name} if name is not None else {})])

    stage = [{"now": 0}] + [
        {"now": 100 + i, "ops": [{"op": "append", "path": s, "text": line}]}
        for i, line in enumerate([
            skill(10, "m1", "brainstorming"), skill(20, "m2", "kit:tdd"), skill(30, "m3", "unknown-skill"),
            skill(5, "m4", "code-review"), skill(40, "m5", "tdd"), skill(41, "m6", None), skill(42, "m7", 7),
            skill(43, "m8", "plugin:x:verify"), skill(44, "m9", "sk-ABCDEFGHIJKL"), skill(45, "m10", "ideate"),
            skill(46, "m11", "git-guardrails-claude-code") + skill(46, "m12", "craft-agent")
            + skill(47, "m13", "to-issues") + skill(48, "m14", "html-report") + skill(49, "m15", "ship"),
        ])
    ]  # fmt: skip
    sub = "p/s5/subagents/agent-"
    streams = [
        {"now": 0},
        {"now": 5, "ops": [{"op": "append", "path": "p/s5.jsonl", "text": asst("s5", 1, "m1", [
            use("g1", "Agent", {"subagent_type": "Explore", "description": CANARY, "prompt": CANARY})])}]},  # fmt: skip
        {"now": 6, "ops": [
            {"op": "write", "path": sub + "a1.meta.json",
             "text": json.dumps({"agentType": "Explore", "description": CANARY})},
            {"op": "append", "path": sub + "a1.jsonl", "text": asst("s5", 2, "n1", [use("h1", "Read")], side=True)}]},
        {"now": 7, "ops": [{"op": "append", "path": sub + "a1.jsonl", "text": user("s5", 3, [("h1", True)], side=True)
                            + user("s5", 3.5, [("h2", True)], side=True) + user("s5", 3.6, [("h3", True)], side=True)
                            }]},  # fmt: skip
        {"now": 8, "ops": [{"op": "append", "path": sub + "a2.jsonl", "text": asst("s5", 4, "n2", [use("h4", "Grep")],
                                                                                   side=True)}]},  # fmt: skip
        {"now": 20, "ops": [{"op": "append", "path": sub + "a2.jsonl", "text": user("s5", 5, [("h4", False)])}]},
        {"now": 39, "ops": [{"op": "append", "path": sub + "a2.jsonl", "text": user("s5", 6, [("h5", False)])}]},
        {"now": 40, "ops": [
            {"op": "write", "path": sub + "a2.meta.json", "text": json.dumps({"agentType": "Late"})},
            {"op": "append", "path": sub + "a2.jsonl", "text": user("s5", 7, [("h6", False)])}]},
        {"now": 41, "ops": [
            {"op": "write", "path": "p/real.meta.json", "text": json.dumps({"agentType": "Linked"})},
            {"op": "symlink", "path": sub + "a3.meta.json", "target": "p/real.meta.json"},
            {"op": "append", "path": sub + "a3.jsonl", "text": user("s5", 8, [("h7", False)])}]},
        {"now": 42, "ops": [
            {"op": "write", "path": sub + "a4.meta.json", "text": json.dumps({"agentType": "Big", "pad": "x" * 5000})},
            {"op": "append", "path": sub + "a4.jsonl", "text": user("s5", 9, [("h8", False)])}]},
        {"now": 43, "ops": [
            {"op": "write", "path": sub + "a5.meta.json", "text": "{not json"},
            {"op": "append", "path": sub + "a5.jsonl", "text": user("s5", 10, [("h9", False)])}]},
        {"now": 44, "ops": [
            {"op": "write", "path": sub + "a6.meta.json", "text": json.dumps({"agentType": "bad label"})},
            {"op": "append", "path": sub + "a6.jsonl", "text": user("s5", 11, [("ha", False)])}]},
        {"now": 45, "ops": [
            {"op": "write", "path": sub + "a7.meta.json", "text": json.dumps(["Explore"])},
            {"op": "append", "path": sub + "a7.jsonl", "text": user("s5", 12, [("hb", False)])}]},
    ]
    usage = [
        {"now": 0},
        {"now": 5, "ops": [{"op": "append", "path": s, "text": asst("s6", 1, "m1", usage=(100, 10, 5, 0), model=PRICED)}]},
        {"now": 6, "ops": [{"op": "append", "path": s, "text": asst("s6", 2, "m1", usage=(100, 10, 5, 0), model=PRICED)}]},
        {"now": 7, "ops": [{"op": "append", "path": s, "text": asst("s6", 3, "m1", usage=(150, 8, 5, 1), model=PRICED)}]},
        {"now": 8, "ops": [{"op": "append", "path": s, "text": asst("s6", 4, "m2", usage=(1, 1, 0, 0), model="nope")}]},
        {"now": 9, "ops": [{"op": "append", "path": s, "text": asst("s6", 5, "m2", usage=(2, 1, 0, 0), model="nope")}]},
        {"now": 10, "ops": [{"op": "append", "path": "p/t.jsonl", "text": asst("s7", 6, "m9", [use("u1", "Read")])}]},
    ]
    many = "".join(user("s8", 100 + i * 0.001, [("r%d" % i, False)]) for i in range(1203))
    lines3 = asst("sa", -100, "m1", [use("f1", "Read")]) + asst("sa", -90, "m2", [use("f2", "Read")])
    third = asst("sa", -80, "m3", [use("f3", "Skill", {"skill": "verify"})])
    odd = ("\r\n".join([asst("sa", 3, "m5")[:-1], ""]) + "\n   \n" + "{bad json\n" + "\t\x0b\x0c\n"
           + json.dumps({"type": "file-history-snapshot", "sessionId": "sa", "timestamp": ts(4)}) + "\n")  # fmt: skip
    files = [
        {"now": 0, "ops": [{"op": "write", "path": a, "text": lines3 + third},
                           {"op": "cursor", "path": a, "offset": len(lines3.encode())},
                           {"op": "mtime", "path": a, "secs": -7200}]},
        {"now": 1, "ops": [{"op": "append", "path": a, "text": asst("sa", 1, "m4", [use("f4", "Glob")])[:40]}]},
        {"now": 2, "ops": [{"op": "append", "path": a, "text": asst("sa", 1, "m4", [use("f4", "Glob")])[40:]}]},
        {"now": 3, "ops": [{"op": "append", "path": a, "text": odd},
                           {"op": "append", "path": a, "hex": "ff fe 7b 0a 00 7b 7d 0a".replace(" ", "")}]},
        {"now": 4, "ops": [{"op": "write", "path": "p/c.jsonl", "text": user("sc", 5, [("c1", True)]) * 3},
                           {"op": "cursor", "path": "p/c.jsonl", "offset": len(user("sc", 5, [("c1", True)]) * 3)}]},
        {"now": 5, "ops": [{"op": "write", "path": "p/c.jsonl", "text": user("sc", 6, [("c2", False)])}]},
        {"now": 6, "ops": [{"op": "replace", "path": "p/c.jsonl", "text": user("sc", 7, [("c3", True)]) * 2}]},
        {"now": 7, "ops": [{"op": "delete", "path": a}]},
        {"now": 8, "ops": [{"op": "write", "path": a, "text": user("sa", 8, [("f9", False)])}]},
        {"now": 200, "ops": [{"op": "append", "path": "p/d.jsonl", "text": many}], "summary": True},
    ]
    w1 = (asst("w1", -1800, "m1", [use("k1", "Skill", {"skill": "tdd"})], usage=(50, 5, 0, 0), model=PRICED)
          + asst("w1", -1700, "m2", [use("e1", "Edit"), use("b1", "Bash", {"command": "make"})])
          + user("w1", -1690, [("b1", True)]) + user("w1", -1680, [("b9", True)]) + user("w1", -1670, [("b8", True)]))
    w1_last = user("w1", -60, [("e1", False)])
    warm = [
        {"now": 0, "ops": [{"op": "write", "path": "p/w1.jsonl", "text": w1 + w1_last},
                           {"op": "cursor", "path": "p/w1.jsonl", "offset": len(w1.encode())},
                           {"op": "mtime", "path": "p/w1.jsonl", "secs": -60},
                           {"op": "write", "path": "p/w2.jsonl", "text": asst("w2", -7300, "m1", [use("z", "Skill",
                                                                            {"skill": "ship"})])},  # fmt: skip
                           {"op": "cursor", "path": "p/w2.jsonl", "offset": 10},
                           {"op": "mtime", "path": "p/w2.jsonl", "secs": -7200}]},
        {"now": 10},
        {"now": 700},
    ]
    buffer = [
        {"now": 0},
        {"now": 300, "ops": [{"op": "append", "path": s, "text": "".join(
            user("s9", 250 + i * 0.001, [("y%d" % i, False)]) for i in range(2001))}],
         "summary": True, "probes": [0, 1, 2, 3, 1999, 2000, 2001, 2002, 5000]},  # fmt: skip
        {"now": 301, "ops": [{"op": "append", "path": s, "text": user("s9", 301, [("y", False)])}],
         "probes": [0, 1, 2, 2001, 2002]},
    ]
    idle = [
        {"now": 0, "ops": [{"op": "append", "path": s, "text": user("s10", -10, [("i1", False)])
                            + user("s11", -20, [("i2", False)])}]},  # fmt: skip
        {"now": 3590},
        {"now": 3592},
        {"now": 3500, "ops": [{"op": "append", "path": s, "text": user("s11", 999999, [("i3", False)])
                               + json.dumps({"type": "user", "sessionId": "s12", "cwd": "/w"}) + "\n"}]},
        {"now": 3700},
    ]
    named = [("pairing", pairing), ("failing", failing), ("waiting", waiting), ("stage", stage),
             ("streams", streams), ("usage", usage), ("files", files), ("warm", warm), ("buffer", buffer),
             ("idle", idle)]  # fmt: skip
    return [run_script(n, st) for n, st in named]


def scripts_2() -> list[dict]:
    """T049 fix round: scripts added after review, kept in their own file so `scripts.json` stays byte-identical."""
    accounts = [
        {"now": 0, "ops": [{"op": "account", "id": 1, "label": "Work", "status": "confirmed", "folder": "acct", "env": "/e"},
                           {"op": "account", "id": 2, "status": "detected", "folder": "home/.claude"}]},
        {"now": 5, "ops": [{"op": "append", "path": "../acct/projects/p/w.jsonl", "text": user("sw", 1, [("w1", True)])}]},
        {"now": 6, "ops": [{"op": "append", "path": "p/d.jsonl", "text": user("sd", 2, [("d1", True)])}]},
    ]  # fmt: skip
    named = [("accounts", accounts)]
    return [run_script(n, st) for n, st in named]


def main() -> None:
    WORK.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(exist_ok=True)
    data = {"items.json": {"source": claude_code.SOURCE, "items": items_table()},
            "labels.json": labels_table(),
            "scripts.json": {"t0": int(T0.timestamp()), "priced": PRICED, "canary": CANARY, "scripts": scripts()},
            "scripts-2.json": {"t0": int(T0.timestamp()), "canary": CANARY, "scripts": scripts_2()}}  # fmt: skip
    for name, value in data.items():
        (OUT / name).write_text(json.dumps(value, indent=1, sort_keys=True) + "\n")
    print({k: len(v) if isinstance(v, list) else len(v.get("items", v.get("scripts", []))) for k, v in data.items()})


if __name__ == "__main__":
    main()
