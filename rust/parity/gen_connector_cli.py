#!/usr/bin/env python3
"""Oracle for the Rust `connector ...` and `doctor` commands (T060): Python's own CLI `main()` over the T050 world.

    rust/parity/.venv/bin/python rust/parity/gen_connector_cli.py            # writes rust/parity/golden-connector-cli/cases.json
    rust/parity/.venv/bin/python rust/parity/gen_connector_cli.py --check    # regenerate in a temp folder and compare: exit 0 equal
    rust/parity/.venv/bin/python rust/parity/gen_connector_cli.py --out DIR  # same, elsewhere

Run it with CLAUDE_CONFIG_DIR unset (it is removed anyway). Each case is a work folder built as `gen_connectors.py` builds it:
a temp copy of `store-rich/store-v11.sqlite` with the case's setup SQL and agent folders, the in-memory keychain of
`tests/connectors/fake_backend.py` (or `keyring.backends.fail` for "no backend"; the real keychain is never set), the fake
`claude` of `golden-connectors/fake-claude.sh` first on PATH (or none) with per-account state, `CLAUDE_CONFIG_DIR=/runner/dir`
in the environment (the default account must drop it), `writer.launcher_path` pinned to `/opt/fake/bin/easy-personal-launch`
(or raising Python's `LaunchError` for a case with no launcher) and `writer.new_id` to `c00000001`, `c00000002`, ...

A step is one `easy-personal <args>` call with the text sent on stdin. The run has **no controlling terminal** (it forks and
calls `setsid` first), so `getpass` cannot open `/dev/tty` and takes its documented fallback: a warning and the prompt on
stderr, the value read from stdin. Python's own `GetPassWarning` line (it names `getpass.py`) is filtered out; its
`Warning: Password input may be echoed.` line is kept. Per step the file records the exit code, stdout and stderr (the work
folder as `<WORK>`, the launcher as `<LAUNCHER>`, `doctor`'s keychain backend class as `<BACKEND>`); per case the keychain
items after, every `claude` call (account folder + argv), the fake `claude` state after, and the store tables that changed.

The Rust test (`crates/app/tests/connector_cli_parity.rs`) builds the same world for the real binary (the debug-build fake
keychain file, the same fake `claude`, no controlling terminal) and requires the same exit codes, stdout and stderr, the same
keychain items, the same `claude` writes (`add-json`, `remove`) in the same order, the same fake state and the same rows.

Not here (`rust/README.md` lists each): `claude`'s stderr after a failed call (Python prints it, Rust never; the fake prints
nothing here); a missing store (Python creates one, Rust refuses); the terminal path of `getpass` (`/dev/tty`, echo off: a
unit test and the README); a `claude` that cannot run (Python takes it as "no entry"; `tests/connector_cli.rs`).
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
import warnings
from pathlib import Path

sys.dont_write_bytecode = True
PARITY = Path(__file__).resolve().parent
sys.path.insert(0, str(PARITY))
import gen_connectors as gc  # noqa: E402
import gen_golden  # noqa: E402

SOURCE = PARITY / "store-rich" / "store-v11.sqlite"
FAKE_LAUNCHER = gc.FAKE_LAUNCHER
SECRET = "xoxb-PLANTED-t060-secret-é☃"
ACME, REG_WORK, PICK_DEFAULT, DIRS, WORK_CCD, OURS, FOREIGN, ITEM = (
    gc.ACME, gc.REG_WORK, gc.PICK_DEFAULT, gc.DIRS, gc.WORK_CCD, gc.OURS, gc.FOREIGN, gc.ITEM)  # fmt: skip
EVERY_KIND = ACME + PICK_DEFAULT + [
    "INSERT INTO connectors VALUES ('discord-acme', 'acme-discord', 0, 'discord', 2)",
    "INSERT INTO connectors VALUES ('teams-acme', 'acme-teams', 0, 'teams', 2)",
    "INSERT INTO connectors VALUES ('sendgrid-acme', 'acme-sendgrid', 0, 'sendgrid', 2)",
    "INSERT INTO account_registrations VALUES (2, 'acme-slack', 'slack-acme')",
    "INSERT INTO account_registrations VALUES (2, 'acme-gmail', 'gmail-acme')",
    "INSERT INTO connectors VALUES ('slack-personal', 'personal-slack', 0, 'slack', 3)",
    "INSERT INTO connectors VALUES ('gmail-personal', 'personal-gmail', 0, 'gmail', 3)",
    "INSERT INTO account_registrations VALUES (2, 'old-one', 'slack-personal')",
    "INSERT INTO account_registrations VALUES (2, 'old-two', 'gmail-personal')",
    "INSERT INTO account_registrations VALUES (2, 'acme-teams', 'teams-acme')",
]  # the T050 `sync_every_step_kind` store: every step kind
KINDS_STATE = {WORK_CCD: {"acme-slack": OURS("slack-acme"), "old-one": OURS("slack-personal"), "acme-teams": FOREIGN,
                          "acme-discord": OURS("discord-acme"), "acme-sendgrid": FOREIGN}, "<unset>": {}}  # fmt: skip


def step(*args, stdin=""):
    return {"args": list(args) if args[:1] == ("doctor",) else ["connector", *args], "stdin": stdin}


def case(name, steps, *, setup=(), dirs=DIRS, keychain=None, claude=None, fail=None, launcher=True):
    """`keychain`: {service: secret} or "none". `claude`: None (absent) or {ccd: {name: [command, args]}}."""
    return {"name": name, "setup": list(setup), "dirs": list(dirs), "keychain": {} if keychain is None else keychain,
            "claude": claude, "fail": fail, "launcher": launcher, "steps": steps}  # fmt: skip


def cases():
    out = []
    put = out.append
    fake = {}  # `claude` on PATH, nothing registered anywhere
    yes = ("--yes",)

    # ---- list
    put(case("list_no_connectors", [step("list")]))
    put(case("list_configured", [step("list")], setup=ACME + ["INSERT INTO connectors VALUES ('slack', 'slack', 0, 'slack', 1)"]))
    put(case("list_extra_arg", [step("list", "x")]))
    put(case("no_action", [step()]))

    # ---- add: the secret through getpass (no terminal: stdin), a plain field through input(), then the question
    add = ("add", "slack", "--workspace", "acme")
    put(case("add_yes", [step(*add, *yes, stdin=f"{SECRET}\n"), step("list")]))
    put(case("add_ask_yes", [step(*add, stdin=f"{SECRET}\ny\n")]))
    put(case("add_ask_no", [step(*add, stdin=f"{SECRET}\nn\n")]))
    put(case("add_ask_eof", [step(*add, stdin=f"{SECRET}\n")]))
    put(case("add_secret_eof", [step(*add, *yes, stdin="")]))
    put(case("add_secret_empty", [step(*add, *yes, stdin="\n")]))
    put(case("add_secret_kept_as_typed", [step(*add, *yes, stdin=f"  {SECRET} \r\n")]))
    put(case("add_secret_no_newline", [step(*add, *yes, stdin=SECRET)]))
    put(case("add_plain_field", [step("add", "teams", "--workspace", "personal", stdin="  tenant-é 1  \nyes\n")]))
    put(case("add_plain_field_eof", [step("add", "gmail", "--workspace", "personal", stdin="")]))
    put(case("add_plain_field_blank", [step("add", "gmail", "--workspace", "personal", *yes, stdin="   \n")]))
    put(case("add_custom_alias", [step(*add, "--alias", "team-chat", *yes, stdin=f"{SECRET}\n")]))
    put(case("add_twice", [step(*add, *yes, stdin=f"{SECRET}\n"), step(*add, *yes, stdin="other\n")]))
    put(case("add_alias_in_use", [step(*add, *yes, stdin=f"{SECRET}\n"),
                                  step("add", "gmail", "--workspace", "personal", "--alias", "acme-slack", *yes, stdin="/k\n")]))  # fmt: skip
    put(case("add_no_backend", [step(*add, *yes, stdin=f"{SECRET}\n")], keychain="none"))
    for label, args in {"unknown_provider": ["add", "nope", "--workspace", "acme"], "upper_provider": ["add", "Slack", "--workspace", "acme"],
                        "unknown_workspace": ["add", "slack", "--workspace", "nope"], "orphan": ["add", "slack", "--workspace", "orphan"],
                        "bad_alias": [*add, "--alias", "Bad Alias"], "long_alias": [*add, "--alias", "a" * 65], "empty_alias": [*add, "--alias", ""],
                        "no_workspace": ["add", "slack"], "no_provider": ["add", "--workspace", "acme"]}.items():  # fmt: skip
        put(case(f"add_bad_{label}", [step(*args, stdin=f"{SECRET}\ny\n")]))

    # ---- remove
    kc = {"easy-personal/slack-acme": ITEM({"T": SECRET})}
    put(case("remove_stored_ask_yes", [step("remove", "slack-acme", stdin="y\n"), step("list")], setup=ACME, keychain=kc, claude=fake))
    put(case("remove_stored_ask_no", [step("remove", "slack-acme", stdin="n\n")], setup=ACME, keychain=kc, claude=fake))
    put(case("remove_stored_ask_eof", [step("remove", "slack-acme", stdin="")], setup=ACME, keychain=kc, claude=fake))
    put(case("remove_stored_registered", [step("remove", "slack-acme", *yes)], setup=ACME + REG_WORK, keychain=kc,
             claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}}))  # fmt: skip
    put(case("remove_stored_claude_fails", [step("remove", "slack-acme", *yes)], setup=ACME + REG_WORK, keychain=kc,
             claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}}, fail="remove acme-slack"))  # fmt: skip
    put(case("remove_stored_no_claude", [step("remove", "slack-acme", *yes)], setup=ACME + REG_WORK, keychain=kc))
    put(case("remove_stored_account_folder_missing", [step("remove", "slack-acme", *yes)], setup=ACME + REG_WORK,
             dirs=["agent-root/.claude"], keychain=kc, claude=fake))  # fmt: skip
    put(case("remove_stored_no_backend", [step("remove", "slack-acme", *yes)], setup=ACME, keychain="none", claude=fake))
    pre = {"easy-personal/slack": ITEM({"T": SECRET})}
    put(case("remove_pre_workspace_ours", [step("remove", "slack", stdin="n\n"), step("remove", "slack", *yes)], keychain=pre,
             claude={"<unset>": {"slack": OURS("slack")}}))  # fmt: skip
    put(case("remove_pre_workspace_foreign", [step("remove", "slack", *yes)], keychain=pre, claude={"<unset>": {"slack": FOREIGN}}))
    put(case("remove_pre_workspace_claude_fails", [step("remove", "slack", *yes)], keychain=pre,
             claude={"<unset>": {"slack": OURS("slack")}}, fail="remove slack"))  # fmt: skip
    put(case("remove_pre_workspace_no_claude", [step("remove", "slack", *yes)], keychain=pre))
    put(case("remove_absent", [step("remove", "discord", stdin="y\n")], claude=fake))
    put(case("remove_pre_workspace_no_backend", [step("remove", "slack", *yes)], keychain="none", claude=fake))
    for label, name in {"not_catalog": "github", "slash": "a/b", "dots": "..", "upper": "Slack", "long": "a" * 65, "empty": ""}.items():
        put(case(f"remove_bad_{label}", [step("remove", name, *yes)], keychain=pre, claude={"<unset>": {"slack": OURS("slack")}}))
    put(case("remove_no_name", [step("remove")]))

    # ---- sync
    put(case("sync_every_step_kind", [step("sync", stdin=""), step("sync", stdin="n\n"), step("sync", *yes), step("sync", *yes)],
             setup=EVERY_KIND, claude=KINDS_STATE))  # fmt: skip
    put(case("sync_ask_yes", [step("sync", stdin="Y\n")], setup=EVERY_KIND, claude=KINDS_STATE))
    put(case("sync_account_filters", [step("sync", "--account", "1", *yes), step("sync", "--account", "@W@/agent-root/.claude-work"),
                                      step("sync", "--account", " 2 ", stdin="n\n"), step("sync", "--account", "99", *yes),
                                      step("sync", "--account", "abc", *yes), step("sync", "--account", "", *yes)],
             setup=EVERY_KIND, claude=KINDS_STATE))  # fmt: skip
    put(case("sync_failing_claude", [step("sync", *yes), step("sync", *yes)], setup=EVERY_KIND, claude=KINDS_STATE, fail="add-json acme-slack"))
    put(case("sync_no_claude", [step("sync", *yes)], setup=EVERY_KIND))
    put(case("sync_skipped_account", [step("sync", *yes)], setup=EVERY_KIND, dirs=["agent-root/.claude-work/projects"], claude=KINDS_STATE))
    put(case("sync_nothing", [step("sync", *yes), step("sync")], claude=fake))
    put(case("sync_no_launcher", [step("sync", *yes)], setup=EVERY_KIND, claude=KINDS_STATE, launcher=False))
    put(case("sync_ignored_account", [step("sync", *yes)], setup=ACME + REG_WORK + ["UPDATE agent_accounts SET status = 'ignored' WHERE id = 2"],
             claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}}))  # fmt: skip

    # ---- move and alias
    reg = dict(setup=ACME + REG_WORK, claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}})
    put(case("move_ask_yes", [step("move", "slack-acme", "personal", stdin="y\n"), step("list")], **reg))
    put(case("move_ask_no", [step("move", "slack-acme", "personal", stdin="n\n")], **reg))
    put(case("move_to_orphan_yes", [step("move", "slack-acme", "orphan", *yes)], **reg))
    put(case("move_claude_fails", [step("move", "slack-acme", "personal", *yes)], **reg, fail="remove acme-slack"))
    put(case("move_no_claude", [step("move", "slack-acme", "personal", *yes)], setup=ACME + REG_WORK))
    for label, args in {"same": ["slack-acme", "acme"], "unknown_workspace": ["slack-acme", "nope"], "unknown_id": ["nope", "personal"],
                        "held": ["slack-acme", "personal"], "no_workspace": ["slack-acme"]}.items():  # fmt: skip
        put(case(f"move_bad_{label}", [step("move", *args, *yes)], setup=ACME + ["INSERT INTO connectors VALUES ('slack-personal', 'personal-slack', 0, 'slack', 3)"],
                 claude=fake))  # fmt: skip
    put(case("alias_custom_then_default", [step("alias", "slack-acme", "team-chat", *yes), step("alias", "slack-acme", "--default", stdin="y\n"),
                                           step("list")], **reg))  # fmt: skip
    put(case("alias_ask_no", [step("alias", "slack-acme", "team-chat", stdin="")], **reg))
    orphaned = ACME + ["INSERT INTO connectors VALUES ('slack-orphan', 'slack', 0, 'slack', 1)"]
    for label, args in {"both": ["slack-acme", "x", "--default"], "neither": ["slack-acme"], "in_use": ["slack-acme", "acme-gmail"],
                        "bad": ["slack-acme", "Bad!"], "long": ["slack-acme", "a" * 65], "unknown_id": ["nope", "x"],
                        "orphan_default": ["slack-orphan", "--default"], "orphan_clash_ok": ["slack-orphan", "acme-gmail"]}.items():  # fmt: skip
        put(case(f"alias_bad_{label}", [step("alias", *args, *yes)], setup=orphaned, claude=fake))

    # ---- adopt-legacy
    legacy_kc = {"easy-personal/slack": ITEM({"T": SECRET}), "easy-personal/discord": ITEM({"D": SECRET}), "easy-personal/gmail": ITEM({})}
    legacy_claude = {"<unset>": {"slack": OURS("slack"), "gmail": FOREIGN, "teams": OURS("teams")}, WORK_CCD: {"sendgrid": OURS("sendgrid")}}
    put(case("legacy_adopt", [step("adopt-legacy", stdin="n\n"), step("adopt-legacy", *yes), step("adopt-legacy", *yes), step("list")],
             keychain=legacy_kc, claude=legacy_claude))  # fmt: skip
    put(case("legacy_ask_yes", [step("adopt-legacy", stdin="yes\n")], keychain=legacy_kc, claude=legacy_claude))
    put(case("legacy_claude_fails", [step("adopt-legacy", *yes)], keychain={"easy-personal/slack": ITEM({})},
             claude={"<unset>": {"slack": OURS("slack")}}, fail="remove slack"))  # fmt: skip
    put(case("legacy_no_backend", [step("adopt-legacy", *yes)], keychain="none", claude=fake))
    put(case("legacy_nothing", [step("adopt-legacy", *yes)], claude=fake))
    put(case("legacy_no_claude", [step("adopt-legacy", *yes)], keychain={"easy-personal/slack": ITEM({})}))

    # ---- doctor
    put(case("doctor_clean", [step("doctor")], claude=fake))
    put(case("doctor_no_claude", [step("doctor")]))
    put(case("doctor_no_backend", [step("doctor")], keychain="none", claude=fake))
    put(case("doctor_no_launcher", [step("doctor")], claude=fake, launcher=False))
    put(case("doctor_pre_workspace", [step("doctor")],
             keychain={"easy-personal/slack": ITEM({}), "easy-personal/discord": ITEM({}), "easy-personal/gmail": ITEM({})},
             claude={"<unset>": {"slack": OURS("slack"), "discord": FOREIGN, "teams": OURS("teams")}}))  # fmt: skip
    put(case("doctor_adopted_skipped", [step("doctor")], setup=["INSERT INTO connectors VALUES ('slack', 'slack', 0, 'slack', 1)"],
             keychain={"easy-personal/slack": ITEM({})}, claude={"<unset>": {"slack": FOREIGN}}))  # fmt: skip
    put(case("doctor_registrations", [step("doctor")],
             setup=ACME + PICK_DEFAULT + ["INSERT INTO account_registrations VALUES (2, 'acme-gmail', 'gmail-acme')",
                                          "INSERT INTO account_registrations VALUES (1, 'acme-slack', 'slack-acme')",
                                          "INSERT INTO account_registrations VALUES (2, 'acme-slack', 'slack-acme')"],
             keychain={"easy-personal/slack-acme": ITEM({})},
             claude={WORK_CCD: {"acme-slack": OURS("slack-acme"), "acme-gmail": FOREIGN}, "<unset>": {}}))  # fmt: skip
    put(case("doctor_registrations_no_claude", [step("doctor")], setup=ACME + REG_WORK, keychain={"easy-personal/slack-acme": ITEM({})}))
    put(case("doctor_account_folder_missing", [step("doctor")], setup=ACME + REG_WORK, dirs=["agent-root/.claude"],
             keychain={"easy-personal/slack-acme": ITEM({}), "easy-personal/gmail-acme": ITEM({})}, claude=fake))  # fmt: skip
    put(case("doctor_stored_no_backend", [step("doctor")], setup=ACME + REG_WORK, keychain="none",
             claude={WORK_CCD: {"acme-slack": OURS("slack-acme")}}))  # fmt: skip
    put(case("doctor_extra_arg", [step("doctor", "x")]))
    return out


# ---- the run ----


class Crash(Exception):
    pass


BACKEND = re.compile(r"^ok keychain: .*$", re.M)


def run_step(cli, argv: list[str], stdin: str, work: str) -> dict:
    out, err = io.StringIO(), io.StringIO()
    old_stdin, sys.stdin = sys.stdin, io.StringIO(stdin, newline=None)  # universal newlines, as the real text-mode stdin
    try:
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                code = cli.main(argv)
            except SystemExit as stop:  # argparse
                code = stop.code if isinstance(stop.code, int) else 1
            except Exception as exc:  # noqa: BLE001 - a traceback is not an answer to compare
                raise Crash(f"{argv}: {type(exc).__name__}: {exc}") from exc
    finally:
        sys.stdin = old_stdin
    clean = lambda text: BACKEND.sub("ok keychain: <BACKEND>", gc.scrub(text, work))  # noqa: E731
    stderr = clean(err.getvalue())
    return {"code": code, "stdout": clean(out.getvalue()), "stderr": stderr, "stderr_compare": "none" if stderr.startswith("usage: ") else "exact"}


def run(out: Path) -> None:
    gen_golden.refuse_real_paths(out)
    os.environ.pop("CLAUDE_CONFIG_DIR", None)
    try:  # getpass must not find a terminal: its fallback (stdin) is the path both sides take
        os.close(os.open("/dev/tty", os.O_RDWR | os.O_NOCTTY))
        sys.exit("a controlling terminal is open: run through main(), which detaches with setsid first")
    except OSError:
        pass
    done = []
    with tempfile.TemporaryDirectory(prefix="parity-connector-cli-") as tmp:
        base = Path(tmp).resolve()
        gen_golden.isolate(base)
        gen_golden.guard_sqlite()
        import getpass

        import keyring
        from keyring.backends import fail

        keyring.set_keyring(fail.Keyring())  # before anything: the real keychain is never the backend
        warnings.simplefilter("ignore", getpass.GetPassWarning)
        sys.path.insert(0, str(gen_golden.REPO / "tests" / "connectors"))
        from fake_backend import MemoryKeyring

        from easy_personal.connectors import writer
        from easy_personal.launch import LaunchError
        from easy_personal.surfaces import cli

        def no_launcher():
            raise LaunchError(f"cannot find {writer.LAUNCHER}; reinstall easy-personal (uv tool install) so it is on disk")

        specs = cases()
        names = [c["name"] for c in specs]
        assert len(names) == len(set(names)), [n for n in names if names.count(n) > 1]
        for number, spec in enumerate(specs):
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
            if spec["keychain"] == "none":
                keyring.set_keyring(fail.Keyring())
            else:
                backend = MemoryKeyring()
                for service, secret in spec["keychain"].items():
                    backend.items[(service, "connector")] = secret
                keyring.set_keyring(backend)
            fake_bin, state, log = work / "bin", work / "claude-state", work / "claude.log"
            fake_bin.mkdir()
            if spec["claude"] is not None:
                shutil.copy(gc.FAKE_CLAUDE, fake_bin / "claude")
                (fake_bin / "claude").chmod(0o755)
                gc.write_state(state, spec["claude"], wtext)
            os.environ.update({"PATH": f"{fake_bin}:/usr/bin:/bin", "FAKE_CLAUDE_LOG": str(log), "FAKE_CLAUDE_STATE": str(state),
                               "CLAUDE_CONFIG_DIR": "/runner/dir", "FAKE_CLAUDE_STDERR": ""})  # fmt: skip
            os.environ.pop("FAKE_CLAUDE_FAIL", None)
            if spec["fail"]:
                os.environ["FAKE_CLAUDE_FAIL"] = spec["fail"]
            assert shutil.which("claude") in (None, str(fake_bin / "claude")), "a claude other than the fake is on PATH"
            writer.launcher_path = (lambda: FAKE_LAUNCHER) if spec["launcher"] else no_launcher
            ids = iter(f"c{n:08x}" for n in range(1, 99))
            writer.new_id = lambda ids=ids: next(ids)
            before = gc.dump(store, wtext)
            try:
                results = [run_step(cli, [a.replace("@W@", wtext) for a in s["args"]], s["stdin"], wtext) for s in spec["steps"]]
            except Crash as exc:
                sys.exit(f"Python crashed in {spec['name']} ({exc}): such a case belongs in the README list, not in the oracle")
            keychain_after = "none" if spec["keychain"] == "none" else {s: v for (s, _), v in sorted(keyring.get_keyring().items.items())}
            after = gc.dump(store, wtext)
            done.append({**spec, "results": results, "keychain_after": keychain_after, "calls": gc.read_log(log, wtext),
                         "claude_after": gc.read_state(state, wtext) if spec["claude"] is not None else None,
                         "changed": {t: rows for t, rows in after.items() if rows != before[t]}})  # fmt: skip
            keyring.set_keyring(fail.Keyring())
        for name in ("FAKE_CLAUDE_LOG", "FAKE_CLAUDE_STATE", "FAKE_CLAUDE_FAIL", "FAKE_CLAUDE_STDERR", "CLAUDE_CONFIG_DIR"):
            os.environ.pop(name, None)
    leaks = [c["name"] for c in done for r in c["results"] if "/home/" in r["stdout"] + r["stderr"] or SECRET in r["stdout"] + r["stderr"]]
    if leaks:
        sys.exit(f"an answer holds a path of a real home or the planted secret: {leaks}")
    out.mkdir(parents=True, exist_ok=True)
    manifest = {"generator": "rust/parity/gen_connector_cli.py", "source_store": "rust/parity/store-rich/store-v11.sqlite",
                "fake_claude": "rust/parity/golden-connectors/fake-claude.sh", "cases": done}  # fmt: skip
    (out / "cases.json").write_text(json.dumps(manifest, indent=1, sort_keys=True, ensure_ascii=False) + "\n")
    steps = sum(len(c["results"]) for c in done)
    calls = sum(len(c["calls"]) for c in done)
    print(f"{len(done)} cases, {steps} steps, {calls} claude calls -> {out}")


def detached(target, *args) -> int:
    """Run `target(*args)` in a child that has called `setsid` (no controlling terminal); its exit code."""
    sys.stdout.flush()
    sys.stderr.flush()
    pid = os.fork()
    if pid == 0:
        code = 0
        try:
            os.setsid()
            target(*args)
        except SystemExit as stop:
            code = stop.code if isinstance(stop.code, int) else (print(stop.code, file=sys.stderr) or 1)
        except BaseException:  # noqa: BLE001 - shown, then a failing exit code
            import traceback

            traceback.print_exc()
            code = 1
        sys.stdout.flush()
        sys.stderr.flush()
        os._exit(code)
    return os.waitstatus_to_exitcode(os.waitpid(pid, 0)[1])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=PARITY / "golden-connector-cli")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if not args.check:
        return detached(run, args.out.resolve())
    with tempfile.TemporaryDirectory() as tmp:
        fresh = Path(tmp) / "golden-connector-cli"
        code = detached(run, fresh)
        if code:
            return code
        if (fresh / "cases.json").read_bytes() != (PARITY / "golden-connector-cli" / "cases.json").read_bytes():
            print("different: golden-connector-cli/cases.json")
            return 1
    print("golden-connector-cli is equal to a fresh run")
    return 0


if __name__ == "__main__":
    sys.exit(main())
