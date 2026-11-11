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
import hashlib
import http.client
import json
import os
import re
import select
import shutil
import signal
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
sys.path.insert(0, str(PARITY))
sys.path.insert(0, str(REPO / "tests"))
import gen_golden

from fakes import jobs_fakes
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
API_ROUTE = re.compile(r'\.route\("(/api/[^"]+)"')
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
    "/api/connectors": ["", "?workspace=acme"],
    "/api/warnings": ["", "?workspace=acme", "?account=1", "?account=nope", "?workspace=nope"],
    "/api/plan": ["", "?workspace=acme", "?account=1", "?account=nope"],
}  # fmt: skip
# Requests the guard or the static fallback answers: (label, method, path, header changes; None drops a header).
GUARD_CALLS = [
    ("no token", "GET", "/api/usage/summary", {"x-dashboard-token": None}),
    ("wrong token", "GET", "/api/usage/summary", {"x-dashboard-token": "wrong"}),
    ("wrong host", "GET", "/api/usage/summary", {"Host": "localhost:1"}),
    ("wrong origin", "GET", "/api/usage/summary", {"Origin": "http://evil.invalid"}),
    ("write method", "POST", "/api/workspaces", {}),
    ("unknown route", "GET", "/api/nope", {}),
    ("static index", "GET", "/", {"x-dashboard-token": None}),
    ("static file", "GET", "/index.html", {"x-dashboard-token": None}),
    ("static missing", "GET", "/nope.js", {"x-dashboard-token": None}),
    ("static dotfile", "GET", "/.env", {"x-dashboard-token": None}),
    ("static traversal", "GET", "/%2e%2e/%2e%2e/etc/passwd", {"x-dashboard-token": None}),
    ("double slash", "GET", "//api/version", {"x-dashboard-token": None}),
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
    "connector_catalog": [{}, {"workspace": "acme"}],
    "connector_status": [{}, {"account": "1"}],
    "connector_suggestions": [{}, {"workspace": "acme"}],
}  # fmt: skip
MCP_RAW = [  # (label, line) answered by the protocol layer, not a tool
    (
        "unknown tool",
        '{"jsonrpc":"2.0","id":900,"method":"tools/call","params":{"name":"nope","arguments":{}}}',
    ),
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
    return lines


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


def request(port: int, method: str, path: str, headers: dict[str, str]) -> tuple[int, bytes]:
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=15)
    try:
        conn.putrequest(method, path, skip_host=True, skip_accept_encoding=True)
        for key, value in headers.items():
            conn.putheader(key, value)
        conn.endheaders()
        res = conn.getresponse()
        body = res.read()
        head = f"{res.status} {res.reason}\n" + "".join(f"{k}: {v}\n" for k, v in res.getheaders())
        return res.status, head.encode() + b"\n" + body
    finally:
        conn.close()


def serve(run: Run, checks: list[str]) -> None:
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
        for label, method, path, change in GUARD_CALLS:
            headers = {k: v for k, v in {**base, **change}.items() if v is not None}
            status, blob = request(port, method, path, headers)
            run.add(f"api {label} {method} {path} -> {status}", blob)
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


def mcp(run: Run, checks: list[str]) -> None:
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


# -- the run ---------------------------------------------------------------------------------------------------


def drive(run: Run, canaries: Canaries, checks: list[str]) -> None:
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
    serve(run, checks)
    mcp(run, checks)
    again = run.ingest("ingest #2 (same input again)")
    checks.append(
        f"check ingest #2 adds nothing         {'ok' if again.get('new_events') == 0 else 'FAILED'} ({again.get('new_events')})"
    )
    run.ingest("ingest #3 --retention-days 3650", "--retention-days", "3650")
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
    if not BINARY.is_file():
        sys.exit(f"no binary at {BINARY.relative_to(REPO)}: run `cargo build` in rust/ first")
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
