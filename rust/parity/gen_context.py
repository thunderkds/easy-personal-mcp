#!/usr/bin/env python3
"""T051 oracle for the `context` source: Python's `sources/context.py` (read-only) against a scripted fake of
easy-context's MCP service on 127.0.0.1. Writes `golden-context/golden.json`.

Each case lists the projects the store knows, the script of the fake service and the URL. The script says how the
fake answers (`mode`: json / sse / chunked, with `session`, `reply`, `reject_roots`, `delay_s`, `break`, and for the leak check a `canary` put in the failure replies). Python's
`context.ingest` runs once per case and the golden keeps what it stored in `context_index`, the tools the fake saw
(`calls`), the root headers it saw (`roots`) and the sessions it was asked to end (`deletes`). The Rust test
(`rust/crates/core/tests/context_parity.rs`) plays the same cases against a Rust fake that follows the same script
and compares. The "normal" case also runs against the real FastMCP fake of `tests/fakes/mcp_server.py` and must give
the same rows: that is what ties this raw fake to a real MCP server.

Cases Python cannot answer (it raises on a number outside the years 1-9999, or on a root listed twice) are not here;
Rust's answer for them is pinned by its unit tests (`ingest/context.rs`). A reply over the 1 MiB cap is a Rust-only
rule (Python has no cap) and is tested there too.

    rust/parity/.venv/bin/python rust/parity/gen_context.py           # (re)write golden-context/
    rust/parity/.venv/bin/python rust/parity/gen_context.py --check   # regenerate in memory and compare: exit 0 equal

Temp folders are made under `rust/target/` and removed; `EASY_PERSONAL_HOME` points at one of them. It refuses to run
when an env var points at the owner's real ~/.easy-personal, ~/.claude*, ~/.codex*. POSIX only.
"""

import argparse
import json
import os
import shutil
import socket
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(REPO / "src"))
sys.path.insert(0, str(REPO / "tests"))

OUT = HERE / "golden-context"
WORK = REPO / "rust" / "target"


def refuse_real_paths() -> None:
    home = Path.home()
    for name, value in os.environ.items():
        if any(str(home / d) in value for d in (".easy-personal", ".claude", ".codex")):
            sys.exit(f"refusing to run: {name} points at a real data folder")


refuse_real_paths()

from easy_personal.core import store  # noqa: E402
from easy_personal.sources import context  # noqa: E402

ROOT = "{ROOT}"


def entry(root: str, last: object, exists: bool = True) -> dict:
    return {"project_id": "id", "root": root, "root_exists": exists, "files": 3, "slices": 9,
            "last_index_at": last, "status": "done"}  # fmt: skip


def p(name: str, dir_: bool = True) -> dict:
    return {"path": f"{ROOT}/{name}", "dir": dir_}


def idx(name: str, last: object = "2026-10-21T10:00:00+00:00", exists: bool = True) -> dict:
    return entry(f"{ROOT}/{name}", last, exists)


TIME_FORMS = [
    ("t01", "2026-10-21T10:00:00.000+00:00"), ("t02", "2026-10-21T10:00:00"),
    ("t03", "2026-10-21T12:00:00+02:00"), ("t04", "2026-10-21T10:00:00.123456Z"),
    ("t05", "2026-10-21"), ("t06", 1760000000), ("t07", 1760000000.5), ("t08", True),
    ("t09", None), ("t10", "garbage"), ("t11", ""), ("t12", "9000-01-01T00:00:00+00:00"),
    ("t13", "0001-01-01T00:00:00+00:00"), ("t14", "2026-10-21T10:00:00.1234567+00:00"),
    ("t15", "2026-10-21 10:00:00+00:00"), ("t16", "20261021T100000Z"), ("t17", 1760000000.1234565),
    ("t18", -1), ("t19", {"a": 1}), ("t20", "2026-W43-3T10:00:00+00:00"),
    ("t21", "2026-10-21T10:00:00.999999+00:00"), ("t22", 0), ("t23", "2026-10-21T10:00:00-05:30"),
    ("t24", False), ("t25", [1]),
]  # fmt: skip

TWO = [p("a"), p("b")]
NORMAL_INDEXES = [idx("a", "2026-10-21T10:00:00.000+00:00"), idx("b"), idx("other"),
                  idx("c", exists=False)]  # fmt: skip


def case(name, projects, server=None, url="http://127.0.0.1:{PORT}/mcp", timeout=None, seed=None, **extra) -> dict:
    return {"name": name, "projects": projects, "server": server, "url": url, "timeout_s": timeout,
            "seed_rows": seed or [], **extra}  # fmt: skip


def srv(indexes, **script) -> dict:
    return {"mode": "json", "session": False, "reply": "structured", "reject_roots": [], "delay_s": 0,
            "break": None, "indexes": indexes, **script}  # fmt: skip


CASES = [
    case("normal", TWO + [p("c", False)], srv(NORMAL_INDEXES)),
    case("empty_list", TWO, srv([])),
    case("root_does_not_exist", [p("live"), p("gone", False)],
         srv([idx("live"), idx("gone", exists=False), idx("live2", exists=False)])),  # fmt: skip
    case("no_project_folder_exists", [p("a", False), p("b", False)], srv([idx("a"), idx("b")])),
    case("no_projects", [], srv([idx("a")])),
    case("service_down", TWO, None, url="http://127.0.0.1:{DEAD}/mcp",
         seed=[[f"{ROOT}/a", "2026-01-01T00:00:00+00:00"]]),  # fmt: skip
    case("slow_service", TWO, srv([idx("a")], delay_s=3), timeout=0.3,
         seed=[[f"{ROOT}/a", "2026-01-01T00:00:00+00:00"]]),  # fmt: skip
    case("replaces_old_rows", TWO, srv([idx("b", "2026-02-02T00:00:00+00:00")]),
         seed=[[f"{ROOT}/a", "2026-01-01T00:00:00+00:00"], [f"{ROOT}/zz", "x"]]),  # fmt: skip
    case("time_forms", [p(n) for n, _ in TIME_FORMS], srv([idx(n, v) for n, v in TIME_FORMS])),
    # bad replies: all are "no data" and the old rows go
    case("bad_status_500", TWO, srv([idx("a")], **{"break": "status500"}), seed=[[f"{ROOT}/a", "old"]]),
    case("bad_redirect_302", TWO, srv([idx("a")], **{"break": "redirect"})),
    case("bad_garbage_json", TWO, srv([idx("a")], **{"break": "garbage"})),
    case("bad_text_plain", TWO, srv([idx("a")], **{"break": "text"})),
    case("bad_empty_body", TWO, srv([idx("a")], **{"break": "empty"})),
    case("bad_jsonrpc_error", TWO, srv([idx("a")], **{"break": "rpc_error"})),
    case("bad_no_content", TWO, srv([idx("a")], reply="no_content")),
    case("bad_text_not_json", TWO, srv([idx("a")], reply="text_garbage")),
    case("bad_indexes_not_a_list", TWO, srv(5)),
    case("bad_reply_is_a_list", TWO, srv([idx("a")], reply="list")),
    case("bad_items_are_not_dicts", TWO, srv([1, "x", None, [idx("a")], idx("b")])),
    case("tool_error_for_every_root", [p("a"), p("b"), p("c"), p("d")],
         srv([idx("a")], reject_roots=[f"{ROOT}/a", f"{ROOT}/b", f"{ROOT}/c", f"{ROOT}/d"])),  # fmt: skip
    # the next root is tried when the service rejects one; only three are tried
    case("next_root_after_a_rejected_one", [p("a"), p("b"), p("c")], srv([idx("c")], reject_roots=[f"{ROOT}/a"])),
    case("fourth_root_is_never_tried", [p("a"), p("b"), p("c"), p("d")],
         srv([idx("d")], reject_roots=[f"{ROOT}/a", f"{ROOT}/b", f"{ROOT}/c"])),  # fmt: skip
    case("root_with_non_ascii_is_no_data", [p("é")], srv([idx("é")])),
    # reply shapes
    case("reply_in_text_block", TWO, srv(NORMAL_INDEXES, reply="text")),
    case("reply_wrapped_in_result", TWO, srv(NORMAL_INDEXES, reply="wrapped")),
    case("sse", TWO, srv(NORMAL_INDEXES, mode="sse")),
    case("chunked_json", TWO, srv(NORMAL_INDEXES, mode="chunked")),
    case("chunked_sse", TWO, srv(NORMAL_INDEXES, mode="sse+chunked")),
    case("stateful_session", TWO, srv(NORMAL_INDEXES, session=True)),
    case("stateful_session_sse", TWO, srv(NORMAL_INDEXES, session=True, mode="sse")),
    case("url_without_path", TWO, srv(NORMAL_INDEXES), url="http://127.0.0.1:{PORT}"),
    # refused hosts: the fake is up, and must see nothing
    case("refused_localhost", TWO, srv(NORMAL_INDEXES), url="http://localhost:{PORT}/mcp"),
    case("refused_userinfo", TWO, srv(NORMAL_INDEXES), url="http://127.0.0.1@evil.example/mcp"),
    case("refused_userinfo_2", TWO, srv(NORMAL_INDEXES), url="http://evil.example@127.0.0.1:{PORT}/mcp"),
    case("refused_https", TWO, srv(NORMAL_INDEXES), url="https://127.0.0.1:{PORT}/mcp"),
    case("refused_any_address", TWO, srv(NORMAL_INDEXES), url="http://0.0.0.0:{PORT}/mcp"),
    case("refused_ipv6", TWO, srv(NORMAL_INDEXES), url="http://[::1]:{PORT}/mcp"),
    case("refused_lookalike", TWO, srv(NORMAL_INDEXES), url="http://127.0.0.1.evil.example/mcp"),
    case("refused_other_loopback", TWO, srv(NORMAL_INDEXES), url="http://127.0.0.2:{PORT}/mcp"),
    case("refused_file", TWO, srv(NORMAL_INDEXES), url="file:///x", seed=[[f"{ROOT}/a", "old"]]),
]  # fmt: skip

# call_tool-level cases: the allow-list
TOOL_CASES = [
    {"name": "refused_tool_index_path", "tool": "index_path"},
    {"name": "refused_tool_search", "tool": "search_semantic"},
    {"name": "refused_tool_empty", "tool": ""},
    {"name": "allowed_tool_list_indexes", "tool": "list_indexes"},
    {"name": "allowed_tool_get_stats", "tool": "get_stats"},
]


def sub(value, work: Path, port: int | None, dead: int):
    if isinstance(value, str):
        return value.replace(ROOT, str(work)).replace("{PORT}", str(port)).replace("{DEAD}", str(dead))
    if isinstance(value, list):
        return [sub(v, work, port, dead) for v in value]
    if isinstance(value, dict):
        return {k: sub(v, work, port, dead) for k, v in value.items()}
    return value


def unsub(value, work: Path):
    if isinstance(value, str):
        return value.replace(str(work), ROOT)
    if isinstance(value, list):
        return [unsub(v, work) for v in value]
    return value


class Fake(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, script: dict):
        super().__init__(("127.0.0.1", 0), Handler)
        self.script = script
        self.calls: list[str] = []
        self.roots: list = []
        self.deletes = 0
        self.thread = threading.Thread(target=self.serve_forever, daemon=True)

    def handle_error(self, request, client_address):  # a client that gave up is not an error here
        pass


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def send(self, status, body: bytes, ctype="application/json", extra=None, chunked=False):
        self.send_response(status)
        for k, v in (extra or {}).items():
            self.send_header(k, v)
        if body or status != 202:
            self.send_header("Content-Type", ctype)
        if chunked:
            self.send_header("Transfer-Encoding", "chunked")
        else:
            self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if chunked:
            for i in range(0, len(body), 37):
                part = body[i:i + 37]
                self.wfile.write(f"{len(part):x}\r\n".encode() + part + b"\r\n")
            self.wfile.write(b"0\r\n\r\n")
        else:
            self.wfile.write(body)

    def message(self, payload: dict, extra=None):
        mode = self.server.script["mode"]
        text = json.dumps(payload)
        if mode.startswith("sse"):
            body, ctype = f"event: message\ndata: {text}\n\n".encode(), "text/event-stream"
        else:
            body, ctype = text.encode(), "application/json"
        self.send(200, body, ctype, extra, chunked=mode.endswith("chunked"))

    def do_GET(self):
        self.send(405, b"", "text/plain")

    def do_DELETE(self):
        self.server.deletes += 1
        self.send(200, b"", "text/plain")

    def do_POST(self):
        script = self.server.script
        raw = self.rfile.read(int(self.headers.get("content-length", 0)))
        message = json.loads(raw)
        breaking = script["break"]
        if breaking == "status500":
            return self.send(500, (script.get("canary") or "boom").encode(), "text/plain")
        if breaking == "redirect":
            self.send_response(302)
            self.send_header("Location", "http://127.0.0.1:1/elsewhere")
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        if breaking == "garbage":
            return self.send(200, b"{not json", "application/json")
        if breaking == "text":
            return self.send(200, b"hello", "text/plain")
        if breaking == "empty":
            return self.send(200, b"", "application/json")
        method = message.get("method")
        if script["session"] and method != "initialize" and self.headers.get("mcp-session-id") != "sess-1":
            return self.send(400, b"no session", "text/plain")
        if method == "initialize":
            extra = {"mcp-session-id": "sess-1"} if script["session"] else None
            version = message["params"]["protocolVersion"]
            result = {"protocolVersion": version, "capabilities": {"tools": {}},
                      "serverInfo": {"name": "fake-easy-context", "version": "1"}}  # fmt: skip
            return self.message({"jsonrpc": "2.0", "id": message["id"], "result": result}, extra)
        if method == "notifications/initialized":
            return self.send(202, b"", "text/plain")
        if method == "tools/call":
            return self.tool_call(message)
        if method == "tools/list":  # the Python SDK lists the tools after a call, to check the result's schema
            tools = [{"name": n, "description": n, "inputSchema": {"type": "object", "properties": {}}}
                     for n in ("list_indexes", "get_stats")]  # fmt: skip
            return self.message({"jsonrpc": "2.0", "id": message["id"], "result": {"tools": tools}})
        self.send(400, b"?", "text/plain")

    def tool_call(self, message):
        script = self.server.script
        name = message["params"]["name"]
        root = self.headers.get("x-easy-context-root")
        self.server.calls.append(name)
        self.server.roots.append(root)
        if script["delay_s"]:
            time.sleep(script["delay_s"])
        reply_id = message["id"]
        if script["break"] == "rpc_error":
            return self.message({"jsonrpc": "2.0", "id": reply_id, "error": {"code": -32000, "message": script.get("canary") or "x"}})
        if root in script["reject_roots"]:
            result = {"content": [{"type": "text", "text": script.get("canary") or "outside the workspace"}], "isError": True}
        else:
            body = {"project_id": "x", "root": root, "indexes": script["indexes"]}
            kind = script["reply"]
            text = json.dumps(body)
            if kind == "structured":
                result = {"content": [{"type": "text", "text": text}], "structuredContent": body, "isError": False}
            elif kind == "text":
                result = {"content": [{"type": "text", "text": text}], "isError": False}
            elif kind == "wrapped":
                wrapped = {"result": body}
                result = {"content": [{"type": "text", "text": json.dumps(wrapped)}], "structuredContent": wrapped,
                          "isError": False}  # fmt: skip
            elif kind == "no_content":
                result = {"structuredContent": body, "isError": False}
            elif kind == "text_garbage":
                result = {"content": [{"type": "text", "text": "nope"}], "isError": False}
            elif kind == "list":
                result = {"content": [{"type": "text", "text": "[1]"}], "structuredContent": None, "isError": False}
                result["content"][0]["text"] = json.dumps([body])
            else:
                raise ValueError(kind)
        self.message({"jsonrpc": "2.0", "id": reply_id, "result": result})


def free_port() -> int:
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


def known(conn, paths: list[str]) -> None:
    for i, path in enumerate(paths):
        conn.execute(
            "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, "
            "project_path, ts_utc) VALUES ('m', 'claude_code', 'p', 0, ?, 0, ?, '2026-10-01T00:00:00+00:00')",
            (i + 1, path),
        )
    conn.commit()


def run_case(spec: dict, work: Path) -> dict:
    work.mkdir(parents=True)
    paths = []
    for project in sub(spec["projects"], work, None, 0):
        paths.append(project["path"])
        if project["dir"]:
            Path(project["path"]).mkdir(parents=True, exist_ok=True)
    conn = store.connect(work / "store.sqlite")
    known(conn, paths)
    for path, at in sub(spec["seed_rows"], work, None, 0):
        conn.execute("INSERT INTO context_index VALUES (?, ?)", (path, at))
    conn.commit()
    fake = None
    port = None
    if spec["server"] is not None:
        fake = Fake(sub(spec["server"], work, None, 0))
        fake.thread.start()
        port = fake.server_address[1]
    url = sub(spec["url"], work, port, free_port())
    old = context.TIMEOUT_S
    if spec["timeout_s"]:
        context.TIMEOUT_S = spec["timeout_s"]
    try:
        result = context.ingest(conn, url=url)
    finally:
        context.TIMEOUT_S = old
    rows = [list(r) for r in conn.execute("SELECT project_path, last_index_at_utc FROM context_index ORDER BY 1")]
    conn.close()
    out = {"name": spec["name"], "result": result, "rows": unsub(rows, work)}
    if fake is not None:
        fake.shutdown()
        fake.server_close()
        out.update(calls=fake.calls, roots=unsub(fake.roots, work), deletes=fake.deletes)
    return out


def run_tool_case(spec: dict) -> dict:
    fake = Fake(srv([]))
    fake.thread.start()
    url = f"http://127.0.0.1:{fake.server_address[1]}/mcp"
    try:
        context.call_tool(spec["tool"], url=url, root="/x")
        outcome = "ok"
    except context.Refused:
        outcome = "Refused"
    except context.ToolError:
        outcome = "ToolError"
    except context.ContextUnavailable:
        outcome = "Unavailable"
    fake.shutdown()
    fake.server_close()
    return {"name": spec["name"], "tool": spec["tool"], "outcome": outcome, "calls": fake.calls, "roots": fake.roots}


def real_server_rows(work: Path) -> list:
    """The `normal` case against the real FastMCP fake of the Python tests: the rows must equal the raw fake's."""
    from fakes.mcp_server import fake_context

    work.mkdir(parents=True)
    paths = [str(work / "a"), str(work / "b"), str(work / "c")]
    (work / "a").mkdir()
    (work / "b").mkdir()
    conn = store.connect(work / "store.sqlite")
    known(conn, paths)
    entries = sub(NORMAL_INDEXES, work, None, 0)
    with fake_context(entries) as (url, calls, roots):
        result = context.ingest(conn, url=url)
    rows = [list(r) for r in conn.execute("SELECT project_path, last_index_at_utc FROM context_index ORDER BY 1")]
    conn.close()
    return [result, unsub(rows, work), calls, unsub(roots, work)]


def build() -> dict:
    WORK.mkdir(exist_ok=True)
    base = Path(tempfile.mkdtemp(prefix="gen-context-", dir=WORK))
    try:
        os.environ["EASY_PERSONAL_HOME"] = str(base / "home")
        cases = [run_case(spec, base / f"c{i}") for i, spec in enumerate(CASES)]
        tools = [run_tool_case(spec) for spec in TOOL_CASES]
        normal = next(c for c in cases if c["name"] == "normal")
        real = real_server_rows(base / "real")
        if real != [normal["result"], normal["rows"], normal["calls"], normal["roots"]]:
            sys.exit(f"the raw fake and the real FastMCP fake disagree on `normal`:\n{real}\n{normal}")
    finally:
        shutil.rmtree(base, ignore_errors=True)
    return {"generator": "rust/parity/gen_context.py", "cases": [
        {**spec, **out} for spec, out in zip(CASES, cases, strict=True)], "tool_cases": tools}  # fmt: skip


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    text = json.dumps(build(), indent=1, sort_keys=True, ensure_ascii=False) + "\n"
    target = OUT / "golden.json"
    if args.check:
        same = target.exists() and target.read_text() == text
        print("golden-context: equal" if same else "golden-context: DIFFERENT")
        return 0 if same else 1
    OUT.mkdir(exist_ok=True)
    target.write_text(text)
    print(f"wrote {target.relative_to(REPO)}: {len(CASES)} cases, {len(TOOL_CASES)} tool cases")
    return 0


if __name__ == "__main__":
    sys.exit(main())
