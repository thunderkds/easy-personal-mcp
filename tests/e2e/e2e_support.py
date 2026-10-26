"""Support for the HUN-71 end-to-end suite: a fake machine, two surfaces, a guard, a leak scan.

`World` is one temp machine: an agent root with three Claude accounts + one Codex (+ files that
match `~/.claude*` but are not accounts), a store dir, a fake keychain (JSON file), a fake `claude`
(per-CLAUDE_CONFIG_DIR state, logs the config dir of every call) and three git repos. Nothing of the
owner's machine is used: HOME, EASY_PERSONAL_HOME, EASY_PERSONAL_AGENT_ROOT and PATH all point inside
the World; `NoTouchGuard` proves the real ones were not touched.

`CliSurface` drives the real CLI in subprocesses. `ApiSurface` drives the dashboard JSON API in
process (Starlette TestClient with the per-launch token). Both expose the same methods so one flow
can run on either and the end states can be compared.
"""

import argparse
import base64
import contextlib
import importlib.util
import io
import json
import os
import re
import secrets
import shutil
import sqlite3
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import quote

import keyring
import pytest

TESTS = Path(__file__).resolve().parents[1]
SRC = str(TESTS.parent / "src")
E2E_DIR = str(Path(__file__).resolve().parent)
REAL_HOME = Path(os.path.expanduser("~"))  # taken at import, before any HOME patch
TOKEN, PORT = "e2e-dashboard-token", 4333
ID = re.compile(r"\bc[0-9a-f]{8}\b")


def _load_connectors_conftest():
    """The existing fakes (`FAKE_CLAUDE` script, `FakeClaude`), reused from tests/connectors/conftest.py."""
    sys.path.insert(0, str(TESTS / "connectors"))
    try:
        spec = importlib.util.spec_from_file_location(
            "_connectors_conftest_for_e2e", TESTS / "connectors" / "conftest.py"
        )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
    finally:
        sys.path.remove(str(TESTS / "connectors"))


_fakes = _load_connectors_conftest()
FAKE_CLAUDE, FakeClaude = _fakes.FAKE_CLAUDE, _fakes.FakeClaude


def canary(label: str) -> str:
    return f"E2ECANARY-{label}-{secrets.token_hex(6)}"


# --- the transcripts -------------------------------------------------------------------------

CANARIES = {  # planted in fixtures / typed secrets: must never leave the keychain or the transcript
    "slack_acme": canary("slack-acme"),
    "slack_personal": canary("slack-personal"),
    "gmail_path": "/planted/" + canary("gmail-oauth") + ".json",
    "legacy": canary("legacy-discord"),
    "prompt": canary("prompt-text"),
    "assistant": canary("assistant-text"),
    "tool_output": canary("tool-output"),
    "url_token": canary("url-token"),
    "bash_secret": canary("bash-secret"),
    "claude_json": canary("claude-json-content"),
    "codex_toml": canary("codex-toml-content"),
}
TYPED_SECRETS = ("slack_acme", "slack_personal", "gmail_path", "legacy")


def _rec(kind, sid, ts, cwd, uuid, message=None, **extra):
    record = {"type": kind, "sessionId": sid, "timestamp": ts, "cwd": cwd, "uuid": uuid,
              "version": "2.1.302", "gitBranch": "main", "isSidechain": False}  # fmt: skip
    if message is not None:
        record["message"] = message
    return json.dumps(record | extra)


def _assistant(sid, mid, ts, cwd, tokens, tools=()):
    inp, out, cc, cr = tokens
    usage = {"input_tokens": inp, "output_tokens": out,
             "cache_creation_input_tokens": cc, "cache_read_input_tokens": cr}  # fmt: skip
    content = [{"type": "text", "text": CANARIES["assistant"]}, *tools]
    message = {"id": mid, "role": "assistant", "model": "claude-fixture-model-1",
               "content": content, "usage": usage}  # fmt: skip
    return _rec("assistant", sid, ts, cwd, "u-" + mid, message)


def _user(sid, ts, cwd, n):
    text = CANARIES["prompt"]
    return [
        _rec("user", sid, ts, cwd, f"p-{sid}-{n}", {"role": "user", "content": text}),
        _rec("user", sid, ts, cwd, f"r-{sid}-{n}", {"role": "user", "content": [
            {"type": "tool_result", "content": CANARIES["tool_output"]}]}),
    ]  # fmt: skip


def _tool(name, **tool_input):
    return {"type": "tool_use", "id": "toolu_x", "name": name, "input": tool_input}


# message id -> (account basename, project key, (in, out, cache_create, cache_read))
MESSAGES = {
    "msgD1": (".claude", "alpha", (100, 50, 10, 5)),
    "msgD2": (".claude", "alpha", (200, 20, 0, 0)),
    "msgW1": (".claude-work", "beta", (300, 30, 0, 40)),
    "msgW2": (".claude-work", "beta", (400, 40, 0, 0)),
    "msgW3": (".claude-work", "alpha/pkg", (500, 60, 7, 0)),  # a subfolder of repo alpha
    "msgF1": (".claude-wife", "gamma", (50, 5, 0, 0)),
}


def expected_totals(keys=None, projects=None):
    """(messages, input, output) over the unique planted messages; msgD1/msgD2 are also copied into wife."""
    rows = [v for k, v in MESSAGES.items() if keys is None or k in keys]
    rows = [r for r in rows if projects is None or r[1].split("/")[0] in projects]
    return len(rows), sum(r[2][0] for r in rows), sum(r[2][1] for r in rows)


# --- one fake machine ------------------------------------------------------------------------


class World:
    def __init__(self, base: Path):
        self.base = base
        (base / "agents").mkdir(parents=True)
        self.agents = (base / "agents").resolve()
        self.data = base / "epm-home"
        self.fakehome = base / "home"
        self.bin = base / "bin"
        self.claude_log = base / "claude.log"
        self.claude_state = base / "claude-state.json"
        self.keyring_file = base / "keyring.json"
        self.outputs: list[tuple[str, object]] = []
        for p in (self.fakehome, self.bin, base / "no-verifier"):
            p.mkdir(parents=True)
        self.repos = {}
        for name in ("alpha", "beta", "gamma"):
            repo = (base / "repos" / name)
            (repo / "pkg").mkdir(parents=True)
            subprocess.run(["git", "init", "-q", str(repo)], check=True, capture_output=True)
            self.repos[name] = repo.resolve()
        self._build_bin()
        self._build_agents()
        self.agent_snapshot = tree_snapshot(self.agents)

    # fake executables ---------------------------------------------------------------------
    def _build_bin(self):
        claude = self.bin / "claude"
        claude.write_text(FAKE_CLAUDE.format(python=sys.executable))
        claude.chmod(0o755)
        launcher = self.bin / "easy-personal-launch"
        launcher.write_text("#!/bin/sh\nexit 0\n")
        launcher.chmod(0o755)
        (self.bin / "git").symlink_to(shutil.which("git"))  # the only real tool on the fake PATH
        self.fake = FakeClaude(self.claude_log)

    # agent folders ------------------------------------------------------------------------
    def _transcript(self, account: str, name: str, lines: list[str]):
        folder = self.agents / account / "projects" / "-p"
        folder.mkdir(parents=True, exist_ok=True)
        path = folder / name
        path.write_text("\n".join(lines) + "\n")
        return path

    def cwd(self, key: str) -> str:
        head, _, tail = key.partition("/")
        return str(self.repos[head] / tail) if tail else str(self.repos[head])

    def _build_agents(self):
        def assistant(mid, ts, sid, tools=()):
            _, key, tokens = MESSAGES[mid]
            return _assistant(sid, mid, ts, self.cwd(key), tokens, tools)

        slack_url = f"https://slack.com/api/chat?token={CANARIES['url_token']}"
        bash = f"curl -H 'Authorization: {CANARIES['bash_secret']}' https://example.invalid"
        d = [*_user("D1", "2026-10-10T10:00:00.000Z", self.cwd("alpha"), 1),
             assistant("msgD1", "2026-10-10T10:01:00.000Z", "D1",
                       [_tool("WebFetch", url=slack_url), _tool("Bash", command=bash)]),
             assistant("msgD2", "2026-10-10T10:05:00.000Z", "D1")]  # fmt: skip
        w1 = [*_user("W1", "2026-10-11T09:00:00.000Z", self.cwd("beta"), 1),
              assistant("msgW1", "2026-10-11T09:01:00.000Z", "W1",
                        [_tool("mcp__acme-slack__post", text=CANARIES["tool_output"])]),
              assistant("msgW2", "2026-10-11T09:02:00.000Z", "W1")]  # fmt: skip
        w2 = [assistant("msgW3", "2026-10-12T08:00:00.000Z", "W2")]
        f1 = [assistant("msgF1", "2026-10-12T12:00:00.000Z", "F1")]
        self._transcript(".claude", "d1.jsonl", d)
        self._transcript(".claude-work", "w1.jsonl", w1)
        self._transcript(".claude-work", "w2.jsonl", w2)
        self._transcript(".claude-wife", "f1.jsonl", f1)
        self._transcript(".claude-wife", "copy-of-d1.jsonl", d)  # the same session copied into a second account
        (self.agents / ".claude-scratch").mkdir()
        (self.agents / ".claude-scratch" / "settings.json").write_text("{}")
        (self.agents / ".codex").mkdir()
        (self.agents / ".codex" / "config.toml").write_text(f"# {CANARIES['codex_toml']}\n")
        # files that match ~/.claude* but are NOT accounts (and hold a canary: they must never be read)
        (self.agents / ".claude.json").write_text(json.dumps({"canary": CANARIES["claude_json"]}))
        (self.agents / ".claude.json.backup").write_text(json.dumps({"canary": CANARIES["claude_json"]}))

    def folder(self, name: str) -> str:
        return str(self.agents / name)

    # environment --------------------------------------------------------------------------
    def env(self) -> dict[str, str]:
        return {
            "HOME": str(self.fakehome),
            "EASY_PERSONAL_HOME": str(self.data),
            "EASY_PERSONAL_AGENT_ROOT": str(self.agents),
            "EASY_PERSONAL_VERIFIER_REPORTS": str(self.base / "no-verifier"),
            "EASY_PERSONAL_CONTEXT_URL": "http://127.0.0.1:9/mcp",  # dead port: never the real service
            "PATH": str(self.bin),  # the real `claude` is unreachable
            "FAKE_CLAUDE_LOG": str(self.claude_log),
            "FAKE_CLAUDE_STATE": str(self.claude_state),
            "E2E_KEYRING_FILE": str(self.keyring_file),
            "PYTHON_KEYRING_BACKEND": "e2e_filekeyring.Keyring",
            "PYTHONPATH": os.pathsep.join([SRC, E2E_DIR]),
        }

    UNSET = ("CLAUDE_CONFIG_DIR", "EASY_PERSONAL_CLAUDE_PROJECTS", "EASY_PERSONAL_UI_REPORTS",
             "FAKE_CLAUDE_FAIL", "FAKE_CLAUDE_GET_ONLY", "FAKE_CLAUDE_RC", "FAKE_CLAUDE_GET_RC")  # fmt: skip

    @contextlib.contextmanager
    def active(self):
        """Process environment + in-process keyring point inside this world."""
        mp = pytest.MonkeyPatch()
        for name in self.UNSET:
            mp.delenv(name, raising=False)
        for name, value in self.env().items():
            mp.setenv(name, value)
        sys.path.insert(0, E2E_DIR)
        import e2e_filekeyring

        previous = keyring.get_keyring()
        keyring.set_keyring(e2e_filekeyring.Keyring())
        try:
            yield self
        finally:
            keyring.set_keyring(previous)
            sys.path.remove(E2E_DIR)
            mp.undo()

    # keychain, claude and store views (read-only) ----------------------------------------------
    def keychain(self) -> dict[str, str]:
        raw = json.loads(self.keyring_file.read_text()) if self.keyring_file.exists() else {}
        return {k.split("\x00")[0]: v for k, v in raw.items()}

    def seed_keychain(self, service: str, payload: dict):
        raw = json.loads(self.keyring_file.read_text()) if self.keyring_file.exists() else {}
        raw[f"{service}\x00connector"] = json.dumps(payload)
        self.keyring_file.write_text(json.dumps(raw))

    def seed_claude(self, config_dir: str, name: str, entry: dict):
        """Put an entry into the fake per-account Claude user scope (e.g. a hand-made server)."""
        state = json.loads(self.claude_state.read_text()) if self.claude_state.exists() else {}
        state.setdefault(config_dir, {})[name] = entry
        self.claude_state.write_text(json.dumps(state))

    def db(self) -> sqlite3.Connection:
        return sqlite3.connect(f"file:{self.data / 'store.sqlite'}?mode=ro", uri=True)

    def writes(self):
        """[(config_dir or '<unset>', argv)] of every `claude` call that is not `mcp get`."""
        return self.fake.write_records()

    def connector_ids(self) -> dict[str, str]:
        """real id -> placeholder (`<provider#n>`), by creation order."""
        with self.db() as conn:
            rows = conn.execute("SELECT id, provider FROM connectors ORDER BY provider, alias").fetchall()
        seen: dict[str, int] = {}
        out = {}
        for cid, provider in rows:
            seen[provider] = seen.get(provider, 0) + 1
            out[cid] = f"<{provider}#{seen[provider]}>"
        return out

    def stored_ids(self) -> dict[str, str]:
        """alias -> id for the connectors in the store."""
        with self.db() as conn:
            return {a: i for i, a in conn.execute("SELECT id, alias FROM connectors")}

    def record(self, label: str, blob):
        self.outputs.append((label, blob))


def tree_snapshot(root: Path) -> dict[str, tuple[int, int]]:
    out = {}
    for path in sorted(root.rglob("*")):
        if path.is_file():
            st = path.stat()
            out[str(path)] = (st.st_size, st.st_mtime_ns)
    return out


# --- the surfaces ----------------------------------------------------------------------------


@dataclass
class Result:
    ok: bool
    text: str = ""
    data: object = None


def _fields(provider: str) -> list[str]:
    from easy_personal.connectors import catalog

    return [f.env for f in catalog.load(provider).fields]


class CliSurface:
    name = "cli"

    def __init__(self, world: World):
        self.w = world

    def run(self, *argv, stdin: str = "") -> tuple[int, str, str]:
        env = {**os.environ, **self.w.env()}
        done = subprocess.run(
            [sys.executable, "-m", "easy_personal.surfaces.cli", *map(str, argv)],
            env=env, input=stdin.encode(), capture_output=True, timeout=120, check=False,
        )  # fmt: skip
        label = "cli " + " ".join(map(str, argv))
        self.w.record(label, {"rc": done.returncode, "stdout": done.stdout, "stderr": done.stderr})
        return done.returncode, done.stdout.decode(), done.stderr.decode()

    def _res(self, *argv, stdin="", parse=False) -> Result:
        rc, out, err = self.run(*argv, stdin=stdin)
        data = json.loads(out) if parse and rc == 0 and out.strip() else None
        return Result(rc == 0, out + err, data)

    # accounts
    def accounts(self):
        return self._res("account", "list", parse=True).data["accounts"]

    def confirm(self, ref, label):
        return self._res("account", "confirm", ref, "--label", label)

    def ignore(self, ref):
        return self._res("account", "ignore", ref)

    def use(self, ref, names):
        return self._res("account", "use", ref, *names)

    def ingest(self):
        return self._res("ingest", parse=True)

    # workspaces
    def ws_add(self, name, repos=(), color=None, email=None):
        argv = ["workspace", "add", name]
        for repo in repos:
            argv += ["--repo", repo]
        if color:
            argv += ["--color", color]
        if email:
            argv += ["--email", email]
        return self._res(*argv)

    def ws_rename(self, name, new, yes=True):
        return self._res("workspace", "edit", name, "--rename", new, *(["--yes"] if yes else []),
                         stdin="" if yes else "n\n")  # fmt: skip

    def ws_delete(self, name, yes=True):
        return self._res("workspace", "delete", name, *(["--yes"] if yes else []), stdin="" if yes else "n\n")

    def ws_list(self):
        return self._res("workspace", "list", parse=True).data["workspaces"]

    # connectors
    def connector_add(self, provider, ws, values: dict, alias=None, yes=True):
        argv = ["connector", "add", provider, "--workspace", ws, *(["--alias", alias] if alias else [])]
        stdin = "".join(values.get(env, "") + "\n" for env in _fields(provider))
        if yes:
            argv.append("--yes")
        else:
            stdin += "n\n"
        return self._res(*argv, stdin=stdin)

    def sync(self, yes=True, account=None):
        argv = ["connector", "sync", *(["--account", account] if account else [])]
        return self._res(*argv, *(["--yes"] if yes else []), stdin="" if yes else "n\n")

    def move(self, cid, ws):
        return self._res("connector", "move", cid, ws, "--yes")

    def alias(self, cid, alias):
        if alias is None:
            return self._res("connector", "alias", cid, "--default", "--yes")
        return self._res("connector", "alias", cid, alias, "--yes")

    def remove(self, cid):
        return self._res("connector", "remove", cid, "--yes")

    def adopt(self, yes=True):
        return self._res("connector", "adopt-legacy", *(["--yes"] if yes else []), stdin="" if yes else "n\n")

    # usage
    def usage(self, since="2026-10-01", until="2026-11-01", workspace=None, account=None, by=None):
        argv = ["usage", "--since", since, "--until", until]
        if workspace:
            argv += ["--workspace", workspace]
        if account:
            argv += ["--account", str(account)]
        if by:
            argv += ["--by", by]
        return self._res(*argv, parse=True)

    def doctor(self):
        return self._res("doctor")


class ApiSurface:
    name = "api"

    def __init__(self, world: World):
        from starlette.testclient import TestClient

        from easy_personal.surfaces import api

        self.w = world
        self.client = TestClient(api.create_app(TOKEN, PORT), base_url=f"http://127.0.0.1:{PORT}")
        self.headers = {api.TOKEN_HEADER: TOKEN}

    def call(self, method, path, body=None, **params) -> tuple[int, dict]:
        headers = dict(self.headers)
        content = None
        if body is not None:
            headers["content-type"] = "application/json"
            content = json.dumps(body)
        res = self.client.request(method, path, headers=headers, content=content, params=params or None)
        self.w.record(f"api {method} {path} {params or ''}", f"{res.status_code} {dict(res.headers)} {res.text}")
        return res.status_code, res.json()

    def _write(self, method, path, body) -> Result:
        status, data = self.call(method, path, body)
        if status >= 400:
            return Result(False, data.get("error", ""), data)
        lines = "\n".join(data.get("lines", [])) if isinstance(data, dict) else ""
        code = data.get("status")
        return Result(code == 0 if isinstance(code, int) else True, lines, data)

    # accounts
    def accounts(self):
        return self.call("GET", "/api/accounts")[1]["accounts"]

    def confirm(self, ref, label):
        return self._write("POST", f"/api/accounts/{ref}/confirm", {"label": label})

    def ignore(self, ref):
        return self._write("POST", f"/api/accounts/{ref}/ignore", {})

    def use(self, ref, names):
        return self._write("PUT", f"/api/accounts/{ref}/workspaces", {"workspaces": list(names)})

    def ingest(self):
        """What `easy-personal dashboard` does before it serves: the same ingest, in this process."""
        from easy_personal.surfaces import cli

        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            rc = cli._ingest(argparse.Namespace(source=None, retention_days=None))
        self.w.record("api-world ingest", out.getvalue() + err.getvalue())
        return Result(rc == 0, out.getvalue() + err.getvalue())

    # workspaces
    def _ws_id(self, name):
        rows = self.call("GET", "/api/workspaces")[1]["workspaces"]
        return next(r["id"] for r in rows if r["name"] == name)

    def ws_add(self, name, repos=(), color=None, email=None):
        body = {"name": name, "repos": list(repos)}
        body |= {k: v for k, v in (("color", color), ("email", email)) if v}
        status, data = self.call("POST", "/api/workspaces", body)
        return Result(status < 400, data.get("error", ""), data)

    def ws_rename(self, name, new, yes=True):
        return self._write("PATCH", f"/api/workspaces/{self._ws_id(name)}", {"name": new, "confirm": yes})

    def ws_delete(self, name, yes=True):
        return self._write("DELETE", f"/api/workspaces/{self._ws_id(name)}", {"confirm": yes})

    def ws_list(self):
        return self.call("GET", "/api/workspaces")[1]["workspaces"]

    # connectors
    def connector_add(self, provider, ws, values: dict, alias=None, yes=True):
        body = {"provider": provider, "workspace": ws, "values": values, "confirm": yes}
        if alias:
            body["alias"] = alias
        return self._write("POST", "/api/connectors", body)

    def sync(self, yes=True, account=None):
        body = {"confirm": yes} | ({"account": account} if account else {})
        return self._write("POST", "/api/sync", body)

    def move(self, cid, ws):
        return self._write("POST", f"/api/connectors/{cid}/move", {"workspace": ws, "confirm": True})

    def alias(self, cid, alias):
        return self._write("PUT", f"/api/connectors/{cid}/alias", {"alias": alias, "confirm": True})

    def remove(self, cid):
        return self._write("DELETE", f"/api/connectors/{cid}", {"confirm": True})

    def adopt(self, yes=True):
        return self._write("POST", "/api/connectors/legacy", {"confirm": yes})

    # usage
    def usage(self, since="2026-10-01", until="2026-11-01", workspace=None, account=None, by=None):
        params = {"since": since, "until": until}
        params |= {k: str(v) for k, v in (("workspace", workspace), ("account", account)) if v}
        path = "/api/usage/summary"
        if by:
            path, params["by"] = "/api/usage/breakdown", by
        status, data = self.call("GET", path, **params)
        return Result(status == 200, json.dumps(data), data)

    def doctor(self):
        raise NotImplementedError("doctor is a CLI command")


# --- state, leak and no-touch checks -----------------------------------------------------------


def _norm(obj, world: World):
    """JSON-roundtrip `obj` with connector ids and the world's base path replaced by placeholders."""
    text = json.dumps(obj, sort_keys=True, default=str)
    ids = world.connector_ids()
    text = ID.sub(lambda m: ids.get(m[0], m[0]), text)
    return json.loads(text.replace(str(world.base.resolve()), "<ROOT>").replace(str(world.base), "<ROOT>"))


def snapshot(world: World) -> dict:
    """The surface-independent end state: store tables, fake Claude config per account, keychain names."""
    with world.db() as conn:
        q = lambda sql: [list(r) for r in conn.execute(sql)]
        snap = {
            "workspaces": q("SELECT name, color, description, email, is_orphan FROM workspaces ORDER BY name"),
            "repos": q("SELECT w.name, r.repo_path FROM workspace_repos r JOIN workspaces w "
                       "ON w.id = r.workspace_id ORDER BY 1, 2"),
            "accounts": q("SELECT folder, provider, label, status, config_dir_env FROM agent_accounts ORDER BY folder"),
            "picks": q("SELECT a.folder, w.name FROM account_workspaces p JOIN agent_accounts a ON a.id = p.account_id "
                       "JOIN workspaces w ON w.id = p.workspace_id ORDER BY 1, 2"),
            "connectors": q("SELECT c.id, c.alias, c.alias_custom, c.provider, w.name FROM connectors c "
                            "JOIN workspaces w ON w.id = c.workspace_id ORDER BY c.alias"),
            "registrations": q("SELECT a.folder, r.alias, r.connector_id FROM account_registrations r "
                               "JOIN agent_accounts a ON a.id = r.account_id ORDER BY 1, 2"),
            "events_by_account": q("SELECT COALESCE(a.label, 'none'), COUNT(*) FROM events e LEFT JOIN agent_accounts a "
                                   "ON a.id = e.agent_account_id WHERE e.message_id IS NOT NULL GROUP BY 1 ORDER BY 1"),
            "user_version": conn.execute("PRAGMA user_version").fetchone()[0],
        }
    claude = json.loads(world.claude_state.read_text()) if world.claude_state.exists() else {}
    snap["claude_config"] = {
        (k if k == "<unset>" else Path(k).name): {
            a: {"command": Path(e["command"]).name, "args": e["args"]} for a, e in entries.items()
        }
        for k, entries in claude.items()
    }
    snap["keychain_services"] = sorted(world.keychain())
    snap["claude_writes"] = [[Path(d).name if d != "<unset>" else d, argv[:3]] for d, argv in world.writes()]
    out = _norm(snap, world)
    out["keychain_services"].sort()  # placeholders sort differently from random ids
    return out


def sqlite_checks(world: World) -> dict:
    with world.db() as conn:
        return {
            "user_version": conn.execute("PRAGMA user_version").fetchone()[0],
            "foreign_key_check": conn.execute("PRAGMA foreign_key_check").fetchall(),
            "integrity_check": conn.execute("PRAGMA integrity_check").fetchall(),
        }


def latest_version() -> int:
    from easy_personal.core.migrations import MIGRATIONS

    return max(n for n, _ in MIGRATIONS)


def store_blobs(world: World) -> dict[str, object]:
    """Every byte the product left in its data dir, plus a logical dump of the store."""
    blobs: dict[str, object] = {}
    for path in sorted(world.data.rglob("*")):
        if path.is_file():
            blobs[f"data file {path.name}"] = path.read_bytes()
    conn = sqlite3.connect(world.data / "store.sqlite")
    try:
        blobs["store .dump"] = "\n".join(conn.iterdump())
    finally:
        conn.close()
    return blobs


def variants(value: str) -> dict[str, bytes]:
    raw = value.encode()
    return {"raw": raw, "json": json.dumps(value)[1:-1].encode(), "url": quote(value, safe="").encode(),
            "base64": base64.b64encode(raw), "hex": raw.hex().encode()}  # fmt: skip


def leaks(blobs: dict[str, object], planted: dict[str, str]) -> list[str]:
    """Names of (blob, canary, encoding) combinations where a canary was found."""
    found = []
    for label, blob in blobs.items():
        text = json.dumps(blob, default=lambda o: o.decode("latin-1") if isinstance(o, bytes) else str(o)) \
            if not isinstance(blob, (bytes, str)) else blob
        data = text if isinstance(text, bytes) else text.encode("utf-8", "replace")
        for name, value in planted.items():
            for how, needle in variants(value).items():
                if needle in data:
                    found.append(f"{label}: {name} ({how})")
    return found


def real_mtimes() -> dict[str, tuple]:
    """Read-only `lstat` of the owner's real ~/.claude*, ~/.codex*, ~/.easy-personal (top level only).

    A `~/.claude*` or `~/.codex*` DIRECTORY is skipped: a running Claude Code session or Codex app (VS Code
    extension) rewrites it all the time, so its mtime is noise (the audit hook covers it). Files such as
    ~/.claude.json are compared.
    """
    out = {}
    for name in sorted(os.listdir(REAL_HOME)):
        if name.startswith((".claude", ".codex", ".easy-personal")):
            st = os.lstat(REAL_HOME / name)
            is_dir = os.path.isdir(REAL_HOME / name)
            if is_dir and name.startswith((".claude", ".codex")):  # live Codex app rewrites ~/.codex
                continue
            out[name] = (st.st_mtime_ns, st.st_size if not is_dir else None)
    return out


_AUDITED = {"open", "os.listdir", "os.scandir", "os.rename", "os.remove", "os.rmdir", "os.mkdir",
            "os.chmod", "os.truncate", "os.symlink", "os.link", "shutil.copyfile", "glob.glob"}  # fmt: skip
_watch = {"on": False, "hits": [], "prefixes": ()}
_installed = []


def _audit(event, args):
    if not _watch["on"] or event not in _AUDITED or not args:
        return
    try:
        path = os.path.abspath(os.fsdecode(args[0]))
    except (TypeError, ValueError):
        return
    if path.startswith(_watch["prefixes"]):
        _watch["hits"].append(f"{event} {path}")


@contextlib.contextmanager
def no_touch_guard(extra_prefixes=()):
    """NFR-003: nothing under the real home's agent / store folders is opened, listed or changed.

    Two checks: an audit hook records every file-system event of this process under the real paths
    (the subprocesses run with HOME inside the World), and top-level `lstat` snapshots before/after.
    Yields the list of audit hits; the caller asserts on it and on `guard.changed`.
    """
    if not _installed:
        sys.addaudithook(_audit)
        _installed.append(True)
    prefixes = tuple(str(REAL_HOME / n) for n in (".claude", ".codex", ".easy-personal", ".easy-verifier-reports",
                                                  ".easy-context")) + tuple(extra_prefixes)  # fmt: skip
    before = real_mtimes()
    _watch.update(on=True, hits=[], prefixes=prefixes)
    guard = Guard()
    try:
        yield guard
    finally:
        _watch["on"] = False
        guard.hits = list(_watch["hits"])
        after = real_mtimes()
        guard.changed = sorted(k for k in before.keys() | after.keys() if before.get(k) != after.get(k))


@dataclass
class Guard:
    hits: list[str] = field(default_factory=list)
    changed: list[str] = field(default_factory=list)
