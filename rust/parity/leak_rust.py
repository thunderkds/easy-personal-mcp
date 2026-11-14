#!/usr/bin/env python3
"""Ingest leak check (T030, NFR-002 / RS-003): planted secrets never reach the Rust store, its surfaces or its logs.

The Python leak suite's canaries and fixtures (`tests/leak/canaries.py`, `tests/fixtures/leak/`, read-only) are planted
into fresh temp folders exactly as `tests/leak/conftest.py::leak_run` plants them; then the **Rust** binary ingests them
(`easy-personal-rs ingest`, an env built from scratch) and every Rust surface is scanned with the suite's `Canaries.find`
(raw, JSON, \\u, URL, base64 at 3 alignments, hex): the ingest's stdout and stderr (three runs), `store-info` (text and
`--json`), every `serve` route over real HTTP with the token (plus guard refusals and the static route), every `mcp` tool
over stdio (with error paths), every stderr stream, the raw bytes of `store.sqlite` / `-wal` / `-shm` (while a reader still
holds the WAL open, and after close), `sqlite3 .dump`, and the data folder listing. One hit fails the run.

    cd rust && cargo build && ./parity/.venv/bin/python parity/leak_rust.py            # exit 0 clean, 1 on a hit or a failed check
    ./parity/.venv/bin/python parity/leak_rust.py --self-test                           # the scanner's own check only
    ./parity/.venv/bin/python parity/leak_rust.py --keep                                # keep the temp folder on a clean run

The report never prints a canary: a hit is shown as `<4 chars>[canary name]<4 chars>`, and the whole report is scanned
before it is printed. The temp folder is removed after a clean run and kept (path printed) after a failed one.
Refuses to run when an env var or the temp folder points at the owner's real home, ~/.claude*, ~/.codex*, ~/.easy-*.
"""

import argparse
import base64
import contextlib
import hashlib
import http.client
import json
import os
import re
import select
import shutil
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from urllib.parse import quote

sys.dont_write_bytecode = True  # importing the suite leaves no .pyc in tests/
PARITY = Path(__file__).resolve().parent
RUST = PARITY.parent
REPO = RUST.parent
BINARY = RUST / "target" / "debug" / "easy-personal-rs"
LAUNCHER = RUST / "target" / "debug" / "easy-personal-launch"  # T050
FAKE_CLAUDE = PARITY / "golden-connectors" / "fake-claude.sh"  # T050: the connector oracle's fake `claude`
sys.path.insert(0, str(PARITY))
sys.path.insert(0, str(REPO / "tests"))
import gen_golden

from fakes import jobs_fakes
from fakes.mcp_server import fake_context
from gen_context import Fake, srv
from leak.canaries import PATTERNS, Canaries, forms_of

CONTROL = "00000000-0000-4000-8000-0000000000aa"  # the template's sessionId: metadata the store keeps on purpose
DATA_FILES = {
    "store.sqlite",
    "store.sqlite-wal",
    "store.sqlite-shm",
    "ingest.lock",
}  # all the data folder may hold
CHILD_TIMEOUT = 60
SERVER_TIMEOUT = 300
API_ROUTE = re.compile(r'\.route\(\s*"(/api/[^"]+)"')  # T050: also a route whose path is on the next line
# Every `/api/*` route of the Rust server -> query variants (the Python suite's API_CALLS for these routes, plus bad input).
API_CALLS: dict[str, list[str]] = {
    "/api/version": [""],
    "/api/usage/summary": ["", "?since=2026-01-01", "?until=2027-01-01", "?workspace=acme", "?workspace=nope",
                           "?account=1", "?account=nope", "?account=1&workspace=acme", "?since=garbage"],
    "/api/usage/breakdown": ["?by=day", "?by=model", "?by=project", "?by=tool", "?by=bad", "?by=project&workspace=acme",
                             "?by=day&account=1", "?by=day&account=nope", "", "?by=day&since=2026-01-01"],
    "/api/usage/outliers": ["", "?since=2026-01-01", "?workspace=acme", "?account=1", "?account=nope"],
    "/api/usage/sessions": ["", "?since=2026-01-01", "?until=2027-01-01", "?workspace=acme", "?account=1", "?account=nope"],
    "/api/workspaces": [""],
    "/api/workspaces/unassigned": [""],
    "/api/workspaces/{id}": [""],  # asked literally with GET (the 405); the writes are in WRITE_CALLS and GUARD_CALLS
    "/api/accounts/{id}/confirm": [""],
    "/api/accounts/{id}/ignore": [""],
    "/api/accounts/{id}/workspaces": [""],
    "/api/connectors": ["", "?workspace=acme"],
    "/api/connectors/legacy": [""],  # T050: no keychain in this run: the fixed 503; the keychain run is `connectors()`
    "/api/connectors/{id}": [""],  # asked literally with GET (the 405); the writes are in `connectors()`
    "/api/connectors/{id}/move": [""],
    "/api/connectors/{id}/alias": [""],
    "/api/warnings": ["", "?workspace=acme", "?account=1", "?account=nope", "?workspace=nope"],
    "/api/plan": ["", "?workspace=acme", "?account=1", "?account=nope"],
    "/api/kit": ["", "?workspace=acme", "?account=1", "?account=nope", "?workspace=nope"],
    "/api/kit/workspaces": ["", "?since=2026-01-01&until=2027-01-01", "?account=1", "?account=nope", "?since=garbage"],
    "/api/kit/reports": [""],
    "/api/kit/report-files": ["", "?workspace=acme&repo_no=0&folder=reports", "?workspace=acme&repo_no=0&folder=..",
                              "?workspace=acme&repo_no=1e3&folder=reports"],
    "/api/kit/report-file": ["", "?workspace=acme&repo_no=0&folder=reports&path=index.html",
                             "?workspace=acme&repo_no=0&folder=reports&path=..%2f..%2fsecret.txt",
                             "?workspace=acme&repo_no=0&folder=reports&path=%2fetc%2fpasswd"],
    "/api/reports": [""],
    "/api/reports/{id}": [""],  # asked literally (not digits: the 404); the numeric ids are in GUARD_CALLS
    "/api/first-run": [""],
    "/api/board": ["", "?workspace=acme", "?account=1", "?account=nope", "?workspace=nope"],
    "/api/memory-graph": ["", "?project=nope", "?project=%2Ftmp", "?project="],
    "/api/accounts": ["", "?by=provider", "?by=bad", "?by="],
    "/api/sync": ["", "?account=1", "?account=99", "?account=nope", "?account=99999999999999999999"],
    "/api/dirs": ["", "?path=%2F", "?path=%2Fproc%2Fself", "?path=%2Fdev", "?path=relative", "?path=%00", "?path=%2Ftmp%2F..%2Ftmp%2F"],
    "/api/jobs": [""],
    "/api/store/health": [""],  # T059: also over a store planted with canaries, in `store_check`
    "/api/live": [],  # a stream that never ends: read bounded by `live_first` (the first message, then close), T056
}  # fmt: skip
# Requests the guard or the static fallback answers: (label, method, path, header changes; None drops a header).
GUARD_CALLS = [
    ("no token", "GET", "/api/usage/summary", {"x-dashboard-token": None}),
    ("wrong token", "GET", "/api/usage/summary", {"x-dashboard-token": "wrong"}),
    ("wrong host", "GET", "/api/usage/summary", {"Host": "localhost:1"}),
    ("wrong origin", "GET", "/api/usage/summary", {"Origin": "http://evil.invalid"}),
    ("workspace add without a body", "POST", "/api/workspaces", {}),
    ("workspace add no token", "POST", "/api/workspaces", {"x-dashboard-token": None}),
    ("workspace edit no token", "PATCH", "/api/workspaces/2", {"x-dashboard-token": None}),
    ("workspace delete foreign origin", "DELETE", "/api/workspaces/2", {"Origin": "http://evil.invalid"}),
    ("account add no token", "POST", "/api/accounts", {"x-dashboard-token": None}),
    ("account confirm wrong host", "POST", "/api/accounts/1/confirm", {"Host": "localhost:1"}),
    ("account ignore no token", "POST", "/api/accounts/1/ignore", {"x-dashboard-token": None}),
    ("account workspaces foreign origin", "PUT", "/api/accounts/1/workspaces", {"Origin": "http://evil.invalid"}),
    ("workspace read method", "GET", "/api/workspaces/2", {}),
    ("unknown route", "GET", "/api/nope", {}),
    ("live no token", "GET", "/api/live", {"x-dashboard-token": None}),
    ("live write method", "POST", "/api/live", {}),
    ("report id", "GET", "/api/reports/1", {}),
    ("report id too long", "GET", "/api/reports/1000000000000000000", {}),
    ("report write method", "DELETE", "/api/reports/1", {}),
    ("report file write method", "POST", "/api/kit/report-file?workspace=acme&repo_no=0&folder=reports&path=index.html", {}),
    ("report files no token", "GET", "/api/kit/report-files?workspace=acme&repo_no=0&folder=reports", {"x-dashboard-token": None}),
    ("account add without a body", "POST", "/api/accounts", {}),
    ("sync apply without a body", "POST", "/api/sync", {}),
    ("account workspaces without a body", "PUT", "/api/accounts/1/workspaces", {}),
    ("dirs write method", "POST", "/api/dirs", {}),
    ("jobs write method", "DELETE", "/api/jobs", {}),
    ("store health write method", "POST", "/api/store/health", {}),
    ("store health no token", "GET", "/api/store/health", {"x-dashboard-token": None}),
    ("store health head", "HEAD", "/api/store/health", {}),
    ("board write method", "POST", "/api/board", {}),
    ("memory graph no token", "GET", "/api/memory-graph", {"x-dashboard-token": None}),
    ("dirs no token", "GET", "/api/dirs", {"x-dashboard-token": None}),
    ("static index", "GET", "/", {"x-dashboard-token": None}),
    ("static file", "GET", "/index.html", {"x-dashboard-token": None}),
    ("static missing", "GET", "/nope.js", {"x-dashboard-token": None}),
    ("static dotfile", "GET", "/.env", {"x-dashboard-token": None}),
    ("static traversal", "GET", "/%2e%2e/%2e%2e/etc/passwd", {"x-dashboard-token": None}),
    ("double slash", "GET", "//api/version", {"x-dashboard-token": None}),
]
# The write routes (T048) with a body: (label, method, path, body). `{key}` is the planted API key: a refused write must not
# echo it, store it or log it. The accepted ones carry harmless text only.
WRITE_CALLS = [
    ("workspace add", "POST", "/api/workspaces", '{"name":"leak-check","description":"plain text","repos":["/leak/check"]}'),
    ("workspace add key as name", "POST", "/api/workspaces", '{"name":"{key}"}'),
    ("workspace add key as colour", "POST", "/api/workspaces", '{"name":"x","color":"{key}"}'),
    ("workspace add key as repo", "POST", "/api/workspaces", '{"name":"x","repos":["{key}","/{key}"]}'),
    ("workspace add key as field", "POST", "/api/workspaces", '{"name":"x","{key}":1}'),
    ("workspace add key not JSON", "POST", "/api/workspaces", '{{key}'),
    ("workspace edit key as name", "PATCH", "/api/workspaces/2", '{"name":"{key}"}'),
    ("workspace edit key as text", "PATCH", "/api/workspaces/2", '{"description":["{key}"],"icon":"{key}"}'),
    ("workspace edit key as confirm", "PATCH", "/api/workspaces/2", '{"confirm":"{key}"}'),
    ("workspace edit unknown", "PATCH", "/api/workspaces/999", '{"description":"{key}"}'),
    ("workspace delete plan", "DELETE", "/api/workspaces/2", ""),
    ("workspace delete key as confirm", "DELETE", "/api/workspaces/2", '{"confirm":"{key}"}'),
    ("workspace delete key as id", "DELETE", "/api/workspaces/{key}", '{"confirm":true}'),
    ("account add key as path", "POST", "/api/accounts", '{"path":"{key}"}'),
    ("account add key as absolute path", "POST", "/api/accounts", '{"path":"/{key}"}'),
    ("account add key as type", "POST", "/api/accounts", '{"path":["{key}"]}'),
    ("account confirm key as type", "POST", "/api/accounts/1/confirm", '{"label":["{key}"]}'),
    ("account confirm unknown", "POST", "/api/accounts/999/confirm", '{"label":"{key}"}'),
    ("account confirm key as id", "POST", "/api/accounts/{key}/confirm", '{"label":"x"}'),
    ("account ignore unknown", "POST", "/api/accounts/999/ignore", '{"label":"{key}"}'),
    ("account ignore key as confirm", "POST", "/api/accounts/1/ignore", '{"confirm":"{key}"}'),
    ("account workspaces key as name", "PUT", "/api/accounts/1/workspaces", '{"workspaces":["{key}"]}'),
    ("account workspaces key as type", "PUT", "/api/accounts/1/workspaces", '{"workspaces":"{key}"}'),
    ("account workspaces accepted", "PUT", "/api/accounts/1/workspaces", '{"workspaces":["leak-check"]}'),
]
# MCP tool -> argument variants (the Python suite's MCP_CALLS for these tools, plus error paths).
MCP_CALLS: dict[str, list[dict]] = {
    "ping": [{}],
    "usage_summary": [{}, {"since": "2026-01-01"}, {"until": "2027-01-01"}, {"workspace": "acme"}, {"account": "1"},
                      {"since": "garbage"}, {"since": 5}],
    "usage_breakdown": [{"by": "day"}, {"by": "model"}, {"by": "project"}, {"by": "tool"}, {"by": "day", "since": "2026-01-01"},
                        {"by": "bad"}, {}, {"by": 7}],
    "session_outliers": [{}, {"since": "2026-01-01"}, {"until": "garbage"}],
    "warnings": [{}, {"workspace": "acme"}, {"account": "1"}, {"workspace": "acme", "account": "1"}, {"account": "nope"}],
    "improvement_plan": [{}, {"workspace": "acme"}, {"account": "1"}],
    "kit_summary": [{}, {"workspace": "acme"}, {"account": "1"}, {"workspace": "nope"}, {"account": "nope"}, {"workspace": 5}],
    "connector_catalog": [{}, {"workspace": "acme"}],
    "connector_status": [{}, {"account": "1"}],
    "connector_suggestions": [{}, {"workspace": "acme"}],
    "kit_findings": [{}, {"limit": 10}, {"since": "2026-01-01"}, {"limit": 0}, {"limit": "x"}, {"since": "garbage"}],
    # the one tool that writes (T047): a good call, an unknown scope, bad types; the canary-in-text calls are in `mcp()`
    "save_report": [{"summary": "leak check report", "fixes": [{"text": "a fix", "tool": "warnings", "number": 1}]},
                    {"summary": "x", "fixes": [], "workspace": "nope"}, {"summary": 5, "fixes": []}, {"summary": "x"},
                    {"summary": "x", "fixes": ["s"]}],
}  # fmt: skip
MCP_RAW = [  # (label, line) answered by the protocol layer, not a tool
    (
        "unknown tool",
        '{"jsonrpc":"2.0","id":900,"method":"tools/call","params":{"name":"nope","arguments":{}}}',
    ),
    ("prompts/list", '{"jsonrpc":"2.0","id":904,"method":"prompts/list"}'),
    ("prompts/get", '{"jsonrpc":"2.0","id":905,"method":"prompts/get","params":{"name":"investigate","arguments":{}}}'),
    ("unknown method", '{"jsonrpc":"2.0","id":901,"method":"nope/method"}'),
    ("protocol ping", '{"jsonrpc":"2.0","id":902,"method":"ping"}'),
    ("parse error", "{not json"),
    ("invalid request", '{"id":903}'),
]


class Fatal(Exception):
    """A check failed or a child misbehaved; the message names no child output."""


# -- guards ----------------------------------------------------------------------------------------------------


def refuse(name: str, value: str | Path) -> None:
    """Exit 1 when `value` is a real home or inside its .claude* / .codex* / .easy-* folders (gen_ingest.refuse)."""
    target = Path(value).expanduser().resolve()
    for home in gen_golden._real_homes():
        if target == home:
            sys.exit(f"refused: {name} is the real home folder (NFR-003); nothing was opened")
        if home in target.parents and target.relative_to(home).parts[0].startswith(
            gen_golden.PROTECTED_PREFIXES
        ):
            sys.exit(
                f"refused: {name} points inside the real {target.relative_to(home).parts[0]} (NFR-003)"
            )


# -- scanning --------------------------------------------------------------------------------------------------


def as_text(blob) -> str:
    return blob.decode("utf-8", "replace") if isinstance(blob, bytes) else blob


def size(blob) -> int:
    return len(blob) if isinstance(blob, bytes) else len(blob.encode())


def excerpt(text: str, name: str, value: str, form: str) -> str:
    """At most 4 characters on each side of the hit, the canary itself replaced by its name."""
    safe = lambda s: "".join(c if c.isascii() and c.isprintable() else "." for c in s)
    for needle in forms_of(value)[form]:
        i = text.find(needle)
        if i >= 0:
            return f"{safe(text[max(0, i - 4) : i])}[{name}]{safe(text[i + len(needle) : i + len(needle) + 4])}"
    return f"[{name}]"


def scan(
    canaries: Canaries, surfaces: dict[str, object]
) -> tuple[list[tuple[str, str, str, str]], dict[str, int]]:
    """`Canaries.find` per canary per surface. Returns (hits as (canary, form, surface, excerpt), surfaces scanned per canary)."""
    hits, scanned = [], dict.fromkeys(canaries.values, 0)
    for label, blob in surfaces.items():
        text = as_text(blob)
        for name, value in canaries.values.items():
            scanned[name] += 1
            for _, form in Canaries({name: value}).find(text):
                hits.append((name, form, label, excerpt(text, name, value, form)))
    return hits, scanned


def self_test(canaries: Canaries, work: Path) -> list[str]:
    """The scanner must see a canary in each of seven encodings, inside a SQLite file, and the report must mask it."""
    value = canaries.values["message_text"]  # non-ASCII: raw, JSON and \\u forms all differ
    raw = value.encode()
    planted = {
        "raw": ("raw", value),
        "json": ("json", json.dumps(value)),
        "unicode-escape": ("unicode-escape", "".join(f"\\u{ord(c):04x}" for c in value)),
        "url": ("url", quote(value, safe="")),
        "hex": ("hex", raw.hex()),
    }
    for pad in range(3):
        planted[f"base64 @{pad}"] = ("base64", base64.b64encode(b"y" * pad + raw).decode())
        planted[f"base64url @{pad}"] = (
            "base64",
            base64.urlsafe_b64encode(b"y" * pad + raw).decode(),
        )
    lines = []
    for label, (form, text) in planted.items():
        found = ("message_text", form) in canaries.find(
            b"\x00prefix:" + text.encode() + b":suffix\xff"
        )
        lines.append(f"self-test {label:<15} {'detected' if found else 'MISSED'}")
    clean = not canaries.find("nothing planted here " + CONTROL)
    lines.append(f"self-test clean blob      {'no hit' if clean else 'FALSE HIT'}")
    db = work / "selftest.sqlite"
    conn = sqlite3.connect(db)
    conn.execute("CREATE TABLE events (note TEXT)")
    conn.execute("INSERT INTO events VALUES (?)", (canaries.values["file_content"],))
    conn.commit()
    in_file = bool(canaries.find(db.read_bytes())) and bool(
        canaries.find("\n".join(conn.iterdump()))
    )
    conn.close()
    lines.append(f"self-test sqlite row      {'detected' if in_file else 'MISSED'}")
    hits, _ = scan(canaries, {"selftest surface": f"abcdefgh{canaries.values['api_key']}ijklmnop"})
    masked = bool(hits) and all(
        h[0] == "api_key" and not canaries.find(h[3]) and len(h[3]) <= 8 + len("[api_key]")
        for h in hits
    )
    lines.append(f"self-test masked report   {'ok' if masked else 'FAILED'}")
    lines.append(f"self-test tree digest     {'ok' if digest_skips_only_the_fakes_log(work) else 'FAILED'}")
    return lines


def digest_skips_only_the_fakes_log(work: Path) -> bool:
    """`tree_digest` ignores `<root>/bin/argv.log` (the fakes' own log) and nothing else, not any file of that name."""
    root = work / "digest"
    (root / "bin").mkdir(parents=True)
    (root / "agent").mkdir()
    (root / "bin" / "argv.log").write_text("one\n")
    (root / "agent" / "argv.log").write_text("one\n")
    before = tree_digest(root)
    (root / "bin" / "argv.log").write_text("two\n")
    fakes_log_ignored = tree_digest(root) == before
    (root / "agent" / "argv.log").write_text("two\n")
    return fakes_log_ignored and tree_digest(root) != before


# -- planting (as tests/leak/conftest.py::leak_run) ------------------------------------------------------------


def plant(canaries: Canaries, root: Path) -> dict[str, Path]:
    projects = canaries.render_fixture(root / "projects")
    repo = canaries.render_kit_trace(root / "repo")
    canaries.render_verifier_report(repo)
    (projects / "-repo").mkdir()
    (projects / "-repo" / "session.jsonl").write_text(
        json.dumps({"type": "user", "version": "2.1.295", "cwd": str(repo),
                    "sessionId": "00000000-0000-4000-8000-0000000000bb",
                    "timestamp": "2026-02-01T09:00:00.000Z"}) + "\n"
    )  # fmt: skip
    ui_reports = canaries.render_ui_reports(root / "ui-reports")
    agent_root = root / "agent-root"
    (agent_root / ".claude" / "projects").mkdir(parents=True)
    (agent_root / ".claude-work" / "projects").mkdir(parents=True)
    (agent_root / ".codex").mkdir()
    (agent_root / ".codex" / "config.toml").write_text("")
    canaries.render_memory_notes(repo, agent_root / ".claude-work")
    canaries.render_easy_memory(repo, root / "easy-memory")
    canaries.render_agent_files(agent_root)
    fake_bin = root / "bin"
    fake_bin.mkdir()
    (fake_bin / "claude").write_text(
        f"#!{sys.executable}\nimport sys\nsys.exit(0 if sys.argv[1:] == ['mcp', 'get', 'slack'] else 1)\n"
    )
    (fake_bin / "claude").chmod(0o755)
    key = canaries.values["api_key"]
    jobs_fakes.install(
        fake_bin,
        timers=[{"unit": "easy-personal-ingest.timer", "activates": "easy-personal-ingest.service", "next": 1_793_444_400_000_000, "last": 1_793_433_606_000_000},
                {"unit": "tidy.timer", "activates": "tidy.service", "next": 1_793_444_400_000_000, "last": 0}],
        props={"easy-personal-ingest.timer": {"TimersCalendar": "{ OnCalendar=daily ; next_elapse=n/a }"},
               "easy-personal-ingest.service": {"ExecStart": jobs_fakes.exec_start(f"/usr/bin/easy-personal ingest --token {key}"), "ExecMainStatus": "0"},
               "tidy.service": {"ExecStart": jobs_fakes.exec_start(f"/usr/bin/rm -rf /tmp/{key}"), "ExecMainStatus": "0"}},
        crontab=f'0 7 * * 1-5 claude -p "{key}"\n5 4 * * * /usr/bin/backup --key {key}\n',
    )  # fmt: skip
    return {"projects": projects, "agent_root": agent_root, "ui_reports": ui_reports, "bin": fake_bin,
            "easy_memory": root / "easy-memory", "verifier": root / "no-verifier-container"}  # fmt: skip


def planted_where(canaries: Canaries, root: Path) -> dict[str, list[str]]:
    """Each canary -> the planted files (relative) that hold it raw or JSON-escaped."""
    files = [
        (p.relative_to(root).as_posix(), p.read_bytes())
        for p in sorted(root.rglob("*"))
        if p.is_file()
    ]
    return {name: [rel for rel, data in files if value.encode() in data or json.dumps(value)[1:-1].encode() in data]
            for name, value in canaries.values.items()}  # fmt: skip


def tree_digest(root: Path) -> str:
    h = hashlib.sha256()
    for p in sorted(root.rglob("*")):
        if p == root / "bin" / "argv.log":  # the fake systemctl/crontab (a test double) logs each call it answers; the binary writes nothing, so no other file of that name is skipped
            continue
        h.update(p.relative_to(root).as_posix().encode() + b"\0")
        if p.is_file():
            h.update(hashlib.sha256(p.read_bytes()).digest())
    return h.hexdigest()


# -- the Rust binary -------------------------------------------------------------------------------------------


class Run:
    def __init__(self, binary: Path, root: Path, planted: dict[str, Path]) -> None:
        self.binary, self.root = binary, root
        self.data, self.nowhere, self.logs = root / "data", root / "nowhere", root / "run"
        self.nowhere.mkdir()
        self.logs.mkdir()
        self.timeout = shutil.which("timeout") or "/usr/bin/timeout"
        self.env = {  # built from scratch: nothing of the caller's env reaches the child
            "PATH": f"{planted['bin']}:/usr/bin:/bin", "HOME": str(self.nowhere), "XDG_CONFIG_HOME": str(self.nowhere / ".config"),
            "EASY_PERSONAL_HOME": str(self.data), "EASY_PERSONAL_AGENT_ROOT": str(planted["agent_root"]),
            "EASY_PERSONAL_CLAUDE_PROJECTS": str(planted["projects"]), "EASY_MEMORY_HOME": str(planted["easy_memory"]),
            "EASY_PERSONAL_UI_REPORTS": str(planted["ui_reports"]), "EASY_PERSONAL_VERIFIER_REPORTS": str(planted["verifier"]),
            "EASY_PERSONAL_LOCK_TIMEOUT": "30", "TZ": "UTC", "LC_ALL": "C.UTF-8", "LANG": "C.UTF-8",
            "RUST_LOG": "trace", "RUST_BACKTRACE": "1", "NO_COLOR": "1",
            "DBUS_SESSION_BUS_ADDRESS": "unix:path=/nonexistent/leak-rs-no-bus",  # T050: no real keychain is reachable
            "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
        }  # fmt: skip
        for name in ("HOME", "EASY_PERSONAL_HOME", "EASY_PERSONAL_AGENT_ROOT", "EASY_PERSONAL_CLAUDE_PROJECTS",
                     "EASY_MEMORY_HOME", "EASY_PERSONAL_UI_REPORTS", "EASY_PERSONAL_VERIFIER_REPORTS"):  # fmt: skip
            refuse(f"child {name}", self.env[name])
        self.out: dict[str, object] = {}

    def add(self, label: str, blob) -> None:
        key, n = label, 2
        while key in self.out:
            key, n = f"{label} #{n}", n + 1
        self.out[key] = blob

    def argv(self, secs: int, *args: str) -> list[str]:
        return [self.timeout, "-k", "5", str(secs), str(self.binary), *args]

    def run(self, label: str, *args: str) -> subprocess.CompletedProcess:
        done = subprocess.run(self.argv(CHILD_TIMEOUT, *args), env=self.env, cwd=self.nowhere, capture_output=True,
                              stdin=subprocess.DEVNULL, timeout=CHILD_TIMEOUT + 15, check=False)  # fmt: skip
        self.add(f"{label} stdout", done.stdout)
        self.add(f"{label} stderr", done.stderr)
        if done.returncode != 0:
            raise Fatal(
                f"{label}: exit {done.returncode}"
                + (" (timed out)" if done.returncode in (124, 137) else "")
            )
        return done

    def ingest(self, label: str, *args: str) -> dict:
        return json.loads(self.run(label, "ingest", *args).stdout)

    def store_files(self, label: str) -> dict[str, bytes]:
        files = {p.name: p.read_bytes() for p in sorted(self.data.glob("store.sqlite*"))}
        for name, data in files.items():
            self.add(f"{label}: {name}", data)
        self.add(f"{label}: files present", " ".join(files) or "(none)")
        return files

    def dump(self, label: str) -> str:
        conn = sqlite3.connect(self.data / "store.sqlite")
        try:
            text = "\n".join(conn.iterdump())
        finally:
            conn.close()
        self.add(label, text)
        return text

    def popen(
        self, secs: int, *args: str, stdin=subprocess.DEVNULL, err: Path
    ) -> tuple[subprocess.Popen, object]:
        errlog = open(err, "wb")  # noqa: SIM115 - closed after the child stops; stderr goes to a file so a full pipe never stalls it
        proc = subprocess.Popen(self.argv(secs, *args), env=self.env, cwd=self.nowhere, stdin=stdin,
                                stdout=subprocess.PIPE, stderr=errlog)  # fmt: skip
        return proc, errlog

    def stop(self, label: str, proc: subprocess.Popen, children: list[int], sig: int | None) -> int:
        """Stop by PID (`timeout` forwards the signal to the binary), then prove no process is left."""
        if proc.poll() is None and sig is not None:
            os.kill(children[0], sig) if children else proc.send_signal(
                sig
            )  # the binary itself, so its exit code is seen
        try:
            rc = proc.wait(15)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
            raise Fatal(f"{label}: did not stop within 15 s; killed") from None
        left = [pid for pid in children if Path(f"/proc/{pid}").exists()]
        if left:
            raise Fatal(f"{label}: process {left} still running after stop")
        return rc


def children(pid: int) -> list[int]:
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        try:
            kids = [int(x) for x in Path(f"/proc/{pid}/task/{pid}/children").read_text().split()]
        except OSError:
            kids = []
        if kids:
            return kids
        time.sleep(0.05)
    return []


class Lines:
    """Whole lines from a pipe, with a deadline (a hang fails the run, never blocks it)."""

    def __init__(self, fd: int) -> None:
        self.fd, self.buf = fd, b""

    def read(self, secs: float) -> bytes | None:
        deadline = time.monotonic() + secs
        while b"\n" not in self.buf:
            left = deadline - time.monotonic()
            if left <= 0 or not select.select([self.fd], [], [], left)[0]:
                return None
            chunk = os.read(self.fd, 65536)
            if not chunk:
                return None
            self.buf += chunk
        line, self.buf = self.buf.split(b"\n", 1)
        return line + b"\n"


def request(port: int, method: str, path: str, headers: dict[str, str], body: bytes | None = None) -> tuple[int, bytes]:
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=15)
    try:
        conn.putrequest(method, path, skip_host=True, skip_accept_encoding=True)
        for key, value in headers.items():
            conn.putheader(key, value)
        if body is not None:
            conn.putheader("content-type", "application/json")
            conn.putheader("content-length", str(len(body)))
        conn.endheaders(body)
        res = conn.getresponse()
        body = res.read()
        head = f"{res.status} {res.reason}\n" + "".join(f"{k}: {v}\n" for k, v in res.getheaders())
        return res.status, head.encode() + b"\n" + body
    finally:
        conn.close()


def live_first(port: int, path: str, headers: dict[str, str]) -> bytes:
    """T056: `GET /api/live` up to its first complete SSE message (the `snapshot`), then close; 15 s at most."""
    head = f"GET {path} HTTP/1.1\r\n" + "".join(f"{k}: {v}\r\n" for k, v in headers.items()) + "\r\n"
    blob = b""
    with socket.create_connection(("127.0.0.1", port), timeout=15) as sock:
        sock.sendall(head.encode())
        while b"\r\n\r\n" not in blob or b"\n\n" not in blob.split(b"\r\n\r\n", 1)[1]:
            chunk = sock.recv(65536)
            if not chunk:
                break
            blob += chunk
    return blob


def serve(run: Run, checks: list[str], key: str) -> None:
    static = run.logs / "static"
    static.mkdir()
    (static / "index.html").write_text("<!doctype html><title>leak check</title>\n")
    rust_routes = set()
    for source in ("api.rs", "server.rs"):
        rust_routes |= set(
            API_ROUTE.findall((RUST / "crates" / "app" / "src" / source).read_text())
        )
    if rust_routes - set(API_CALLS):
        raise Fatal(
            f"unregistered /api route(s) in the Rust server: {sorted(rust_routes - set(API_CALLS))}"
        )
    proc, errlog = run.popen(
        SERVER_TIMEOUT, "serve", "--static-dir", str(static), err=run.logs / "serve.stderr"
    )
    kids, first, rc = [], b"", None
    try:
        lines = Lines(proc.stdout.fileno())
        first = lines.read(20) or b""
        match = re.fullmatch(rb"dashboard: http://127\.0\.0\.1:(\d+)/#([A-Za-z0-9_-]{43})\n", first)
        if not match:
            raise Fatal("serve: no `dashboard:` line within 20 s")
        port, token = int(match[1]), match[2].decode()  # the token is never printed
        kids = children(proc.pid)
        base = {"Host": f"127.0.0.1:{port}", "x-dashboard-token": token}
        for route, queries in API_CALLS.items():
            for query in queries:
                status, blob = request(port, "GET", route + query, base)
                run.add(f"api GET {route}{query}", blob)
                if route == "/api/usage/summary" and query == "":
                    body = json.loads(blob.split(b"\n\n", 1)[1])
                    ok = status == 200 and body.get("messages", 0) > 0
                    checks.append(
                        f"check api summary has data          {'ok' if ok else 'FAILED'} (status {status})"
                    )
        # T053 (as `tests/leak/harness.py` does for HUN-164): the graph of every project the list names, with its easy-memory
        # layer (the planted prompt, answer, session and trash canaries must stay out of every one)
        status, blob = request(port, "GET", "/api/memory-graph", base)
        listed = json.loads(blob.split(b"\n\n", 1)[1]).get("projects", []) if status == 200 else []
        checks.append(f"check memory-graph lists a project   {'ok' if listed else 'FAILED'} ({len(listed)} listed)")
        layered = False
        for project in listed:
            _, blob = request(port, "GET", f"/api/memory-graph?project={quote(project['project'], safe='')}", base)
            run.add(f"api GET /api/memory-graph project={project['name']}", blob)
            layered = layered or b'"kind":"easy_memory"' in blob
        checks.append(f"check a graph carries the easy-memory layer  {'ok' if layered else 'FAILED'}")
        for label, path, change in (
            ("query token", f"/api/live?token={token}", {"x-dashboard-token": None}),
            ("header token", "/api/live", {}),
            ("resume", "/api/live", {"Last-Event-ID": "1"}),
        ):
            blob = live_first(port, path, {k: v for k, v in {**base, **change}.items() if v is not None})
            run.add(f"api GET /api/live ({label}, first message)", blob)
            ok = blob.startswith(b"HTTP/1.1 200") and b"event: snapshot" in blob
            checks.append(f"check api live sends a snapshot ({label})  {'ok' if ok else 'FAILED'}")
        for label, method, path, change in GUARD_CALLS:
            headers = {k: v for k, v in {**base, **change}.items() if v is not None}
            status, blob = request(port, method, path, headers)
            run.add(f"api {label} {method} {path} -> {status}", blob)
        for label, method, path, body in WRITE_CALLS:
            status, blob = request(port, method, path.replace("{key}", key), base, body.replace("{key}", key).encode())
            run.add(f"api write {label} {method} {path} -> {status}", blob)
            if label == "workspace add":
                checks.append(f"check api workspace add works        {'ok' if status == 201 else 'FAILED'} (status {status})")
        run.add("serve stdout (first line, token inside)", first + lines.buf)
    finally:
        rc = run.stop("serve", proc, kids, signal.SIGINT)
        errlog.close()
        rest = proc.stdout.read() if proc.stdout else b""
        run.add("serve stdout (after stop)", rest)
        run.add("serve stderr", (run.logs / "serve.stderr").read_bytes())
    if not kids:
        raise Fatal("serve: could not find the server's PID under `timeout`")
    checks.append(f"check serve stopped by PID (SIGINT)   ok (exit {rc}, no process left)")


def mcp(run: Run, checks: list[str], canaries: Canaries) -> None:
    proc, errlog = run.popen(
        SERVER_TIMEOUT, "mcp", stdin=subprocess.PIPE, err=run.logs / "mcp.stderr"
    )
    kids, n = [], 0
    try:
        kids = children(proc.pid)
        lines = Lines(proc.stdout.fileno())

        def send(label: str, line: str, reply: bool = True) -> dict | None:
            proc.stdin.write(line.encode() + b"\n")
            proc.stdin.flush()
            if not reply:
                return None
            answer = lines.read(30)
            if answer is None:
                raise Fatal(f"mcp: no answer to {label} within 30 s")
            run.add(f"mcp {label}", answer)
            return json.loads(answer)

        init = {"jsonrpc": "2.0", "id": 0, "method": "initialize",
                "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "leak-rust", "version": "0"}}}  # fmt: skip
        if "result" not in send("initialize", json.dumps(init)):
            raise Fatal("mcp: initialize was refused")
        send(
            "notifications/initialized",
            '{"jsonrpc":"2.0","method":"notifications/initialized"}',
            reply=False,
        )
        listed = send("tools/list", '{"jsonrpc":"2.0","id":1,"method":"tools/list"}')
        names = {t["name"] for t in listed["result"]["tools"]}
        if names - set(MCP_CALLS):
            raise Fatal(f"unregistered MCP tool(s): {sorted(names - set(MCP_CALLS))}")
        n = 10
        for tool, variants in MCP_CALLS.items():
            for args in variants:
                n += 1
                call = {
                    "jsonrpc": "2.0",
                    "id": n,
                    "method": "tools/call",
                    "params": {"name": tool, "arguments": args},
                }
                answer = send(f"tools/call {tool} {json.dumps(args)}", json.dumps(call))
                if tool == "usage_summary" and not args:
                    result = answer.get("result", {})
                    data = result.get("structuredContent") or json.loads(
                        result["content"][0]["text"]
                    )
                    ok = not result.get("isError") and data.get("messages", 0) > 0
                    checks.append(f"check mcp usage_summary has data    {'ok' if ok else 'FAILED'}")
        # a refused `save_report` call carries a secret-shaped canary in every text field: nothing may echo or store it
        key = canaries.values["api_key"]
        for args in (
            {"summary": key, "fixes": []},
            {"summary": "x", "fixes": [{"text": key}]},
            {"summary": "x", "fixes": [{"text": "x", "tool": key}]},
            {"summary": "x", "fixes": [{"text": "x", "number": key}]},
            {"summary": "x", "fixes": [{"text": "x", "extra": key}]},
            {"summary": "x", "fixes": [key]},
            {"summary": 7, "fixes": [], "workspace": key},
        ):
            n += 1
            call = {"jsonrpc": "2.0", "id": n, "method": "tools/call", "params": {"name": "save_report", "arguments": args}}
            send("tools/call save_report (canary in the text)", json.dumps(call))
        for label, line in MCP_RAW:
            send(label, line)
    finally:
        if proc.stdin and not proc.stdin.closed:
            proc.stdin.close()  # end of input: the server exits on its own
        rc = run.stop("mcp", proc, kids, None)
        errlog.close()
        run.add("mcp stdout (after close)", proc.stdout.read() if proc.stdout else b"")
        run.add("mcp stderr", (run.logs / "mcp.stderr").read_bytes())
    checks.append(
        f"check mcp stopped (stdin closed)     ok (exit {rc}, {n - 10} tool calls, no process left)"
    )


def mcp_http(run: Run, checks: list[str], canaries: Canaries) -> None:
    """`mcp --transport http` (T051): every tool variant and the raw protocol lines over real HTTP, then Ctrl-C."""
    errfile = run.logs / "mcp-http.stderr"
    proc, errlog = run.popen(SERVER_TIMEOUT, "mcp", "--transport", "http", "--port", "0", err=errfile)
    kids, n, rc, port = [], 0, None, 0
    try:
        kids = children(proc.pid)
        deadline = time.monotonic() + 20
        while not port and time.monotonic() < deadline:
            found = re.search(rb"MCP over HTTP: http://127\.0\.0\.1:(\d+)/mcp\n", errfile.read_bytes())
            port = int(found[1]) if found else 0
            time.sleep(0.05)
        if not port:
            raise Fatal("mcp http: no start line within 20 s")

        def post(label: str, body: str, host: str | None = None, origin: str | None = None) -> tuple[int, bytes]:
            headers = {"Host": host or f"127.0.0.1:{port}", "Accept": "application/json, text/event-stream"}
            if origin:
                headers["Origin"] = origin
            status, answer = request(port, "POST", "/mcp", headers, body.encode())
            run.add(f"mcp http {label}", answer)
            return status, answer

        init = {"jsonrpc": "2.0", "id": 0, "method": "initialize",
                "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "leak-rust", "version": "0"}}}  # fmt: skip
        if post("initialize", json.dumps(init))[0] != 200:
            raise Fatal("mcp http: initialize was refused")
        status, answer = post("tools/list", '{"jsonrpc":"2.0","id":1,"method":"tools/list"}')
        names = {t["name"] for t in json.loads(answer.split(b"\n\n", 1)[1])["result"]["tools"]}
        if names - set(MCP_CALLS):
            raise Fatal(f"unregistered MCP tool(s): {sorted(names - set(MCP_CALLS))}")
        n = 10
        for tool, variants in MCP_CALLS.items():
            for args in variants:
                n += 1
                call = {"jsonrpc": "2.0", "id": n, "method": "tools/call", "params": {"name": tool, "arguments": args}}
                post(f"tools/call {tool} {json.dumps(args)}", json.dumps(call))
        key = canaries.values["api_key"]
        for args in ({"summary": key, "fixes": []}, {"summary": "x", "fixes": [{"text": key}]},
                     {"summary": 7, "fixes": [], "workspace": key}):  # fmt: skip
            n += 1
            call = {"jsonrpc": "2.0", "id": n, "method": "tools/call", "params": {"name": "save_report", "arguments": args}}
            post("tools/call save_report (canary in the text)", json.dumps(call))
        for label, line in MCP_RAW:
            post(label, line)
        # the guard: a rebinding Host and a foreign Origin are refused, a canary in either header is never echoed
        refused = [post("guard host", MCP_RAW[4][1], host=f"{key}.example:{port}")[0],
                   post("guard origin", MCP_RAW[4][1], origin=f"http://{key}.example")[0]]  # fmt: skip
        for method in ("GET", "DELETE"):
            status, answer = request(port, method, "/mcp", {"Host": f"127.0.0.1:{port}"})
            run.add(f"mcp http {method}", answer)
            refused.append(status)
        if refused != [421, 403, 405, 405]:
            raise Fatal(f"mcp http: the guard answered {refused}, expected [421, 403, 405, 405]")
    finally:
        rc = run.stop("mcp http", proc, kids, signal.SIGINT)
        errlog.close()
        run.add("mcp http stdout (after stop)", proc.stdout.read() if proc.stdout else b"")
        run.add("mcp http stderr", errfile.read_bytes())
    checks.append(f"check mcp http stopped (SIGINT)    ok (exit {rc}, {n - 10} tool calls, no process left)")


# -- T050: connector writes, sync apply, legacy, launcher ------------------------------------------------------


def connectors(run: Run, checks: list[str], canaries: Canaries) -> None:
    """Canaries in env values, header values and keychain secrets (the debug-build fake keychain file), in `claude`'s
    stderr (the oracle's fake `claude`, a failing call) and in an account folder's path. Every new route over real HTTP,
    then the launcher with a provider child. Scanned: the server's stdout and stderr, every answer, the fake `claude`'s
    argv log, the store and `.dump`, the launcher's output and the provider's `/proc` command line. The keychain file is
    the keychain (it holds the secrets on purpose) and is not a surface. The account-path marker may show as an account's
    folder (by design, `GET /api/accounts`), so it is checked on the argv log and the logs only, with a positive control."""
    key = canaries.values["api_key"]
    header = canaries.values[next(n for n in sorted(canaries.values) if n != "api_key")]
    marker = f"acctpath-{hashlib.sha256(key.encode()).hexdigest()[:12]}"
    agents = run.root / "conn-agents"
    hostile = agents / f".claude-{marker}"
    for folder in (agents / ".claude" / "projects", hostile / "projects"):
        folder.mkdir(parents=True)
    cbin, log, state, keychain = run.root / "conn-bin", run.logs / "claude-argv.log", run.logs / "claude-state", run.root / "keychain.json"
    cbin.mkdir()
    shutil.copy(FAKE_CLAUDE, cbin / "claude")
    (cbin / "claude").chmod(0o755)
    item = lambda command, env, headers: json.dumps({"command": command, "env": env, "headers": headers})  # noqa: E731
    keychain.write_text(json.dumps({
        "easy-personal/discord": item(["npx", "-y", "x@1"], {"DISCORD_TOKEN": key}, {"Authorization": header}),
        "easy-personal/leakprov": item(["/bin/sh", "-c", "sleep 5"], {"LEAK_SECRET": key}, {"X-Leak": header}),
    }))  # fmt: skip
    saved = dict(run.env)
    run.env.update({"PATH": f"{cbin}:/usr/bin:/bin", "EASY_PERSONAL_AGENT_ROOT": str(agents), "EASY_PERSONAL_TEST_KEYCHAIN": str(keychain),
                    "FAKE_CLAUDE_LOG": str(log), "FAKE_CLAUDE_STATE": str(state), "FAKE_CLAUDE_STDERR": key,
                    "FAKE_CLAUDE_FAIL": "add-json leak-conn-slack"})  # fmt: skip
    proc, errlog = run.popen(SERVER_TIMEOUT, "serve", "--no-ingest", err=run.logs / "conn-serve.stderr")
    kids, first, statuses = [], b"", {}
    try:
        lines = Lines(proc.stdout.fileno())
        first = lines.read(20) or b""
        match = re.fullmatch(rb"dashboard: http://127\.0\.0\.1:(\d+)/#([A-Za-z0-9_-]{43})\n", first)
        if not match:
            raise Fatal("connectors serve: no `dashboard:` line within 20 s")
        port, token = int(match[1]), match[2].decode()
        kids = children(proc.pid)
        base = {"Host": f"127.0.0.1:{port}", "x-dashboard-token": token}

        def call(label: str, method: str, path: str, body=None) -> dict:
            data = None if body is None else json.dumps(body).encode()
            status, blob = request(port, method, path, base, data)
            run.add(f"conn {label} {method} -> {status}", blob)
            statuses[label] = status
            try:
                return json.loads(blob.split(b"\n\n", 1)[1])
            except ValueError:
                return {}

        call("workspace add", "POST", "/api/workspaces", {"name": "leak-conn"})
        added = call("add confirm", "POST", "/api/connectors", {"provider": "slack", "workspace": "leak-conn", "values": {"SLACK_MCP_XOXB_TOKEN": key}, "confirm": True})
        cid = next((m for m in re.findall(r"\bc[0-9a-f]{8}\b", " ".join(added.get("lines", [])))), "c00000000")
        kept = json.loads(keychain.read_text()).get(f"easy-personal/{cid}", "")
        ok = statuses.get("add confirm") == 200 and added.get("status") == 0 and key in kept  # positive control
        checks.append(f"check conn add stored the secret in the keychain only  {'ok' if ok else 'FAILED'}")
        call("add preview", "POST", "/api/connectors", {"provider": "gmail", "workspace": "leak-conn", "values": {"GMAIL_OAUTH_PATH": key}})
        call("add twice", "POST", "/api/connectors", {"provider": "slack", "workspace": "leak-conn", "values": {"SLACK_MCP_XOXB_TOKEN": key}, "confirm": True})
        for label, body in (("key as provider", {"provider": key, "workspace": "leak-conn"}), ("key as workspace", {"provider": "slack", "workspace": key, "values": {"SLACK_MCP_XOXB_TOKEN": key}, "confirm": True}),
                            ("key as alias", {"provider": "slack", "workspace": "leak-conn", "alias": key, "values": {"SLACK_MCP_XOXB_TOKEN": key}, "confirm": True}),
                            ("key as value name", {"provider": "slack", "workspace": "leak-conn", "values": {key: 1}})):  # fmt: skip
            call(f"add {label}", "POST", "/api/connectors", body)
        account = call("account add", "POST", "/api/accounts", {"path": str(hostile)})
        aid = account.get("id", 0)
        call("account confirm", "POST", f"/api/accounts/{aid}/confirm", {"label": "Leak work"})
        call("account picks", "PUT", f"/api/accounts/{aid}/workspaces", {"workspaces": ["leak-conn"]})
        call("sync plan", "GET", "/api/sync")
        call("sync preview", "POST", "/api/sync", {})
        synced = call("sync apply (claude fails, a canary on its stderr)", "POST", "/api/sync", {"confirm": True})
        call("sync one account", "POST", "/api/sync", {"account": aid, "confirm": True})
        call("sync key as account", "POST", "/api/sync", {"account": key})
        call("alias key", "PUT", f"/api/connectors/{cid}/alias", {"alias": key, "confirm": True})
        call("alias", "PUT", f"/api/connectors/{cid}/alias", {"alias": "leak-chat", "confirm": True})
        call("move key", "POST", f"/api/connectors/{cid}/move", {"workspace": key, "confirm": True})
        call("move to orphan", "POST", f"/api/connectors/{cid}/move", {"workspace": "orphan", "confirm": True})
        call("legacy status", "GET", "/api/connectors/legacy")
        call("legacy preview", "POST", "/api/connectors/legacy", {})
        call("legacy adopt", "POST", "/api/connectors/legacy", {"confirm": True})
        call("remove", "DELETE", f"/api/connectors/{cid}", {"confirm": True})
        call("remove key as name", "DELETE", f"/api/connectors/{quote(key, safe='')}", {"confirm": True})
        call("remove pre-workspace", "DELETE", "/api/connectors/slack", {"confirm": True})
        call("connectors list", "GET", "/api/connectors")
        run.add("conn serve stdout (first line, token inside)", first + lines.buf)
    finally:
        rc = run.stop("connectors serve", proc, kids, signal.SIGINT)
        errlog.close()
        run.add("conn serve stdout (after stop)", proc.stdout.read() if proc.stdout else b"")
        run.add("conn serve stderr", (run.logs / "conn-serve.stderr").read_bytes())
    argv_log = log.read_text() if log.exists() else ""
    run.add("conn fake claude argv log", argv_log)
    ok = f"easy-personal/{cid}" not in json.loads(keychain.read_text())
    checks.append(f"check conn remove deleted the keychain item           {'ok' if ok else 'FAILED'}")
    ok = "boom" not in json.dumps(synced) and synced.get("status") == 1 and "add-json" in argv_log
    checks.append(f"check conn claude failed, its stderr not shown         {'ok' if ok else 'FAILED'}")
    argv_lines = [ln for ln in argv_log.splitlines() if not ln.startswith("CCD=")]
    ok = f"CCD={hostile}" in argv_log and not any(marker in ln for ln in argv_lines)
    ok = ok and marker not in (run.logs / "conn-serve.stderr").read_text(errors="replace")
    checks.append(f"check conn account path only in CLAUDE_CONFIG_DIR     {'ok' if ok else 'FAILED'}")
    adopted = statuses.get("legacy adopt") == 200 and statuses.get("legacy status") == 200
    checks.append(f"check conn legacy routes served                      {'ok' if adopted else 'FAILED'}")
    # the launcher: the provider child has the secret in its environment and not in its command line
    env = {**run.env}
    child = subprocess.Popen([str(LAUNCHER), "exec", "leakprov"], env=env, cwd=run.nowhere, stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)  # fmt: skip
    try:
        cmdline, environ = b"", b""
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline and not cmdline.startswith(b"/bin/sh"):
            time.sleep(0.05)
            with contextlib.suppress(OSError):
                cmdline = Path(f"/proc/{child.pid}/cmdline").read_bytes()
                environ = Path(f"/proc/{child.pid}/environ").read_bytes()
        run.add("launcher provider /proc cmdline", cmdline)
        ok = cmdline.startswith(b"/bin/sh") and key.encode() in environ  # positive control: the secret is in its env
        checks.append(f"check launcher provider env holds the secret, argv not  {'ok' if ok else 'FAILED'}")
    finally:
        child.kill()  # our own child, by PID
        out, err = child.communicate(timeout=15)
        run.add("launcher exec stdout", out)
        run.add("launcher exec stderr", err)
    for label, args in (("unknown", ["exec", "nope"]), ("bad name", ["exec", key]), ("headers unknown", ["headers", "nope"])):
        done = subprocess.run([str(LAUNCHER), *args], env=env, cwd=run.nowhere, capture_output=True, stdin=subprocess.DEVNULL,
                              timeout=CHILD_TIMEOUT, check=False)  # fmt: skip
        run.add(f"launcher {label} stdout", done.stdout)
        run.add(f"launcher {label} stderr", done.stderr)
    run.env.clear()
    run.env.update(saved)
    run.store_files("store (after the connector writes)")
    run.dump("store .dump (after the connector writes)")


# -- T060: the `connector ...` and `doctor` commands -----------------------------------------------------------

# (expected exit code, argv tail, stdin). `{key}` is the planted API key, typed as a secret at the prompt (stdin: the run has
# no terminal) and as a value no message may echo; `{cid}` the id the first add made; `{acct}` the planted account folder
# whose name holds the path marker. Not asked with the key: `connector add <provider>`, whose refusal names the provider the
# owner typed, as Python's does (the owner's own terminal; "decided for you" in rust/README.md).
CONNECTOR_CLI_CALLS: list[tuple[int, list[str], str]] = [
    (0, ["workspace", "add", "leak-cli"], ""),
    (0, ["connector", "add", "slack", "--workspace", "leak-cli", "--yes"], "{key}\n"),
    (0, ["connector", "add", "teams", "--workspace", "leak-cli"], "{key}\ny\n"),  # a plain field: input(), then the question
    (1, ["connector", "add", "sendgrid", "--workspace", "leak-cli"], "{key}\nn\n"),  # asked, answered no
    (2, ["connector", "add", "discord", "--workspace", "leak-cli", "--yes"], ""),  # end of input at the secret prompt
    (1, ["connector", "add", "slack", "--workspace", "leak-cli", "--yes"], "{key}\n"),  # the workspace has a slack already
    (2, ["connector", "add", "gmail", "--workspace", "{key}", "--yes"], "{key}\n"),
    (2, ["connector", "add", "gmail", "--workspace", "leak-cli", "--alias", "{key}", "--yes"], "{key}\n"),
    (0, ["connector", "list"], ""),
    (0, ["account", "add", "{acct}"], ""),
    (0, ["account", "confirm", "{acct}", "--label", "Leak cli"], ""),
    (0, ["account", "use", "{acct}", "leak-cli"], ""),
    (1, ["connector", "sync"], ""),  # asked, end of input: aborted
    (1, ["connector", "sync", "--yes"], ""),  # `claude` fails with the key on its stderr
    (1, ["connector", "sync", "--account", "{acct}", "--yes"], ""),
    (2, ["connector", "sync", "--account", "{key}", "--yes"], ""),
    (1, ["doctor"], ""),
    (2, ["connector", "alias", "{cid}", "{key}", "--yes"], ""),
    (2, ["connector", "move", "{cid}", "{key}", "--yes"], ""),
    (2, ["connector", "move", "{key}", "orphan", "--yes"], ""),
    (2, ["connector", "remove", "{key}", "--yes"], ""),
    (0, ["connector", "alias", "{cid}", "leak-chat", "--yes"], ""),
    (0, ["connector", "move", "{cid}", "orphan", "--yes"], ""),
    (1, ["connector", "adopt-legacy"], "n\n"),
    (0, ["connector", "adopt-legacy", "--yes"], ""),
    (0, ["connector", "remove", "{cid}", "--yes"], ""),
    (0, ["connector", "remove", "sendgrid", "--yes"], ""),  # adopted above: a stored connector now
    (0, ["connector", "list"], ""),
    (1, ["doctor"], ""),
]


def connector_cli(run: Run, checks: list[str], canaries: Canaries) -> None:
    """Every `connector` action and `doctor` as one process each, with no terminal (`setsid`): the key typed as a secret on
    stdin and given where a refusal could echo it; canaries in keychain secrets (the debug-build fake keychain file), in
    `claude`'s stderr (the oracle's fake, a failing `add-json`) and in an account folder's path. Scanned: each stdout and
    stderr, the fake `claude`'s argv log, the store and `.dump`. The keychain file is the keychain and is not a surface."""
    key = canaries.values["api_key"]
    header = canaries.values[next(n for n in sorted(canaries.values) if n != "api_key")]
    marker = f"cliacct-{hashlib.sha256(key.encode()).hexdigest()[:12]}"
    agents = run.root / "cli-agents"
    hostile = agents / f".claude-{marker}"
    for folder in (agents / ".claude" / "projects", hostile / "projects"):
        folder.mkdir(parents=True)
    cbin, log, state, keychain = run.root / "cli-bin", run.logs / "cli-claude-argv.log", run.logs / "cli-claude-state", run.root / "cli-keychain.json"
    cbin.mkdir()
    shutil.copy(FAKE_CLAUDE, cbin / "claude")
    (cbin / "claude").chmod(0o755)
    item = lambda command, env, headers: json.dumps({"command": command, "env": env, "headers": headers})  # noqa: E731
    # a pre-workspace item (`connectors()` above already adopted `discord` into this store)
    keychain.write_text(json.dumps({"easy-personal/sendgrid": item(["npx", "-y", "x@1"], {"SENDGRID_API_KEY": key}, {"Authorization": header})}))
    saved = dict(run.env)
    run.env.update({"PATH": f"{cbin}:/usr/bin:/bin", "EASY_PERSONAL_AGENT_ROOT": str(agents), "EASY_PERSONAL_TEST_KEYCHAIN": str(keychain),
                    "FAKE_CLAUDE_LOG": str(log), "FAKE_CLAUDE_STATE": str(state), "FAKE_CLAUDE_STDERR": key,
                    "FAKE_CLAUDE_FAIL": "add-json leak-cli-slack", "CLAUDE_CONFIG_DIR": f"/runner/{marker}"})  # fmt: skip
    cid, outputs = "", []
    try:
        for number, (want, argv, stdin) in enumerate(CONNECTOR_CLI_CALLS, start=1):
            fill = lambda text: text.replace("{key}", key).replace("{cid}", cid).replace("{acct}", str(hostile))  # noqa: E731
            argv = [fill(a) for a in argv]
            done = subprocess.run(run.argv(CHILD_TIMEOUT, *argv), env=run.env, cwd=run.nowhere, input=fill(stdin).encode(),
                                  capture_output=True, timeout=CHILD_TIMEOUT + 15, check=False, start_new_session=True)  # fmt: skip
            label = f"connector cli #{number} {' '.join(argv[:2])}".replace(key, "<key>")
            run.add(f"{label} stdout", done.stdout)
            run.add(f"{label} stderr", done.stderr)
            outputs.append(done.stdout + done.stderr)
            if done.returncode != want:
                raise Fatal(f"{label}: exit {done.returncode}, expected {want}")
            if number == 2:
                cid = next(iter(re.findall(r"\bconnector (c[0-9a-f]{8}) ", done.stdout.decode())), "")
                kept = json.loads(keychain.read_text()).get(f"easy-personal/{cid}", "")
                ok = bool(cid) and key in kept  # positive control: the typed secret is in the keychain item
                checks.append(f"check cli add stored the typed secret in the keychain  {'ok' if ok else 'FAILED'}")
    finally:
        run.env.clear()
        run.env.update(saved)
    argv_log = log.read_text() if log.exists() else ""
    run.add("connector cli fake claude argv log", argv_log)
    ok = "add-json" in argv_log and not any(b"boom" in o for o in outputs)
    checks.append(f"check cli claude failed, its stderr not shown          {'ok' if ok else 'FAILED'}")
    argv_lines = [ln for ln in argv_log.splitlines() if not ln.startswith("CCD=")]
    ok = f"CCD={hostile}" in argv_log and not any(marker in ln for ln in argv_lines)
    checks.append(f"check cli account path only in CLAUDE_CONFIG_DIR      {'ok' if ok else 'FAILED'}")
    ok = f"easy-personal/{cid}" not in json.loads(keychain.read_text()) and "easy-personal/sendgrid" not in json.loads(keychain.read_text())
    checks.append(f"check cli remove deleted the keychain items            {'ok' if ok else 'FAILED'}")
    checks.append(f"check cli connector/doctor exit codes as expected      ok ({len(CONNECTOR_CLI_CALLS)} calls, one process each)")
    run.store_files("store (after the connector cli)")
    run.dump("store .dump (after the connector cli)")


# -- the run ---------------------------------------------------------------------------------------------------


# T058: the CLI commands. (expected exit code, argv tail). `{key}` is the planted API key typed where a refused command could echo
# its input, `{projects}` the planted transcript root, `{account}` the id of the planted `.claude-work` account (read from
# `account list`). The order matters: `acme` is created before `usage --workspace acme` and `account use` read it.
# A bad `--since` / `--limit` / unknown subcommand is not asked with the key: Python's argparse and clap name the value the
# owner typed, on stderr, in the owner's own terminal ("decided for you" in TASK_REVIEW_T058.md).
CLI_CALLS: list[tuple[int, list[str]]] = [
    (0, ["ping"]),
    (0, ["workspace", "list"]),
    (0, ["account", "list"]),  # first: it gives `{account}`
    (0, ["account", "confirm", "{account}", "--label", "Claude work"]),  # an account filter needs a confirmed account
    (0, ["workspace", "add", "acme", "--color", "#336699", "--repo", "{projects}", "--repo", "/work/leak-demo"]),
    (0, ["workspace", "add", "scratch"]),
    (0, ["workspace", "edit", "acme", "--description", "company work", "--add-repo", "/work/other"]),
    (0, ["workspace", "edit", "scratch", "--rename", "scratch2"]),
    (1, ["workspace", "delete", "scratch2"]),  # no --yes and no stdin: it asks, the answer is no
    (0, ["workspace", "delete", "scratch2", "--yes"]),
    (0, ["workspace", "create", "scratch3"]),
    (0, ["workspace", "delete", "scratch3", "--yes"]),
    (0, ["workspace", "list"]),
    (0, ["usage"]), (0, ["usage", "--workspace", "acme"]), (0, ["usage", "--workspace", "acme", "--by", "project"]),
    (0, ["usage", "--account", "{account}"]), (0, ["usage", "--account", "{account}", "--by", "day"]),
    (0, ["usage", "--account", "{account}", "--workspace", "acme"]), (0, ["usage", "--since", "2026-01-01"]),
    (0, ["usage", "--until", "2027-01-01"]), (0, ["usage", "--by", "day"]), (0, ["usage", "--by", "model"]),
    (0, ["usage", "--by", "project"]), (0, ["usage", "--by", "tool"]),
    (0, ["outliers"]), (0, ["outliers", "--since", "2026-01-01"]), (0, ["outliers", "--workspace", "acme"]),
    (0, ["outliers", "--account", "{account}"]),
    (0, ["warnings"]), (0, ["warnings", "--workspace", "acme"]), (0, ["warnings", "--account", "{account}"]),
    (0, ["warnings", "--account", "{account}", "--workspace", "acme"]),
    (0, ["plan"]), (0, ["plan", "--workspace", "acme"]), (0, ["plan", "--account", "{account}"]),
    (0, ["kit"]), (0, ["kit", "--workspace", "acme"]), (0, ["kit", "--account", "{account}"]),
    (0, ["kit", "--account", "{account}", "--workspace", "acme"]),
    (0, ["kit-findings"]), (0, ["kit-findings", "--json"]),
    (0, ["kit-findings", "--json", "--limit", "10", "--since", "2026-01-01"]),
    (0, ["account", "list", "--by", "provider"]), (0, ["account", "list", "--by", "account"]),
    (0, ["account", "use", "{account}", "acme"]), (0, ["account", "use", "{account}"]), (0, ["account", "use", "{account}", "acme"]),
    (0, ["account", "add", "{agent_root}/.claude-work"]),
    (0, ["dashboard", "--help"]),
    # refused: the key is the value no message may echo
    (2, ["workspace", "add", "{key}"]), (2, ["workspace", "add", "x", "--color", "{key}"]),
    (2, ["workspace", "add", "x", "--icon", "{key}{key}{key}{key}{key}{key}{key}{key}{key}{key}"]),
    (2, ["workspace", "add", "x", "--description", "{key}" * 40]),
    (2, ["workspace", "add", "x", "--email", "{key}" * 40]), (2, ["workspace", "add", "x", "--repo", "{key}"]),
    (2, ["workspace", "add", "x", "--repo", "/ok", "--repo", "{key}"]),
    (2, ["workspace", "edit", "{key}", "--description", "x"]), (2, ["workspace", "edit", "acme", "--rename", "{key}"]),
    (2, ["workspace", "edit", "acme", "--color", "{key}"]), (2, ["workspace", "edit", "acme", "--add-repo", "{key}"]),
    (2, ["workspace", "edit", "acme", "--remove-repo", "{key}"]), (2, ["workspace", "delete", "{key}", "--yes"]),
    (2, ["workspace", "delete", "orphan", "--yes"]), (2, ["workspace", "edit", "orphan", "--description", "x"]),
    (2, ["account", "confirm", "{key}", "--label", "x"]), (2, ["account", "confirm", "{account}", "--label", "{key}" * 10]),
    (2, ["account", "confirm", "{account}", "--label", ""]), (2, ["account", "ignore", "{key}"]),
    (2, ["account", "add", "{key}"]), (2, ["account", "add", "/{key}"]), (2, ["account", "add", "{agent_root}/{key}"]),
    (2, ["account", "use", "{key}", "acme"]), (2, ["account", "use", "{account}", "{key}"]),
    (2, ["usage", "--workspace", "{key}"]), (2, ["usage", "--account", "{key}"]), (2, ["usage", "--by", "{key}"]),
    (2, ["usage", "--since", "garbage"]), (2, ["usage", "--workspace", ""]), (2, ["outliers", "--workspace", "{key}"]),
    (2, ["outliers", "--account", "{key}"]), (2, ["warnings", "--workspace", "{key}"]), (2, ["warnings", "--account", "{key}"]),
    (2, ["plan", "--workspace", "{key}"]), (2, ["plan", "--account", "{key}"]), (2, ["kit", "--workspace", "{key}"]),
    (2, ["kit", "--account", "{key}"]), (2, ["kit", "--workspace", ""]), (2, ["kit-findings", "--limit", "0"]), (2, ["kit-findings", "--since", "garbage"]),
]
# Subcommands that are not in CLI_CALLS and are driven elsewhere in this file (or by their own task's check).
CLI_ELSEWHERE = {"ingest", "schedule", "store-info", "store-check", "serve", "mcp", "app", "help", "connector", "doctor"}  # T060: `connector_cli`


def cli(run: Run, checks: list[str], canaries: Canaries) -> None:
    """Every CLI command T058 added, over real argv with a planted key, one process each: stdout and stderr are scanned."""
    key, agent_root = canaries.values["api_key"], str(run.env["EASY_PERSONAL_AGENT_ROOT"])
    helped = subprocess.run(run.argv(30, "--help"), env=run.env, cwd=run.nowhere, capture_output=True, stdin=subprocess.DEVNULL, check=False)
    run.add("cli --help stdout", helped.stdout)
    listed = re.findall(r"^  ([a-z][a-z-]*)\s", helped.stdout.decode().split("Commands:")[-1].split("Options:")[0], re.M)
    asked = {argv[0] for _, argv in CLI_CALLS}
    missing = sorted(set(listed) - asked - CLI_ELSEWHERE)
    checks.append(f"check cli commands all registered   {'ok' if listed and not missing else 'FAILED'} ({len(listed)} commands" + (f"; unregistered {missing})" if missing else ")"))
    account = ""
    for number, (want, argv) in enumerate(CLI_CALLS, start=1):
        argv = [a.replace("{key}", key).replace("{projects}", str(run.env["EASY_PERSONAL_CLAUDE_PROJECTS"])).replace("{agent_root}", agent_root)
                .replace("{account}", account) for a in argv]  # fmt: skip
        done = subprocess.run(run.argv(CHILD_TIMEOUT, *argv), env=run.env, cwd=run.nowhere, capture_output=True, stdin=subprocess.DEVNULL,
                              timeout=CHILD_TIMEOUT + 15, check=False)  # fmt: skip
        label = f"cli #{number} {argv[0]} {' '.join(a for a in argv[1:2])}".replace(key, "<key>")
        run.add(f"{label} stdout", done.stdout)
        run.add(f"{label} stderr", done.stderr)
        if done.returncode != want:
            raise Fatal(f"{label}: exit {done.returncode}, expected {want}")
        if argv[:2] == ["account", "list"] and not account:
            rows = json.loads(done.stdout)["accounts"]
            work = [r for r in rows if r["provider"] == "claude" and r["config_dir_env"] and r["folder"].endswith(".claude-work")]
            if not work:
                raise Fatal("cli: the planted .claude-work account was not listed")
            account = str(work[0]["id"])
    checks.append(f"check cli exit codes as expected    ok ({len(CLI_CALLS)} calls, one process each)")
    checks.append(f"check cli account was found         {'ok' if account else 'FAILED'}")


def context_failures(run: Run, canaries: Canaries, checks: list[str], url_host_port: str, repo: Path) -> None:
    """T051: canaries in every failure path of the context source: a 500 body, a JSON-RPC error, a tool error text, and a
    URL with a canary in the user info or the path. Nothing may echo them (stdout, stderr, the store, the data folder)."""
    key = canaries.values["api_key"]
    entry = {"root": str(repo), "root_exists": True, "last_index_at": "2026-02-02T03:04:05+00:00"}
    fakes = {"status 500 body": srv([entry], **{"break": "status500", "canary": key}),
             "json-rpc error message": srv([entry], **{"break": "rpc_error", "canary": key}),
             "tool error text": srv([entry], reject_roots=[str(repo)], canary=key)}  # fmt: skip
    for label, script in fakes.items():
        fake = Fake(script)
        fake.thread.start()
        try:
            run.env["EASY_PERSONAL_CONTEXT_URL"] = f"http://127.0.0.1:{fake.server_address[1]}/mcp"
            result = run.ingest(f"ingest context failure ({label})")
        finally:
            fake.shutdown()
            fake.server_close()
            run.env.pop("EASY_PERSONAL_CONTEXT_URL", None)
        ok = result.get("context") == {"status": "no data"}
        checks.append(f"check context {label:<22} is no data  {'ok' if ok else 'FAILED'}")
    for label, url in {"user info": f"http://{key}@{url_host_port.removeprefix('http://')}/mcp",
                       "path": f"{url_host_port}/{key}", "query": f"{url_host_port}/mcp?x={key}"}.items():  # fmt: skip
        try:
            run.env["EASY_PERSONAL_CONTEXT_URL"] = url
            result = run.ingest(f"ingest context url with a canary ({label})")
        finally:
            run.env.pop("EASY_PERSONAL_CONTEXT_URL", None)
        ok = result.get("context") == {"status": "no data"}
        checks.append(f"check context url {label:<10} is no data   {'ok' if ok else 'FAILED'}")
    run.store_files("store (after the context failures)")
    run.dump("store .dump (after the context failures)")


def exercise_store_check(run: Run, checks: list[str], label: str, home: Path, want: set[int], route: bool = True) -> None:
    """`store-check` (default, `--json`, `--full`, `--full --json`) and `GET /api/store/health` over the data folder `home`: every
    output becomes a surface. The route is asked twice (the second answer comes from the cache), then without the token, with
    POST and with HEAD. Nothing may appear beside the store, and no `easy-personal-check-*` folder may stay in the temp folder."""
    tmp, saved = Path(run.env["TMPDIR"]), run.env.get("EASY_PERSONAL_HOME")
    listing = sorted(p.name for p in home.iterdir()) if home.is_dir() else None
    run.env["EASY_PERSONAL_HOME"] = str(home)
    try:
        codes = {}
        for args in ([], ["--json"], ["--full"], ["--full", "--json"]):
            done = subprocess.run(run.argv(CHILD_TIMEOUT, "store-check", *args), env=run.env, cwd=run.nowhere,
                                  capture_output=True, stdin=subprocess.DEVNULL, timeout=CHILD_TIMEOUT + 15, check=False)  # fmt: skip
            name = f"store-check {' '.join(args)} ({label})"
            run.add(f"{name} stdout", done.stdout)
            run.add(f"{name} stderr", done.stderr)
            codes[" ".join(args)] = done.returncode
        ok = set(codes.values()) <= want
        checks.append(f"check store-check {label}, 4 forms, exit {sorted(want)}  {'ok' if ok else 'FAILED'} ({codes})")
        status = None
        if route:
            proc, errlog = run.popen(SERVER_TIMEOUT, "serve", "--no-ingest", err=run.logs / f"store-check-{label}.stderr")
            kids = []
            try:
                first = Lines(proc.stdout.fileno()).read(20) or b""
                match = re.fullmatch(rb"dashboard: http://127\.0\.0\.1:(\d+)/#([A-Za-z0-9_-]{43})\n", first)
                if not match:
                    raise Fatal("store-check serve: no `dashboard:` line within 20 s")
                port, token = int(match[1]), match[2].decode()
                kids = children(proc.pid)
                base = {"Host": f"127.0.0.1:{port}", "x-dashboard-token": token}
                for again in ("", " cached"):
                    status, blob = request(port, "GET", "/api/store/health", base)
                    run.add(f"store health route{again} ({label})", blob)
                for name, method, change in (("no token", "GET", {"x-dashboard-token": None}), ("post", "POST", {}), ("head", "HEAD", {})):
                    headers = {k: v for k, v in {**base, **change}.items() if v is not None}
                    run.add(f"store health route {name} ({label})", request(port, method, "/api/store/health", headers)[1])
            finally:
                run.stop(f"store-check serve ({label})", proc, kids, signal.SIGINT)
                errlog.close()
                run.add(f"store-check serve stderr ({label})", (run.logs / f"store-check-{label}.stderr").read_bytes())
            checks.append(f"check store health route {label}  {'ok' if status == 200 else 'FAILED'} (status {status})")
    finally:
        run.env["EASY_PERSONAL_HOME"] = saved
    left = sorted(p.name for p in tmp.iterdir() if p.name.startswith("easy-personal-check-"))
    checks.append(f"check no private copy left after {label}  {'ok' if not left else 'FAILED'} ({len(left)} folders)")
    if listing is not None:
        after = sorted(p.name for p in home.iterdir())
        checks.append(f"check store-check made no file beside the store ({label})  {'ok' if after == listing else 'FAILED'} ({len(after)} files)")


def store_check(run: Run, checks: list[str], canaries: Canaries) -> None:
    """T059: `exercise_store_check` over copies of the data folder that hold canaries: in a table's rows (the table has a
    hostile name: a table name is shown by design, so a canary in a name would always be a hit), a source path, `meta` and file
    names beside the store; healthy, then with a corrupted page; a WAL with no SHM (the private-copy branch); a source name and
    a time that are text longer than the 64 characters shown (the canary is past the cut), or a BLOB; and the error answers: no
    store, not a database, a locked store, a symlinked store, no data folder."""
    values, key = canaries.values, canaries.values["api_key"]
    home = run.logs / "check-home"
    tmp = run.logs / "tmp"
    home.mkdir()
    tmp.mkdir()
    for name in ("store.sqlite", "store.sqlite-wal"):
        if (run.data / name).exists():
            shutil.copyfile(run.data / name, home / name)
    conn = sqlite3.connect(home / "store.sqlite")
    try:
        conn.execute('CREATE TABLE "evil\x1b[31m\ntable" (a, b)')
        conn.execute('INSERT INTO "evil\x1b[31m\ntable" VALUES (?, ?)', (key, values["message_text"]))
        conn.execute("UPDATE events SET source_path = source_path || ?", (values["file_path"],))
        conn.execute("INSERT INTO source_files VALUES ('m', 'claude_code', ?, 1, 0)", (values["file_path"],))
        conn.execute("INSERT INTO meta VALUES ('planted', ?)", (key,))
        conn.commit()
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    finally:
        conn.close()
    pristine = run.logs / "check-pristine.sqlite"  # the planted store before the corrupted state; the variants start from it
    shutil.copyfile(home / "store.sqlite", pristine)
    planted = [f"{values['agent_name'].replace(' ', '_')}.txt", "store.sqlite.bak-v2", f"store.sqlite.bak-v3-{key}"]
    for name in planted:
        (home / name).write_bytes(b"x")

    def variant(name: str, sql: list[tuple[str, tuple]]) -> Path:
        """A copy of the planted folder with `sql` run over its store."""
        folder = run.logs / f"check-{name}"
        folder.mkdir()
        shutil.copyfile(pristine, folder / "store.sqlite")
        c = sqlite3.connect(folder / "store.sqlite")
        try:
            for statement, params in sql:
                c.execute(statement, params)
            c.commit()
            c.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        finally:
            c.close()
        return folder

    saved = {name: run.env.get(name) for name in ("EASY_PERSONAL_HOME", "TMPDIR", "HOME")}
    refuse("check-home", home)
    run.env["TMPDIR"] = str(tmp)  # the checks' private copies go here, so a leftover is seen and no other run is counted
    try:
        exercise_store_check(run, checks, "healthy", home, {0})
        # a source name and a time past the 64 characters shown, and the same two as BLOBs (a canary must not show in either)
        first = "(SELECT min(rowid) FROM events)"
        exercise_store_check(run, checks, "text past the cut", variant("cut", [(
            f"UPDATE events SET source = ?, ts_utc = ? WHERE rowid = {first}", ("S" * 64 + key, "T" * 64 + key))]), {0})
        exercise_store_check(run, checks, "blob source", variant("blobsource", [(
            f"UPDATE events SET source = CAST(? AS BLOB) WHERE rowid = {first}", (key,))]), {1})
        exercise_store_check(run, checks, "blob time", variant("blobtime", [(
            f"UPDATE events SET ts_utc = CAST(? AS BLOB) WHERE rowid = {first}", (key,))]), {1})
        # a WAL with no SHM: the store and its `-wal` are read from a private copy
        folder = run.logs / "check-walnoshm"
        folder.mkdir()
        shutil.copyfile(run.data / "store.sqlite", folder / "live.sqlite")
        writer = sqlite3.connect(folder / "live.sqlite")
        try:
            writer.execute("PRAGMA journal_mode=WAL")
            writer.execute("PRAGMA wal_autocheckpoint=0")
            writer.execute("INSERT OR REPLACE INTO meta VALUES ('planted-wal', ?)", (key,))
            writer.commit()
            shutil.copyfile(folder / "live.sqlite", folder / "store.sqlite")
            shutil.copyfile(folder / "live.sqlite-wal", folder / "store.sqlite-wal")
        finally:
            writer.close()
        for stale in folder.glob("live.sqlite*"):
            stale.unlink()
        wal = (folder / "store.sqlite-wal").stat().st_size
        checks.append(f"check wal-no-shm folder has a -wal and no -shm  {'ok' if wal > 0 and not (folder / 'store.sqlite-shm').exists() else 'FAILED'} ({wal} bytes)")
        exercise_store_check(run, checks, "wal without shm", folder, {0})
        # the error answers
        (run.logs / "check-empty").mkdir()
        exercise_store_check(run, checks, "no store", run.logs / "check-empty", {1})
        notdb = run.logs / "check-notdb"
        notdb.mkdir()
        (notdb / "store.sqlite").write_bytes((key + " " + values["message_text"] + "\n").encode() * 300)
        exercise_store_check(run, checks, "not a database", notdb, {1})
        linked = run.logs / "check-linked"
        linked.mkdir()
        (linked / "store.sqlite").symlink_to(pristine)
        exercise_store_check(run, checks, "symlinked store", linked, {0})
        locked = run.logs / "check-locked"
        locked.mkdir()
        shutil.copyfile(pristine, locked / "store.sqlite")
        (locked / "store.sqlite-wal").write_bytes(b"")
        (locked / "store.sqlite-shm").write_bytes(b"")
        holder = sqlite3.connect(locked / "store.sqlite", isolation_level=None)  # exclusive WAL mode: every reader is "locked"
        try:
            holder.execute("PRAGMA locking_mode=EXCLUSIVE")
            holder.execute("SELECT count(*) FROM meta").fetchone()
            exercise_store_check(run, checks, "locked", locked, {3})
        finally:
            holder.close()
        page = int(sqlite3.connect(home / "store.sqlite").execute("PRAGMA page_size").fetchone()[0])
        with open(home / "store.sqlite", "r+b") as handle:  # one page of the planted store overwritten
            handle.seek(page * max(3, (home / "store.sqlite").stat().st_size // page // 2))
            handle.write(b"\xff" * page)
        exercise_store_check(run, checks, "corrupted", home, {1})
        for name in ("EASY_PERSONAL_HOME", "HOME"):  # no data folder at all
            run.env.pop(name, None)
        done = subprocess.run(run.argv(CHILD_TIMEOUT, "store-check", "--json"), env=run.env, cwd=run.nowhere,
                              capture_output=True, stdin=subprocess.DEVNULL, timeout=CHILD_TIMEOUT + 15, check=False)  # fmt: skip
        run.add("store-check no data folder stdout", done.stdout)
        run.add("store-check no data folder stderr", done.stderr)
        checks.append(f"check store-check with no data folder, exit 1  {'ok' if done.returncode == 1 else 'FAILED'} ({done.returncode})")
    finally:
        for name, value in saved.items():
            if value is None:
                run.env.pop(name, None)
            else:
                run.env[name] = value
    left = sorted(p.name for p in tmp.iterdir() if p.name.startswith("easy-personal-check-"))
    checks.append(f"check no easy-personal-check-* folder left in the temp folder  {'ok' if not left else 'FAILED'} ({len(left)} folders)")


def drive(run: Run, canaries: Canaries, checks: list[str]) -> None:
    # T051: easy-context's service is a fake on 127.0.0.1 whose index entries carry canaries in fields the source must not keep.
    key = canaries.values["api_key"]
    repo = run.root / "plant" / "repo"
    entry = {"project_id": key, "root": str(repo), "root_exists": True, "files": 3, "slices": 9, "status": "done",
             "last_index_at": "2026-02-02T03:04:05+00:00", "summary": key, "path": f"/home/{key}", "token": key}  # fmt: skip
    other = {**entry, "root": f"/elsewhere/{key}", "summary": key}
    unreadable = {**entry, "root": "/work/leak-demo", "last_index_at": key}  # a known project; the time is not a time
    with fake_context([entry, other, unreadable]) as (url, calls, roots):
        run.env["EASY_PERSONAL_CONTEXT_URL"] = url
        drive_with_service(run, canaries, checks)
        run.env.pop("EASY_PERSONAL_CONTEXT_URL")
    conn = sqlite3.connect(run.data / "store.sqlite")
    try:
        stored = conn.execute("SELECT project_path, last_index_at_utc FROM context_index").fetchall()
    finally:
        conn.close()
    ok = sorted(stored) == sorted([(str(repo), "2026-02-02T03:04:05+00:00"), ("/work/leak-demo", None)])
    checks.append(f"check context_index holds root+time  {'ok' if ok else 'FAILED'} ({len(stored)} rows)")
    ok = bool(calls) and set(calls) == {"list_indexes"} and set(roots) == {str(repo)}
    checks.append(f"check context service saw 1 read tool {'ok' if ok else 'FAILED'} ({len(calls)} calls)")
    context_failures(run, canaries, checks, url_host_port=url.rsplit("/", 1)[0], repo=repo)
    connectors(run, checks, canaries)
    connector_cli(run, checks, canaries)


def drive_with_service(run: Run, canaries: Canaries, checks: list[str]) -> None:
    db = run.data / "store.sqlite"
    empty = run.logs / "empty-source"
    empty.mkdir()
    first = run.ingest("ingest #0 (empty source: creates the store)", "--source", str(empty))
    if first.get("new_events") != 0:
        raise Fatal("ingest #0 read events from an empty folder")
    # A reader holds a snapshot, so the Rust writer's frames stay in the WAL after it closes (unmerged content).
    pin = sqlite3.connect(db, isolation_level=None)
    try:
        pin.execute("BEGIN")
        pin.execute("SELECT count(*) FROM events").fetchone()
        result = run.ingest("ingest #1")
        open_files = run.store_files("store (writer closed, a reader still open)")
    finally:
        pin.close()
    ok = result.get("new_events", 0) > 0
    checks.append(
        f"check ingest #1 new_events > 0       {'ok' if ok else 'FAILED'} ({result.get('new_events')})"
    )
    wal = bool(open_files.get("store.sqlite-wal")) and any(
        Canaries({"control": CONTROL}).find(b) for b in open_files.values()
    )
    checks.append(
        f"check control canary in open WAL set {'ok' if wal else 'FAILED'} (WAL {len(open_files.get('store.sqlite-wal', b''))} bytes)"
    )
    closed = run.store_files("store (after close)")
    seen = bool(Canaries({"control": CONTROL}).find(closed.get("store.sqlite", b"")))
    checks.append(f"check control canary in store file   {'ok' if seen else 'FAILED'}")
    run.dump("store .dump (after ingest #1)")
    run.run("store-info", "store-info")
    run.run("store-info --json", "store-info", "--json")
    serve(run, checks, canaries.values["api_key"])
    store_check(run, checks, canaries)
    mcp(run, checks, canaries)
    mcp_http(run, checks, canaries)
    again = run.ingest("ingest #2 (same input again)")
    checks.append(
        f"check ingest #2 adds nothing         {'ok' if again.get('new_events') == 0 else 'FAILED'} ({again.get('new_events')})"
    )
    run.ingest("ingest #3 --retention-days 3650", "--retention-days", "3650")
    cli(run, checks, canaries)
    run.store_files("store (final)")
    dump = run.dump("store .dump (final)")
    conn = sqlite3.connect(db)
    try:
        events = conn.execute("SELECT count(*) FROM events").fetchone()[0]
        errors = conn.execute("SELECT SUM(tool_error_count) FROM events").fetchone()[0]
        kit = {t: conn.execute(f"SELECT count(*) FROM {t}").fetchone()[0]
               for t in ("kit_trace_events", "ui_sessions", "verifier_scores")}  # T033: now ingested, not only scanned
    finally:
        conn.close()
    ok = events > 0 and 'INSERT INTO "events"' in dump and "/work/leak-demo" in dump
    checks.append(
        f"check events rows exist              {'ok' if ok else 'FAILED'} ({events} rows)"
    )
    ok = all(kit.values())
    checks.append(
        f"check kit/ui/verifier rows exist     {'ok' if ok else 'FAILED'} ({kit['kit_trace_events']} / {kit['ui_sessions']} / {kit['verifier_scores']} rows)"
    )
    checks.append(
        f"check failed tool_result counted     {'ok' if errors == 1 else 'FAILED'} (SUM(tool_error_count) = {errors})"
    )
    listing = sorted(p.relative_to(run.data).as_posix() for p in run.data.rglob("*"))
    run.add("data folder listing", "\n".join(listing))
    stray = [name for name in listing if name not in DATA_FILES]
    checks.append(
        f"check no stray file in data folder   {'ok' if not stray else 'FAILED'} ({len(listing)} entries)"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--self-test", action="store_true", help="run the scanner's own check only")
    parser.add_argument(
        "--keep", action="store_true", help="keep the temp folder after a clean run"
    )
    opts = parser.parse_args()
    for name in gen_golden.ENV_PATHS:
        if os.environ.get(name):
            refuse(name, os.environ[name])
    base = Path(tempfile.gettempdir()).resolve()
    refuse("TMPDIR", base)
    if base.parts[1:2] == ("mnt",):
        sys.exit("refused: the temp folder is on /mnt (a Windows drive under WSL); set TMPDIR=/tmp")
    gen_golden.guard_sqlite()  # opening anything under the real ~/.easy-personal fails loudly
    canaries = Canaries.new()
    root = Path(tempfile.mkdtemp(prefix="leak-rs-", dir=base))
    report = self_test(canaries, root)
    selftest_ok = not any(line.endswith(("MISSED", "FALSE HIT", "FAILED")) for line in report)
    report.append(
        f"self-test: {'ok' if selftest_ok else 'FAILED'} (raw, json, \\u, url, base64, base64url at 3 alignments, hex; sqlite; masking)"
    )
    if opts.self_test or not selftest_ok:
        print("\n".join(report))
        shutil.rmtree(root)
        return 0 if selftest_ok else 1
    for binary in (BINARY, LAUNCHER):
        if not binary.is_file():
            sys.exit(f"no binary at {binary.relative_to(REPO)}: run `cargo build` in rust/ first")
    plant_root = root / "plant"
    planted = plant(canaries, plant_root)
    where = planted_where(canaries, plant_root)
    before = tree_digest(plant_root)
    run = Run(BINARY, root, planted)
    checks, failure = [], None
    try:
        drive(run, canaries, checks)
    except Fatal as exc:
        failure = str(exc)
    untouched = tree_digest(plant_root) == before
    checks.append(
        f"check planted sources unchanged      {'ok' if untouched else 'FAILED'} (sha256 of the tree)"
    )
    hits, scanned = scan(canaries, run.out)
    surfaces = len(run.out)
    total = sum(size(b) for b in run.out.values())
    fingerprint = hashlib.sha256("\0".join(sorted(canaries.values.values())).encode()).hexdigest()[
        :12
    ]  # differs per run
    report.append(
        f"canaries: {len(canaries.values)} (patterns in tests/leak/canaries.py: {len(PATTERNS)}), set fingerprint {fingerprint}"
    )
    for name in canaries.values:
        files = where[name]
        shown = ", ".join(files[:3]) + (f" +{len(files) - 3} more" if len(files) > 3 else "")
        report.append(
            f"canary {name:<24} planted in {shown or 'NOWHERE'}; scanned on {scanned[name]}/{surfaces} surfaces"
        )
    for label, blob in run.out.items():
        report.append(f"surface {label}: {size(blob)} bytes")
    report.extend(checks)
    problems = [c for c in checks if " FAILED" in c]
    problems += [f"canary {n} was planted nowhere" for n, files in where.items() if not files]
    problems += [
        f"canary {n} was scanned on {k} of {surfaces} surfaces"
        for n, k in scanned.items()
        if k != surfaces
    ]
    if len(canaries.values) != len(PATTERNS):
        problems.append("the canary set differs from PATTERNS")
    if failure:
        problems.append(failure)
    for name, form, label, cut in hits:
        report.append(f"HIT canary {name} as {form} in [{label}]: {cut}")
    text = "\n".join(report)
    if canaries.find(text):  # the report must not become a leak
        print("leak check: the report itself would hold a canary; it was withheld")
        print(f"temp folder kept: {root}")
        return 1
    print(text)
    for problem in problems:
        print(f"FAILED: {problem}")
    if hits or problems:
        print(f"temp folder kept: {root}")
        print(
            f"leak check: {len(hits)} hits over {len(canaries.values)} canaries, {surfaces} surfaces, {total} bytes"
            + (f"; {len(problems)} failed checks" if problems else "")
        )
        return 1
    if opts.keep:
        print(f"temp folder kept: {root}")
    else:
        shutil.rmtree(root)
    print(
        f"leak check: 0 hits over {len(canaries.values)} canaries, {surfaces} surfaces, {total} bytes"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
