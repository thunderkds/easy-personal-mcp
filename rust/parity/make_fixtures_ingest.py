#!/usr/bin/env python3
"""Writes `rust/parity/ingest/fixtures/` (T026): the invented transcripts that the ingest oracle feeds to the real
Python `easy-personal ingest` (`gen_ingest.py` only reads them). Run it again only to change the data:

    python3 rust/parity/make_fixtures_ingest.py        (stdlib only; deterministic; rewrites only its own files)

Everything is fictional. Planted secret-like strings are obvious fakes (`sk-FAKE...`, `AKIAFAKE...`, `FAKEFAKE...`).
No file holds an `@`: the one URL with user info writes it as the JSON escape `\\u0040` (json.loads reads it as `@`).
`%WORK%` in a file is the work root of the run: `gen_ingest.py` replaces it with a folder whose path is exactly
`WORK_LEN` characters long, so every byte offset below is the same on every machine.

Output:
  synthetic/          the S6 set (copied to <AGENT_ROOT>/.claude/projects/): one folder per theme
  outside/            target of the symlinked folder `<PROJECTS>/linked` (S6)
  crash-*/            one crashing case each, with a file before and after it (S6a-S6d)
  s3/, s5/, s7/       inputs of single scenarios (README "Ingest oracle")
  cases.json          every line of every file: file, line number, byte offset (after the %WORK% substitution), case
  repos.json          the work-root layout (folders, git commands with a fixed env, symlinks) and the cwd cases
"""

import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "ingest" / "fixtures"
GENERATOR = "rust/parity/make_fixtures_ingest.py"
WORK = "%WORK%"
WORK_LEN = 240  # characters of the work root path in every run (gen_ingest.py pads its temp folder to it)
MODEL = "claude-fixture-model-1"
VERSION = "2.1.400"
ABSENT = "/nonexistent-ingest-oracle/app"  # an absolute cwd that exists on no machine: no git call
OMIT = object()  # a key left out of the record

# Copied from `src/easy_personal/sources/claude_code.py` METADATA_TYPES (2026-11-11); gen_ingest.py checks it still matches.
METADATA_TYPES = (
    "agent-name", "custom-title", "last-prompt", "atis-latch", "queue-operation", "cost-state",
    "file-history-snapshot", "file-history-delta", "artifact-comment-monitor", "artifact-autoreact-ledger",
    "frame-link", "api-request", "api-request-blob", "api-request-shape", "tools-used", "ai-title", "mode",
    "permission-mode", "bridge-session",
)  # fmt: skip


def sid(n: int) -> str:
    return f"00000000-0000-4000-8000-{n:012d}"


class Clock:
    """One timestamp per record, a second apart, from 2026-01-10T10:00:00Z (every date is in the past)."""

    def __init__(self, hour: int = 10) -> None:
        self.hour, self.n = hour, 0

    def __call__(self) -> str:
        n, self.n = self.n, self.n + 1
        return f"2026-01-10T{self.hour:02d}:{n // 60:02d}:{n % 60:02d}.000Z"


def rec(rtype, session, ts, **fields) -> dict:
    """A versioned record with the usual keys; a field set to OMIT is left out, any other replaces the default."""
    base = {
        "type": rtype, "sessionId": session, "timestamp": ts, "version": VERSION, "cwd": ABSENT,
        "entrypoint": "cli", "gitBranch": "main", "isSidechain": False, "userType": "external",
    }  # fmt: skip
    base.update(fields)
    return {k: v for k, v in base.items() if v is not OMIT}


def usage(i=10, o=5, cr=100, cc=20) -> dict:
    return {"input_tokens": i, "output_tokens": o, "cache_read_input_tokens": cr, "cache_creation_input_tokens": cc}


def asst(session, ts, mid, content=None, use=OMIT, model=MODEL, **fields) -> dict:
    message = {"id": mid, "type": "message", "role": "assistant", "model": model,
               "content": content if content is not None else [{"type": "text", "text": "INVENTED_REPLY"}],
               "usage": usage() if use is OMIT else use}  # fmt: skip
    if use is None:
        message["usage"] = None
    return rec("assistant", session, ts, message=message, **fields)


def user(session, ts, content="INVENTED_PROMPT", **fields) -> dict:
    return rec("user", session, ts, message={"role": "user", "content": content}, **fields)


def line(obj) -> bytes:
    """One JSONL line; `@` never appears in a fixture byte (written as \\u0040, json.loads reads it as `@`)."""
    text = json.dumps(obj, separators=(",", ":"), ensure_ascii=False, allow_nan=True)
    return (text.replace("@", "\\u0040") + "\n").encode("utf-8")


def tool(name, **inp) -> dict:
    return {"type": "tool_use", "id": "toolu_invented", "name": name, "input": inp}


def result(is_error=OMIT) -> dict:
    block = {"type": "tool_result", "tool_use_id": "toolu_invented", "content": "INVENTED_RESULT"}
    if is_error is not OMIT:
        block["is_error"] = is_error
    return block


# -- the synthetic set (S6) -------------------------------------------------------------------------------------


def framing() -> list[tuple[str, bytes]]:
    s, t = sid(1001), Clock()
    record = user(s, "2026-01-10T09:00:00.000Z")
    text = json.dumps(record, separators=(",", ":"))
    return [
        ("baseline assistant record", line(asst(s, t(), "msg_frame_1"))),
        ("blank line (just a newline)", b"\n"),
        ("whitespace-only line", b" \t \n"),
        ("invalid JSON (cut object)", b'{"type":"user",\n'),
        ("valid JSON, not an object: array", b"[1,2,3]\n"),
        ("valid JSON, not an object: string", b'"just text"\n'),
        ("valid JSON, not an object: number", b"42\n"),
        ("valid JSON, not an object: null", b"null\n"),
        ("valid JSON, not an object: true", b"true\n"),
        ("non-UTF-8 byte (0xff) inside a string", text[:-1].encode() + b',"note":"x\xffx"}\n'),
        ("UTF-16-LE encoded record, then a 0x0a byte", text.encode("utf-16-le") + b"\n"),
        ("UTF-8 BOM before the object", b"\xef\xbb\xbf" + line(user(s, t()))),
        ("CRLF line ending", line(user(s, t()))[:-1] + b"\r\n"),
        ("NaN in a usage field", line(asst(s, t(), "msg_frame_nan", use=usage(i=math.nan)))),
        ("Infinity in a usage field", line(asst(s, t(), "msg_frame_inf", use=usage(o=math.inf)))),
        ("duplicate key: type given twice (brand-new-type, then user)",
         b'{"type":"brand-new-type",' + line(user(s, t()))[1:]),
        ("truncated last line (no newline at the end)", b'{"type":"assistant","sessionId":"' + s.encode()),
    ]  # fmt: skip


def stamps() -> list[tuple[str, bytes]]:
    s = sid(1002)
    cases = [
        ("timestamp naive (no zone)", "2026-01-10T10:00:00"),
        ("timestamp +07:00 offset", "2026-01-10T17:00:01+07:00"),
        ("timestamp -05:30 offset", "2026-01-10T04:30:02-05:30"),
        ("timestamp +00:00 offset", "2026-01-10T10:00:03+00:00"),
        ("timestamp Z, 0 fractional digits", "2026-01-10T10:00:04Z"),
        ("timestamp Z, 1 fractional digit", "2026-01-10T10:00:05.5Z"),
        ("timestamp Z, 3 fractional digits", "2026-01-10T10:00:06.123Z"),
        ("timestamp Z, 6 fractional digits", "2026-01-10T10:00:07.123456Z"),
        ("timestamp Z, 9 fractional digits", "2026-01-10T10:00:08.123456789Z"),
        ("timestamp comma as decimal sign", "2026-01-10T10:00:09,25Z"),
        ("timestamp ISO basic format (no separators)", "20260110T100010Z"),
        ("timestamp space instead of T", "2026-01-10 10:00:11Z"),
        ("timestamp minutes only", "2026-01-10T10:12Z"),
        ("timestamp hour only", "2026-01-10T10Z"),
        ("timestamp ISO week date", "2026-W02-6T10:00:13Z"),
        ("timestamp 24:00 (end of day)", "2026-01-10T24:00:00Z"),
        ("timestamp lowercase z", "2026-01-10T10:00:14z"),
        ("timestamp leap second :60", "2026-01-10T23:59:60Z"),
        ("timestamp date only", "2026-01-10"),
        ("timestamp empty string", ""),
    ]
    rows = [(case, line(user(s, ts))) for case, ts in cases]
    rows += [
        ("timestamp a number (epoch seconds)", line(user(s, 1768039200))),
        ("timestamp missing", line(user(s, OMIT))),
        ("assistant record with a naive timestamp", line(asst(s, "2026-01-10T10:00:15", "msg_ts_naive"))),
        ("metadata record (last-prompt) with a naive timestamp",
         line({"type": "last-prompt", "sessionId": s, "timestamp": "2026-01-10T10:00:16", "lastPrompt": "INVENTED"})),
        ("metadata record (last-prompt) with a +07:00 offset",
         line({"type": "last-prompt", "sessionId": s, "timestamp": "2026-01-10T17:00:17+07:00"})),
    ]  # fmt: skip
    return rows


def cwds() -> list[tuple[str, bytes]]:
    s, t = sid(1003), Clock()
    return [
        ("cwd missing", line(user(s, t(), cwd=OMIT))),
        ("cwd null", line(user(s, t(), cwd=None))),
        ("cwd empty string", line(user(s, t(), cwd=""))),
        ("cwd not a string: number", line(user(s, t(), cwd=123))),
        ("cwd not a string: list", line(user(s, t(), cwd=["x"]))),
        ("cwd not a string: object", line(user(s, t(), cwd={"a": 1}))),
        ("cwd absolute, absent on disk", line(user(s, t(), cwd=ABSENT))),
        ("cwd with a trailing newline", line(user(s, t(), cwd=ABSENT + "\n"))),
    ]


def usages() -> list[tuple[str, bytes]]:
    s, t = sid(1004), Clock()
    bad_msg = rec("assistant", s, "2026-01-10T10:59:00.000Z", message="INVENTED_MESSAGE_TEXT")
    no_msg = rec("assistant", s, "2026-01-10T10:59:01.000Z")
    no_id = asst(s, "2026-01-10T10:59:02.000Z", "x")
    del no_id["message"]["id"]
    no_usage = asst(s, "2026-01-10T10:59:03.000Z", "msg_use_nousage")
    del no_usage["message"]["usage"]
    missing_one = usage()
    del missing_one["cache_creation_input_tokens"]
    return [
        ("usage all ints (baseline)", line(asst(s, t(), "msg_use_ok"))),
        ("usage bool (input_tokens true)", line(asst(s, t(), "msg_use_bool", use=usage(i=True)))),
        ("usage float (output_tokens 5.0)", line(asst(s, t(), "msg_use_float", use=usage(o=5.0)))),
        ("usage string (cache_read_input_tokens \"10\")", line(asst(s, t(), "msg_use_str", use=usage(cr="10")))),
        ("usage negative int (input_tokens -5)", line(asst(s, t(), "msg_use_neg", use=usage(i=-5)))),
        ("usage zeros", line(asst(s, t(), "msg_use_zero", use=usage(0, 0, 0, 0)))),
        ("usage int64 max", line(asst(s, t(), "msg_use_max", use=usage(i=2**63 - 1)))),
        ("usage int64 min", line(asst(s, t(), "msg_use_min", use=usage(i=-(2**63))))),
        ("usage key missing", line(no_usage)),
        ("usage null", line(asst(s, t(), "msg_use_null", use=None))),
        ("usage empty object", line(asst(s, t(), "msg_use_empty", use={}))),
        ("usage not an object (list)", line(asst(s, t(), "msg_use_list", use=[]))),
        ("usage with one field missing", line(asst(s, t(), "msg_use_three", use=missing_one))),
        ("usage present, message.id missing", line(no_id)),
        ("usage present, message.id not a label (space)", line(asst(s, t(), "msg bad id"))),
        ("assistant message is a string", line(bad_msg)),
        ("assistant message missing", line(no_msg)),
    ]  # fmt: skip


def labels() -> list[tuple[str, bytes]]:
    s, t = sid(1005), Clock()
    ok200 = "abcdefghi-" * 20  # 200 characters, no 32-character letter-and-digit run
    return [
        ("label trailing newline (gitBranch \"feature\\n\")", line(user(s, t(), gitBranch="feature\n"))),
        ("label two trailing newlines", line(user(s, t(), gitBranch="feature\n\n"))),
        ("label illegal character (space)", line(user(s, t(), gitBranch="two words"))),
        ("label illegal character (non-ASCII)", line(user(s, t(), gitBranch="café"))),
        ("label empty string", line(user(s, t(), gitBranch=""))),
        ("label 200 characters (the maximum)", line(user(s, t(), gitBranch=ok200))),
        ("label 201 characters", line(user(s, t(), gitBranch=ok200 + "x"))),
        ("label every allowed punctuation but the at sign",line(user(s, t(), gitBranch="a_b.c:d/e+f-g"))),
        ("sessionId with a trailing newline", line(user(sid(1006) + "\n", t()))),
        ("sessionId not a string", line(user(1006, t()))),
        ("model <synthetic>", line(asst(s, t(), "msg_lbl_synth", model="<synthetic>"))),
        ("model with a trailing newline", line(asst(s, t(), "msg_lbl_nl", model=MODEL + "\n"))),
        ("version with a trailing newline", line(user(s, t(), version="2.1.999\n"))),
        ("entrypoint with a trailing newline", line(user(s, t(), entrypoint="sdk-cli\n"))),
        ("type with a trailing newline (\"user\\n\")", line(user(s, t(), type="user\n"))),
        ("isSidechain the string \"true\"", line(user(s, t(), isSidechain="true"))),
    ]  # fmt: skip


def split() -> list[tuple[str, bytes]]:
    s, t = sid(1007), Clock()
    return [
        ("split message: first record of msg_split_1 (output 5)",
         line(asst(s, t(), "msg_split_1", [{"type": "thinking", "thinking": "INVENTED"}], use=usage(o=5)))),
        ("split message: second record of msg_split_1 (output 50, a tool call)",
         line(asst(s, t(), "msg_split_1", [tool("Read", file_path="/invented/file.txt")], use=usage(o=50)))),
        ("assistant isApiErrorMessage true", line(asst(s, t(), "msg_split_err", isApiErrorMessage=True))),
        ("assistant content is a string", line(asst(s, t(), "msg_split_str", "INVENTED_TEXT"))),
    ]  # fmt: skip


def tools() -> list[tuple[str, bytes]]:
    s, t = sid(1008), Clock()
    plain = [
        tool("Bash", command="git status"), tool("Bash", command="  ls -la"),
        tool("Bash", command="/usr/bin/env python3 x"), tool("Bash", command="./run.sh --flag"),
        tool("Bash", command="git status"), tool("Bash", command=""), tool("Bash", command="   "),
        tool("Bash", command=["git"]), {"type": "tool_use", "id": "toolu_invented", "name": "Bash", "input": "git"},
        tool("Bash", command="échec now"), tool("Bash", command="tool-name_v2.1+x run"),
    ]  # fmt: skip
    secret = [
        tool("Bash", command="sk-FAKE0000000000 --flag"),
        tool("Bash", command="FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE plan"),
        tool("Bash", command="AKIAFAKE00000000 run"),
    ]
    fetch = [
        tool("WebFetch", url="https://user:pw@Example.COM:8443/path/x?q=1#frag"),
        tool("WebFetch", url="http://localhost:8080/"), tool("WebFetch", url="http://127.0.0.1/x"),
        tool("WebFetch", url="ftp://example.org/x"), tool("WebFetch", url="http://[::1"),
        tool("WebFetch", url=42), tool("WebFetch", url="https://EXAMPLE.org./trail"),
        tool("WebFetch", url="https://sub.example.net"),
    ]  # fmt: skip
    names = [
        tool("Read", file_path="/invented"), tool("Bad Name"), {"type": "tool_use", "name": 7},
        {"type": "tool_use", "id": "toolu_noname"}, tool("mcp__easy-ui__check"), {"type": "text", "text": "INVENTED"}, "x",
    ]  # fmt: skip
    return [
        ("Bash first words: plain, leading spaces, a path, ./script, a duplicate, empty, blank, not a string, "
         "input not an object, non-ASCII, allowed punctuation", line(asst(s, t(), "msg_tool_plain", plain))),
        ("Bash first words secret-like: sk-FAKE..., 32 letters and digits, AKIAFAKE...",
         line(asst(s, t(), "msg_tool_secret", secret))),
        ("WebFetch URLs: user info + port + path, localhost, IPv4, ftp, invalid, not a string, trailing dot, plain",
         line(asst(s, t(), "msg_tool_fetch", fetch))),
        ("tool names: a label, not a label, not a string, missing, an MCP name; a text block; a non-object block",
         line(asst(s, t(), "msg_tool_names", names))),
        ("tool_result is_error true, false, absent", line(user(s, t(), [result(True), result(False), result()]))),
        ("tool_result is_error false only", line(user(s, t(), [result(False)]))),
        ("tool_result is_error the string \"true\"", line(user(s, t(), [result("true")]))),
        ("tool_result is_error true twice", line(user(s, t(), [result(True), result(True)]))),
        ("user content a string (no tool_result)", line(user(s, t(), "INVENTED_PROMPT"))),
        ("user content a list without tool_result", line(user(s, t(), [{"type": "text", "text": "INVENTED"}]))),
        ("user content an object", line(user(s, t(), {"type": "tool_result"}))),
    ]  # fmt: skip


def types() -> list[tuple[str, bytes]]:
    s, t = sid(1009), Clock()
    rows = [
        (f"metadata type {name}", line({"type": name, "sessionId": s, "timestamp": t(), "version": VERSION}))
        for name in METADATA_TYPES
    ]
    rows += [
        ("versioned type system", line(rec("system", s, t(), subtype="invented"))),
        ("versioned type attachment", line(rec("attachment", s, t(), attachment={"text": "INVENTED"}))),
        ("unknown type", line(rec("brand-new-type", s, t()))),
        ("type missing", line(rec(OMIT, s, t()))),
        ("type a number", line(rec(5, s, t()))),
        ("type empty string", line(rec("", s, t()))),
        ("metadata type without sessionId or timestamp", line({"type": "mode", "mode": "INVENTED"})),
    ]  # fmt: skip
    return rows


def unversioned() -> list[tuple[str, bytes]]:
    s, t = sid(1010), Clock()
    return [
        ("unversioned user record", line(user(s, t(), version=OMIT))),
        ("unversioned assistant record", line(asst(s, t(), "msg_unver_1", version=OMIT))),
    ]


def entry() -> list[tuple[str, bytes]]:
    e1, e2, e3, t = sid(1011), sid(1012), sid(1013), Clock()
    return [
        ("entrypoint absent, session E1 first record", line(user(e1, t(), entrypoint=OMIT))),
        ("entrypoint sdk-cli, session E1 second record", line(user(e1, t(), entrypoint="sdk-cli"))),
        ("entrypoint absent, session E1 third record", line(user(e1, t(), entrypoint=OMIT))),
        ("entrypoint cli, session E2 first record", line(user(e2, t(), entrypoint="cli"))),
        ("entrypoint sdk-ts, session E2 second record", line(user(e2, t(), entrypoint="sdk-ts"))),
        ("entrypoint null, session E3", line(user(e3, t(), entrypoint=None))),
        ("entrypoint absent, session E3", line(user(e3, t(), entrypoint=OMIT))),
    ]


def side_main() -> list[tuple[str, bytes]]:
    s, t = sid(1014), Clock()
    return [
        ("main thread of the sidechain session (entrypoint cli)", line(user(s, t()))),
        ("main thread assistant", line(asst(s, t(), "msg_side_main"))),
    ]


def side_agent() -> list[tuple[str, bytes]]:
    s, t = sid(1014), Clock(hour=11)
    return [
        ("sidechain record (isSidechain true, no entrypoint)", line(user(s, t(), isSidechain=True, entrypoint=OMIT))),
        ("sidechain assistant (isSidechain true, no entrypoint)",
         line(asst(s, t(), "msg_side_agent", isSidechain=True, entrypoint=OMIT))),
    ]  # fmt: skip


REPO_CASES = (  # (cwd as written in the transcript, what it is); %WORK% = the work root
    (f"{WORK}/plain", "plain folder (no git)"),
    (f"{WORK}/plain/", "plain folder with a trailing slash"),
    (f"{WORK}/repo", "git repo top level"),
    (f"{WORK}/repo/sub", "subfolder of a git repo"),
    (f"{WORK}/wt", "linked worktree of the repo"),
    (f"{WORK}/repo-empty", "git repo with no commit"),
    (f"{WORK}/repo-link", "symlink to the repo"),
    (f"{WORK}/plain-link", "symlink to the plain folder"),
    (f"{WORK}/gone", "folder that no longer exists (made, then removed)"),
    (WORK, "the work root itself (not a repo)"),
    ("relative/missing", "relative cwd, absent from the process cwd"),
    ("repo/sub", "relative cwd that exists from the process cwd (<WORK>)"),
    ("plain", "relative cwd of a plain folder from the process cwd"),
)


def repos() -> list[tuple[str, bytes]]:
    s, t = sid(1015), Clock()
    return [(f"project resolution: {what} (cwd {cwd!r})", line(user(s, t(), cwd=cwd))) for cwd, what in REPO_CASES]


def outside() -> list[tuple[str, bytes]]:
    return [("record behind the symlinked folder <PROJECTS>/linked", line(user(sid(1016), "2026-01-10T12:00:00.000Z")))]


# -- inputs of single scenarios ----------------------------------------------------------------------------------


def valid(n: int, label: str) -> list[tuple[str, bytes]]:
    return [(f"valid record ({label})", line(asst(sid(n), "2026-01-10T13:00:00.000Z", f"msg_{n}")))]


def crash(n: int, case: str, bad: bytes) -> list[tuple[str, bytes]]:
    s = sid(n)
    return [
        ("valid record before the crashing line, same file", line(user(s, "2026-01-10T14:00:00.000Z"))),
        (case, bad),
        ("valid record after the crashing line, same file", line(user(s, "2026-01-10T14:00:02.000Z"))),
    ]


def s3() -> dict[str, list[tuple[str, bytes]]]:
    s = sid(1030)
    return {
        "s3/session.jsonl": [
            ("S3 first record, no entrypoint", line(user(s, "2026-01-10T15:00:00.000Z", entrypoint=OMIT))),
            ("S3 second record, no entrypoint", line(user(s, "2026-01-10T15:00:01.000Z", entrypoint=OMIT))),
        ],
        "s3/append.jsonl": [
            ("S3 appended line: entrypoint cli, usage, a Bash call",
             line(asst(s, "2026-01-10T15:00:02.000Z", "msg_s3", [tool("Bash", command="make test")], entrypoint="cli"))),
        ],
    }  # fmt: skip


def s5() -> dict[str, list[tuple[str, bytes]]]:
    s = sid(1031)
    third = line(asst(s, "2026-01-10T16:00:02.000Z", "msg_s5_3"))
    cut = 40
    return {
        "s5/session.jsonl": [
            ("S5 first record", line(user(s, "2026-01-10T16:00:00.000Z"))),
            ("S5 second record", line(asst(s, "2026-01-10T16:00:01.000Z", "msg_s5_2"))),
            ("S5 third record, cut after 40 bytes (no newline)", third[:cut]),
        ],
        "s5/rest.part": [("S5 rest of the third record, with its newline (appended before the second run)", third[cut:])],
    }


def s7() -> dict[str, list[tuple[str, bytes]]]:
    def one(n, label):
        return [(f"S7 {label}", line(asst(sid(n), "2026-01-10T17:00:00.000Z", f"msg_s7_{n}")))]

    # Stored without the leading dot (as fixtures-rich/ does); gen_ingest.py copies each `s7/agents/<name>` to
    # `<AGENT_ROOT>/.<name>`.
    return {
        "s7/agents/claude/projects/p/a.jsonl": one(1040, "default account (.claude)"),
        "s7/agents/claude-work/projects/p/b.jsonl": one(1041, "work account (.claude-work), confirmed through the CLI"),
        "s7/agents/claude-other/projects/p/c.jsonl": one(1042, "detected account (.claude-other), never confirmed: not read"),
        "s7/agents/codex/config.toml": [("S7 Codex folder marker", b"# invented\n")],
        "s7/agents/claude.json": [("S7 a file named .claude.json (not a folder): skipped", b"{}\n")],
        "s7/agents/claude-notes/readme.txt": [("S7 a .claude-* folder with no projects/ or settings.json", b"invented\n")],
    }  # fmt: skip


def files() -> dict[str, list[tuple[str, bytes]]]:
    out = {
        "synthetic/framing/lines.jsonl": framing(),
        "synthetic/time/stamps.jsonl": stamps(),
        "synthetic/fields/cwd.jsonl": cwds(),
        "synthetic/fields/usage.jsonl": usages(),
        "synthetic/fields/labels.jsonl": labels(),
        "synthetic/split/session.jsonl": split(),
        "synthetic/tools/session.jsonl": tools(),
        "synthetic/types/all.jsonl": types(),
        "synthetic/unversioned/session.jsonl": unversioned(),
        "synthetic/entry/session.jsonl": entry(),
        f"synthetic/side/{sid(1014)}.jsonl": side_main(),
        f"synthetic/side/{sid(1014)}/subagents/agent-invented1.jsonl": side_agent(),
        "synthetic/projects/repos.jsonl": repos(),
        "outside/real.jsonl": outside(),
        "crash-deep/a.jsonl": valid(1020, "first file, sorted before p/"),
        "crash-deep/p/m-deep.jsonl": crash(1021, "deep nesting: 100,000 [ on one line", b"[" * 100_000 + b"\n"),
        "crash-deep/p-q/z.jsonl": valid(1022, "p-q/ sorts after p/ by path parts, before it as a string"),
        "crash-bigint/a.jsonl": valid(1023, "first file"),
        "crash-bigint/m-bigint.jsonl": crash(
            1024, "usage input_tokens 2**63 (beyond int64)",
            line(asst(sid(1024), "2026-01-10T14:00:01.000Z", "msg_bigint", use=usage(i=2**63)))),
        "crash-bigint/z.jsonl": valid(1025, "last file"),
        "crash-type/a.jsonl": valid(1026, "first file"),
        "crash-type/m-type.jsonl": crash(1027, "type is a list (unhashable)", line(rec(["user"], sid(1027), "2026-01-10T14:00:01.000Z"))),
        "crash-type/z.jsonl": valid(1028, "last file"),
        "crash-dir/a.jsonl": valid(1029, "first file (x.jsonl/ is a folder made by gen_ingest.py)"),
        "crash-dir/z.jsonl": valid(1032, "last file"),
    }  # fmt: skip
    for extra in (s3(), s5(), s7()):
        out.update(extra)
    return out


GIT_ENV = {  # fixed author and date: the repos are the same on every machine (the commit ids are never stored)
    "GIT_AUTHOR_NAME": "Oracle", "GIT_AUTHOR_EMAIL": "oracle.invalid", "GIT_AUTHOR_DATE": "2026-01-01T00:00:00+00:00",
    "GIT_COMMITTER_NAME": "Oracle", "GIT_COMMITTER_EMAIL": "oracle.invalid",
    "GIT_COMMITTER_DATE": "2026-01-01T00:00:00+00:00", "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
    "TZ": "UTC", "LC_ALL": "C",
}  # fmt: skip


def repos_json() -> dict:
    w = "<WORK>"
    return {
        "generator": GENERATOR,
        "note": "The work-root layout of the S6 run, in order. <WORK> is a fresh folder whose path is exactly "
        "work_path_length characters (pad the last component); %WORK% in a transcript is replaced by it before the copy. "
        "The ingest process runs with its cwd at <WORK> (relative cwd cases resolve against it). git runs with git_env "
        "plus PATH and HOME (an empty folder).",
        "work_path_length": WORK_LEN,
        "placeholder": WORK,
        "process_cwd": w,
        "git_env": GIT_ENV,
        "work": [
            {"op": "mkdir", "path": f"{w}/plain"},
            {"op": "mkdir", "path": f"{w}/repo/sub"},
            {"op": "git", "cwd": f"{w}/repo", "args": ["init", "-q", "-b", "main"]},
            {"op": "write", "path": f"{w}/repo/README.txt", "text": "invented\n"},
            {"op": "git", "cwd": f"{w}/repo", "args": ["add", "README.txt"]},
            {"op": "git", "cwd": f"{w}/repo", "args": ["commit", "-q", "-m", "invented"]},
            {"op": "git", "cwd": f"{w}/repo", "args": ["worktree", "add", "-q", "-b", "wt", f"{w}/wt"]},
            {"op": "mkdir", "path": f"{w}/repo-empty"},
            {"op": "git", "cwd": f"{w}/repo-empty", "args": ["init", "-q", "-b", "main"]},
            {"op": "symlink", "path": f"{w}/repo-link", "target": "repo"},
            {"op": "symlink", "path": f"{w}/plain-link", "target": "plain"},
            {"op": "mkdir", "path": f"{w}/gone"},
            {"op": "rmdir", "path": f"{w}/gone"},
        ],
        "synthetic": [  # after `synthetic/` is copied to <PROJECTS> and `outside/` to <AGENT_ROOT>/outside
            {"op": "symlink", "path": "<PROJECTS>/linked", "target": "<AGENT_ROOT>/outside"},
        ],
        "crash-dir": [{"op": "mkdir", "path": "<PROJECTS>/x.jsonl"}],
        "cwd_cases": [{"cwd": cwd.replace(WORK, w), "what": what} for cwd, what in REPO_CASES],
    }


def main() -> None:
    marker = OUT / "repos.json"
    if OUT.exists() and not (marker.is_file() and GENERATOR in marker.read_text()):
        sys.exit(f"refused: {OUT} exists and was not written by {GENERATOR}; nothing was changed")
    known = json.loads((OUT / "cases.json").read_text())["files"] if (OUT / "cases.json").is_file() else []
    written, cases = set(), []
    pad = "w" * WORK_LEN  # offsets are counted after the substitution: the work root is always WORK_LEN characters
    for rel, lines in sorted(files().items()):
        data = b"".join(b for _, b in lines)
        if b"@" in data:
            raise RuntimeError(f"{rel}: holds an @")
        path = OUT / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        written.add(rel)
        offset = 0
        for number, (case, raw) in enumerate(lines, start=1):
            cases.append({"file": rel, "line": number, "byte_offset": offset, "case": case})
            offset += len(raw.replace(WORK.encode(), pad.encode()))
    (OUT / "repos.json").write_text(json.dumps(repos_json(), indent=2, sort_keys=True) + "\n")
    index = {"generator": GENERATOR, "files": sorted(written), "cases": cases}
    (OUT / "cases.json").write_text(json.dumps(index, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
    for rel in sorted(set(known) - written):  # a file an earlier run wrote and this one does not
        (OUT / rel).unlink(missing_ok=True)
        for parent in (OUT / rel).parents:  # and the folders it leaves empty, up to OUT
            if parent == OUT or any(parent.iterdir()):
                break
            parent.rmdir()
    total = sum(p.stat().st_size for p in OUT.rglob("*") if p.is_file())
    if total > 200 * 1024:
        raise RuntimeError(f"synthetic set is {total} bytes, over 200 KB")
    print(f"{len(written)} files, {len(cases)} cases, {total} bytes in {OUT.relative_to(HERE.parents[1])}")


if __name__ == "__main__":
    main()
