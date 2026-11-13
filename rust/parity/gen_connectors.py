#!/usr/bin/env python3
"""Oracle for the Rust connector writes, sync apply, legacy adoption and launcher (T050).

    rust/parity/.venv/bin/python rust/parity/gen_connectors.py            # writes rust/parity/golden-connectors/
    rust/parity/.venv/bin/python rust/parity/gen_connectors.py --out DIR  # same, elsewhere (a re-run must diff empty)

Run it with CLAUDE_CONFIG_DIR unset (it is removed anyway). For each case the Python app (`api.create_app`, Starlette's
`TestClient`) runs on a **temp copy** of `store-rich/store-v11.sqlite`, with:

- an in-memory keychain (`MemoryKeyring` of `tests/connectors/fake_backend.py`), or `keyring.backends.fail` for "no
  backend"; the real keychain is never set (the run starts by installing the failing backend);
- the fake `claude` of `golden-connectors/fake-claude.sh` first on PATH (or no `claude` at all), with per-account state;
  `CLAUDE_CONFIG_DIR=/runner/dir` in the environment, which every `claude` call for the default account must drop;
- `writer.launcher_path` pinned to `/opt/fake/bin/easy-personal-launch` and `writer.new_id` to `c00000001`, `c00000002`...

It records per request the status and JSON body, and per case the keychain items after, every `claude` call (account
folder + argv), the fake `claude` state after, and the store tables that changed. The Rust test
(`crates/app/tests/connector_parity.rs`) builds the same world for the real `serve` binary (the debug-build fake
keychain file, the same fake `claude`) and requires equal answers, keychain items, calls, state and rows.
Normalised on both sides: the work folder is `<WORK>`, the launcher path `<LAUNCHER>`, a new connector id the golden's.

The `launcher` section: items Python's `keychain.store` wrote (bytes recorded) and what `launch.main` printed for
`headers` and for each refusal; the Rust test replays them on the `easy-personal-launch` binary.

Not here (the Rust tests and the README list each): `claude`'s stderr, which Python shows as a line and Rust never shows
(the failing cases print nothing on stderr); a keychain that raises on read (Python answers 500); the `exec` success
path (it replaces the process: `tests/launcher.rs`).
"""

import argparse
import contextlib
import io
import json
import os
import re
import shutil
import sqlite3
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True  # importing the app and its tests leaves no .pyc behind
PARITY = Path(__file__).resolve().parent
sys.path.insert(0, str(PARITY))
import gen_golden  # noqa: E402

SOURCE = PARITY / "store-rich" / "store-v11.sqlite"
FAKE_CLAUDE = PARITY / "golden-connectors" / "fake-claude.sh"
FAKE_LAUNCHER = "/opt/fake/bin/easy-personal-launch"
TOKEN, PORT = "parity-token", 5555
JSON = "application/json"
SECRET = "xoxb-PLANTED-t050-secret-é☃"

ACME = ["INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2)",
        "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('gmail-acme', 'acme-gmail', 0, 'gmail', 2)"]  # fmt: skip
REG_WORK = ["INSERT INTO account_registrations VALUES (2, 'acme-slack', 'slack-acme')"]
PICK_DEFAULT = ["INSERT INTO account_workspaces VALUES (1, 2)"]
DIRS = ["agent-root/.claude", "agent-root/.claude-work/projects"]
WORK_CCD = "@W@/agent-root/.claude-work"
OURS = lambda cid: ["@LAUNCHER@", f"exec {cid}"]  # noqa: E731 - an entry that points at our launcher
FOREIGN = ["/usr/bin/some-server", "--serve"]
ITEM = lambda env: json.dumps({"command": ["npx", "-y", "x@1"], "env": env, "headers": {}})  # noqa: E731


def req(method, path, body=None, *, raw=None, type=JSON):
    data = raw.encode() if raw is not None else (b"" if body is None else json.dumps(body, ensure_ascii=False).encode())
    return {"method": method, "path": path, "type": type, "body_hex": data.hex()}


def add(body, **kw):
    return req("POST", "/api/connectors", body, **kw)


def remove(name, body=None, **kw):
    return req("DELETE", f"/api/connectors/{name}", {} if body is None else body, **kw)


def move(cid, body):
    return req("POST", f"/api/connectors/{cid}/move", body)


def alias(cid, body):
    return req("PUT", f"/api/connectors/{cid}/alias", body)


def sync(body=None, **kw):
    return req("POST", "/api/sync", {} if body is None else body, **kw)


def legacy(method="POST", body=None):
    return req(method, "/api/connectors/legacy", {} if body is None and method == "POST" else body, type=JSON if method == "POST" else None)


def case(name, requests, *, setup=(), dirs=DIRS, keychain=None, claude=None, fail=None):
    """`keychain`: {service: secret} or "none" (no backend). `claude`: None (absent) or {ccd: {name: [command, args]}}."""
    return {"name": name, "setup": list(setup), "dirs": list(dirs), "keychain": {} if keychain is None else keychain,
            "claude": claude, "fail": fail, "requests": requests}  # fmt: skip


def slack_add(**extra):
    return {"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": SECRET}, **extra}


def cases():
    out = []
    put = out.append
    fake = {}  # `claude` on PATH, nothing registered anywhere

    # ---- POST /api/connectors
    put(case("add_preview", [add(slack_add())], claude=fake))
    put(case("add_confirm", [add(slack_add(confirm=True)), req("GET", "/api/connectors", type=None)], claude=fake))
    put(case("add_custom_alias", [add(slack_add(alias="team-chat", confirm=True))], claude=fake))
    put(case("add_env_plain_field", [add({"provider": "gmail", "workspace": "personal", "values": {"GMAIL_OAUTH_PATH": "/k/é keys.json"}, "confirm": True})]))
    put(case("add_no_backend", [add(slack_add(confirm=True)), add(slack_add())], keychain="none", claude=fake))
    put(case("add_double_submit", [add(slack_add(confirm=True)), add(slack_add(confirm=True, values={"SLACK_MCP_XOXB_TOKEN": "other-value"}))], claude=fake))
    put(case("add_alias_in_use", [add(slack_add(confirm=True)), add({"provider": "gmail", "workspace": "personal", "alias": "acme-slack", "values": {"GMAIL_OAUTH_PATH": "/k"}, "confirm": True})]))
    put(case("add_empty_value", [add({"provider": "slack", "workspace": "acme", "values": {}, "confirm": True}), add({"provider": "slack", "workspace": "acme", "values": {"SLACK_MCP_XOXB_TOKEN": ""}, "confirm": True})]))
    put(case("add_refusals", [
        add({"provider": "nope", "workspace": "acme", "values": {}}), add({"provider": 5, "workspace": "acme"}),
        add({"workspace": "acme"}), add({"provider": "slack"}), add({"provider": "slack", "workspace": ""}),
        add({"provider": "slack", "workspace": 3}), add({"provider": "slack", "workspace": "acme", "alias": 3}),
        add({"provider": "slack", "workspace": "acme", "values": "x"}), add({"provider": "slack", "workspace": "acme", "values": {"A": 1}}),
        add({"provider": "slack", "workspace": "nope-PLANTED", "values": {"SLACK_MCP_XOXB_TOKEN": SECRET}, "confirm": True}),
        add({"provider": "slack", "workspace": "orphan", "values": {"SLACK_MCP_XOXB_TOKEN": SECRET}, "confirm": True}),
        add(slack_add(alias="Bad Alias", confirm=True)), add(slack_add(alias="a" * 65, confirm=True)),
        add(slack_add(confirm="yes")), add(None, raw="not json"), add(None, raw="[1]"),
        add(slack_add(confirm=True), type="text/plain"), add(slack_add(confirm=True), type=None),
    ]))  # fmt: skip

    # ---- DELETE /api/connectors/{name}
    put(case("remove_stored_never_registered", [remove("slack-acme"), remove("slack-acme", {"confirm": True})],
             setup=ACME, keychain={"easy-personal/slack-acme": ITEM({"T": SECRET})}, claude=fake))  # fmt: skip
    put(case("remove_stored_registered", [remove("slack-acme", {"confirm": True}), req("GET", "/api/sync", type=None)],
             setup=ACME + REG_WORK, keychain={"easy-personal/slack-acme": ITEM({"T": SECRET})},
             claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}}))  # fmt: skip
    put(case("remove_stored_claude_fails", [remove("slack-acme", {"confirm": True})], setup=ACME + REG_WORK,
             keychain={"easy-personal/slack-acme": ITEM({"T": SECRET})}, claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}},
             fail="remove acme-slack"))  # fmt: skip
    put(case("remove_stored_no_claude", [remove("slack-acme", {"confirm": True})], setup=ACME + REG_WORK,
             keychain={"easy-personal/slack-acme": ITEM({"T": SECRET})}))  # fmt: skip
    put(case("remove_stored_account_folder_missing", [remove("slack-acme", {"confirm": True})], setup=ACME + REG_WORK,
             dirs=["agent-root/.claude"], keychain={"easy-personal/slack-acme": ITEM({"T": SECRET})}, claude=fake))  # fmt: skip
    put(case("remove_pre_workspace_ours", [remove("slack"), remove("slack", {"confirm": True})],
             keychain={"easy-personal/slack": ITEM({"T": SECRET})}, claude={"<unset>": {"slack": OURS("slack")}}))  # fmt: skip
    put(case("remove_pre_workspace_foreign", [remove("slack", {"confirm": True})],
             keychain={"easy-personal/slack": ITEM({"T": SECRET})}, claude={"<unset>": {"slack": FOREIGN}}))  # fmt: skip
    put(case("remove_pre_workspace_claude_fails", [remove("slack", {"confirm": True})],
             keychain={"easy-personal/slack": ITEM({"T": SECRET})}, claude={"<unset>": {"slack": OURS("slack")}}, fail="remove slack"))  # fmt: skip
    put(case("remove_absent", [remove("discord", {"confirm": True})], claude=fake))
    put(case("remove_absent_no_claude", [remove("discord", {"confirm": True})]))
    put(case("remove_pre_workspace_no_backend", [remove("slack", {"confirm": True})], keychain="none", claude=fake))
    put(case("remove_unknown", [remove("github", {"confirm": True}), remove("Bad%20Name", {"confirm": True}), remove("a" * 70, {"confirm": True}),
                                remove("slack", raw="nope"), remove("slack", type="text/plain")], claude=fake))  # fmt: skip

    # ---- move and alias (HUN-66)
    put(case("move_to_workspace", [move("slack-acme", {"workspace": "personal"}), move("slack-acme", {"workspace": "personal", "confirm": True})],
             setup=ACME + REG_WORK, claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}}))  # fmt: skip
    put(case("move_to_orphan_keeps_alias", [move("slack-acme", {"workspace": "orphan", "confirm": True})], setup=ACME, claude=fake))
    put(case("move_refusals", [
        move("slack-acme", {"workspace": "acme"}), move("slack-acme", {"workspace": "nope", "confirm": True}),
        move("nope", {"workspace": "personal"}), move("slack-acme", {}), move("slack-acme", {"workspace": 5}),
        move("slack-acme", {"workspace": "personal", "confirm": "yes"}),
    ], setup=ACME + ["INSERT INTO connectors VALUES ('slack-personal', 'personal-slack', 0, 'slack', 3)"], claude=fake))  # fmt: skip
    put(case("alias_custom_then_default", [alias("slack-acme", {"alias": "team-chat", "confirm": True}), alias("slack-acme", {"alias": None, "confirm": True})],
             setup=ACME + REG_WORK, claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}}))  # fmt: skip
    put(case("alias_refusals", [
        alias("slack-acme", {"alias": "acme-gmail", "confirm": True}), alias("slack-acme", {"alias": "Bad!", "confirm": True}),
        alias("slack-acme", {"alias": 5}), alias("nope", {"alias": "x"}), alias("slack-orphan", {"confirm": True}),
        alias("slack-orphan", {"alias": "acme-gmail", "confirm": True}),
    ], setup=ACME + ["INSERT INTO connectors VALUES ('slack-orphan', 'slack', 0, 'slack', 1)"], claude=fake))  # fmt: skip
    put(case("workspace_rename_applies", [req("PATCH", "/api/workspaces/2", {"name": "firm", "confirm": True})],
             setup=ACME + REG_WORK, claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}}))  # fmt: skip
    put(case("workspace_delete_applies", [req("DELETE", "/api/workspaces/2", {"confirm": True})],
             setup=ACME + REG_WORK, claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}}))  # fmt: skip

    # ---- POST /api/sync: each step kind
    every_kind = ACME + PICK_DEFAULT + [
        "INSERT INTO connectors VALUES ('discord-acme', 'acme-discord', 0, 'discord', 2)",
        "INSERT INTO connectors VALUES ('teams-acme', 'acme-teams', 0, 'teams', 2)",
        "INSERT INTO connectors VALUES ('sendgrid-acme', 'acme-sendgrid', 0, 'sendgrid', 2)",
        "INSERT INTO account_registrations VALUES (2, 'acme-slack', 'slack-acme')",  # still wanted, still there: no step
        "INSERT INTO account_registrations VALUES (2, 'acme-gmail', 'gmail-acme')",  # wanted, gone from Claude: re-add
        "INSERT INTO connectors VALUES ('slack-personal', 'personal-slack', 0, 'slack', 3)",  # `personal`: nobody picks it
        "INSERT INTO connectors VALUES ('gmail-personal', 'personal-gmail', 0, 'gmail', 3)",
        "INSERT INTO account_registrations VALUES (2, 'old-one', 'slack-personal')",  # not wanted, still there: remove
        "INSERT INTO account_registrations VALUES (2, 'old-two', 'gmail-personal')",  # not wanted, gone: forget
        "INSERT INTO account_registrations VALUES (2, 'acme-teams', 'teams-acme')",  # wanted, points elsewhere: remove + re-add
    ]  # fmt: skip
    kinds_state = {WORK_CCD: {"acme-slack": OURS("slack-acme"), "old-one": OURS("slack-personal"), "acme-teams": FOREIGN,
                              "acme-discord": OURS("discord-acme"), "acme-sendgrid": FOREIGN},  # record, refused
                   "<unset>": {}}  # fmt: skip
    put(case("sync_every_step_kind", [req("GET", "/api/sync", type=None), sync(), sync({"confirm": True}), sync({"confirm": True})],
             setup=every_kind, claude=kinds_state))  # fmt: skip
    put(case("sync_one_account", [sync({"account": 1, "confirm": True}), sync({"account": "2"})], setup=every_kind, claude=kinds_state))
    put(case("sync_failing_claude", [sync({"confirm": True}), sync({"confirm": True})], setup=every_kind, claude=kinds_state,
             fail="add-json acme-slack"))  # fmt: skip
    put(case("sync_no_claude", [sync({"confirm": True})], setup=every_kind))
    put(case("sync_skipped_account", [sync({"confirm": True})], setup=every_kind, dirs=["agent-root/.claude-work/projects"], claude=kinds_state))
    put(case("sync_refusals", [sync({"account": "x"}), sync({"account": True}), sync({"account": 1.5}), sync({"account": 999, "confirm": True}),
                               sync({"account": -5}), sync({"confirm": "yes"}), sync(None, raw="{"), sync({}, type=None)], claude=fake))  # fmt: skip
    put(case("sync_nothing", [sync({"confirm": True})], claude=fake))

    # ---- legacy (HUN-67)
    put(case("legacy_adopt", [legacy("GET"), legacy(), legacy("POST", {"confirm": True}), legacy("GET"), legacy("POST", {"confirm": True})],
             keychain={"easy-personal/slack": ITEM({"T": SECRET}), "easy-personal/discord": ITEM({"D": SECRET}), "easy-personal/gmail": ITEM({})},
             claude={"<unset>": {"slack": OURS("slack"), "gmail": FOREIGN, "teams": OURS("teams")}, WORK_CCD: {"sendgrid": OURS("sendgrid")}}))  # fmt: skip
    put(case("legacy_claude_fails", [legacy("POST", {"confirm": True})], keychain={"easy-personal/slack": ITEM({})},
             claude={"<unset>": {"slack": OURS("slack")}}, fail="remove slack"))  # fmt: skip
    put(case("legacy_no_backend", [legacy("GET"), legacy("POST", {"confirm": True})], keychain="none", claude=fake))
    put(case("legacy_nothing", [legacy("GET"), legacy("POST", {"confirm": True})], setup=["INSERT INTO connectors VALUES ('slack', 'slack', 0, 'slack', 1)"],
             keychain={"easy-personal/slack": ITEM({})}))  # fmt: skip
    put(case("legacy_refusals", [legacy("POST", {"confirm": "yes"}), req("POST", "/api/connectors/legacy", raw="x"),
                                 req("POST", "/api/connectors/legacy", {}, type=None), remove("legacy", {"confirm": True})]))  # fmt: skip
    return out


def scrub(value, work: str):
    if isinstance(value, str):
        return value.replace(work, "<WORK>").replace(FAKE_LAUNCHER, "<LAUNCHER>")
    if isinstance(value, list):
        return [scrub(v, work) for v in value]
    if isinstance(value, dict):
        return {k: scrub(v, work) for k, v in value.items()}
    return value


def dump(path: Path, work: str) -> dict[str, list]:
    conn = sqlite3.connect(path)
    try:
        names = [r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
        return {n: sorted((scrub(list(r), work) for r in conn.execute(f'SELECT * FROM "{n}"')), key=json.dumps) for n in names}
    finally:
        conn.close()


def get_runs_sorted(calls: list[dict]) -> list[dict]:
    """Each unbroken run of `mcp get` calls sorted (legacy `detect` asks in parallel; a read's order is not behaviour)."""
    out, run = [], []
    for call in calls + [None]:
        if call is not None and call["argv"][:2] == ["mcp", "get"]:
            run.append(call)
            continue
        out += sorted(run, key=json.dumps)
        run = []
        if call is not None:
            out.append(call)
    return out


def read_log(path: Path, work: str) -> list[dict]:
    """The fake `claude`'s log as [{"ccd", "argv"}], runs of `mcp get` sorted."""
    calls, cur = [], None
    for line in path.read_text().splitlines() if path.exists() else []:
        if cur is None:
            cur = {"ccd": scrub(line.removeprefix("CCD="), work), "argv": []}
        elif line == "--":
            calls.append(cur)
            cur = None
        else:
            cur["argv"].append(scrub(line, work))
    return get_runs_sorted(calls)


def ccd_key(ccd: str) -> str:
    """The fake's folder name for an account: every byte outside `[A-Za-z0-9._-]` as `_` (its `.ccd` file holds the value)."""
    return re.sub(rb"[^A-Za-z0-9._-]", b"_", ccd.encode()).decode()


def write_state(state_dir: Path, claude: dict, work: str) -> None:
    for ccd, entries in claude.items():
        value = ccd.replace("@W@", work)
        folder = state_dir / ccd_key(value)
        folder.mkdir(parents=True, exist_ok=True)
        (folder / ".ccd").write_text(value)
        for name, (command, args) in entries.items():
            (folder / name).write_text(f"Command: {command.replace('@LAUNCHER@', FAKE_LAUNCHER)}\nArgs: {args}\n")


def read_state(state_dir: Path, work: str) -> dict:
    out = {}
    for folder in sorted(p for p in state_dir.iterdir() if p.is_dir()) if state_dir.exists() else []:
        ccd = scrub((folder / ".ccd").read_text(), work)
        out[ccd] = {f.name: scrub(f.read_text(), work) for f in sorted(folder.iterdir()) if f.name != ".ccd"}
    return out


def run_launcher(keyring_mod, memory_cls) -> dict:
    """Items Python's `keychain.store` writes and what `launch.main` prints for them (no exec: see the module text)."""
    from easy_personal import launch
    from easy_personal.connectors import keychain
    from keyring.backends import fail

    backend = memory_cls()
    keyring_mod.set_keyring(backend)
    keychain.store("slack", ["npx", "-y", "slack-mcp-server@1.3.0", "--transport", "stdio"], env={"SLACK_MCP_XOXB_TOKEN": SECRET},
                   headers={"Authorization": f"Bearer {SECRET}", "X-Extra": "line1\nline2 \"q\" \\"})  # fmt: skip
    keychain.store("plain", ["prog"], env={"A": "1", "B": "é\u0001\u007f"})
    keychain.store("empty", [])
    backend.items[("easy-personal/broken", "connector")] = '{"command": "not-a-list"}'
    backend.items[("easy-personal/notjson", "connector")] = "{nope"
    items = {service: secret for (service, _), secret in sorted(backend.items.items())}
    runs = []
    for args in (["headers", "slack"], ["headers", "plain"], ["headers", "empty"], ["headers", "broken"], ["headers", "notjson"],
                 ["exec", "empty"], ["exec", "broken"], ["exec", "nope"], ["headers", "nope"], ["exec", "Bad/../x"], ["exec"],
                 ["run", "slack"]):  # fmt: skip
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = launch.main(args)
        runs.append({"args": args, "keychain": "fake", "code": code, "stdout": out.getvalue(), "stderr": err.getvalue()})
    keyring_mod.set_keyring(fail.Keyring())
    for args in (["exec", "slack"], ["headers", "slack"]):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = launch.main(args)
        runs.append({"args": args, "keychain": "none", "code": code, "stdout": out.getvalue(), "stderr": err.getvalue()})
    return {"items": items, "runs": runs}


def run(out: Path) -> None:
    gen_golden.refuse_real_paths(out)
    os.environ.pop("CLAUDE_CONFIG_DIR", None)
    done = []
    with tempfile.TemporaryDirectory(prefix="parity-connectors-") as tmp:
        base = Path(tmp).resolve()
        gen_golden.isolate(base)
        gen_golden.guard_sqlite()
        import keyring
        from keyring.backends import fail
        from starlette.testclient import TestClient

        keyring.set_keyring(fail.Keyring())  # before anything: the real keychain is never the backend
        sys.path.insert(0, str(gen_golden.REPO / "tests" / "connectors"))
        from fake_backend import MemoryKeyring

        from easy_personal.connectors import writer
        from easy_personal.surfaces import api

        writer.launcher_path = lambda: FAKE_LAUNCHER
        for number, spec in enumerate(cases()):
            work = base / f"case-{number}"
            work.mkdir()
            wtext = str(work)
            gen_golden.isolate(work)
            for d in spec["dirs"]:
                (work / d).mkdir(parents=True, exist_ok=True)
            store = work / "home" / "store.sqlite"
            shutil.copy(SOURCE, store)
            conn = sqlite3.connect(store)
            conn.execute("PRAGMA foreign_keys = ON")
            conn.execute("UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?), config_dir_env = replace(config_dir_env, '/work/rich', ?)", (wtext, wtext))
            for sql in spec["setup"]:
                conn.execute(sql.replace("@W@", wtext))
            conn.commit()
            conn.close()
            # the keychain
            if spec["keychain"] == "none":
                keyring.set_keyring(fail.Keyring())
            else:
                backend = MemoryKeyring()
                for service, secret in spec["keychain"].items():
                    backend.items[(service, "connector")] = secret
                keyring.set_keyring(backend)
            # `claude`: the fake first on PATH, or none at all
            fake_bin, state, log = work / "bin", work / "claude-state", work / "claude.log"
            fake_bin.mkdir()
            if spec["claude"] is not None:
                shutil.copy(FAKE_CLAUDE, fake_bin / "claude")
                (fake_bin / "claude").chmod(0o755)
                write_state(state, spec["claude"], wtext)
            os.environ.update({"PATH": f"{fake_bin}:/usr/bin:/bin", "FAKE_CLAUDE_LOG": str(log), "FAKE_CLAUDE_STATE": str(state),
                               "CLAUDE_CONFIG_DIR": "/runner/dir", "FAKE_CLAUDE_STDERR": ""})  # fmt: skip
            os.environ.pop("FAKE_CLAUDE_FAIL", None)
            if spec["fail"]:
                os.environ["FAKE_CLAUDE_FAIL"] = spec["fail"]
            found = shutil.which("claude")
            assert found in (None, str(fake_bin / "claude")), "a claude other than the fake is on PATH"
            ids = iter(f"c{n:08x}" for n in range(1, 99))
            writer.new_id = lambda ids=ids: next(ids)
            before = dump(store, wtext)
            client = TestClient(api.create_app(TOKEN, PORT, work / "absent"), base_url=f"http://127.0.0.1:{PORT}", raise_server_exceptions=False)
            answers = []
            for r in spec["requests"]:
                headers = {api.TOKEN_HEADER: TOKEN}
                if r["type"] is not None:
                    headers["content-type"] = r["type"]
                res = client.request(r["method"], r["path"], headers=headers, content=bytes.fromhex(r["body_hex"]))
                kind = res.headers.get("content-type", "")
                answers.append({"status": res.status_code, "json": scrub(res.json(), wtext) if kind.startswith(JSON) else None})
            keychain_after = "none" if spec["keychain"] == "none" else {s: v for (s, _), v in sorted(keyring.get_keyring().items.items())}
            after = dump(store, wtext)
            done.append({**spec, "answers": answers, "keychain_after": keychain_after, "calls": read_log(log, wtext),
                         "claude_after": read_state(state, wtext) if spec["claude"] is not None else None,
                         "changed": {t: rows for t, rows in after.items() if rows != before[t]}, "tables": sorted(after)})  # fmt: skip
            keyring.set_keyring(fail.Keyring())
        launcher = run_launcher(keyring, MemoryKeyring)
        for name in ("FAKE_CLAUDE_LOG", "FAKE_CLAUDE_STATE", "FAKE_CLAUDE_FAIL", "FAKE_CLAUDE_STDERR", "CLAUDE_CONFIG_DIR"):
            os.environ.pop(name, None)
    crashed = [(c["name"], a["status"]) for c in done for a in c["answers"] if a["status"] >= 500 and a["status"] != 503]  # 503: the legacy route with no keychain
    if crashed:
        sys.exit(f"Python answered 5xx in: {crashed} (such a case belongs in the README list, not in the oracle)")
    out.mkdir(parents=True, exist_ok=True)
    manifest = {"generator": "rust/parity/gen_connectors.py", "source_store": "rust/parity/store-rich/store-v11.sqlite",
                "fake_claude": "rust/parity/golden-connectors/fake-claude.sh", "fake_launcher": "<LAUNCHER>", "cases": done}  # fmt: skip
    (out / "cases.json").write_text(json.dumps(manifest, indent=1, sort_keys=True, ensure_ascii=False) + "\n")
    (out / "launcher.json").write_text(json.dumps(launcher, indent=1, sort_keys=True, ensure_ascii=False) + "\n")
    requests = sum(len(c["answers"]) for c in done)
    calls = sum(len(c["calls"]) for c in done)
    print(f"{len(done)} cases, {requests} requests, {calls} claude calls, {len(launcher['runs'])} launcher runs -> {out}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=PARITY / "golden-connectors")
    run(parser.parse_args().out.resolve())
