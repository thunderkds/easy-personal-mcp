#!/usr/bin/env python3
"""Parity run of every Python CLI subcommand, API route and MCP tool against Rust (T061, Retire Python 4).

    cd rust && cargo build && env -u CLAUDE_CONFIG_DIR ./parity/.venv/bin/python parity/run_parity.py     # writes docs/PARITY_T061.md
    ./parity/.venv/bin/python parity/run_parity.py --check      # run, compare with the committed report: exit 0 equal
    ./parity/.venv/bin/python parity/run_parity.py --list       # print the inventory read from Python and exit

What it does. The inventory is read from Python itself (argparse tree, Starlette route table, FastMCP tool and prompt managers).
Each surface has one or more probes (`probes.py`-style tables below). A probe builds a world (a fresh folder: a copy of
`store-rich`, the agent folders, a fake `claude`, a fake keychain, fake `systemctl` and `crontab`), runs the **Python** app in it,
throws the world away, builds the same world at the same path and runs the **Rust** app. They never run together on one store.
Both apps are real child processes with an environment built from scratch (no `HOME`, no `CLAUDE_CONFIG_DIR`, no display, a
dead D-Bus address): Python as `python -c "...cli.main()"` over this checkout's `src/`, Rust as `target/debug/easy-personal-rs`.
Nothing here opens `~/.easy-personal`, a `~/.claude*` or `~/.codex*` folder, the real keychain or the real `claude`.

Compared: CLI = exit code, stdout, stderr; route = status, content type, JSON body (parsed; raw key order is reported, see the
report's rules); MCP = the `tools/list` entry and the result of each call; and, after each probe, the store rows that changed, the
keychain items, the `claude` calls and the fake `claude`'s state. A difference is `differs: README <list> item <n>` only when the
README item exists, its text holds the stated words, and the item's own transformation of both outputs makes them equal
(`EXPLAINS`); otherwise it is `differs: new`, a finding with a reproduction. Nothing is fixed here.

The clock: Rust has no clock hook, so both apps run on the machine's clock; the clock's own values (`as_of`, `as_of_utc`,
`saved_utc`) are `<NOW>` (rule N3) and the rich store's events are months old, so every rule abstains, as in `gen_cli.py`. The
frozen-clock proof of the rules stays in the goldens.
"""

import argparse
import contextlib
import copy
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

sys.dont_write_bytecode = True
PARITY = Path(__file__).resolve().parent
RUST = PARITY.parent
REPO = RUST.parent
BINARY = RUST / "target" / "debug" / "easy-personal-rs"
RS_LAUNCHER = RUST / "target" / "debug" / "easy-personal-launch"
REPORT = RUST / "docs" / "PARITY_T061.md"
README = RUST / "README.md"
VENV_PY = Path(sys.executable)
PY_LAUNCHER = VENV_PY.parent / "easy-personal-launch"
sys.path.insert(0, str(PARITY))
sys.path.insert(0, str(REPO / "tests"))
import gen_cli  # noqa: E402
import gen_connectors as gc  # noqa: E402
import gen_golden  # noqa: E402

ZONES = ["UTC", "Pacific/Kiritimati", "Pacific/Pago_Pago"]
TIMEOUT = 90
SECRET = "xoxb-PLANTED-t061-secret"
PY_CLI = "import sys; from easy_personal.surfaces.cli import main; sys.exit(main())"
PY_SERVE = (
    "import sys\nfrom pathlib import Path\nfrom easy_personal.core import live\nfrom easy_personal.surfaces import api\n"
    "api.static_dir = lambda: Path(sys.argv[1])\n"
    "tail = live.LiveTail().start()\ntry:\n    api.serve(open_browser=lambda url: None)\nfinally:\n    tail.stop()\n"
)
FAKE_KEYRING = '''import json, os
from keyring.backend import KeyringBackend
from keyring.errors import PasswordDeleteError


class FileKeyring(KeyringBackend):
    """A keyring in one JSON file {service: secret}: the same file the Rust debug build's fake keychain reads."""

    priority = 1

    def _read(self):
        try:
            with open(os.environ["PARITY_KEYCHAIN"]) as handle:
                return json.load(handle)
        except FileNotFoundError:
            return {}

    def _write(self, items):
        with open(os.environ["PARITY_KEYCHAIN"], "w") as handle:
            json.dump(items, handle)

    def get_password(self, service, username):
        return self._read().get(service)

    def set_password(self, service, username, password):
        items = self._read()
        items[service] = password
        self._write(items)

    def delete_password(self, service, username):
        items = self._read()
        if service not in items:
            raise PasswordDeleteError("absent")
        del items[service]
        self._write(items)
'''
STATIC_FILES = {"index.html": "<!doctype html><title>t061</title><div id=root></div>\n", "assets/app.js": "console.log('t061');\n",
                "favicon.svg": "<svg xmlns='http://www.w3.org/2000/svg'/>\n", "404.html": ""}
STATIC_FILES.pop("404.html")


class Refused(SystemExit):
    pass


# ---------------------------------------------------------------------------------------------------------------- worlds


def spec(store="rich", setup=(), tree=None, dirs=(), keychain=None, claude=None, fail=None, projects="none", ccd=False, tz="UTC",
         launcher=True, static=False, path_only_bin=False):
    """A world. `store`: rich | rich-disk | none. `keychain`: {service: json text} | "none". `claude`: None | {ccd: {name: [cmd, args]}}."""
    return {"store": store, "setup": list(setup), "tree": tree or {"dirs": [], "files": {}}, "dirs": list(dirs), "keychain": keychain,
            "claude": claude, "fail": fail, "projects": projects, "ccd": ccd, "tz": tz, "launcher": launcher, "static": static}


def launcher_of(app):
    return str(PY_LAUNCHER if app == "py" else RS_LAUNCHER)


PATH_COLUMNS = (("source_files", "path"), ("events", "source_path"), ("events", "project_path"), ("kit_trace_events", "source_path"),
                ("kit_trace_events", "project_path"), ("context_index", "project_path"), ("agent_accounts", "folder"),
                ("agent_accounts", "config_dir_env"), ("workspace_repos", "repo_path"))  # where the saved store holds `/work/rich`


def rewrite_paths(conn, old, new):
    for table, column in PATH_COLUMNS:
        conn.execute(f'UPDATE "{table}" SET "{column}" = replace("{column}", ?, ?)', (old, new))


def build_world(app, root: Path, sp, canaries_dir: Path | None = None) -> dict:
    """Make the world at `root` for `app`; returns the paths the run needs."""
    shutil.rmtree(root, ignore_errors=True)
    root.mkdir(parents=True)
    work = str(root)
    gen_cli.build_tree(root, sp["tree"])
    for name in ("home", "em-none", "verifier-none", "projects-none", "bin", "pylib", "claude-state", "static"):
        (root / name).mkdir(exist_ok=True)
    for d in sp["dirs"]:
        (root / d).mkdir(parents=True, exist_ok=True)
    if sp["store"] in ("rich", "rich-disk"):
        shutil.copy(gc.SOURCE, root / "home" / "store.sqlite")
        conn = sqlite3.connect(root / "home" / "store.sqlite")
        conn.execute("PRAGMA foreign_keys = ON")
        if sp["store"] == "rich-disk":
            gen_golden.FIXTURES = gen_golden.RICH_FIXTURES
            gen_golden.rich_work_tree(root)
            for transcript in (root / "agent-root").rglob("*.jsonl"):  # the saved store knows them under other inodes: Python's live tail would
                transcript.unlink()  # store them again (README T049 item 1: Rust's tail never writes the store)
            rewrite_paths(conn, "/work/rich", work)
        else:
            conn.execute("UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?), config_dir_env = replace(config_dir_env, '/work/rich', ?)",
                         (work, work))
        for sql in sp["setup"]:
            conn.execute(sql.replace("@W@", work))
        conn.commit()
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        conn.close()
    for rel, text in STATIC_FILES.items():
        (root / "static" / rel).parent.mkdir(parents=True, exist_ok=True)
        (root / "static" / rel).write_text(text)
    for tool in ("systemctl", "crontab"):  # the machine's timers are not the test's business: both answer "not available"
        (root / "bin" / tool).write_text("#!/bin/sh\nexit 1\n")
        (root / "bin" / tool).chmod(0o755)
    if sp["claude"] is not None:
        shutil.copy(gc.FAKE_CLAUDE, root / "bin" / "claude")
        (root / "bin" / "claude").chmod(0o755)
        saved, gc.FAKE_LAUNCHER = gc.FAKE_LAUNCHER, launcher_of(app)
        try:
            gc.write_state(root / "claude-state", sp["claude"], work)
        finally:
            gc.FAKE_LAUNCHER = saved
    (root / "pylib" / "fakekeyring.py").write_text(FAKE_KEYRING)
    keychain = root / "keychain.json"
    if sp["keychain"] == "none":
        keychain.write_text("unusable") if app == "rs" else None
    elif sp["keychain"]:
        keychain.write_text(json.dumps(sp["keychain"]))
    projects = {"none": root / "projects-none", "canary": canaries_dir}[sp["projects"]]
    return {"root": root, "projects": projects}


def env_for(app, root: Path, sp, projects: Path) -> dict:
    env = {
        "PATH": f"{root / 'bin'}:/usr/bin:/bin", "HOME": str(root / "nowhere"), "XDG_CONFIG_HOME": str(root / "nowhere" / ".config"),
        "TZ": sp["tz"], "LANG": "C.UTF-8", "EASY_PERSONAL_HOME": str(root / "home"), "EASY_PERSONAL_AGENT_ROOT": str(root / "agent-root"),
        "EASY_PERSONAL_CLAUDE_PROJECTS": str(projects), "EASY_MEMORY_HOME": str(root / "em-none"),
        "EASY_PERSONAL_VERIFIER_REPORTS": str(root / "verifier-none"), "EASY_PERSONAL_CONTEXT_URL": "http://127.0.0.1:9/mcp",
        "DBUS_SESSION_BUS_ADDRESS": "unix:path=/nonexistent/bus", "FAKE_CLAUDE_LOG": str(root / "claude.log"),
        "FAKE_CLAUDE_STATE": str(root / "claude-state"), "FAKE_CLAUDE_STDERR": "", "BROWSER": "/bin/true",
    }
    if sp["ccd"]:
        env["CLAUDE_CONFIG_DIR"] = "/runner/dir"  # the default account must drop it
    if sp["fail"]:
        env["FAKE_CLAUDE_FAIL"] = sp["fail"]
    if app == "py":
        env |= {"PYTHONPATH": f"{REPO / 'src'}:{root / 'pylib'}", "PYTHONDONTWRITEBYTECODE": "1", "PARITY_KEYCHAIN": str(root / "keychain.json"),
                "PYTHON_KEYRING_BACKEND": "keyring.backends.fail.Keyring" if sp["keychain"] == "none" else "fakekeyring.FileKeyring",
                "PYTHONHASHSEED": "0"}
    else:
        env["EASY_PERSONAL_TEST_KEYCHAIN"] = str(root / "keychain.json")
    return env


# ---------------------------------------------------------------------------------------------------------------- running


def cli_command(app, argv):
    return [str(VENV_PY), "-c", PY_CLI, *argv] if app == "py" else [str(BINARY), *argv]


def run_process(cmd, env, cwd, stdin: str = "") -> dict:
    """One child with no controlling terminal (so a secret prompt reads stdin, as in the T060 oracle)."""
    try:
        done = subprocess.run(cmd, input=stdin.encode(), capture_output=True, env=env, cwd=cwd, start_new_session=True, timeout=TIMEOUT, check=False)
    except subprocess.TimeoutExpired:
        return {"code": "timeout", "stdout": "", "stderr": f"no answer within {TIMEOUT} s"}
    return {"code": done.returncode, "stdout": done.stdout.decode("utf-8", "replace"), "stderr": done.stderr.decode("utf-8", "replace")}


def dump_store(root: Path, app) -> dict:
    path = root / "home" / "store.sqlite"
    if not path.exists():
        return {"<store>": "absent"}
    saved, gc.FAKE_LAUNCHER = gc.FAKE_LAUNCHER, launcher_of(app)
    try:
        return gc.dump(path, str(root))
    finally:
        gc.FAKE_LAUNCHER = saved


def capture_state(root: Path, app, before: dict, sp) -> dict:
    """What the probe left behind: changed store tables, keychain items, `claude` calls, the fake `claude`'s state."""
    after = dump_store(root, app)
    keychain = root / "keychain.json"
    items = "none"
    if sp["keychain"] == "none":
        pass  # no backend: Python's `fail` keyring, Rust's `unusable` file; both hold nothing
    elif keychain.exists():
        text = keychain.read_text()
        try:
            items = json.loads(text)
        except ValueError:
            items = text
    saved, gc.FAKE_LAUNCHER = gc.FAKE_LAUNCHER, launcher_of(app)
    try:
        calls = gc.read_log(root / "claude.log", str(root))
        claude = gc.read_state(root / "claude-state", str(root))
    finally:
        gc.FAKE_LAUNCHER = saved
    return {"store_changed": {t: rows for t, rows in after.items() if rows != before.get(t)}, "keychain": items, "claude_calls": calls, "claude_state": claude}


class Run:
    """One app in one world: builds it, runs steps and captures the state; the world is removed by the caller."""

    def __init__(self, app, root: Path, sp, canaries_dir=None):
        self.app, self.root, self.sp = app, root, sp
        info = build_world(app, root, sp, canaries_dir)
        self.env = env_for(app, root, sp, info["projects"])
        self.before = dump_store(root, app)

    def cli(self, argv, stdin="", env_extra=None, cmd=None):
        env = self.env | (env_extra or {})
        return run_process(cmd or cli_command(self.app, argv), env, self.root / "nowhere", stdin)

    def state(self):
        return capture_state(self.root, self.app, self.before, self.sp)


# ---------------------------------------------------------------------------------------------------------------- servers


class LineReader:
    """Lines from a child's stdout through `os.read` (a `select` on a buffered file misses lines already read into its buffer)."""

    def __init__(self, stream):
        self.fd, self.buf = stream.fileno(), b""

    def line(self, seconds):
        end = time.monotonic() + seconds
        while b"\n" not in self.buf:
            left = end - time.monotonic()
            if left <= 0:
                return None
            ready, _, _ = select.select([self.fd], [], [], left)
            if ready:
                chunk = os.read(self.fd, 65536)
                if not chunk:
                    rest, self.buf = self.buf, b""
                    return rest.decode("utf-8", "replace") if rest else ""
                self.buf += chunk
        text, _, self.buf = self.buf.partition(b"\n")
        return text.decode("utf-8", "replace") + "\n"


def static_arg(root: Path, page: bool) -> str:
    """The folder Python's server serves as the page. Python's `create_app` falls back to the packaged bundle (`src/easy_personal/dashboard/static/`,
    a gitignored build output) when it is not told, so it is always told: the world's folder, or one that does not exist (no page, no mount)."""
    return str(root / "static") if page else str(root / "no-static")


def connect_with_retry(port):
    """Python prints its URL before uvicorn listens (the socket is bound, not yet listening): retry a refused connection for 10 s."""
    end = time.monotonic() + 10
    while True:
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=30)
        try:
            conn.connect()
            return conn
        except ConnectionRefusedError:
            conn.close()
            if time.monotonic() > end:
                raise
            time.sleep(0.1)


class Server:
    """`serve` of the app on a free port with a per-run token; read the URL line, then ask over HTTP."""

    def __init__(self, run: Run):
        self.run = run
        static = static_arg(run.root, run.sp["static"])
        if run.app == "py":
            cmd = [str(VENV_PY), "-c", PY_SERVE, static]
        else:
            cmd = [str(BINARY), "serve", "--no-ingest", *(["--static-dir", static] if run.sp["static"] else [])]
        self.proc = subprocess.Popen(cmd, env=run.env, cwd=run.root / "nowhere", stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                     stderr=subprocess.PIPE, start_new_session=True)
        self.reader = LineReader(self.proc.stdout)
        line = self.reader.line(30)
        found = re.match(r"dashboard: http://127\.0\.0\.1:(\d+)/#(\S+)", line or "")
        if not found:
            self.close()
            raise Refused(f"{run.app} serve did not print its URL: {line!r}")
        self.port, self.token = int(found.group(1)), found.group(2)

    def request(self, method, path, body=b"", ctype=None, token=True, headers=None, stream=False) -> dict:
        conn = connect_with_retry(self.port)
        head = dict(headers or {})
        if token:
            head["X-Dashboard-Token"] = self.token
        if ctype:
            head["Content-Type"] = ctype
        try:
            conn.request(method, path, body=body or None, headers=head)
            response = conn.getresponse()
            if stream and "event-stream" in (response.getheader("content-type") or ""):
                data = b""
                while b"\n\n" not in data:
                    chunk = response.fp.readline()
                    if not chunk:
                        break
                    data += chunk
            else:
                data = response.read()
            out = {"status": response.status, "type": response.getheader("content-type"), "body": data}
            out["headers"] = {k.lower(): v for k, v in response.getheaders()}
            return out
        except (OSError, http.client.HTTPException) as exc:
            return {"status": f"error: {type(exc).__name__}", "type": None, "body": b"", "headers": {}}
        finally:
            conn.close()

    def close(self):
        if self.proc.poll() is None:
            os.killpg(self.proc.pid, signal.SIGINT)
            try:
                self.proc.wait(8)
            except subprocess.TimeoutExpired:
                os.killpg(self.proc.pid, signal.SIGKILL)
                self.proc.wait()
        for stream in (self.proc.stdout, self.proc.stderr):
            with contextlib.suppress(Exception):
                stream.close()
        return self.proc.returncode


class McpSession:
    """The MCP server over stdio: one JSON line per message; waits for each answer (a closed pipe would drop the in-flight ones)."""

    def __init__(self, run: Run):
        cmd = cli_command(run.app, ["mcp"])
        self.proc = subprocess.Popen(cmd, env=run.env, cwd=run.root / "nowhere", stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.PIPE, start_new_session=True)
        self.next_id = 0
        self.reader = LineReader(self.proc.stdout)

    def send(self, message, reply=True):
        self.proc.stdin.write((json.dumps(message) + "\n").encode())
        self.proc.stdin.flush()
        if not reply:
            return None
        while True:
            line = self.reader.line(60)
            if line is None:
                return {"error": "no answer within 60 s"}
            if line == "":
                return {"error": "the server closed its output"}
            try:
                answer = json.loads(line)
            except ValueError:
                return {"error": "not JSON", "text": line}
            if answer.get("id") == message.get("id"):
                return answer

    def call(self, method, params=None):
        self.next_id += 1
        message = {"jsonrpc": "2.0", "id": self.next_id, "method": method}
        if params is not None:
            message["params"] = params
        return self.send(message)

    def start(self):
        init = self.call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t061", "version": "0"}})
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"}, reply=False)
        return init

    def close(self):
        with contextlib.suppress(Exception):
            self.proc.stdin.close()
        try:
            self.proc.wait(10)
        except subprocess.TimeoutExpired:
            os.killpg(self.proc.pid, signal.SIGKILL)
            self.proc.wait()
        err = self.proc.stderr.read().decode("utf-8", "replace")
        for stream in (self.proc.stdout, self.proc.stderr):
            with contextlib.suppress(Exception):
                stream.close()
        return self.proc.returncode, err


# ---------------------------------------------------------------------------------------------------------------- inventory

INVENTORY_CODE = r'''
import argparse, json, sys, tempfile
from pathlib import Path
from easy_personal.surfaces import api, cli, mcp
parser = cli.build_parser()
keys = []
def leaves(p, path=()):
    subs = [a for a in p._actions if isinstance(a, argparse._SubParsersAction)]
    if not subs:
        yield path
        return
    for action in subs:
        for name, sub in action.choices.items():
            yield from leaves(sub, path + (name,))
keys += ["cli: " + " ".join(p) for p in leaves(parser)]
keys += ["cli: " + a.option_strings[0] for a in parser._actions if a.option_strings and a.dest not in ("help",)]
with tempfile.TemporaryDirectory() as static:
    app = api.create_app("token", 1, static=Path(static))
for route in app.routes:
    methods = getattr(route, "methods", None)
    if methods is None:
        keys.append("route: GET / (static files of the page)")
    else:
        keys += [f"route: {m} {route.path}" for m in sorted(methods - {"HEAD"})]
server = mcp.create_app()
keys += [f"mcp tool: {t.name}" for t in server._tool_manager.list_tools()]
keys += [f"mcp prompt: {p.name}" for p in server._prompt_manager.list_prompts()]
print(json.dumps(sorted(keys)))
'''


def inventory() -> list[str]:
    """Every surface of the Python app, read from the app itself (a fresh child; no store opened, nothing written)."""
    with tempfile.TemporaryDirectory(prefix="parity-inventory-") as tmp:
        env = {"PATH": "/usr/bin:/bin", "HOME": tmp, "EASY_PERSONAL_HOME": f"{tmp}/home", "PYTHONPATH": str(REPO / "src"),
               "PYTHONDONTWRITEBYTECODE": "1", "TZ": "UTC"}
        done = subprocess.run([str(VENV_PY), "-c", INVENTORY_CODE], env=env, capture_output=True, text=True, cwd=tmp, timeout=120, check=False)
    if done.returncode:
        raise Refused(f"the inventory could not be read from Python: {done.stderr[-400:]}")
    return json.loads(done.stdout)


# ---------------------------------------------------------------------------------------------------------------- normalising and comparing

# Rule N1: what differs per run or per app and is not behaviour. The world's folder, the temp folder, each app's launcher and
# binary, and the Python interpreter are written as a word. Both apps run at the same world path, so a path never differs by itself.
# Rule N2: Python's stderr line starts `easy-personal: `, Rust's `easy-personal-rs: ` (README, T058 "Exit codes").
# Rule N3: the clock's own values in a JSON text or object: `as_of`, `as_of_utc`, `saved_utc` (the apps share no clock hook).
# Rule N4: ids made at random by the app: a new connector id (`c` and 8 hex digits) and the store's `meta.machine_id` (both apps make
# their own in a new store).
NEW_ID = re.compile(r"\bc[0-9a-f]{8}\b")
NOW_KEYS = ("as_of", "as_of_utc", "now_as_of_utc", "saved_utc")
NOW_TEXT = re.compile(r'("(?:' + "|".join(NOW_KEYS) + r')": ?)"[^"]*"')


def scrub_text(text: str, root: Path | None) -> str:
    if not isinstance(text, str):
        return text
    for needle, word in ((str(root) if root else None, "<WORK>"), (str(PY_LAUNCHER), "<LAUNCHER>"), (str(RS_LAUNCHER), "<LAUNCHER>"),
                         (str(BINARY), "<RUST_BIN>"), (str(VENV_PY), "<PYTHON>")):
        if needle:
            text = text.replace(needle, word)
    text = re.sub(r"(?m)^easy-personal-rs: ", "easy-personal: ", text)
    text = NEW_ID.sub("<ID>", text)
    return NOW_TEXT.sub(r'\1"<NOW>"', text)


def scrub_value(value, root):
    if isinstance(value, str):
        return scrub_text(value, root)
    if isinstance(value, list):
        return [scrub_value(v, root) for v in value]
    if isinstance(value, dict):
        return {scrub_text(k, root): ("<NOW>" if k in NOW_KEYS and isinstance(v, str) else scrub_value(v, root)) for k, v in value.items()}
    return value


def compare(py, rs) -> bool:
    """Equal means equal: no field is dropped, no number rounded. Only N1..N3 (applied before this) and nothing else."""
    return json.dumps(py, sort_keys=True) == json.dumps(rs, sort_keys=True)


# ---------------------------------------------------------------------------------------------------------------- the README

# README paragraphs that state a difference but are not a numbered list: ref -> (regex where it starts, regex where it ends). Item 1 is the paragraph.
PROSE = {
    "ConnRoute": (r"\*\*`configured` is always `false`\*\*", r"\n- `GET /api/kit`"),
    "ConnTool": (r"`connector_status` -> `\{\"connectors\"", r"\n`connector_suggestions`"),
    "McpInit": (r"`initialize` reports `easy-personal-rs` and the Cargo version", r"\. The server ends"),
    "T040": (r"Decided for you \(T040; Rust differs from Python\)", r"\n- `GET"),
    "Static": (r"`serve --static-dir <DIR>` also serves", r"\n\nThe read routes"),
}


def readme_item(ref: str, number: int) -> str | None:
    """The text of item `number` of a README list, or None when the list or the item is not there.

    `T058` ... : the numbered list under "Decided for you (T058, ...". `Ingest`: row `number` of the table under "Differences from Python"
    in "Ingest (P2a, P2b)". `Schedule`: bullet `number` of "Scheduled ingest (P2c)". Items count from 1.
    """
    text = README.read_text()
    if ref == "Ingest":
        found = re.search(r"\*\*Differences from Python\*\* \(each on purpose\):\n\n\| Python \| Rust \| Why \|\n\|---\|---\|---\|\n((?:\|.*\n)+)", text)
        rows = found.group(1).splitlines() if found else []
        return rows[number - 1] if 0 < number <= len(rows) else None
    if ref == "T047":  # an inline list: "*Decided for you.* (1) ... (2) ..." in one paragraph
        found = re.search(r"\*Decided for you\.\* (\(1\).*?)(?=\n\n)", text, re.S)
        parts = re.split(r"(?:^|(?<=[.)`]) )\((\d+)\) ", found.group(1)) if found else []
        items = {int(parts[i]): parts[i + 1] for i in range(1, len(parts) - 1, 2)}
        return items.get(number)
    if ref in PROSE:
        start, end = PROSE[ref]
        found = re.search(start + r"(.*?)(?=" + end + r"|\Z)", text, re.S)
        return found.group(0) if found and number == 1 else None
    if ref == "McpHttp":
        found = re.search(r"(?ms)^### `mcp --transport http`\n(.*?)(?=^### )", text)
        rows = re.findall(r"(?m)^\| (?!Input|---)(.*)$", found.group(1)) if found else []
        return rows[number - 1] if 0 < number <= len(rows) else None
    if ref == "Schedule":
        found = re.search(r"(?ms)^## Scheduled ingest \(P2c\)\n(.*?)(?=^## )", text)
        bullets = re.findall(r"(?ms)^- (.*?)(?=^- |^\S|\Z)", found.group(1)) if found else []
        return bullets[number - 1] if 0 < number <= len(bullets) else None
    found = re.search(r"Decided for you[^\n]*\(" + re.escape(ref) + r"\b", text)
    if not found:
        return None
    rest = text[found.end():]
    end = re.search(r"(?m)^#{2,3} ", rest)
    section = rest[: end.start()] if end else rest
    items = re.findall(r"(?ms)^\s{0,3}(\d+)\. (.*?)(?=^\s{0,3}\d+\. |\Z)", section)
    for index, body in items:
        if int(index) == number:
            return body
    return None


def readme_ok(ref: str, number: int, words: tuple[str, ...]) -> bool:
    body = readme_item(ref, number)
    return body is not None and all(w.lower() in body.lower() for w in words)


# ---------------------------------------------------------------------------------------------------------------- the report table

CLASS = re.compile(r"^(equal|equal except JSON key order|differs: new|not run: .+|differs: README [A-Za-z0-9]+ item \d+(?:, [A-Za-z0-9]+ item \d+)*)$")
ROW = re.compile(r"^\| `([^`]+)` \| (\d+) \| (.+?) \| (.*) \|$")
COUNTS = re.compile(r"^Counts: equal (\d+) \+ equal except JSON key order (\d+), differs README (\d+), differs new (\d+), not run (\d+)\.$", re.M)


def parse_report(text: str) -> list[dict]:
    rows = []
    for line in text.splitlines():
        found = ROW.match(line)
        if found:
            rows.append({"key": found.group(1), "probes": int(found.group(2)), "result": found.group(3), "note": found.group(4)})
    return rows


def missing_surfaces(inventory_keys, row_keys) -> list[str]:
    """Surfaces Python has and the table lacks (the completeness test fails on any)."""
    return sorted(set(inventory_keys) - set(row_keys))


def readme_refs(result: str) -> list[tuple[str, int]]:
    return [(m.group(1), int(m.group(2))) for m in re.finditer(r"([A-Za-z0-9]+) item (\d+)", result)] if result.startswith("differs: README") else []


# ---------------------------------------------------------------------------------------------------------------- the leak scan


def canaries():
    """The repo's canaries (`tests/leak/canaries.py`, same patterns), with values fixed per name so that two runs plant the same bytes."""
    from leak.canaries import PATTERNS, Canaries

    return Canaries({n: p.format(t=hashlib.sha256(n.encode()).hexdigest()[:16]) for n, p in PATTERNS.items()})


def scan(text) -> list[tuple[str, str]]:
    return canaries().find(text)


# ---------------------------------------------------------------------------------------------------------------- probes

PROBES: list[dict] = []


def probe(key, pid, kind, sp, *, steps=None, reqs=None, calls=None, run=None, tzs=None, explain=(), note="", launcher_rs=False):
    """One run of a surface on both apps. kind: cli | http | mcp | custom."""
    PROBES.append({"key": key, "id": pid, "kind": kind, "spec": sp, "steps": steps or [], "reqs": reqs or [], "calls": calls or [], "run": run,
                   "tzs": tzs, "explain": list(explain), "note": note, "launcher_rs": launcher_rs})


def parse_body(raw: bytes, root):
    text = raw.decode("utf-8", "replace")
    try:
        value = json.loads(text)
    except ValueError:
        return scrub_text(text, root), None
    return scrub_value(value, root), key_order(text)


def key_order(text: str):
    """The key order of every object in a JSON text, as nested lists (parsed equality ignores it; the report says when it differs)."""
    return json.loads(text, object_pairs_hook=lambda pairs: [[k for k, _ in pairs], [v for _, v in pairs]])


def order_only(py_order, rs_order) -> bool:
    return py_order is not None and rs_order is not None and json.dumps(py_order) != json.dumps(rs_order)


def outcome_step(out: dict, root) -> dict:
    return {"code": out["code"], "stdout": scrub_text(out["stdout"], root), "stderr": scrub_text(out["stderr"], root)}


def scrubbed_state(state: dict, root) -> dict:
    for row in state["store_changed"].get("investigate_reports", []):
        row[1] = "<NOW>"  # the bare `saved_utc` column: the second the report was saved in (N3)
    meta = state["store_changed"].get("meta")
    if meta:
        state["store_changed"]["meta"] = [["machine_id", "<ID>"] if row and row[0] == "machine_id" else row for row in meta]
    return scrub_value(state, root)


def exec_cli(p, app, base, canaries_dir) -> dict:
    zones = p["tzs"] or [p["spec"]["tz"]]
    result = {}
    for zone in zones:
        sp = {**p["spec"], "tz": zone}
        root = base / "world"
        run = Run(app, root, sp, canaries_dir)
        try:
            steps = []
            for argv, stdin in p["steps"]:
                argv = [a.replace("@W@", str(root)) for a in argv]
                cmd = [str(RS_LAUNCHER), *argv] if app == "rs" and p.get("launcher_rs") else None
                steps.append(outcome_step(run.cli(argv, stdin, cmd=cmd), root))
            result[zone] = {"steps": steps, "state": scrubbed_state(run.state(), root)}
        finally:
            shutil.rmtree(root, ignore_errors=True)
    return result


def http_outcome(raw: dict, root) -> tuple[dict, object]:
    body, order = parse_body(raw["body"], root) if raw["status"] != "error" else ("", None)
    return {"status": raw["status"], "type": raw["type"], "body": body}, order


def exec_http(p, app, base, canaries_dir) -> dict:
    sp = p["spec"]
    root = base / "world"
    run = Run(app, root, sp, canaries_dir)
    server = Server(run)
    answers, orders = [], []
    try:
        for r in p["reqs"]:
            path = r["path"].replace("@W@", str(root))
            if r.get("query_token"):
                path += ("&" if "?" in path else "?") + "token=" + server.token
            raw = server.request(r["method"], path, r.get("body", b""), r.get("type"), r.get("token", True) and not r.get("query_token"),
                                 r.get("headers"), r.get("stream", False))
            if r.get("stream"):
                raw["body"] = raw["body"].split(b"\n\n")[0] + b"\n\n" if raw["body"] else b""
            answer, order = http_outcome(raw, root)
            if r.get("text"):
                answer["body"] = scrub_text(raw["body"].decode("utf-8", "replace"), root)
                order = None
            answers.append(answer)
            orders.append(order)
        code = server.close()
        state = scrubbed_state(run.state(), root)
        return {"UTC": {"steps": answers, "state": state, "orders": orders, "exit": code}}
    finally:
        server.close()
        shutil.rmtree(root, ignore_errors=True)


SDK_LOG = re.compile(r"(?m)^Processing request of type \w+\n")


def mcp_view(answer, root, pick=None):
    """An MCP answer, scrubbed; a tool's text content is parsed when it is JSON (whitespace is the SDK's business, not behaviour)."""
    answer = scrub_value(answer, root)
    result = answer.get("result") if isinstance(answer, dict) else None
    if isinstance(result, dict):
        if pick and isinstance(result.get("tools"), list):
            result["tools"] = [t for t in result["tools"] if t.get("name") == pick]
        if pick and isinstance(result.get("prompts"), list):
            result["prompts"] = [t for t in result["prompts"] if t.get("name") == pick]
        for part in result.get("content", []) or []:
            if isinstance(part, dict) and isinstance(part.get("text"), str):
                try:
                    part["text"] = {"json": json.loads(part["text"])}
                except ValueError:
                    pass
    return answer


def exec_mcp(p, app, base, canaries_dir) -> dict:
    sp = p["spec"]
    root = base / "world"
    run = Run(app, root, sp, canaries_dir)
    session = McpSession(run)
    try:
        init = session.start()
        answers = [{"method": "initialize", "answer": mcp_view(init, root)}] if p.get("init") else []
        for method, params, pick in p["calls"]:
            answers.append({"method": method, "params": params, "answer": mcp_view(session.call(method, params), root, pick)})
        code, err = session.close()
        if not p.get("raw_stderr"):  # N7: Python's SDK logs one line per request on stderr, Rust nothing; reported once, by `stderr-log`
            err = SDK_LOG.sub("", err)
        return {"UTC": {"steps": answers, "state": scrubbed_state(run.state(), root), "exit": code, "stderr": scrub_text(err, root)}}
    finally:
        with contextlib.suppress(Exception):
            session.close()
        shutil.rmtree(root, ignore_errors=True)


def exec_custom(p, app, base, canaries_dir) -> dict:
    return p["run"](app, base, canaries_dir)


EXECUTORS = {"cli": exec_cli, "http": exec_http, "mcp": exec_mcp, "custom": exec_custom}


# ---------------------------------------------------------------------------------------------------------------- the CLI probes

import gen_connector_cli as gcc  # noqa: E402  (its case table: the T060 oracle's worlds and steps)

CLI_CASES = {c["name"]: c for c in gen_cli.cases()}
CONN_CASES = {c["name"]: c for c in gcc.cases()}
RICH = spec()
RICH_DISK = spec("rich-disk")
NO_STORE = spec("none")


def cli_case(key, name, *, explain=(), note=""):
    """A case of the T058 oracle (`gen_cli.py`): its store, tree, setup SQL, zones and steps."""
    c = CLI_CASES[name]
    assert c["store"] in ("rich", "none"), name
    sp = spec("rich" if c["store"] == "rich" else "none", setup=c["setup"], tree=c["tree"])
    probe(key, f"{name}", "cli", sp, steps=[(s["args"], s["stdin"]) for s in c["steps"]], tzs=c["zones"] if len(c["zones"]) > 1 else None, explain=explain, note=note)


def conn_case(key, name, *, explain=(), note=""):
    """A case of the T060 oracle (`gen_connector_cli.py`): its world (keychain, fake `claude`, agent folders) and steps."""
    c = CONN_CASES[name]
    assert c["launcher"], name
    sp = spec("rich", setup=c["setup"], dirs=c["dirs"], keychain=c["keychain"] or None, claude=c["claude"], fail=c["fail"], ccd=True)
    probe(key, name, "cli", sp, steps=[(s["args"], s["stdin"]) for s in c["steps"]], explain=explain, note=note)


def cli(key, pid, argv, sp=RICH, stdin="", **kw):
    probe(key, pid, "cli", sp, steps=[(argv, stdin)], **kw)


def define_cli_probes():
    cli("cli: ping", "ping", ["ping"], NO_STORE)
    cli("cli: --version", "version", ["--version"], NO_STORE)
    cli("cli: -h", "help", ["--help"], NO_STORE) if False else None
    # ingest (the planted-canary tree: the leak scan then has something to find)
    cli("cli: ingest", "canary-tree", ["ingest"], spec(projects="canary"))
    probe("cli: ingest", "twice-then-usage", "cli", spec(projects="canary"), steps=[(["ingest"], ""), (["ingest"], ""), (["usage", "--by", "project"], "")])
    cli("cli: ingest", "retention-zero", ["ingest", "--retention-days", "0"])
    cli("cli: ingest", "no-store", ["ingest"], NO_STORE)
    # usage, outliers
    for pid, argv in {"plain": [], "by-model": ["--by", "model"], "by-tool": ["--by", "tool"], "window": ["--since", "2026-01-13T00:00:00Z", "--until", "2026-01-20T00:00:00Z"],
                      "scope-acme": ["--workspace", "acme"], "scope-account": ["--account", "Claude work"], "unknown-workspace": ["--workspace", "nope"],
                      "bad-by": ["--by", "week"], "bad-date": ["--since", "banana"]}.items():
        cli("cli: usage", pid, ["usage", *argv])
    cli("cli: usage", "by-day-three-zones", ["usage", "--by", "day", "--since", "2026-01-13T08:30:00", "--until", "2026-01-15T23:00:00"], tzs=ZONES)
    cli("cli: usage", "by-project-disk", ["usage", "--by", "project"], RICH_DISK)
    cli("cli: usage", "no-store", ["usage"], NO_STORE)
    for pid, argv in {"plain": [], "window": ["--since", "2026-01-13", "--until", "2026-01-16"], "scope": ["--workspace", "acme"], "unknown-account": ["--account", "nope"]}.items():
        cli("cli: outliers", pid, ["outliers", *argv])
    cli("cli: outliers", "three-zones", ["outliers", "--since", "2026-01-13T08:30:00"], tzs=ZONES)
    for key, command in (("cli: warnings", "warnings"), ("cli: plan", "plan"), ("cli: kit", "kit")):
        cli(key, "plain", [command])
        cli(key, "scope-acme", [command, "--workspace", "acme"])
        cli(key, "scope-account", [command, "--account", "Claude work"])
        cli(key, "unknown-workspace", [command, "--workspace", "nope"])
        cli(key, "on-disk", [command], RICH_DISK)
        cli(key, "bad-flag", [command, "--since", "2026-01-01"])
    cli("cli: kit-findings", "text", ["kit-findings"])
    cli("cli: kit-findings", "json", ["kit-findings", "--json"])
    cli("cli: kit-findings", "limit-since", ["kit-findings", "--json", "--limit", "1", "--since", "2026-01-06T00:00:00Z"])
    cli("cli: kit-findings", "since-three-zones", ["kit-findings", "--since", "2026-01-15T08:00:00"], tzs=ZONES)
    cli("cli: kit-findings", "bad-limit", ["kit-findings", "--limit", "0"])
    # workspace
    cli("cli: workspace list", "rich", ["workspace", "list"])
    cli("cli: workspace list", "no-store", ["workspace", "list"], NO_STORE)
    cli_case("cli: workspace add", "ws_add_minimal")
    cli_case("cli: workspace add", "ws_add_full")
    cli_case("cli: workspace add", "ws_add_bad_taken")
    cli_case("cli: workspace add", "ws_add_bad_bad_color")
    cli_case("cli: workspace edit", "ws_edit_rename")
    cli_case("cli: workspace edit", "ws_edit_repos")
    cli_case("cli: workspace edit", "ws_edit_fields")
    cli_case("cli: workspace edit", "ws_edit_bad_unknown")
    cli_case("cli: workspace edit", "ws_edit_rename_held_yes")
    cli_case("cli: workspace edit", "ws_edit_rename_held_ask_y")
    cli_case("cli: workspace delete", "ws_delete_yes")
    cli_case("cli: workspace delete", "ws_delete_ask_no")
    cli_case("cli: workspace delete", "ws_delete_held_ask_yes")
    cli_case("cli: workspace delete", "ws_delete_bad_orphan")
    # account
    cli_case("cli: account list", "acct_list_detects")
    cli_case("cli: account list", "acct_list_by_provider")
    cli_case("cli: account list", "acct_bad_list_xby_colour")
    cli_case("cli: account confirm", "acct_confirm_by_id")
    cli_case("cli: account confirm", "acct_confirm_bad_label_61")
    cli_case("cli: account ignore", "acct_ignore")
    cli_case("cli: account ignore", "acct_ignore_bad_default")
    cli_case("cli: account add", "acct_add_claude_folder")
    cli_case("cli: account add", "acct_add_bad_missing")
    cli_case("cli: account use", "acct_use")
    cli_case("cli: account use", "acct_use_bad_orphan")
    # connector, doctor
    conn_case("cli: connector list", "list_configured")
    conn_case("cli: connector add", "add_yes")
    conn_case("cli: connector add", "add_ask_no")
    conn_case("cli: connector add", "add_no_backend")
    conn_case("cli: connector add", "add_bad_unknown_provider")
    conn_case("cli: connector remove", "remove_stored_registered")
    conn_case("cli: connector remove", "remove_pre_workspace_ours")
    conn_case("cli: connector remove", "remove_bad_not_catalog")
    conn_case("cli: connector sync", "sync_every_step_kind")
    conn_case("cli: connector sync", "sync_nothing")
    conn_case("cli: connector move", "move_ask_yes")
    conn_case("cli: connector move", "move_bad_unknown_id")
    conn_case("cli: connector alias", "alias_custom_then_default")
    conn_case("cli: connector alias", "alias_bad_both")
    conn_case("cli: connector adopt-legacy", "legacy_adopt")
    conn_case("cli: connector adopt-legacy", "legacy_nothing")
    conn_case("cli: doctor", "doctor_clean")
    conn_case("cli: doctor", "doctor_registrations")
    conn_case("cli: doctor", "doctor_no_claude")
    # store-check
    cli("cli: store-check", "text", ["store-check"])
    cli("cli: store-check", "json", ["store-check", "--json"])
    cli("cli: store-check", "full", ["store-check", "--full", "--json"])
    cli("cli: store-check", "no-store", ["store-check"], NO_STORE)
    # schedule (the systemd calls are the fake `systemctl`, which says "not available"; nothing is installed)
    cli("cli: schedule install", "dry-run", ["schedule", "install", "--dry-run"])
    cli("cli: schedule install", "dry-run-retention", ["schedule", "install", "--retention-days", "30", "--dry-run"])
    cli("cli: schedule install", "no-systemd", ["schedule", "install"])
    cli("cli: schedule remove", "dry-run", ["schedule", "remove", "--dry-run"])


# ---------------------------------------------------------------------------------------------------------------- README differences

# Each rule names a README item (`ref`, number), the words that item must hold, and a transformation of both outputs that stands for
# what the item says. A probe is `differs: README ...` only when it declares the rule AND the transformation makes the two outputs
# equal; a part of the difference the item does not describe stays, and the probe is `differs: new`. A rule counts as used only when it
# changed an output.
NO_STORE_TEXT = "easy-personal: no store yet: nothing has been ingested into the data folder\n"
GETPASS = re.compile(r"(?m)^.*GetPassWarning: Can not control echo on the terminal\.\n  passwd = fallback_getpass\(prompt, stream\)\n")
RULES = {}
EXPLAIN_WORDS = {}


def rule(rid, ref, number, words):
    def register(fn):
        RULES[rid] = fn
        EXPLAIN_WORDS[(ref, number)] = tuple(words)
        fn.ref = (ref, number)
        return fn

    return register


def steps_of(o):
    return o["steps"]


@rule("T058#1", "T058", 1, ("missing store", "refused", "creates"))
def r_missing_store(py, rs):
    """Python makes a store where Rust refuses and makes nothing (exact text, exit 1, empty stdout, no store row written)."""
    first = rs["steps"][0] if rs["steps"] else {}
    made = py["steps"][0] if py["steps"] else {}
    if (first.get("code") == 1 and first.get("stdout") == "" and first.get("stderr") == NO_STORE_TEXT and rs["state"]["store_changed"] == {}
            and made.get("code") == 0 and py["state"]["store_changed"].get("workspaces")):  # Python ran, and made the store
        return rs, rs
    return py, rs


@rule("T060#4", "T060", 4, ("missing store is refused", "doctor"))
def r_missing_store_connector(py, rs):
    return r_missing_store(py, rs)


@rule("T058#8", "T058", 8, ("usage error", "clap's text"))
def r_usage_error(py, rs):
    """Both refuse with exit 2 and an empty stdout; the usage text is argparse's in Python and clap's in Rust."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for a, b in zip(py["steps"], rs["steps"]):
        if (a.get("code") == b.get("code") == 2 and a["stdout"] == b["stdout"] == "" and a["stderr"].startswith("usage:")
                and re.match(r"(?s)error: .*(?:For more information, try '--help'\.\n|Usage: )", b["stderr"] + "")
                and "invalid iso_datetime" not in a["stderr"]):
            a["stderr"] = b["stderr"] = "<usage error>"
    return py, rs


@rule("T060#12", "T060", 12, ("usage errors", "clap's text"))
def r_usage_error_connector(py, rs):
    return r_usage_error(py, rs)


@rule("T060#1", "T060", 1, ("getpasswarning", "not printed"))
def r_getpass_warning(py, rs):
    """Python prints its own `GetPassWarning` line (it names getpass.py); Rust does not (its `Warning: Password input may be echoed.` line is the same)."""
    py = copy.deepcopy(py)
    for step in py["steps"]:
        step["stderr"] = GETPASS.sub("", step["stderr"])
    return py, rs


@rule("T060#3", "T060", 3, ("mcp get", "three times"))
def r_get_reads(py, rs):
    """Rust asks `claude mcp get` again after a yes (up to three times); only the writes (`add-json`, `remove`) are compared."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for o in (py, rs):
        o["state"]["claude_calls"] = [c for c in o["state"]["claude_calls"] if c["argv"][:2] != ["mcp", "get"]]
    return py, rs


@rule("T060#5", "T060", 5, ("keychain line", "backend"))
def r_doctor_backend(py, rs):
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for o in (py, rs):
        for step in o["steps"]:
            step["stdout"] = re.sub(r"(?m)^ok keychain: \S+$", "ok keychain: <BACKEND>", step["stdout"])
    return py, rs


@rule("Ingest#1", "Ingest", 1, ("same four keys", "skipped"))
def r_ingest_skipped(py, rs):
    """Rust's ingest JSON has one more key, `skipped` (entries it could not read); Python has none (it crashes there)."""
    rs = copy.deepcopy(rs)
    for a, b in zip(py["steps"], rs["steps"]):
        try:
            ja, jb = json.loads(a["stdout"]), json.loads(b["stdout"])
        except ValueError:
            continue
        if isinstance(ja, dict) and isinstance(jb, dict) and "skipped" in jb and "skipped" not in ja:
            del jb["skipped"]
            b["stdout"] = json.dumps(jb, indent=2, sort_keys=True) + "\n"
    return py, rs


@rule("Schedule#1", "Schedule", 1, ("the command line is the rust binary",))
def r_schedule_command(py, rs):
    """The unit's `ExecStart` (and the cron line) names the running program: Python's `python -m ...cli`, Rust's own binary."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for step in py["steps"]:
        step["stdout"] = step["stdout"].replace("<PYTHON> -m easy_personal.surfaces.cli", "<COMMAND>")
    for step in rs["steps"]:
        step["stdout"] = step["stdout"].replace("<RUST_BIN>", "<COMMAND>")
    return py, rs


@rule("T051#1", "T051", 1, ("prints the ingest's json", "one progress line"))
def r_dashboard_ingest(py, rs):
    """Python's `dashboard` prints the ingest's JSON before the URL; Rust prints one progress line (on stderr) and starts."""
    py = copy.deepcopy(py)
    for step in py["steps"]:
        head, sep, tail = step["stdout"].partition("dashboard: ")
        if sep and head.lstrip().startswith("{"):
            step["stdout"] = sep + tail
    return py, rs


@rule("McpHttp#1", "McpHttp", 1, ("a host other than", "one line"))
def r_mcp_bad_host(py, rs):
    """A host other than 127.0.0.1: exit 2 in both; Python prints argparse's usage and its error, Rust one line."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for a, b in zip(py["steps"], rs["steps"]):
        if (a.get("code") == b.get("code") == 2 and a["stdout"] == b["stdout"] == "" and a["stderr"].startswith("usage:")
                and "refusing host '0.0.0.0': HTTP must bind 127.0.0.1 only" in a["stderr"]
                and b["stderr"] == 'easy-personal: refusing host "0.0.0.0": HTTP must bind 127.0.0.1 only\n'):
            a["stderr"] = b["stderr"] = "<refusing host>"
    return py, rs


@rule("T058#12", "T058", 12, ("exec", "headers", "easy-personal-launch"))
def r_launcher_split(py, rs):
    """`exec` and `headers` are the separate `easy-personal-launch` program, not subcommands of `easy-personal-rs` (the launcher probes are equal)."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for a, b in zip(py["steps"], rs["steps"]):
        if b.get("code") == 2 and re.match(r"error: unrecognized subcommand '(exec|headers)'", b["stderr"]) and a.get("code") == 0:
            a.update(code="<in easy-personal-launch>", stdout="", stderr="")
            b.update(code="<in easy-personal-launch>", stdout="", stderr="")
    return py, rs


@rule("T040#1", "T040", 1, ("first-run hint", "easy-personal schedule install"))
def r_first_run_hint(py, rs):
    """The first-run timer hint: Python asks systemd and, with no user session, answers a crontab line; Rust always says `easy-personal schedule install`."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for a, b in zip(py["steps"], rs["steps"]):
        steps_a, steps_b = (a.get("body") or {}), (b.get("body") or {})
        if isinstance(steps_a, dict) and isinstance(steps_b, dict) and isinstance(steps_a.get("steps"), list) and isinstance(steps_b.get("steps"), list):
            for x, y in zip(steps_a["steps"], steps_b["steps"]):
                if (x.get("id") == y.get("id") == "ingest_timer" and x.get("hint", "").startswith("no systemd user session: run crontab -e and add: ")
                        and y.get("hint") == "easy-personal schedule install"):
                    x["hint"] = y["hint"] = "<timer hint>"
    return py, rs


@rule("Static#1", "Static", 1, ("any other method is `405`", "empty `404`"))
def r_empty_404(py, rs):
    """A miss and a refused method: Starlette's plain-text `Not Found` / `Method Not Allowed`, Rust's fixed empty body."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    fixed = {404: "Not Found", 405: "Method Not Allowed"}
    for a, b in zip(py["steps"], rs["steps"]):
        if (a.get("status") == b.get("status") in fixed and a.get("body") == fixed[a["status"]] and a.get("type") == "text/plain; charset=utf-8"
                and b.get("body") == "" and b.get("type") is None):
            for x in (a, b):
                x["body"], x["type"] = f"<fixed {x['status']}>", None
    return py, rs


def leaf_diffs(a, b, path="", out=None, limit=6):
    """The first `limit` places where two JSON-like values differ: `(path, python value, rust value)`."""
    out = [] if out is None else out
    if len(out) >= limit:
        return out
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                out.append((f"{path}/{k}", a.get(k, "<absent>"), b.get(k, "<absent>")))
            else:
                leaf_diffs(a[k], b[k], f"{path}/{k}", out, limit)
    elif isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        for i, (x, y) in enumerate(zip(a, b)):
            leaf_diffs(x, y, f"{path}[{i}]", out, limit)
    elif a != b:
        out.append((path, a, b))
    return out[:limit]


@rule("T047#1", "T047", 1, ("wrong argument type", "fixed text", "pydantic"))
def r_argument_error(py, rs):
    """A tool argument of the wrong type, or a missing one: Python's pydantic text echoes the value; Rust's is a short fixed text. Both are `isError` results."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for a, b in zip(py["steps"], rs["steps"]):
        ra, rb = (a.get("answer") or {}).get("result"), (b.get("answer") or {}).get("result")
        if isinstance(ra, dict) and isinstance(rb, dict) and ra.get("isError") is True and rb.get("isError") is True:
            ta, tb = ra["content"][0].get("text"), rb["content"][0].get("text")
            if (isinstance(ta, str) and isinstance(tb, str) and re.match(r"Error executing tool \w+: \d+ validation error", ta)
                    and re.match(r"Error executing tool \w+: argument `", tb)):
                ra["content"][0]["text"] = rb["content"][0]["text"] = "<argument error>"
    return py, rs


@rule("T047#6", "T047", 6, ("server_version", "cargo version"))
def r_server_version(py, rs):
    """The saved row's `server_version` is the running build's version: Python's package version, Rust's Cargo version."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for o in (py, rs):
        for row in o["state"]["store_changed"].get("investigate_reports", []):
            row[2] = "<VERSION>"
    return py, rs


@rule("McpInit#1", "McpInit", 1, ("initialize", "easy-personal-rs", "cargo version"))
def r_server_info(py, rs):
    """`initialize` names the server: Python `easy-personal` and its package version, Rust `easy-personal-rs` and its Cargo version."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for o in (py, rs):
        for step in o["steps"]:
            info = (((step.get("answer") or {}).get("result") or {}).get("serverInfo"))
            if isinstance(info, dict):
                info["name"], info["version"] = "<server>", "<version>"
    return py, rs


@rule("ConnRoute#1", "ConnRoute", 1, ("`configured` is always `false`", "python says `true`"))
def r_configured_route(py, rs):
    """`configured` in `GET /api/connectors` is always false in Rust (it asks neither the keychain nor `claude`); Python says true when both agree."""
    return r_configured(py, rs, "body")


@rule("ConnTool#1", "ConnTool", 1, ("`configured` always `false`",))
def r_configured_tool(py, rs):
    return r_configured(py, rs, "mcp")


def r_configured(py, rs, where):
    py = copy.deepcopy(py)

    def flatten(node):
        if isinstance(node, dict):
            if "configured" in node and isinstance(node["configured"], bool):
                yield node
            for v in node.values():
                yield from flatten(v)
        elif isinstance(node, list):
            for v in node:
                yield from flatten(v)

    if all(n["configured"] is False for step in rs["steps"] for n in flatten(step)):
        for step in py["steps"]:
            for n in flatten(step):
                n["configured"] = False
        for call in py["state"]["claude_calls"]:  # Rust asks `claude` nothing here either
            pass
        py["state"]["claude_calls"] = [c for c in py["state"]["claude_calls"] if c["argv"][:2] != ["mcp", "get"]] if not rs["state"]["claude_calls"] else py["state"]["claude_calls"]
    return py, rs


@rule("T051#2", "T051", 2, ("stateless", "json, not sse", "mcp-session-id"))
def r_mcp_http_form(py, rs):
    """`mcp --transport http`: Python answers in SSE with a session id, Rust statelessly in JSON with none (the JSON-RPC results are compared as they are)."""
    py, rs = copy.deepcopy(py), copy.deepcopy(rs)
    for a, b in zip(py["steps"], rs["steps"]):
        if a.get("form") == "sse" and b.get("form") == "json":
            a["form"] = b["form"] = "<sse or json>"
        if a.get("session_header") is True and b.get("session_header") is False:
            a["session_header"] = b["session_header"] = "<session id or none>"
    return py, rs


def classify_zone(p, py, rs):
    """`('equal', [])`, `('readme', [rule ids used])` or `('new', [])` for one zone of one probe."""
    if equal_zone(p, py, rs):
        return "equal", []
    used = []
    for rid in p["explain"]:
        if rid not in RULES:  # an order rule: judged by `classify_probe`
            continue
        new_py, new_rs = RULES[rid](py, rs)
        if (new_py, new_rs) != (py, rs):
            used.append(rid)
        py, rs = new_py, new_rs
    return ("readme", used) if used and equal_zone(p, py, rs) else ("new", [])


# ---------------------------------------------------------------------------------------------------------------- custom CLI probes


def read_until(reader, wanted: int, seconds: float) -> list[str]:
    lines = []
    end = time.monotonic() + seconds
    while len(lines) < wanted and time.monotonic() < end:
        line = reader.line(0.5)
        if line == "":
            break
        if line:
            lines.append(line)
    return lines


def run_dashboard(app, base, canaries_dir):
    """`dashboard` (Rust: its alias of `serve`): start, read what it prints, ask one API route, Ctrl-C, read the exit code."""
    root = base / "world"
    run = Run(app, root, RICH)
    proc = subprocess.Popen(cli_command(app, ["dashboard"]), env=run.env, cwd=root / "nowhere", stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    reader = LineReader(proc.stdout)
    try:
        lines, url = [], None
        end = time.monotonic() + 40
        while time.monotonic() < end and url is None:
            lines += read_until(reader, 1, 1)
            url = next((m for m in (re.match(r"dashboard: http://127\.0\.0\.1:(\d+)/#(\S+)", ln) for ln in lines) if m), None)
        status = None
        if url:
            lines += read_until(reader, 1, 5)
            conn = connect_with_retry(int(url.group(1)))
            conn.request("GET", "/api/usage/summary", headers={"X-Dashboard-Token": url.group(2)})
            status = conn.getresponse().status
            conn.close()
        os.killpg(proc.pid, signal.SIGINT)
        try:
            code = proc.wait(15)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)
            code = "did not stop"
        err = proc.stderr.read().decode("utf-8", "replace")
        shape = [re.sub(r"127\.0\.0\.1:\d+/#\S+", "127.0.0.1:<PORT>/#<TOKEN>", ln) for ln in lines]
        step = {"code": code, "stdout": scrub_text("".join(shape), root), "stderr": scrub_text(err, root), "api_status": status}
        return {"UTC": {"steps": [step], "state": scrubbed_state(run.state(), root)}}
    finally:
        with contextlib.suppress(Exception):
            if proc.poll() is None:
                os.killpg(proc.pid, signal.SIGKILL)
        for stream in (proc.stdout, proc.stderr):
            with contextlib.suppress(Exception):
                stream.close()
        shutil.rmtree(root, ignore_errors=True)


def free_port() -> int:
    import socket

    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def json_rpc_body(raw: bytes):
    """A JSON-RPC answer from a JSON body or an SSE body (`event: message` / `data: {...}`); the form it came in."""
    text = raw.decode("utf-8", "replace")
    if text.lstrip().startswith(("event:", "data:")):
        data = "".join(line[5:].strip() for line in text.splitlines() if line.startswith("data:"))
        return "sse", json.loads(data) if data else None
    return "json", json.loads(text) if text.strip() else None


def run_mcp_http(app, base, canaries_dir):
    """`mcp --transport http`: the handshake and one tool call over POST, as an HTTP client does."""
    root = base / "world"
    run = Run(app, root, RICH)
    port = free_port()
    proc = subprocess.Popen(cli_command(app, ["mcp", "--transport", "http", "--port", str(port)]), env=run.env, cwd=root / "nowhere",
                            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    steps = []
    try:
        head = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
        session = None
        for name, message in (("initialize", {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t061", "version": "0"}}}),
                              ("initialized", {"jsonrpc": "2.0", "method": "notifications/initialized"}),
                              ("ping-tool", {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "ping", "arguments": {}}}),
                              ("bad-json", None)):
            conn = connect_with_retry(port)
            try:
                headers = dict(head) | ({"Mcp-Session-Id": session} if session else {})
                conn.request("POST", "/mcp", body=json.dumps(message).encode() if message else b"{", headers=headers)
                response = conn.getresponse()
                raw = response.read()
                session = session or response.getheader("mcp-session-id")
                form, parsed = (json_rpc_body(raw) if raw.strip() else ("empty", None)) if response.status < 400 else ("error", None)
                result = (parsed or {}).get("result") if isinstance(parsed, dict) else None
                steps.append({"step": name, "status": response.status, "form": form, "session_header": bool(response.getheader("mcp-session-id")),
                              "result": {k: result[k] for k in ("content", "isError", "structuredContent") if result and k in result} if name == "ping-tool" else None,
                              "protocol": result.get("protocolVersion") if result else None})
            finally:
                conn.close()
        os.killpg(proc.pid, signal.SIGINT)
        try:
            code = proc.wait(15)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)
            code = "did not stop"
        log = re.sub(r"127\.0\.0\.1:\d+ - ", "127.0.0.1:<PORT> - ", proc.stdout.read().decode("utf-8", "replace"))
        steps.append({"step": "exit", "code": code, "stdout": scrub_text(log, root)})
        return {"UTC": {"steps": scrub_value(steps, root), "state": scrubbed_state(run.state(), root)}}
    finally:
        with contextlib.suppress(Exception):
            if proc.poll() is None:
                os.killpg(proc.pid, signal.SIGKILL)
        for stream in (proc.stdout, proc.stderr):
            with contextlib.suppress(Exception):
                stream.close()
        shutil.rmtree(root, ignore_errors=True)


def define_special_cli_probes():
    probe("cli: dashboard", "start-and-stop", "custom", RICH, run=run_dashboard)
    probe("cli: mcp", "stdin-closed", "cli", RICH, steps=[(["mcp"], "")])
    probe("cli: mcp", "http-handshake-and-ping", "custom", RICH, run=run_mcp_http)
    probe("cli: mcp", "bad-host", "cli", RICH, steps=[(["mcp", "--transport", "http", "--host", "0.0.0.0"], "")])
    item = json.dumps({"command": ["/bin/sh", "-c", "printf '%s' \"$T\""], "env": {"T": "from-keychain"}, "headers": {"Authorization": "Bearer t061"}})
    world = spec(setup=gc.ACME, dirs=gc.DIRS, keychain={"easy-personal/slack-acme": item}, ccd=True)
    for name in ("exec", "headers"):
        probe(f"cli: {name}", "launcher", "cli", world, steps=[([name, "slack-acme"], "")], launcher_rs=True,
              note="Rust: the separate `easy-personal-launch` program")
        probe(f"cli: {name}", "launcher-missing-item", "cli", world, steps=[([name, "gmail-acme"], "")], launcher_rs=True)
        probe(f"cli: {name}", "in-the-main-binary", "cli", world, steps=[([name, "slack-acme"], "")],
              note="Rust: `easy-personal-rs " + name + "` (no such command)")


# ---------------------------------------------------------------------------------------------------------------- route probes

JSON_TYPE = "application/json"
GC_CASES = {c["name"]: c for c in gc.cases()}
RICH_DISK_STATIC = spec("rich-disk", static=True)


def R(method, path, body=None, type=None, **kw):
    """One request. A dict body is sent as JSON text."""
    data = b"" if body is None else (json.dumps(body, ensure_ascii=False).encode() if not isinstance(body, bytes) else body)
    return {"method": method, "path": path, "body": data, "type": type or (JSON_TYPE if body is not None else None), **kw}


def route(key, pid, reqs, sp=RICH_DISK, **kw):
    probe(key, pid, "http", sp, reqs=reqs, **kw)


def gc_http(key, name, *, only=None, **kw):
    """The requests of a case of the T050 oracle (`gen_connectors.py`), with its world (keychain, fake `claude`, agent folders)."""
    c = GC_CASES[name]
    sp = spec("rich", setup=c["setup"], dirs=c["dirs"], keychain=c["keychain"] or None, claude=c["claude"], fail=c["fail"], ccd=True)
    reqs = [{"method": r["method"], "path": r["path"], "body": bytes.fromhex(r["body_hex"]), "type": r["type"]} for r in c["requests"]]
    route(key, name, reqs if only is None else [reqs[i] for i in only], sp, **kw)


def define_http_probes():
    window = "since=2026-01-13T00:00:00Z&until=2026-01-20T00:00:00Z"
    q = lambda route, *queries: [R("GET", route + (f"?{x}" if x else "")) for x in queries]  # noqa: E731
    route("route: GET /api/usage/summary", "queries", q("/api/usage/summary", "", window, "workspace=acme", "account=Claude%20work", "workspace=nope",
                                                       "since=banana", "since=2026-01-13&until=2026-01-15"))
    route("route: GET /api/usage/breakdown", "queries", q("/api/usage/breakdown", "by=day", "by=model", "by=project", "by=tool", "by=day&" + window,
                                                         "by=week", "", "by=project&workspace=acme"))
    route("route: GET /api/usage/outliers", "queries", q("/api/usage/outliers", "", window, "workspace=acme", "account=nope"))
    route("route: GET /api/usage/sessions", "queries", q("/api/usage/sessions", "", window, "workspace=acme", "account=Claude%20work"))
    route("route: GET /api/dirs", "queries", q("/api/dirs", "path=@W@", "path=@W@/kit", "path=@W@/nope", "path=relative", "", "path=@W@/kit/alpha/../beta"))
    route("route: GET /api/workspaces", "list", q("/api/workspaces", ""))
    route("route: GET /api/workspaces/unassigned", "list", q("/api/workspaces/unassigned", ""))
    route("route: GET /api/warnings", "queries", q("/api/warnings", "", "workspace=acme", "account=1", "workspace=nope"))
    route("route: GET /api/plan", "queries", q("/api/plan", "", "workspace=acme", "account=Claude%20work", "account=nope"))
    route("route: GET /api/reports", "list", q("/api/reports", ""))
    route("route: GET /api/reports/{id}", "ids", q("/api/reports/1", "") + [R("GET", "/api/reports/2"), R("GET", "/api/reports/99"), R("GET", "/api/reports/abc")])
    route("route: GET /api/kit", "queries", q("/api/kit", "", "workspace=acme", "account=1", "workspace=nope"))
    route("route: GET /api/kit/workspaces", "queries", q("/api/kit/workspaces", "", window, "account=Claude%20work"))
    route("route: GET /api/kit/reports", "list", q("/api/kit/reports", ""))
    route("route: GET /api/kit/report-files", "queries", q("/api/kit/report-files", "workspace=acme&repo_no=0&folder=reports", "workspace=acme&repo_no=0&folder=smoke-reports",
                                                          "workspace=acme&repo_no=0&folder=docs", "workspace=acme&repo_no=7&folder=reports", ""))
    route("route: GET /api/kit/report-file", "queries", q("/api/kit/report-file", "workspace=acme&repo_no=0&folder=reports&path=index.html",
                                                         "workspace=acme&repo_no=0&folder=reports&path=session-001.json",
                                                         "workspace=acme&repo_no=0&folder=reports&path=nope.html",
                                                         "workspace=acme&repo_no=0&folder=reports&path=../index.html"))
    route("route: GET /api/board", "queries", q("/api/board", "", "workspace=acme", "account=1", "workspace=nope"))
    route("route: GET /api/accounts", "queries", q("/api/accounts", "", "by=provider", "by=colour"))
    route("route: GET /api/jobs", "list", q("/api/jobs", ""))
    route("route: GET /api/memory-graph", "queries", q("/api/memory-graph", "", "project=@W@/kit/alpha", "project=nope"))
    route("route: GET /api/store/health", "get-and-head", q("/api/store/health", "") + [R("HEAD", "/api/store/health")])
    route("route: GET /api/first-run", "list", q("/api/first-run", ""))
    route("route: GET /api/live", "first-event", [R("GET", "/api/live", stream=True, headers={"Accept": "text/event-stream"})])
    route("route: GET /api/live", "query-token", [R("GET", "/api/live", stream=True, query_token=True, headers={"Accept": "text/event-stream"}),
                                                 R("GET", "/api/live", stream=True, token=False, headers={"Accept": "text/event-stream"})])
    gc_http("route: GET /api/sync", "sync_every_step_kind", only=[0])
    gc_http("route: GET /api/connectors", "add_confirm", only=[1])
    item = json.dumps({"command": ["npx"], "env": {"T": "x"}, "headers": {}})
    route("route: GET /api/connectors", "configured", [R("GET", "/api/connectors")],
          spec(setup=gc.ACME + gc.REG_WORK, dirs=gc.DIRS, keychain={"easy-personal/slack": item}, claude={"<unset>": {"slack": gc.OURS("slack")}}, ccd=True))
    gc_http("route: GET /api/connectors/legacy", "legacy_adopt", only=[0])
    gc_http("route: POST /api/connectors/legacy", "legacy_adopt", only=[0, 1, 2, 3])
    # guard: the same answers on a missing token, a wrong host, a foreign origin (a route of its own: every route sits behind it)
    route("route: GET /api/workspaces", "guard", [R("GET", "/api/workspaces", token=False), R("GET", "/api/workspaces", headers={"Host": "evil.example"}),
                                                  R("GET", "/api/workspaces", headers={"Origin": "http://evil.example"}), R("GET", "/api/nope"),
                                                  R("GET", "/api/workspaces/1"), R("POST", "/api/usage/summary", {})])
    route("route: GET /api/workspaces", "guard-with-page", [R("GET", "/api/workspaces/1"), R("GET", "/api/nope"), R("POST", "/api/usage/summary", {}),
                                                            R("DELETE", "/api/usage/summary"), R("GET", "/api/workspaces", token=False)], RICH_DISK_STATIC,
          note="with the page served (a real deployment): Python's static mount answers where its routes would say 405")
    # writes: each on a fresh world, with the store rows, keychain and `claude` calls compared after
    route("route: POST /api/workspaces", "add", [R("POST", "/api/workspaces", {"name": "newone", "repos": ["/work/x", "/work/y/"], "color": "#112233", "description": "d"})], RICH)
    route("route: POST /api/workspaces", "refusals", [R("POST", "/api/workspaces", {"name": "acme"}), R("POST", "/api/workspaces", {"name": "Bad Name"}),
                                                     R("POST", "/api/workspaces", {"name": "x", "bogus": 1}), R("POST", "/api/workspaces", b"nope", "text/plain"),
                                                     R("POST", "/api/workspaces", b"{", JSON_TYPE)], RICH)
    route("route: PATCH /api/workspaces/{id}", "fields", [R("PATCH", "/api/workspaces/2", {"description": "changed", "repos": ["/work/new"]}),
                                                         R("PATCH", "/api/workspaces/99", {"description": "x"}), R("PATCH", "/api/workspaces/1", {"description": "x"})], RICH)
    route("route: PATCH /api/workspaces/{id}", "rename", [R("PATCH", "/api/workspaces/2", {"name": "firm"})], RICH)
    gc_http("route: PATCH /api/workspaces/{id}", "workspace_rename_applies")
    route("route: DELETE /api/workspaces/{id}", "delete", [R("DELETE", "/api/workspaces/3", {"confirm": True}), R("DELETE", "/api/workspaces/99", {"confirm": True}),
                                                          R("DELETE", "/api/workspaces/1", {"confirm": True})], RICH)
    gc_http("route: DELETE /api/workspaces/{id}", "workspace_delete_applies")
    tree = gen_cli.ACCOUNT_TREE
    acct = lambda *r: spec(tree=tree)  # noqa: E731
    route("route: POST /api/accounts", "add", [R("POST", "/api/accounts", {"path": "@W@/extra/.claude-x"}), R("POST", "/api/accounts", {"path": "@W@/nope"}),
                                              R("POST", "/api/accounts", {"path": 5})], acct())
    route("route: POST /api/accounts/{id}/confirm", "confirm", [R("POST", "/api/accounts/2/confirm", {"label": "Work renamed"}), R("POST", "/api/accounts/2/confirm", {"label": ""}),
                                                              R("POST", "/api/accounts/99/confirm", {"label": "x"}), R("POST", "/api/accounts/abc/confirm", {"label": "x"})], acct())
    route("route: POST /api/accounts/{id}/ignore", "ignore", [R("POST", "/api/accounts/3/ignore", {}), R("POST", "/api/accounts/1/ignore", {}),
                                                            R("POST", "/api/accounts/99/ignore", {})], acct())
    route("route: PUT /api/accounts/{id}/workspaces", "use", [R("PUT", "/api/accounts/2/workspaces", {"workspaces": ["acme", "personal"]}),
                                                            R("PUT", "/api/accounts/2/workspaces", {"workspaces": ["orphan"]}), R("PUT", "/api/accounts/2/workspaces", {"workspaces": "x"}),
                                                            R("PUT", "/api/accounts/99/workspaces", {"workspaces": []})], acct())
    gc_http("route: POST /api/connectors", "add_confirm", only=[0])
    gc_http("route: POST /api/connectors", "add_refusals")
    gc_http("route: DELETE /api/connectors/{name}", "remove_stored_registered", only=[0])
    gc_http("route: DELETE /api/connectors/{name}", "remove_pre_workspace_ours")
    gc_http("route: POST /api/connectors/{id}/move", "move_to_workspace")
    gc_http("route: PUT /api/connectors/{id}/alias", "alias_custom_then_default")
    gc_http("route: POST /api/sync", "sync_every_step_kind", only=[1, 2, 3])
    gc_http("route: POST /api/sync", "sync_refusals")
    # the static files of the page: both serve the same folder
    route("route: GET / (static files of the page)", "files", [R("GET", "/"), R("GET", "/index.html"), R("GET", "/assets/app.js"), R("GET", "/favicon.svg"),
                                                              R("GET", "/nope"), R("GET", "/assets/"), R("GET", "/%2e%2e/etc/passwd"), R("GET", "/..%2f..%2fetc/passwd"),
                                                              R("GET", "/.hidden"), R("POST", "/index.html", {})], RICH_DISK_STATIC)


# ---------------------------------------------------------------------------------------------------------------- MCP probes


def mcp(key, pid, calls, sp=RICH_DISK, init=False, **kw):
    """`calls`: (method, params, pick) or (tool, arguments): a tool call is `tools/call`. The tool's own `tools/list` entry is asked first."""
    full = []
    tool = key.split(": ", 1)[1] if key.startswith("mcp tool: ") else None
    if tool:
        full.append(("tools/list", None, tool))
    elif key.startswith("mcp prompt: "):
        full.append(("prompts/list", None, key.split(": ", 1)[1]))
    for c in calls:
        full.append(c if len(c) == 3 else ("tools/call", {"name": c[0], "arguments": c[1]}, None))
    probe(key, pid, "mcp", sp, calls=full, **kw)
    PROBES[-1]["init"] = init


def define_mcp_probes():
    window = {"since": "2026-01-13T00:00:00Z", "until": "2026-01-20T00:00:00Z"}
    mcp("mcp protocol: initialize", "handshake", [], init=True)
    mcp("mcp protocol: initialize", "protocol-ping", [("ping", None, None)])
    mcp("mcp protocol: initialize", "unknown-method", [("nope/method", None, None)])
    mcp("mcp protocol: initialize", "resources-list", [("resources/list", None, None)])
    mcp("mcp protocol: initialize", "stderr-log", [("ping", {})])
    PROBES[-1]["raw_stderr"] = True
    mcp("mcp tool: ping", "call", [("ping", {})])
    mcp("mcp tool: usage_summary", "calls", [("usage_summary", {}), ("usage_summary", window), ("usage_summary", {"since": "banana"}), ("usage_summary", {"since": 5})])
    mcp("mcp tool: usage_breakdown", "calls", [("usage_breakdown", {"by": b}) for b in ("day", "model", "project", "tool")] +
        [("usage_breakdown", {"by": "week"}), ("usage_breakdown", {}), ("usage_breakdown", {"by": "day", **window})])
    mcp("mcp tool: session_outliers", "calls", [("session_outliers", {}), ("session_outliers", window)])
    for tool in ("warnings", "improvement_plan", "kit_summary"):
        mcp(f"mcp tool: {tool}", "calls", [(tool, {}), (tool, {"workspace": "acme"}), (tool, {"account": "Claude work"}), (tool, {"workspace": "nope"})])
    mcp("mcp tool: kit_findings", "calls", [("kit_findings", {}), ("kit_findings", {"limit": 1}), ("kit_findings", {"since": "2026-01-06T00:00:00Z"}),
                                           ("kit_findings", {"limit": 0}), ("kit_findings", {"limit": 99})])
    mcp("mcp tool: connector_catalog", "call", [("connector_catalog", {})])
    mcp("mcp tool: connector_suggestions", "call", [("connector_suggestions", {})])
    item = json.dumps({"command": ["npx"], "env": {"T": "x"}, "headers": {}})
    stat = spec(setup=gc.ACME + gc.REG_WORK, dirs=gc.DIRS, keychain={"easy-personal/slack": item}, claude={"<unset>": {"slack": gc.OURS("slack")}}, ccd=True)
    mcp("mcp tool: connector_status", "keychain-and-claude", [("connector_status", {})], stat)
    mcp("mcp tool: connector_status", "no-claude", [("connector_status", {})], spec(keychain={"easy-personal/slack": item}, ccd=True))
    fixes = [{"text": "Run /compact earlier.", "number": 205000}, {"text": "Keep the prefix stable.", "tool": "Read"}]
    mcp("mcp tool: save_report", "valid-and-store", [("save_report", {"summary": "Invented summary.", "fixes": fixes}),
                                                    ("save_report", {"summary": "Scoped.", "fixes": [], "workspace": "acme"})], RICH)
    mcp("mcp tool: save_report", "refusals", [("save_report", {"summary": "", "fixes": []}), ("save_report", {"summary": "x" * 2001, "fixes": []}),
                                              ("save_report", {"summary": "ok", "fixes": [{"text": "t"}] * 6}), ("save_report", {"summary": "ok", "fixes": [], "workspace": "nope"}),
                                              ("save_report", {"summary": "my key is sk-abcdefghijklmnopqrstuvwx", "fixes": []}), ("save_report", {"fixes": []})], RICH)
    mcp("mcp prompt: investigate", "prompts", [("prompts/get", {"name": "investigate", "arguments": {}}, None),
                                               ("prompts/get", {"name": "investigate", "arguments": {"workspace": "demo"}}, None),
                                               ("prompts/get", {"name": "investigate", "arguments": {"workspace": "a\"b", "account": "c"}}, None),
                                               ("prompts/get", {"name": "nope", "arguments": {}}, None)])


# The README item each probe is expected to need (a probe that has none must be equal; one whose rule does not make it equal is `new`).
EXPECT = {
    ("cli: ingest", "canary-tree"): ["Ingest#1"], ("cli: ingest", "twice-then-usage"): ["Ingest#1"], ("cli: ingest", "no-store"): ["Ingest#1"],
    ("cli: ingest", "retention-zero"): ["T058#8"],
    ("cli: usage", "no-store"): ["T058#1"],
    ("cli: warnings", "bad-flag"): ["T058#8"], ("cli: plan", "bad-flag"): ["T058#8"], ("cli: kit", "bad-flag"): ["T058#8"],
    ("cli: kit-findings", "bad-limit"): ["T058#8"], ("cli: workspace list", "no-store"): ["T058#1"],
    ("cli: account list", "acct_bad_list_xby_colour"): ["T058#8"],
    ("cli: connector add", "add_yes"): ["T060#1"], ("cli: connector add", "add_ask_no"): ["T060#1"],
    ("cli: connector remove", "remove_stored_registered"): ["T060#3"], ("cli: connector remove", "remove_pre_workspace_ours"): ["T060#3"],
    ("cli: connector sync", "sync_every_step_kind"): ["T060#3"], ("cli: connector move", "move_ask_yes"): ["T060#3"],
    ("cli: connector alias", "alias_custom_then_default"): ["T060#3"], ("cli: connector adopt-legacy", "legacy_adopt"): ["T060#3"],
    ("cli: doctor", "doctor_clean"): ["T060#5"], ("cli: doctor", "doctor_registrations"): ["T060#5"], ("cli: doctor", "doctor_no_claude"): ["T060#5"],
    ("cli: schedule install", "dry-run"): ["Schedule#1"], ("cli: schedule install", "dry-run-retention"): ["Schedule#1"],
    ("cli: schedule install", "no-systemd"): ["Schedule#1"], ("cli: dashboard", "start-and-stop"): ["T051#1"],
    ("route: GET /api/first-run", "list"): ["T040#1"], ("route: GET /api/workspaces", "guard"): ["Static#1"],
    ("route: GET / (static files of the page)", "files"): ["Static#1"], ("route: GET /api/workspaces", "guard-with-page"): ["Static#1"],
    ("mcp protocol: initialize", "handshake"): ["McpInit#1"],
    ("mcp tool: usage_summary", "calls"): ["T047#1"], ("mcp tool: usage_breakdown", "calls"): ["T047#1"],
    ("mcp tool: save_report", "valid-and-store"): ["T047#6"], ("mcp tool: save_report", "refusals"): ["T047#1"],
    ("mcp tool: connector_status", "keychain-and-claude"): ["ConnTool#1"],
    ("route: GET /api/connectors", "configured"): ["ConnRoute#1"],
    ("cli: mcp", "bad-host"): ["McpHttp#1"], ("cli: mcp", "http-handshake-and-ping"): ["T051#2"], ("cli: exec", "in-the-main-binary"): ["T058#12"], ("cli: headers", "in-the-main-binary"): ["T058#12"],
}


def apply_expectations():
    known = {(p["key"], p["id"]) for p in PROBES}
    stale = sorted(set(EXPECT) - known)
    if stale:
        raise Refused(f"EXPECT names probes that do not exist: {stale}")
    for p in PROBES:
        p["explain"] = list(dict.fromkeys(p["explain"] + EXPECT.get((p["key"], p["id"]), [])))


# ---------------------------------------------------------------------------------------------------------------- running a probe


def blind_failure(res) -> str | None:
    """Why a run is a failed run, not a result: an app that did not answer, did not stop, or answered with the runner's own error. An error
    the app is meant to give (a 4xx of a refused request, a JSON-RPC error object, exit 2) is a result."""
    for app in ("py", "rs"):
        for zone_name, z in res[app].items():
            for i, step in enumerate(z["steps"]):
                if not isinstance(step, dict):
                    continue
                where = f"{app} {zone_name} step {i}"
                if isinstance(step.get("status"), str) and step["status"].startswith("error"):
                    return f"{where}: no HTTP answer ({step['status']})"
                if step.get("code") in ("timeout", "did not stop"):
                    return f"{where}: {step['code']}"
                answer = step.get("answer")
                if isinstance(answer, dict) and isinstance(answer.get("error"), str):
                    return f"{where}: {answer['error']}"
                if "api_status" in step and step["api_status"] != 200:
                    return f"{where}: the server did not answer its route (status {step['api_status']})"
                if step.get("step") in ("initialize", "initialized", "ping-tool") and isinstance(step.get("status"), int) and step["status"] >= 400:
                    return f"{where}: {step['step']} answered {step['status']}"
    return None


def run_probe(p, base: Path, canaries_dir: Path) -> dict:
    """Both apps on the same world path, one after the other; `py` and `rs` are the outcomes per zone."""
    out = {"key": p["key"], "id": p["id"], "kind": p["kind"]}
    for app in ("py", "rs"):
        used = {**p, "spec": p["rs_spec"]} if app == "rs" and p.get("rs_spec") else p
        out[app] = EXECUTORS[p["kind"]](used, app, base, canaries_dir)
    blind = blind_failure(out)
    if blind and not p.get("allow_error"):
        raise Refused(f"{p['key']} / {p['id']}: {blind}: a probe that fails on both sides would look equal")
    return out


def equal_zone(p, py, rs) -> bool:
    return (compare(py["steps"], rs["steps"]) and compare(py["state"], rs["state"]) and py.get("exit") == rs.get("exit")
            and py.get("stderr") == rs.get("stderr"))


def setup_canaries(base: Path) -> Path:
    dest = base / "canary-projects"
    canaries().render_fixture(dest)
    return dest


def selected(only):
    return [p for p in PROBES if not only or any(o in f"{p['key']} / {p['id']}" for o in only)]


def define_control_probe():
    """A negative control: Rust runs on a world with one more workspace. The run fails unless the comparison calls it a difference."""
    probe("control: negative", "different-store", "cli", RICH, steps=[(["workspace", "list"], "")])
    PROBES[-1]["rs_spec"] = spec(setup=["INSERT INTO workspaces (id, name, is_orphan) VALUES (9, 'extra', 0)"])


def define_all():
    define_control_probe()
    define_cli_probes()
    define_special_cli_probes()
    define_http_probes()
    define_mcp_probes()
    apply_expectations()


def main_trial(only):
    define_all()
    with tempfile.TemporaryDirectory(prefix="parity-world-") as tmp:
        base = Path(tmp)
        canary_dir = setup_canaries(base)
        for p in selected(only):
            res = run_probe(p, base, canary_dir)
            for zone in res["py"]:
                kind, used = classify_zone(p, res["py"][zone], res["rs"][zone])
                orders = sum(order_only(a, b) for a, b in zip(res["py"][zone].get("orders", []), res["rs"][zone].get("orders", [])))
                print(f"{kind.upper():6} {p['key']} / {p['id']} [{zone}] {used or ''}" + (f"  KEY-ORDER differs on {orders}" if orders else ""))
                if kind == "new":
                    for i, (a, b) in enumerate(zip(res["py"][zone]["steps"], res["rs"][zone]["steps"])):
                        if not compare(a, b):
                            for path, x, y in leaf_diffs(a, b):
                                print(f"  step {i} {path}\n     py: {json.dumps(x, ensure_ascii=False)[:300]}\n     rs: {json.dumps(y, ensure_ascii=False)[:300]}")
                    for path, x, y in leaf_diffs(res["py"][zone]["state"], res["rs"][zone]["state"]):
                        print(f"  state {path}\n     py: {json.dumps(x, ensure_ascii=False)[:300]}\n     rs: {json.dumps(y, ensure_ascii=False)[:300]}")


# ---------------------------------------------------------------------------------------------------------------- the whole run

# A surface the runner does not run, with the reason (only one that cannot run offline). Every other surface of the inventory needs a probe.
NOT_RUN: dict[str, str] = {}
# Rows that are not a surface of the Python inventory but are compared too (the protocol under the MCP tools).
EXTRA_KEYS = ["mcp protocol: initialize"]
# Findings: key -> (cause in one line, proposed task). A `differs: new` row with no entry here fails the run (a finding needs its proposed task).
FINDINGS: dict[str, tuple[str, str]] = {
    "route: GET /api/workspaces": (
        "With the page served (Python's built bundle, as on a real machine; Rust's `serve --static-dir`), a `GET` of a route that has only write methods is `404 Not Found` in Python and `405` in Rust: "
        "Python's static mount at `/` matches the path before the route's own method check can answer 405. README T048 item 9 says Rust's is `405`; it does not say Python's differs. "
        "(Without a page the two agree on 405; the plain-text `Not Found` / `Method Not Allowed` bodies against Rust's empty ones are `Static` item 1.)",
        "C0, Risk Low: add the 404-versus-405 line to the T048 \"Decided for you\" list, or make Rust answer 404 there when the page is served."),
    "cli: usage": (
        "Two usage errors give another text and exit 2 in both, with an empty stdout. `--by week`: Python's argparse says `invalid choice: 'week' (choose from day, model, project, tool)` "
        "after its usage text; Rust says the core's one line `easy-personal: by must be one of day, model, project, tool` (T058 item 8 names *clap's* text, and this is not clap's). "
        "`--since banana`: Python's argparse says `argument --since: invalid iso_datetime value: 'banana'`; Rust says `easy-personal: Invalid isoformat string: 'banana'` "
        "(T058 item 7 says only that Rust echoes the bad value, \"as Python does\", not that the text differs). The `acct_bad_list_xby_colour`-style errors that clap itself prints are item 8 and stay `differs: README`.",
        "C0, Risk Low: document both texts in T058's \"Decided for you\" list, or print Python's."),
    "cli: --version": (
        "Python prints the bare package version (`0.4.0`); Rust prints clap's `easy-personal-rs 0.2.0` (its Cargo version). The README's build section shows the Rust text but "
        "lists it in no \"Decided for you\" list. The numbers differ because the two packages are versioned apart.",
        "C0, Risk Low: decide whether `--version` should print `easy-personal-rs <version>` on purpose (then add a line to a README list) or Python's bare form."),
    "route: GET /api/live": (
        "The content type is `text/event-stream; charset=utf-8` in Python and `text/event-stream` in Rust. The README (T056 \"The route\") states Rust's value but not that Python's differs. "
        "A browser `EventSource` reads both.",
        "C0, Risk Low: add `; charset=utf-8` to the Rust header (or add the difference to the T056 list)."),
    "mcp protocol: initialize": (
        "Two differences no README item names. (1) `initialize` capabilities: Python also sends `experimental: {}`, `resources: {subscribe, listChanged: false}` and `listChanged: false` "
        "under `tools` and `prompts`; Rust sends `prompts: {}` and `tools: {}` only (the README names the `prompts` one, in the `investigate` paragraph; the server name and version are named by "
        "\"McpInit\" and are not part of this finding). (2) An unknown JSON-RPC method: Python answers `-32602` `Invalid request parameters`, Rust `-32601` with the method name "
        "(`-32601` is the JSON-RPC code for it). (3) stderr of `mcp` over stdio (probe `stderr-log`, not stripped): Python's SDK writes `Processing request of type <X>` for every request, Rust writes nothing. "
        "Every other MCP probe drops those Python lines (rule N7) so this is reported once.",
        "C1, Risk Low: align the capabilities (a client that checks `resources` sees none) or list both differences in the T047 \"Decided for you\" list; keep `-32601` on purpose and say so."),
    "cli: mcp": (
        "`mcp --transport http` stopped with Ctrl-C: Python's process ends by the SIGINT signal (exit -2, shell 130) after uvicorn's shutdown, and uvicorn prints one access-log line per request "
        "on stdout; Rust exits 0 with an empty stdout. The README (\"`mcp --transport http`\") states Rust's side (\"stdout stays empty\", \"Ctrl-C stops the server with exit 0\") and does not say Python differs. "
        "(The SSE body and the session id are named by T051 item 2 and are not part of this finding.)",
        "C0, Risk Low: add one line to the T051 \"Decided for you\" list (Rust's exit 0 is the better one); nothing to change in the code."),
    "mcp prompt: investigate": (
        "`prompts/get` of an unknown prompt: Python's error code is `0`, Rust's `-32602`; the message `Unknown prompt: nope` is equal. The README says only \"a JSON-RPC error\".",
        "C0, Risk Low: add the code to the T047 \"Decided for you\" list (Rust's `-32602` is the correct one)."),
}
ORDER_RULES = {"T059#6": ("T059", 6, ("json key order in the route", "sorted keys"))}
EXPLAIN_WORDS[("T059", 6)] = ORDER_RULES["T059#6"][2]
EXPECT[("route: GET /api/store/health", "get-and-head")] = ["T059#6"]


CLOCK_NOTE = "no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens"
CLOCK_KEYS = {"cli: warnings", "cli: plan", "cli: kit", "cli: outliers", "cli: kit-findings", "route: GET /api/warnings", "route: GET /api/plan",
              "route: GET /api/kit", "route: GET /api/usage/outliers", "mcp tool: warnings", "mcp tool: improvement_plan", "mcp tool: kit_summary",
              "mcp tool: session_outliers", "mcp tool: kit_findings"}


def rule_ref(rid: str) -> str:
    if rid in ORDER_RULES:
        ref, number, _ = ORDER_RULES[rid]
    else:
        ref, number = RULES[rid].ref
    return f"{ref} item {number}"


def zone_orders_differ(py, rs) -> int:
    return sum(order_only(a, b) for a, b in zip(py.get("orders", []), rs.get("orders", [])))


def classify_probe(p, res):
    """(kind, rule ids, number of bodies whose key order differs) over the zones of one probe."""
    kinds, used, orders = [], [], 0
    for zone in res["py"]:
        py, rs = res["py"][zone], res["rs"][zone]
        kind, ids = classify_zone(p, py, rs)
        differing = zone_orders_differ(py, rs)
        if kind == "equal" and differing:
            order_ids = [r for r in p["explain"] if r in ORDER_RULES]
            if order_ids:
                kind, ids = "readme", order_ids
            orders += differing
        kinds.append(kind)
        used += ids
    kind = "new" if "new" in kinds else "readme" if "readme" in kinds else "equal"
    return kind, list(dict.fromkeys(used)), orders


def describe_probe(p) -> tuple[str, str, str]:
    """(what is run, on which world, the one-line command) for a finding."""
    sp = p["spec"]
    world = f"store={sp['store']}" + (f", {len(sp['setup'])} setup statements" if sp["setup"] else "") + (", fake keychain" if sp["keychain"] else "") + (
        ", fake `claude`" if sp["claude"] is not None else "") + (f", TZ {sp['tz']}" if sp["tz"] != "UTC" else "")
    if p["kind"] == "cli":
        steps = " ; ".join(" ".join(a) + (f"   (stdin {stdin!r})" if stdin else "") for a, stdin in p["steps"])
        return steps, world, "cli"
    if p["kind"] == "http":
        return " ; ".join(f"{r['method']} {r['path']}" + (f" {r['body'].decode()[:120]}" if r.get("body") else "") for r in p["reqs"]), world, "http"
    if p["kind"] == "mcp":
        return " ; ".join(f"{m} {json.dumps(a)[:100] if a is not None else ''}" for m, a, _ in p["calls"]) or "initialize", world, "mcp"
    return p["id"], world, "custom"


def cut(value, n=220) -> str:
    text = json.dumps(value, ensure_ascii=False)
    return text if len(text) <= n else text[:n] + "..."


def undescribed_diffs(p, py, rs):
    """The differences left after the probe's own README rules: what no README item describes."""
    for rid in p["explain"]:
        if rid in RULES:
            py, rs = RULES[rid](py, rs)
    out = [("steps" + path, a, b) for path, a, b in leaf_diffs(py["steps"], rs["steps"], "", None, 8)]
    out += [("state" + path, a, b) for path, a, b in leaf_diffs(py["state"], rs["state"], "", None, 4)]
    return out[:8]


def run_all(only=None):
    define_all()
    probes = selected(only)
    base = Path(tempfile.mkdtemp(prefix="parity-world-"))
    results, captured = [], []
    try:
        canary_dir = setup_canaries(base)
        for p in probes:
            res = run_probe(p, base, canary_dir)
            kind, used, orders = classify_probe(p, res)
            if p["key"] == "control: negative":
                if kind != "new":
                    raise Refused("the negative control was not seen as a difference: the comparison is blind")
                continue
            results.append({"probe": p, "res": res, "kind": kind, "used": used, "orders": orders})
            captured.append(json.dumps(res, ensure_ascii=False, sort_keys=True))
        rust_only = rust_only_surfaces(base)
    finally:
        shutil.rmtree(base, ignore_errors=True)
    return results, captured, rust_only


def rust_only_surfaces(base: Path) -> list[str]:
    """What `easy-personal-rs` has and Python's inventory lacks: its subcommands from `--help`, and the route it serves besides Python's."""
    done = subprocess.run([str(BINARY), "--help"], capture_output=True, text=True, env={"PATH": "/usr/bin:/bin"}, cwd=base, check=False)
    commands = set(re.findall(r"(?m)^  ([a-z][a-z-]*)  ", done.stdout.split("Commands:")[1].split("Options:")[0])) - {"help"}
    py_top = {k.split(": ", 1)[1].split()[0] for k in inventory() if k.startswith("cli: ") and not k.startswith("cli: -")}
    extra = [f"cli: {c}" for c in sorted(commands - py_top)]
    return extra + ["route: GET /api/version"]


def build_rows(results, inv):
    by_key: dict[str, list] = {}
    for r in results:
        by_key.setdefault(r["probe"]["key"], []).append(r)
    rows = []
    for key in sorted(set(inv) | set(EXTRA_KEYS)):
        if key in NOT_RUN:
            rows.append({"key": key, "probes": 0, "result": f"not run: {NOT_RUN[key]}", "note": "", "items": []})
            continue
        items = by_key.get(key)
        if not items:
            raise Refused(f"no probe for {key!r}: write one (or list it in NOT_RUN with the reason)")
        kinds = [i["kind"] for i in items]
        used = sorted(dict.fromkeys(rule_ref(rid) for i in items for rid in i["used"]))
        orders = sum(i["orders"] for i in items)
        if "new" in kinds:
            result = "differs: new"
        elif "readme" in kinds:
            result = "differs: README " + ", ".join(used)
        else:
            result = "equal except JSON key order" if orders else "equal"
        notes = []
        differing = [f"{i['probe']['id']}" for i in items if i["kind"] != "equal"]
        if differing and len(items) > 1:
            notes.append(f"{len(differing)} of {len(items)} probes differ: " + ", ".join(f"`{d}`" for d in differing[:4]) + (" ..." if len(differing) > 4 else ""))
        if orders:
            notes.append(f"key order of {orders} {'body' if orders == 1 else 'bodies'} differs (N6)")
        extra = [i["probe"]["note"] for i in items if i["probe"]["note"]]
        notes += list(dict.fromkeys(extra))
        if key in CLOCK_KEYS:
            notes.append(CLOCK_NOTE)
        rows.append({"key": key, "probes": len(items), "result": result, "note": "; ".join(notes).replace("|", "/"), "items": items})
    return rows


def finding_text(row) -> str:
    key = row["key"]
    cause, task = FINDINGS.get(key, ("(no cause written)", None))
    if task is None:
        raise Refused(f"{key!r} is `differs: new` and has no entry in FINDINGS (cause and proposed task)")
    lines = [f"### Finding: `{key}`", "", f"Cause: {cause}", ""]
    shown = 0
    for item in row["items"]:
        if item["kind"] != "new" or shown >= 3:
            continue
        shown += 1
        p = item["probe"]
        what, world, kind = describe_probe(p)
        lines += [f"Reproduce: `./parity/.venv/bin/python parity/run_parity.py --only '{p['key']} / {p['id']}'` (probe `{p['id']}`; world: {world}).",
                  f"Run: `{what}`" if kind != "custom" else f"Run: probe `{p['id']}`", ""]
        for zone in item["res"]["py"]:
            kind_z, _ = classify_zone(p, item["res"]["py"][zone], item["res"]["rs"][zone])
            if kind_z != "new":
                continue
            for path, a, b in undescribed_diffs(p, item["res"]["py"][zone], item["res"]["rs"][zone]):
                lines += [f"- at `{path}` (zone {zone}):", f"  - Python: `{cut(a)}`", f"  - Rust: `{cut(b)}`"]
            break
        lines.append("")
    lines += [f"Proposed task: {task}", ""]
    return "\n".join(lines)


def render(rows, inv, rust_only, scanned) -> str:
    n = lambda pred: sum(1 for r in rows if pred(r["result"]))  # noqa: E731
    counts = (n(lambda x: x == "equal"), n(lambda x: x == "equal except JSON key order"), n(lambda x: x.startswith("differs: README")),
              n(lambda x: x == "differs: new"), n(lambda x: x.startswith("not run")))
    out = ["# T061: parity of Python and Rust, surface by surface", "",
           "Made by `rust/parity/run_parity.py`; checked by `rust/parity/test_parity_report.py`. Nothing in this file is written by hand.", "",
           f"Counts: equal {counts[0]} + equal except JSON key order {counts[1]}, differs README {counts[2]}, differs new {counts[3]}, not run {counts[4]}.", "",
           "`equal except JSON key order`: the parsed bodies are equal and the key order of at least one body differs (Python sends insertion order, Rust sorted; rule N6).", "",
           f"{len(inv)} surfaces were read from the Python app (argparse tree, route table, MCP tools and prompt); {len(EXTRA_KEYS)} protocol row is added. "
           f"{sum(r['probes'] for r in rows)} probes ran, each on the Python app and then on the Rust app over the same world at the same path. "
           f"Leak scan: {scanned} captured outputs and this report scanned with the repo canaries (`rust/parity/leak_rust.py` scanner, `Canaries.find`): 0 hits.", "",
           "## How it ran", "",
           "- A *probe* is one or more calls of a surface. Each probe builds a world (a copy of `store-rich`, the agent folders, a fake `claude`, a file keychain, "
           "fake `systemctl` and `crontab`), runs Python (`python -c \"...cli.main()\"` over this checkout's `src/`, or its `serve` and `mcp`), removes the world, builds "
           "the same world at the same path, and runs `target/debug/easy-personal-rs`. The two never run together on one store.",
           "- The routes are asked over a real socket: Python's `serve` runs after `LiveTail().start()` (what `dashboard` does after its ingest), with the page's folder pointed at the world's; "
           "Rust's is `serve --no-ingest [--static-dir]`. `dashboard` itself is probed separately, with its ingest.",
           "- JSON key order: Python sends insertion order, Rust sorted. The headline counts `equal except JSON key order` apart from `equal`. The rule is the project's "
           "(`rust/parity/README.md`, \"Key order: which rule wins (T007)\", and `rust/README.md` T059 item 6); see N6. MCP result key order is not tracked.",
           "- Both children get an environment built from scratch: no `CLAUDE_CONFIG_DIR`, no display, a dead D-Bus address, `HOME`, `EASY_PERSONAL_HOME`, `EASY_PERSONAL_AGENT_ROOT` "
           "and the other folders inside the world, `TZ` (UTC; `Pacific/Kiritimati` and `Pacific/Pago_Pago` too for the rows that print a time). No owner folder, real keychain item or real `claude` is touched.",
           "- Compared: CLI = exit code, stdout, stderr; route = status, content type, parsed JSON body (or the text); MCP = the `tools/list` entry and each call's result; and after every probe "
           "the store rows that changed, the keychain items, the `claude` calls and the fake `claude`'s state.",
           "- A row is `differs: README <list> item <n>` only when that README item exists, holds the words the runner states for it, and a transformation that stands for the item makes the two outputs "
           "equal (the part the item does not describe stays, and the row is `differs: new`). The test checks the item and its words.",
           "- The `<list>` names: `Tnnn` is the numbered \"Decided for you (Tnnn ...\" list of `rust/README.md` (T047's is the inline `(1) (2)` list; T040's is one paragraph, item 1). "
           "`Ingest` is the table \"Differences from Python\" of \"Ingest (P2a, P2b)\" (rows count from 1), `Schedule` the bullets of \"Scheduled ingest (P2c)\", `McpHttp` the table of "
           "\"`mcp --transport http`\", and `Static`, `ConnRoute`, `ConnTool`, `McpInit` single README paragraphs (item 1). These sections state a difference but are not \"Decided for you\" lists.", "",
           "## Result per surface", "", "| Surface | Probes | Result | Note |", "|---|---|---|---|"]
    out += [f"| `{r['key']}` | {r['probes']} | {r['result']} | {r['note']} |" for r in rows]
    out += ["", "## Findings (`differs: new`)", ""]
    new_rows = [r for r in rows if r["result"] == "differs: new"]
    out += ["None." if not new_rows else "Each is reported, not fixed (AC5). The proposed task names a Complexity and a Risk.", ""]
    out += [finding_text(r) for r in new_rows]
    out += ["## Differences the README already names", "", "| README item | What it says (start) | Rows |", "|---|---|---|"]
    cited: dict[str, list[str]] = {}
    for r in rows:
        for ref in re.findall(r"([A-Za-z0-9]+ item \d+)", r["result"]) if r["result"].startswith("differs: README") else []:
            cited.setdefault(ref, []).append(r["key"])
    for ref, keys in sorted(cited.items()):
        name, number = ref.split(" item ")
        body = " ".join((readme_item(name, int(number)) or "?").split())
        out.append(f"| {ref} | {body[:150].replace('|', '/')}... | {', '.join(f'`{k}`' for k in keys)} |")
    out += ["", "## Rules that make two outputs comparable", "",
            "- **N1** The world's folder, each app's launcher and binary and the Python interpreter are written as `<WORK>`, `<LAUNCHER>`, `<RUST_BIN>`, `<PYTHON>`. Both apps run at the same world path.",
            "- **N2** A line that starts `easy-personal-rs: ` is read as `easy-personal: ` (README, T058 \"Exit codes\").",
            "- **N3** The clock's own values (`as_of`, `as_of_utc`, `now_as_of_utc`, `saved_utc`, as a JSON key or inside a JSON text) and the bare `saved_utc` column of `investigate_reports` "
            "(the second the report was saved in: the two apps run a moment apart) are `<NOW>`. Rust has no clock hook: both apps read the machine's clock, and the rich store's events are months old, "
            "so every rule abstains (as in `gen_cli.py`).",
            "- **N4** Ids the app makes at random: a new connector id (`c` and 8 hex digits) and `meta.machine_id`. Session and message ids of the store are never rewritten.",
            "- **N5** The keychain is compared as its items; a world with no backend is `none` for both.",
            "- **N6** A route body is compared as parsed JSON (key order ignored, list order kept). Python sends keys in insertion order, Rust sorted. The rule is the project's own: "
            "`rust/parity/README.md`, \"Key order: which rule wins (T007)\" (the Rust tests compare parsed JSON), and `rust/README.md` T059 item 6 for `/api/store/health`, which cites it. "
            "A row whose bodies are equal except for key order is counted apart in the headline (`equal except JSON key order`) and its Note gives the number of bodies; "
            "T059 item 6 is the only README item that names it, so that row is `differs: README`. MCP result key order is not tracked: tool results are compared as parsed JSON "
            "(the text content is parsed when it is JSON).",
            "- **N7** Python's MCP SDK writes `Processing request of type <X>` on stderr for every request; Rust writes nothing. Those lines are dropped from every MCP probe's stderr, "
            "except in `mcp protocol: initialize / stderr-log`, which compares stderr as it is and carries the finding.", "",
            "## Rust-only surfaces (not in the Python inventory, not compared)", ""]
    out += [f"- `{k}`" for k in rust_only]
    out += ["", "## What this run does not reach", "",
            "- The real keychain, the real `claude`, `systemctl`/`crontab` and the real browser: fakes only. `schedule install` is run with `--dry-run` and with no systemd (the cron path); the install itself is not.",
            "- Python's live tail writes events to the store (README T049 item 1: Rust's never does); the world holds no transcript, so the tail has nothing to read and `GET /api/live` is compared on its first (empty) snapshot.",
            "- The clock: see N3. Rules that fire, and the improvement plan over a store with recent events, are proven by the frozen-clock goldens, not here.",
            "- Performance, fuzzing, a second store, macOS and Windows (cut list of the guide).", "",
            "## Known limits", "",
            "- The negative control (Rust on a store with one more workspace) proves the CLI stdout comparison only, not the state, keychain, `claude`-call, HTTP or MCP comparisons.",
            "- Rule T058 item 1 (`r_missing_store`) checks Rust's exact refusal and, for Python, only that its step 0 exited 0 and that store rows were written.",
            "- Rule T047 item 1 (`r_argument_error`) is applied to a missing argument as well as a wrong type; the item names only \"a wrong argument type\" (`save_report` without `summary`, `usage_breakdown` without `by`).",
            "- Rule T047 item 6 sets column 2 (`server_version`) of every `investigate_reports` row to `<VERSION>` without checking it.",
            "- Rule T060 item 3 drops every `claude mcp get` call, argv included: a wrong `get` argument would not show.",
            "- Mask N4's `c` + 8 hex digits is applied to all text, not only to ids.",
            "- A SIGTERM to the runner skips its `finally` blocks and leaves the world folder in the temp folder.",
            "- Python's `dashboard` command (probe `cli: dashboard`) cannot be told a static folder, so it serves the packaged bundle if the machine has one; the probe asks one API route, which the bundle does not change.",
            "- The static fixture holds no dotfile and no symlink (the README's `Static` paragraph names both).",
            "- Rules N3 and the clock caveat: the rows for warnings, plan, kit, outliers and kit-findings carry it in their Note.", ""]
    return "\n".join(out)


def check_leaks(text: str, captured: list[str]) -> int:
    hits = [("report", scan(text))] + [(f"output {i}", scan(c)) for i, c in enumerate(captured)]
    bad = [(where, h) for where, h in hits if h]
    if bad:
        for where, h in bad:
            print(f"LEAK in {where}: {[(name, form) for name, form in h]}", file=sys.stderr)
        raise Refused("a canary is in a captured output or in the report")
    return len(captured)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--only", action="append", help="run the probes whose `key / id` holds this text and print what differs (no report)")
    ap.add_argument("--list", action="store_true", help="print the inventory read from Python")
    ap.add_argument("--check", action="store_true", help="run and compare with the committed report instead of writing it")
    ap.add_argument("--out", type=Path, default=REPORT)
    args = ap.parse_args(argv)
    # A job started in the background of a non-interactive shell has SIGINT ignored, and `exec` keeps that: the children would not end by the
    # signal (Python's `mcp --transport http` shows exit -2 on Ctrl-C). A handler, unlike SIG_IGN, is reset to the default by `exec`.
    signal.signal(signal.SIGINT, signal.default_int_handler)
    if signal.getsignal(signal.SIGTERM) == signal.SIG_IGN:
        signal.signal(signal.SIGTERM, signal.SIG_DFL)
    if args.list:
        print("\n".join(inventory()))
        return 0
    if args.only:
        main_trial(args.only)
        return 0
    for need in (BINARY, RS_LAUNCHER):
        if not need.exists():
            raise Refused(f"{need} does not exist: run `cargo build` in rust/ first")
    for name in ("EASY_PERSONAL_HOME", "EASY_PERSONAL_AGENT_ROOT", "EASY_PERSONAL_CLAUDE_PROJECTS"):
        os.environ.pop(name, None)
    gen_golden.refuse_real_paths(args.out.resolve())
    if os.environ.get("CLAUDE_CONFIG_DIR"):
        raise Refused("CLAUDE_CONFIG_DIR is set: run with `env -u CLAUDE_CONFIG_DIR`")
    inv = inventory()
    results, captured, rust_only = run_all()
    rows = build_rows(results, inv)
    text = render(rows, inv, rust_only, 0)
    scanned = check_leaks(text, captured)
    text = render(rows, inv, rust_only, scanned)
    check_leaks(text, [])
    if args.check:
        same = args.out.exists() and args.out.read_text() == text
        print("the report equals a fresh run" if same else "the report differs from a fresh run")
        return 0 if same else 1
    args.out.write_text(text)
    counts = re.search(r"^Counts: .*$", text, re.M).group(0)
    print(f"{len(rows)} rows, {sum(r['probes'] for r in rows)} probes -> {args.out}\n{counts}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
