#!/usr/bin/env python3
"""Writes `rust/parity/fixtures-rich/` (T007): the invented transcripts, kernel traces and kit-project files of the
rich oracle. The output is committed; `gen_golden.py --rich` only reads it. Run it again only to change the data:

    python3 rust/parity/make_fixtures_rich.py        (stdlib only; rewrites the whole folder)

Everything is fictional. The clock is the generator's frozen `now`, 2026-01-20T00:00:00Z. Two agent accounts:
`claude/` (the default account, `~/.claude` in the real world) and `claude-work/` (`~/.claude-work`).
A `cwd` of `@KIT@/<name>` is a kit project folder: `gen_golden.py --rich` copies `kit/<name>` into its work folder and
replaces `@KIT@` with that folder when it copies the transcripts. Any other `cwd` is a folder that does not exist
(`/work/rich/...`), and a session with no `cwd` has no project (it is listed as `(unknown)`).

What each group of sessions is for (the rule table is in README.md):
  K  24 short sessions on 2026-01-02..05 (outside both rule windows): half call an easy-ui tool, so `kit_findings`
     has 12 sessions on each side in one stratum (project alpha, one model, one tool-call band).
  P  previous week (2026-01-06..12), account A: small and low cache reuse.
  A  this week (2026-01-13..19), account A: low cache reuse, huge contexts, tool errors, unreadable records.
  B  account B, one template used in both weeks (offset 0 and 7 days): high cache reuse, small contexts, no errors,
     no unreadable records, an even split between a folder project and a no-project session.
"""

import json
import shutil
import sys
from datetime import datetime, timedelta
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "fixtures-rich"
OPUS, SONNET, UNPRICED = "claude-opus-5-5", "claude-sonnet-5-5", "claude-fixture-model-1"
ALPHA, BETA = "@KIT@/alpha", "@KIT@/beta"
GONE_A, GONE_B, GONE_C = "/work/rich/gone-a", "/work/rich/gone-b", "/work/rich/gone-c"
VERSION_A, VERSION_B = "2.1.330", "2.1.331"
DAY0 = datetime(2026, 1, 1)


def stamp(day: int, hour: int = 9, minute: int = 0, second: int = 0) -> str:
    moment = DAY0 + timedelta(days=day - 1, hours=hour, minutes=minute, seconds=second)
    return moment.strftime("%Y-%m-%dT%H:%M:%S.000Z")


class Session:
    """One transcript file: assistant messages (each may be followed by the user record holding its tool results)."""

    def __init__(self, number: int, account: str, cwd: str | None, model: str, version: str, entry: str = "cli", branch: str = "main"):
        self.number, self.account, self.cwd, self.model, self.version = number, account, cwd, model, version
        self.entry, self.branch = entry, branch
        self.id = f"00000000-0000-4000-8000-{number:012d}"
        self.lines: list[dict] = []
        self.side: list[dict] = []  # records of the sub-agent file
        self.count = 0

    def _base(self, kind: str, ts: str, sidechain: bool = False) -> dict:
        self.count += 1
        record = {
            "parentUuid": None,
            "isSidechain": sidechain,
            "userType": "external",
            "entrypoint": self.entry,
        }
        if self.cwd:
            record["cwd"] = self.cwd
        record.update(
            sessionId=self.id,
            version=self.version,
            gitBranch=self.branch,
            uuid=f"{self.number:08d}-0000-4000-8000-{self.count:012d}",
            timestamp=ts,
            type=kind,
        )
        return record

    def message(self, ts, ctx, out, cache=0.1, tools=(), errors=None, msg_no=None, model=None, side=False, dup_of=None):
        """An assistant record with a prompt of `ctx` tokens (`cache` of it read from cache) and `out` output tokens.

        `errors` (a count) adds the user record, one second later, whose tool results hold that many failed ones.
        `dup_of` writes a second record of an earlier message (same message id, as a streamed message has).
        """
        number = msg_no if msg_no is not None else (dup_of if dup_of is not None else self.count + 1)
        cache_read = int(ctx * cache)
        usage = {"input_tokens": 10, "output_tokens": out, "cache_read_input_tokens": cache_read, "cache_creation_input_tokens": ctx - 10 - cache_read}
        blocks = [{"type": "text", "text": "INVENTED_REPLY_TEXT"}]
        blocks += [{"type": "tool_use", "id": f"toolu_{self.number}_{number}_{i}", "name": name, "input": {"file_path": "INVENTED_PATH"}} for i, name in enumerate(tools)]
        record = self._base("assistant", ts, side)
        record["requestId"] = f"req_{self.number}_{number}"
        record["message"] = {
            "id": f"msg_{self.number}_{number}",
            "type": "message",
            "role": "assistant",
            "model": model or self.model,
            "content": blocks,
            "stop_reason": None,
            "stop_sequence": None,
            "usage": usage,
        }
        (self.side if side else self.lines).append(record)
        if errors is not None:
            user = self._base("user", _plus_one(ts), side)
            user["message"] = {
                "role": "user",
                "content": [
                    {"type": "tool_result", "tool_use_id": f"toolu_{self.number}_{number}_{i}", "content": "INVENTED_RESULT", "is_error": i < errors}
                    for i in range(max(len(tools), errors))
                ],
            }
            (self.side if side else self.lines).append(user)

    def unknown(self, ts, kind):
        """A record of a type the reader does not know: it is stored as abstained and counted as unreadable."""
        self.lines.append({"type": kind, "version": self.version, "timestamp": ts, "sessionId": self.id, "payload": "INVENTED_PAYLOAD"})


def _plus_one(ts: str) -> str:
    moment = datetime.strptime(ts, "%Y-%m-%dT%H:%M:%S.000Z") + timedelta(seconds=1)
    return moment.strftime("%Y-%m-%dT%H:%M:%S.000Z")


def build() -> list[Session]:
    sessions: list[Session] = []
    number = [100]

    def new(account, cwd, model, version, **kw) -> Session:
        number[0] += 1
        sessions.append(Session(number[0], account, cwd, model, version, **kw))
        return sessions[-1]

    # K: kit-finding sessions, account A, project alpha, one model, two tool calls each (one band), 12 with easy-ui
    for k in range(24):
        s = new("A", ALPHA, SONNET, VERSION_A, branch="feature-" + ("ui" if k % 2 == 0 else "plain"))
        with_ui = k % 2 == 0
        s.message(
            stamp(2 + k // 8, 9 + k % 8, 10 + k % 3),
            ctx=(7000 if with_ui else 10000) + 100 * (k % 3),
            out=800 if with_ui else 1000,
            cache=0.8,
            tools=("Read", "mcp__easy-ui__check") if with_ui else ("Read", "Edit"),
            errors=0,
        )

    # P: previous week, account A (small, cache reuse 0.5)
    for day, cwd, model in ((7, GONE_A, SONNET), (8, ALPHA, SONNET), (9, GONE_B, OPUS), (11, GONE_A, OPUS)):
        s = new("A", cwd, model, VERSION_A)
        for m, ctx in enumerate((20000, 30000, 40000)):
            s.message(stamp(day, 10, 5 * m), ctx=ctx, out=800, cache=0.5, tools=("Read",), errors=0)

    # A: this week, account A
    def huge(cwd, model, series, day, tools=("Read", "Bash"), errors=1, branch="main", side_series=()):
        s = new("A", cwd, model, VERSION_A, branch=branch)
        for m, ctx in enumerate(series):
            s.message(stamp(day, 9 + m, 3 * m), ctx=ctx, out=2000, cache=0.1, tools=tools, errors=errors if m % 2 == 0 else 0)
        for m, ctx in enumerate(side_series):
            s.message(stamp(day, 9, 20 + m), ctx=ctx, out=300, cache=0.2, tools=("Grep",), errors=0, side=True, msg_no=900 + m)
        return s

    a1 = huge(GONE_A, OPUS, (60000, 110000, 160000, 205000), 14, side_series=(35000, 40000))
    a2 = huge(GONE_A, OPUS, (50000, 120000, 190000), 15, tools=("Read", "mcp__easy-context__search"))
    huge(ALPHA, SONNET, (40000, 100000, 180000), 15, branch="feature-ui")
    huge(GONE_A, SONNET, (30000, 90000, 172000), 16)
    a5 = huge(GONE_B, OPUS, (50000, 100000, 160000), 16)
    huge(GONE_A, OPUS, (40000, 100000, 155000), 17)
    huge(ALPHA, SONNET, (50000, 120000, 175000), 17, branch="feature-plain")
    huge(GONE_A, SONNET, (60000, 130000, 171000), 18)
    # a session that compacts: the context falls below half of its peak
    a6 = new("A", GONE_A, SONNET, VERSION_A)
    for m, ctx in enumerate((70000, 150000, 60000, 80000)):
        a6.message(stamp(18, 9 + m, 0), ctx=ctx, out=1500, cache=0.1, tools=("Read",), errors=0)
    # a tool-heavy session in a folder that no workspace holds: 3 messages x 30 calls = 90 (an outlier by tool calls)
    a7 = new("A", GONE_C, SONNET, VERSION_A)
    for m in range(3):
        a7.message(stamp(19, 9, 10 * m), ctx=20000, out=1200, cache=0.1, tools=("Read",) * 30, errors=2)
    # a headless session with no cwd: no project, agent work, and a model the price table does not know
    a8 = new("A", None, UNPRICED, VERSION_A, entry="sdk-cli")
    for m in range(2):
        a8.message(stamp(19, 15, 10 * m), ctx=15000, out=600, cache=0.1, tools=("WebSearch",), errors=0)
    # straddles the window start (2026-01-13T00:00): message 2 has two records, the first before and the second after it
    a9 = new("A", GONE_A, OPUS, VERSION_A)
    a9.message(stamp(12, 23, 30), ctx=40000, out=900, cache=0.1, tools=("Read",), errors=0)
    a9.message(stamp(12, 23, 59, 59), ctx=45000, out=500, cache=0.1, tools=("Edit",), msg_no=2)
    a9.message(stamp(13, 0, 0, 30), ctx=45000, out=700, cache=0.1, tools=("Edit",), dup_of=2)  # same message id, later
    a9.message(stamp(13, 1, 30), ctx=50000, out=900, cache=0.1, tools=("Read",), errors=0)  # 90 minutes later: a timeline gap
    # records the reader cannot interpret (version 2.1.330, account A only): 4 of about 90 records
    a1.unknown(stamp(14, 9, 40), "future-widget-qqq")
    a2.unknown(stamp(15, 9, 40), "future-widget-qqq")
    a5.unknown(stamp(16, 9, 40), "future-widget-qqq")
    a5.unknown(stamp(16, 9, 41), "another-new-thing")

    # B: one template, two weeks (offset 0 and 7 days): equal volume, so tokens_jump measures 0 for this account
    for offset in (0, 7):
        for day, cwd, model, ctx, entry in (
            (8, BETA, SONNET, 25000, "cli"),
            (9, GONE_B, SONNET, 25000, "cli"),
            (10, None, OPUS, 70000, "sdk-cli"),
            (11, ALPHA, SONNET, 25000, "cli"),
        ):
            s = new("B", cwd, model, VERSION_B, entry=entry)
            for m in range(3):
                s.message(stamp(day + offset, 10 + m, 0), ctx=ctx, out=500, cache=0.9, tools=("Read",), errors=0)
    return sessions


TRACE_TOOLS = ("Bash", "Edit", "Read")


def trace_lines(rows):
    out = []
    for ts, tool, error in rows:
        out.append(json.dumps({"timestamp": ts, "tool_name": tool, "is_error": error, "summary": "INVENTED_TRACE_SUMMARY"}, separators=(",", ":")))
    return out


def write_tree() -> None:
    if OUT.exists():
        shutil.rmtree(OUT)
    folders = {"A": OUT / "claude" / "projects", "B": OUT / "claude-work" / "projects"}
    for session in build():
        label = {GONE_A: "gone-a", GONE_B: "gone-b", GONE_C: "gone-c", ALPHA: "alpha", BETA: "beta", None: "no-cwd"}[session.cwd]
        folder = folders[session.account] / f"proj-{label}"
        folder.mkdir(parents=True, exist_ok=True)
        text = "".join(json.dumps(r, separators=(",", ":")) + "\n" for r in session.lines)
        (folder / f"{session.id}.jsonl").write_text(text)
        if session.side:
            side = folder / session.id / "subagents"
            side.mkdir(parents=True)
            (side / "agent-rich0001.jsonl").write_text("".join(json.dumps(r, separators=(",", ":")) + "\n" for r in session.side))

    kit = OUT / "kit"
    # alpha: 30 kernel-trace calls in the window, 12 failed (Bash 9, Edit 3) = 0.4; four older calls outside the window
    t1 = [(stamp(14 + i % 4, 10, i), "Bash", i < 9) for i in range(12)] + [(stamp(15, 11, i), "Edit", i < 3) for i in range(8)]
    t2 = [(stamp(16, 12, i), "Read", False) for i in range(10)]
    old = [(stamp(8, 9, i), "Bash", True) for i in range(4)]
    (kit / "alpha" / "memory" / "event-trace").mkdir(parents=True)
    (kit / "alpha" / "memory" / "event-trace" / "T001.jsonl").write_text("\n".join(trace_lines(t1 + old)) + "\nnot json at all INVENTED_BAD_LINE\n")
    (kit / "alpha" / "memory" / "event-trace" / "T002.jsonl").write_text("\n".join(trace_lines(t2)) + "\n")
    # beta: 30 calls, 3 failed = 0.1 exactly (the rule fires above 0.1, so this is the boundary)
    t3 = [(stamp(13 + i // 6, 13, i), "Edit" if i < 3 else "Read", i < 3) for i in range(30)]
    (kit / "beta" / "memory" / "event-trace").mkdir(parents=True)
    (kit / "beta" / "memory" / "event-trace" / "T001.jsonl").write_text("\n".join(trace_lines(t3)) + "\n")

    (kit / "alpha" / ".claude").mkdir()
    (kit / "alpha" / ".claude" / "board.json").write_text('{"board": "linear"}\n')
    (kit / "alpha" / "PROJECT_KANBAN.md").write_text(
        "# PROJECT_KANBAN (invented)\n\n## Board\n\n### Todo\n\n- **R-001** — Write the invented parser\n- R-002: Check the invented cache\n\n"
        "### In Progress\n\n| ID | Title |\n|----|-------|\n| R-003 | Port the invented report |\n\n### Done\n\n- **R-004** — Ship the invented slice\n\n"
        "### Strange\n\n- **R-005** — A row under an unknown status counts as abstained\n\n## Blocked\n\n| ID | Title |\n|----|-------|\n| R-006 | Waiting for an invented answer |\n"
    )
    (kit / "beta" / "PROJECT_KANBAN.md").write_text("# PROJECT_KANBAN (invented)\n\n## Board\n\n### Todo\n\n- **B-001** — A board with no board.json\n")

    memory = kit / "alpha" / "memory"
    (memory / "MEMORY.md").write_text("# Memory index (invented)\n\n- [[decisions]] and [[glossary]] and [[missing-note]]\n")
    (memory / "decisions.md").write_text("---\nname: decisions\ndescription: invented decision log\n---\nBody INVENTED_NOTE_BODY links [[glossary]].\n")
    (memory / "glossary.md").write_text("---\nname: glossary\ndescription: invented terms\n---\nBody INVENTED_NOTE_BODY.\n")

    (kit / "alpha" / "reports").mkdir()
    (kit / "alpha" / "reports" / "index.html").write_text("<!doctype html><title>invented</title><p>INVENTED_REPORT</p>\n")
    (kit / "alpha" / "reports" / "session-001.json").write_text('{"verdict": "pass", "checks": 3}\n')
    (kit / "alpha" / "smoke-reports").mkdir()
    (kit / "alpha" / "smoke-reports" / "run.md").write_text("# invented smoke run\n")
    (kit / "alpha" / "docs").mkdir()  # a folder whose name has no `report` in it: never listed
    (kit / "alpha" / "docs" / "notes.md").write_text("INVENTED_DOC\n")


if __name__ == "__main__":
    write_tree()
    print(f"wrote {OUT.relative_to(HERE.parent.parent)}", file=sys.stderr)
