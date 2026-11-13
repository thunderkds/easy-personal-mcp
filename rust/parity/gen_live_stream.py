"""T056 oracle: the message sequence of Python's `GET /api/live` (`surfaces/api.py` `live_stream`, read-only).

Writes `golden-live-stream/cases.json`. Each case is a script of steps over a temp `projects/` tree and a temp store
(the file operations and line builders of `gen_live.py`). A step applies its file operations, sets the frozen clock,
runs one `LiveTail.poll_once`, then lets the stream run exactly one pass of its loop and records what it sent:
`[event, id, data]` per message (a `: ping` comment cannot occur: a pass takes far less than `PING_S`). The stream
opens at step `open_at` with the case's `Last-Event-ID` (`resume`, a header text or null); `tail: false` means no
tail runs (`live.current()` is None). The stream's `asyncio.sleep` is replaced by a wait the driver releases, so a pass
boundary is exact; nothing else of `live_stream` is changed.

Python runs as shipped (its tail also stores what it reads); the Rust tail is publish-only. The cases keep to inputs
where that cannot change an answer. Every temp folder is made under `rust/target/` and removed. Never touches a real
home: `EASY_PERSONAL_HOME` and `EASY_PERSONAL_CLAUDE_PROJECTS` point at the temp folder.

Regenerate: `rust/parity/.venv/bin/python rust/parity/gen_live_stream.py` (from the repo root), or
`uv run python rust/parity/gen_live_stream.py`.
"""

import asyncio
import json
import os
import shutil
import sys
import tempfile
from datetime import timedelta
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from gen_live import CANARY, PRICED, REPO, T0, WORK, apply, asst, use, user  # noqa: E402

sys.path.insert(0, str(REPO / "src"))

from easy_personal.core import live, store  # noqa: E402
from easy_personal.surfaces import api  # noqa: E402

OUT = HERE / "golden-live-stream"


class _Headers(dict):
    pass


class _Request:
    """What `live_stream` reads from a request: headers, `is_disconnected`, `app.state` (no `should_stop`)."""

    def __init__(self, resume):
        self.headers = _Headers({} if resume is None else {"last-event-id": resume})
        self.app = type("App", (), {"state": type("State", (), {})()})()

    async def is_disconnected(self):
        return False


def _parse(chunks: list[bytes]) -> list[list]:
    out = []
    for block in b"".join(chunks).decode().split("\n\n"):
        if not block or block.startswith(":"):
            continue
        event, event_id, data = None, None, None
        for line in block.split("\n"):
            key, _, value = line.partition(": ")
            if key == "event":
                event = value
            elif key == "id":
                event_id = int(value)
            elif key == "data":
                data = json.loads(value)
            else:
                raise ValueError(line)
        out.append([event, event_id, data])
    return out


async def _run_case(case: dict) -> dict:
    work = Path(tempfile.mkdtemp(dir=WORK, prefix="gen-live-stream-"))
    try:
        os.environ["EASY_PERSONAL_HOME"] = str(work / "home")
        os.environ["EASY_PERSONAL_CLAUDE_PROJECTS"] = str(work / "projects")
        root = work / "projects"
        root.mkdir()
        clock = [T0]
        tail = live.LiveTail(clock=lambda: clock[0])
        live._current = tail if case.get("tail", True) else None
        conn = store.connect(work / "store.sqlite")
        released, paused = asyncio.Event(), asyncio.Event()

        async def fake_sleep(_seconds):
            paused.set()
            await released.wait()
            released.clear()

        api.asyncio.sleep = fake_sleep
        chunks: list[bytes] = []
        task = None
        out = []
        for index, step in enumerate(case["steps"]):
            for op in step.get("ops", []):
                apply(op, root, conn)
            clock[0] = T0 + timedelta(seconds=step["now"])
            tail.poll_once(conn)
            if index == case["open_at"]:
                body = (await api.live_stream(_Request(case.get("resume")))).body_iterator

                async def pump(body=body):
                    async for chunk in body:
                        chunks.append(chunk if isinstance(chunk, bytes) else chunk.encode())

                task = asyncio.ensure_future(pump())
            elif task is not None:
                released.set()  # the next pass
            if task is not None:
                await paused.wait()
                paused.clear()
            out.append({"now": step["now"], "ops": step.get("ops", []), "messages": _parse(chunks)})
            chunks.clear()
        if task is not None:
            task.cancel()
        conn.close()
        return {"name": case["name"], "open_at": case["open_at"], "resume": case.get("resume"),
                "tail": case.get("tail", True), "steps": out}  # fmt: skip
    finally:
        live._current = None
        shutil.rmtree(work)


def cases() -> list[dict]:
    s = "p/s.jsonl"
    sub = "p/s4/subagents/agent-"
    three = (asst("s1", 1, "m1", [use("t1", "Read", {"file_path": CANARY})], usage=(100, 10, 0, 0), model=PRICED)
             + asst("s1", 2, "m2", [use("t2", "Grep", {"pattern": CANARY})])
             + asst("s1", 3, "m3", [use("t3", "Glob", {"pattern": CANARY})]))  # fmt: skip
    many = "".join(asst("s6", 1 + i / 1000, f"m{i}", [use(f"u{i}", "Read", {})]) for i in range(2005))
    return [
        {"name": "empty-no-tail", "tail": False, "resume": "3", "open_at": 0, "steps": [{"now": 0}, {"now": 1}]},
        {"name": "empty-tail", "open_at": 0, "steps": [{"now": 0}, {"now": 1}, {"now": 2}]},
        {"name": "new-session", "open_at": 1, "steps": [
            {"now": 0},
            {"now": 1},
            {"now": 5, "ops": [{"op": "append", "path": s, "text": asst("s1", 4, "m1", [], usage=(100, 10, 0, 0),
                                                                         model=PRICED)}]},  # fmt: skip
            {"now": 6},
        ]},
        {"name": "call-and-result", "open_at": 1, "steps": [
            {"now": 0},
            {"now": 1},
            {"now": 5, "ops": [{"op": "append", "path": s, "text": asst("s2", 4, "m1", [
                use("c1", "Bash", {"command": "git status " + CANARY}), use("c2", "Skill", {"skill": "tdd"})],
                usage=(50, 5, 0, 0), model=PRICED)}]},  # fmt: skip
            {"now": 6, "ops": [{"op": "append", "path": s, "text": user("s2", 5, [("c1", False), ("c2", False)])}]},
        ]},
        {"name": "error-streak", "open_at": 1, "steps": [
            {"now": 0},
            {"now": 1},
            {"now": 5, "ops": [{"op": "append", "path": s, "text": asst("s3", 1, "m1", [
                use("b1", "Bash", {"command": "make"}), use("b2", "Bash", {"command": "make"}),
                use("b3", "Bash", {"command": "make"})])}]},  # fmt: skip
            {"now": 6, "ops": [{"op": "append", "path": s, "text": user("s3", 2, [("b1", True)])
                                + user("s3", 3, [("b2", True)])}]},  # fmt: skip
            {"now": 7, "ops": [{"op": "append", "path": s, "text": user("s3", 4, [("b3", True)])}]},
            {"now": 8},
        ]},
        {"name": "sub-agent", "open_at": 1, "steps": [
            {"now": 0},
            {"now": 1},
            {"now": 5, "ops": [
                {"op": "append", "path": "p/s4.jsonl", "text": asst("s4", 1, "m1", [
                    use("a1", "Agent", {"subagent_type": "Explore", "prompt": CANARY})])},  # fmt: skip
                {"op": "write", "path": sub + "x1.meta.json",
                 "text": json.dumps({"agentType": "Explore", "description": CANARY})},
                {"op": "append", "path": sub + "x1.jsonl",
                 "text": asst("s4", 2, "m2", [use("r1", "Read", {"file_path": CANARY})], side=True)},
            ]},
            {"now": 6, "ops": [{"op": "append", "path": sub + "x1.jsonl", "text": user("s4", 3, [("r1", True)], side=True)}]},
        ]},
        {"name": "resume-good", "resume": "1", "open_at": 1, "steps": [
            {"now": 0},
            {"now": 5, "ops": [{"op": "append", "path": s, "text": three}]},
            {"now": 6, "ops": [{"op": "append", "path": s, "text": user("s1", 4, [("t1", False)])}]},
            {"now": 7},
        ]},
        {"name": "resume-above-last-id", "resume": "99", "open_at": 1, "steps": [
            {"now": 0},
            {"now": 5, "ops": [{"op": "append", "path": s, "text": three}]},
            {"now": 6},
        ]},
        {"name": "resume-older-than-buffer", "resume": "1", "open_at": 1, "steps": [
            {"now": 0},
            {"now": 5, "ops": [{"op": "append", "path": s, "text": many}]},
            {"now": 6},
        ]},
        {"name": "resume-not-a-number", "resume": " x1 ", "open_at": 1, "steps": [
            {"now": 0},
            {"now": 5, "ops": [{"op": "append", "path": s, "text": three}]},
            {"now": 6},
        ]},
    ]


def main() -> None:
    WORK.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(exist_ok=True)
    data = {"t0": int(T0.timestamp()), "canary": CANARY, "cases": [asyncio.run(_run_case(c)) for c in cases()]}
    (OUT / "cases.json").write_text(json.dumps(data, indent=1, sort_keys=True) + "\n")
    print({c["name"]: sum(len(s["messages"]) for s in c["steps"]) for c in data["cases"]})


if __name__ == "__main__":
    main()
