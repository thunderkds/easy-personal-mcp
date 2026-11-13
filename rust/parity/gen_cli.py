#!/usr/bin/env python3
"""Oracle for the Rust CLI commands (T058): the Python CLI's own `main()` over copies of `store-rich` and over an empty store.

    rust/parity/.venv/bin/python rust/parity/gen_cli.py            # writes rust/parity/golden-cli/cases.json
    rust/parity/.venv/bin/python rust/parity/gen_cli.py --check    # regenerate in a temp folder and compare: exit 0 equal
    rust/parity/.venv/bin/python rust/parity/gen_cli.py --out DIR  # same, elsewhere

Each case is a work folder (`agent-root` with the folders the case needs, a store copy, `nowhere` as `HOME`) and a list of
steps. A step is one `easy-personal <args>` call with the text sent on stdin (`input()` reads it, and EOF when it is empty).
For every step and every zone of the case the file records the exit code, stdout and stderr, with the work folder written
as `<WORK>` and the `as_of` of `kit-findings --json` (the one value that is the clock's) written as `<NOW>`. The Rust test
(`crates/app/tests/cli_parity.rs`) builds the same work folder (the same `setup` SQL and `tree`), runs the real binary with the
same arguments and stdin under the same `TZ`, and requires the same exit code and stdout, and the same stderr where the
record says so (`stderr_compare`: `exact`, `prefix` = Python's `easy-personal: ` line with `easy-personal-rs: ` in its place,
`none` = argparse's usage text).

Not here, because Python crashes (a traceback), cannot be asked of it, or differs by design (`rust/README.md` lists each):
`workspace create` (the guide's word for `workspace add`; Python has only `add`, Rust takes both),
a missing store (Python creates and migrates one; Rust refuses and creates nothing), a plan that needs the `claude` CLI and
is confirmed (Python runs it; Rust refuses with the T048 text), `kit-findings --limit` in non-ASCII digits, a date outside
Python's range.

Python is only read. Nothing here opens the owner's folders (`gen_golden.refuse_real_paths`, `gen_golden.guard_sqlite`).
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
import time
from pathlib import Path

PARITY = Path(__file__).resolve().parent
sys.path.insert(0, str(PARITY))
import gen_golden  # noqa: E402

SOURCE = PARITY / "store-rich" / "store-v11.sqlite"
UTC = ["UTC"]
ZONES = ["UTC", "Pacific/Kiritimati", "Pacific/Pago_Pago"]
ACCOUNT_TREE = {
    "dirs": ["agent-root/.claude-new/projects", "agent-root/.codex", "extra/.claude-x/projects", "extra/not-agent", "extra/.codex-y"],
    "files": {"agent-root/.codex/config.toml": "", "extra/.codex-y/config.toml": "", "extra/a-file": "x"},
}
CONNECTORS = [  # two connectors in `acme` (workspace 2), which account 2 picks
    "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2)",
    "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('gmail-acme', 'acme-gmail', 0, 'gmail', 2)",
]
UNPICK = ["DELETE FROM account_workspaces"]  # nobody picks `acme`: its plan has no step, so no `claude` is needed
EXTRA_WORKSPACE = ["INSERT INTO workspaces (id, name, is_orphan) VALUES (9, 'taken', 0)"]


def step(*args, stdin=""):
    return {"args": list(args), "stdin": stdin}


def case(name, steps, *, store="rich", setup=(), tree=None, zones=UTC):
    return {"name": name, "store": store, "setup": list(setup), "tree": tree or {"dirs": [], "files": {}}, "zones": list(zones),
            "steps": steps}


def cases():
    out = []
    add = out.append
    one = lambda name, *args, **kw: add(case(name, [step(*args)], **kw))  # noqa: E731
    both = lambda name, *args: (one(f"rich_{name}", *args, zones=ZONES), one(f"empty_{name}", *args, store="empty", zones=ZONES))  # noqa: E731

    one("ping", "ping", store="none")
    # ---- usage
    both("usage", "usage")
    for by in ("day", "model", "project", "tool"):
        both(f"usage_by_{by}", "usage", "--by", by)
    win = ["--since", "2026-01-13T00:00:00Z", "--until", "2026-01-20T00:00:00Z"]
    one("rich_usage_window", "usage", *win, zones=ZONES)
    one("rich_usage_window_by_day", "usage", "--by", "day", *win, zones=ZONES)
    one("rich_usage_window_by_project", "usage", "--by", "project", *win, zones=ZONES)
    one("rich_usage_mid", "usage", "--since", "2026-01-12T18:00:00Z", "--until", "2026-01-15T06:00:00Z", zones=ZONES)
    one("rich_usage_naive_dates", "usage", "--since", "2026-01-13", "--until", "2026-01-15", zones=ZONES)
    one("rich_usage_naive_datetimes_by_day", "usage", "--by", "day", "--since", "2026-01-13T08:30:00", "--until", "2026-01-15T23:00:00", zones=ZONES)
    one("rich_usage_offset_dates", "usage", "--since", "2026-01-13T00:00:00+05:00", "--until", "2026-01-15T00:00:00-03:30", zones=ZONES)
    one("rich_usage_since_only", "usage", "--since", "2026-01-15", zones=ZONES)
    one("rich_usage_until_only", "usage", "--until", "2026-01-05", zones=ZONES)
    one("rich_usage_since_after_until", "usage", "--since", "2026-01-15", "--until", "2026-01-10", zones=ZONES)
    one("rich_usage_since_equals_until", "usage", "--since", "2026-01-15", "--until", "2026-01-15", zones=ZONES)
    one("rich_usage_future", "usage", "--since", "2030-01-01", zones=ZONES)
    for who in (["--workspace", "acme"], ["--workspace", "personal"], ["--workspace", "orphan"], ["--account", "2"], ["--account", "Claude work"],
                ["--account", "claude DEFAULT"], ["--account", "1"], ["--workspace", "acme", "--account", "2"], ["--workspace", ""], ["--account", ""], ["--workspace", "", "--account", ""]):
        label = "_".join(re.sub(r"\W+", "", w.lower()) or "empty" for w in who)
        one(f"rich_usage_scope_{label}", "usage", *who, zones=ZONES)
        one(f"rich_usage_by_model_scope_{label}", "usage", "--by", "model", *who)
    for who in (["--workspace", "nope"], ["--account", "nope"], ["--account", "9"], ["--workspace", "nope", "--account", "nope"], ["--workspace", "ACME"]):
        one("rich_usage_unknown_" + "_".join(re.sub(r"\W+", "", w.lower()) for w in who), "usage", *who)
    for bad in (["--since", "banana"], ["--until", "banana"], ["--since", ""], ["--since", "2026-13-01"], ["--since", "2026-01-13 08:00"],
                ["--by", "week"], ["--by", ""], ["--by", "DAY"], ["--workspace"], ["--limit", "3"], ["extra"]):
        one("rich_usage_bad_" + "_".join(re.sub(r"\W+", "", w.lower()) or "empty" for w in bad), "usage", *bad)
    # ---- outliers
    both("outliers", "outliers")
    one("rich_outliers_window", "outliers", *win, zones=ZONES)
    one("rich_outliers_naive", "outliers", "--since", "2026-01-13T08:30:00", "--until", "2026-01-16", zones=ZONES)
    one("rich_outliers_future", "outliers", "--since", "2030-01-01")
    for who in (["--workspace", "acme"], ["--workspace", "personal"], ["--account", "2"], ["--account", "Claude work"], ["--workspace", "acme", "--account", "2"]):
        one("rich_outliers_scope_" + "_".join(re.sub(r"\W+", "", w.lower()) for w in who), "outliers", *who)
    for who in (["--workspace", "nope"], ["--account", "nope"], ["--since", "banana"], ["--by", "day"]):
        one("rich_outliers_bad_" + "_".join(re.sub(r"\W+", "", w.lower()) for w in who), "outliers", *who)
    # ---- kit-findings
    both("kit_findings", "kit-findings")
    both("kit_findings_json", "kit-findings", "--json")
    for limit in ("1", "2", "10", "11", "99999999999999999999", "+3", " 4 ", "1_0"):
        one(f"rich_kit_findings_limit_{re.sub(r'[^0-9a-z]+', 'x', limit)}", "kit-findings", "--limit", limit)
    one("rich_kit_findings_json_limit_1", "kit-findings", "--json", "--limit", "1")
    for since in ("2026-01-06T00:00:00Z", "2026-01-15", "2026-01-15T08:00:00", "2030-01-01"):
        one(f"rich_kit_findings_since_{re.sub(r'[^0-9]+', '', since)}", "kit-findings", "--since", since, zones=ZONES)
        one(f"rich_kit_findings_json_since_{re.sub(r'[^0-9]+', '', since)}", "kit-findings", "--json", "--since", since, zones=ZONES)
    for bad in (["--limit", "0"], ["--limit", "-1"], ["--limit", "abc"], ["--limit", ""], ["--limit", "1.5"], ["--since", "banana"], ["--workspace", "acme"]):
        one("rich_kit_findings_bad_" + "_".join(re.sub(r"\W+", "x", w) for w in bad), "kit-findings", *bad)
    # ---- workspace list / add / edit / delete
    both("workspace_list", "workspace", "list")
    one("rich_workspace_list_extra_arg", "workspace", "list", "x")
    one("workspace_no_action", "workspace")
    add(case("ws_add_minimal", [step("workspace", "add", "newone"), step("workspace", "list")]))
    add(case("ws_add_full", [step("workspace", "add", "full-1", "--color", "#aBcDeF", "--icon", "\U0001f680", "--description", "né ☕ line\nnext",
                                  "--email", "me@example.invalid", "--repo", "/work/a/", "--repo", "/work/b/../c", "--repo", "/work/a", "--repo", "~/proj"),
                             step("workspace", "list")]))
    add(case("ws_add_empty_text_fields", [step("workspace", "add", "blank", "--icon", "", "--description", "", "--email", ""), step("workspace", "list")]))
    add(case("ws_add_repos_sorted_deduped", [step("workspace", "add", "sorted", "--repo", "/z", "--repo", "/a", "--repo", "/a/", "--repo", "//a", "--repo", "/a/./b")]))
    add(case("ws_add_symlink_repo", [step("workspace", "add", "linked", "--repo", "@W@/link/x", "--repo", "@W@/link")],
             tree={"dirs": ["real"], "files": {}, "symlinks": {"link": "@W@/real"}}))
    for label, args in {
        "taken": ["acme"], "orphan": ["orphan"], "upper": ["Bad-Secret"], "empty": [""], "long41": ["a" * 41], "dash_first": ["-x"], "underscore": ["x_y"],
        "unicode": ["ünï"], "bad_color": ["ok", "--color", "red"], "short_color": ["ok", "--color", "#12345"], "empty_color": ["ok", "--color", ""],
        "long_icon": ["ok", "--icon", "i" * 33], "long_description": ["ok", "--description", "d" * 501], "ok_description_500": ["ok", "--description", "d" * 500],
        "long_email": ["ok", "--email", "e" * 255], "relative_repo": ["ok", "--repo", "relative"], "empty_repo": ["ok", "--repo", ""], "blank_repo": ["ok", "--repo", "  "],
        "second_repo_bad": ["ok", "--repo", "/fine", "--repo", "bad"], "bad_color_and_name": ["Bad", "--color", "red"], "bad_repo_and_name": ["Bad", "--repo", "rel"],
        "no_name": [], "unknown_flag": ["ok", "--yes"],
    }.items():
        add(case(f"ws_add_bad_{label}", [step("workspace", "add", *args), step("workspace", "list")]))
    add(case("ws_edit_rename", [step("workspace", "edit", "acme", "--rename", "company"), step("workspace", "list")]))
    add(case("ws_edit_rename_same_name", [step("workspace", "edit", "acme", "--rename", "acme")]))
    add(case("ws_edit_repos", [step("workspace", "edit", "acme", "--add-repo", "/work/new", "--remove-repo", "/work/rich/gone-a", "--remove-repo", "/not/there")]))
    add(case("ws_edit_remove_all_repos", [step("workspace", "edit", "acme", "--remove-repo", "/work/rich/gone-a", "--remove-repo", "/work/rich/kit/alpha")]))
    add(case("ws_edit_fields", [step("workspace", "edit", "personal", "--color", "#00ff00", "--icon", "P", "--description", "mine", "--email", "")]))
    add(case("ws_edit_clear_fields", [step("workspace", "edit", "acme", "--color", "", "--description", "")]))
    add(case("ws_edit_nothing", [step("workspace", "edit", "acme")]))
    add(case("ws_edit_everything", [step("workspace", "edit", "acme", "--rename", "everything", "--color", "#010203", "--icon", "E", "--description", "d",
                                         "--email", "e@example.invalid", "--add-repo", "/x/y", "--remove-repo", "/work/rich/kit/alpha")]))
    for label, args in {
        "unknown": ["nope", "--description", "x"], "orphan_rename": ["orphan", "--rename", "x"], "orphan_text": ["orphan", "--description", "x"],
        "to_orphan": ["acme", "--rename", "orphan"], "to_taken": ["acme", "--rename", "personal"], "bad_name": ["acme", "--rename", "Bad Name"],
        "empty_name": ["acme", "--rename", ""], "bad_color": ["acme", "--color", "red"], "bad_color_then_name": ["acme", "--color", "red", "--rename", "Bad"],
        "bad_name_then_repo": ["acme", "--rename", "Bad", "--add-repo", "rel"], "rel_add_repo": ["acme", "--add-repo", "relative"],
        "empty_add_repo": ["acme", "--add-repo", ""], "rel_remove_repo": ["acme", "--remove-repo", "relative"], "long_icon": ["acme", "--icon", "i" * 33],
        "unknown_and_bad": ["nope", "--rename", "Bad"],
    }.items():
        add(case(f"ws_edit_bad_{label}", [step("workspace", "edit", *args), step("workspace", "list")]))
    # a rename of a workspace that holds connectors shows the plan first; with nobody picking it the plan has no step
    held = dict(setup=[*CONNECTORS, *UNPICK])
    add(case("ws_edit_rename_held_yes", [step("workspace", "edit", "acme", "--rename", "company", "--yes"), step("workspace", "list")], **held))
    for label, text in {"y": "y\n", "yes": "yes\n", "Y_spaces": "  Y \n", "YES": "YES\n", "no": "n\n", "empty_line": "\n", "eof": "", "other": "maybe\n",
                        "y_no_newline": "y", "yes_then_more": "y\ny\n"}.items():
        add(case(f"ws_edit_rename_held_ask_{label}", [step("workspace", "edit", "acme", "--rename", "company", stdin=text), step("workspace", "list")], **held))
    add(case("ws_edit_rename_held_custom_alias", [step("workspace", "edit", "acme", "--rename", "company", "--yes"), step("workspace", "list")],
             setup=[*CONNECTORS, *UNPICK, "UPDATE connectors SET alias_custom = 1, alias = 'mine' WHERE id = 'gmail-acme'"]))
    add(case("ws_edit_rename_held_alias_clash", [step("workspace", "edit", "acme", "--rename", "zeta", "--yes"), step("workspace", "list")],
             setup=[*CONNECTORS, *UNPICK, *EXTRA_WORKSPACE, "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('slack-x', 'zeta-slack', 1, 'slack', 9)"]))
    add(case("ws_edit_held_no_rename", [step("workspace", "edit", "acme", "--description", "kept")], **held))
    add(case("ws_edit_held_rename_bad_name", [step("workspace", "edit", "acme", "--rename", "Bad", "--yes")], **held))
    add(case("ws_edit_rename_held_to_taken", [step("workspace", "edit", "acme", "--rename", "personal", "--yes")], **held))
    # delete
    add(case("ws_delete_yes", [step("workspace", "delete", "acme", "--yes"), step("workspace", "list"), step("account", "list")]))
    add(case("ws_delete_personal_yes", [step("workspace", "delete", "personal", "--yes"), step("workspace", "list")]))
    add(case("ws_delete_empty_yes", [step("workspace", "add", "temp"), step("workspace", "delete", "temp", "--yes"), step("workspace", "list")]))
    for label, text in {"y": "y\n", "yes": "yes\n", "Y_spaces": "  Y \n", "YES": "YES\n", "no": "n\n", "empty_line": "\n", "eof": "", "other": "maybe\n",
                        "y_no_newline": "y", "tab_y": "\ty\t\n"}.items():
        add(case(f"ws_delete_ask_{label}", [step("workspace", "delete", "acme", stdin=text), step("workspace", "list")]))
    add(case("ws_delete_held_yes", [step("workspace", "delete", "acme", "--yes"), step("workspace", "list")], **held))
    add(case("ws_delete_held_ask_no", [step("workspace", "delete", "acme", stdin="n\n"), step("workspace", "list")], **held))
    add(case("ws_delete_held_ask_yes", [step("workspace", "delete", "acme", stdin="y\n"), step("workspace", "list")], **held))
    add(case("ws_delete_held_two_workspaces", [step("workspace", "delete", "acme", "--yes"), step("workspace", "delete", "personal", "--yes"), step("workspace", "list")],
             setup=[*CONNECTORS, *UNPICK, "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('slack-p', 'personal-slack', 0, 'slack', 3)"]))
    for label, args in {"unknown": ["nope", "--yes"], "orphan": ["orphan", "--yes"], "orphan_ask": ["orphan"], "unknown_ask": ["nope"], "no_name": ["--yes"],
                        "upper": ["ACME", "--yes"]}.items():
        add(case(f"ws_delete_bad_{label}", [step("workspace", "delete", *args, stdin="y\n"), step("workspace", "list")]))
    # ---- account list
    tree = ACCOUNT_TREE
    both("account_list", "account", "list")
    add(case("acct_list_detects", [step("account", "list")], tree=tree, zones=ZONES))
    add(case("acct_list_by_account", [step("account", "list", "--by", "account")], tree=tree, zones=ZONES))
    add(case("acct_list_by_provider", [step("account", "list", "--by", "provider")], tree=tree, zones=ZONES))
    add(case("acct_list_twice", [step("account", "list"), step("account", "list", "--by", "provider")], tree=tree))
    add(case("acct_list_empty_agent_root", [step("account", "list")], store="empty"))
    add(case("acct_list_default_only", [step("account", "list")], store="empty", tree={"dirs": ["agent-root/.claude/projects"], "files": {}}))
    add(case("acct_list_gone_folder", [step("account", "list")], tree=tree, setup=["UPDATE agent_accounts SET folder = '@W@/gone/.claude-gone', config_dir_env = '@W@/gone/.claude-gone' WHERE id = 2"]))
    add(case("acct_list_ignored_account", [step("account", "list")], tree=tree, setup=["UPDATE agent_accounts SET status = 'ignored' WHERE id = 2"]))
    add(case("acct_list_identity_files", [step("account", "list")], tree={"dirs": ["agent-root/.claude-work"], "files": {
        "agent-root/.claude-work/.claude.json": json.dumps({"oauthAccount": {"emailAddress": "work@example.invalid", "billingType": "api",
                                                                             "organizationName": "Acme ☕", "accessToken": "must-not-appear"}})}}, zones=ZONES))
    for bad in (["list", "--by", "colour"], ["list", "--by", ""], ["list", "extra"], []):
        add(case("acct_bad_" + "_".join(re.sub(r"\W+", "x", w) for w in bad or ["none"]), [step("account", *bad)], tree=tree))
    # ---- account confirm / ignore / add / use
    new = "3"  # the first folder `detect` adds on top of the rich store's two accounts is `.claude-new` (sorted by name)
    add(case("acct_confirm_by_id", [step("account", "confirm", "2", "--label", "Work renamed"), step("account", "list")], tree=tree))
    add(case("acct_confirm_detected", [step("account", "confirm", new, "--label", "New one"), step("account", "list")], tree=tree))
    add(case("acct_confirm_codex", [step("account", "confirm", "4", "--label", "Codex main")], tree=tree))
    add(case("acct_confirm_by_folder", [step("account", "confirm", "@W@/agent-root/.claude-new", "--label", "By folder")], tree=tree))
    add(case("acct_confirm_label_stripped", [step("account", "confirm", "2", "--label", "   spaced   ")], tree=tree))
    add(case("acct_confirm_label_unicode", [step("account", "confirm", "2", "--label", 'Café ☕ "work"')], tree=tree))
    add(case("acct_confirm_label_60", [step("account", "confirm", "2", "--label", "l" * 60)], tree=tree))
    for label, args in {
        "label_61": ["2", "--label", "l" * 61], "label_empty": ["2", "--label", ""], "label_blank": ["2", "--label", "   "], "unknown_id": ["99", "--label", "x"],
        "unknown_folder": ["@W@/agent-root/.claude-nope", "--label", "x"], "unknown_id_bad_label": ["99", "--label", ""], "word": ["abc", "--label", "x"],
        "no_label": ["2"], "no_account": ["--label", "x"], "zero": ["0", "--label", "x"], "padded_id": [" 2 ", "--label", "x"],
    }.items():
        add(case(f"acct_confirm_bad_{label}", [step("account", "confirm", *args), step("account", "list")], tree=tree))
    add(case("acct_ignore", [step("account", "ignore", new), step("account", "ignore", "2"), step("account", "list")], tree=tree))
    add(case("acct_ignore_by_folder", [step("account", "ignore", "@W@/agent-root/.claude-work")], tree=tree))
    add(case("acct_ignore_again", [step("account", "ignore", "2"), step("account", "ignore", "2")], tree=tree))
    for label, args in {"default": ["1"], "default_folder": ["@W@/agent-root/.claude"], "unknown": ["99"], "word": ["abc"], "no_account": []}.items():
        add(case(f"acct_ignore_bad_{label}", [step("account", "ignore", *args), step("account", "list")], tree=tree))
    add(case("acct_add_claude_folder", [step("account", "add", "@W@/extra/.claude-x"), step("account", "list")], tree=tree))
    add(case("acct_add_codex_folder", [step("account", "add", "@W@/extra/.codex-y")], tree=tree))
    add(case("acct_add_twice", [step("account", "add", "@W@/extra/.claude-x"), step("account", "add", "@W@/extra/.claude-x/")], tree=tree))
    add(case("acct_add_known_folder", [step("account", "add", "@W@/agent-root/.claude-new")], tree=tree))
    add(case("acct_add_default_folder", [step("account", "add", "@W@/agent-root/.claude")], store="empty", tree={"dirs": ["agent-root/.claude/projects"], "files": {}}))
    add(case("acct_add_symlinked_folder", [step("account", "add", "@W@/link")], tree={"dirs": ["extra/.claude-x/projects"], "files": {}, "symlinks": {"link": "@W@/extra/.claude-x"}}))
    for label, args in {"missing": ["@W@/extra/nope"], "file": ["@W@/extra/a-file"], "no_markers": ["@W@/extra/not-agent"], "empty": [""], "blank": ["   "], "no_path": [],
                        "tilde_nowhere": ["~/nope"]}.items():
        add(case(f"acct_add_bad_{label}", [step("account", "add", *args), step("account", "list")], tree=tree))
    add(case("acct_use", [step("account", "use", "2", "acme", "personal"), step("account", "list")], tree=tree))
    add(case("acct_use_clear", [step("account", "use", "2"), step("account", "list")], tree=tree))
    add(case("acct_use_replace", [step("account", "use", "2", "personal"), step("account", "use", "2", "acme", "acme")], tree=tree))
    add(case("acct_use_by_folder", [step("account", "use", "@W@/agent-root/.claude-work", "personal")], tree=tree))
    add(case("acct_use_default_account", [step("account", "use", "1", "personal")], tree=tree))
    add(case("acct_use_after_confirm", [step("account", "confirm", new, "--label", "New"), step("account", "use", new, "acme")], tree=tree))
    for label, args in {"unknown_workspace": ["2", "nope"], "orphan": ["2", "orphan"], "orphan_after_ok": ["2", "acme", "orphan"], "unknown_after_ok": ["2", "acme", "nope"],
                        "detected": [new, "acme"], "codex": ["4", "acme"], "unknown_account": ["99", "acme"], "unknown_account_clear": ["99"], "word": ["abc", "acme"],
                        "upper": ["2", "ACME"], "no_account": []}.items():
        add(case(f"acct_use_bad_{label}", [step("account", "use", *args), step("account", "list")], tree=tree))
    add(case("acct_use_then_workspace_delete", [step("account", "use", "2", "acme", "personal"), step("workspace", "delete", "acme", "--yes"), step("account", "list")], tree=tree))
    return out


class Crash(Exception):
    pass


def run_step(cli, argv: list[str], stdin: str, work: str) -> dict:
    out, err = io.StringIO(), io.StringIO()
    old_stdin, sys.stdin = sys.stdin, io.StringIO(stdin)
    try:
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                code = cli.main(argv)
            except SystemExit as stop:  # argparse
                code = stop.code if isinstance(stop.code, int) else 1
            except Exception as exc:  # noqa: BLE001 - a traceback is not an answer to compare
                raise Crash(f"{argv}: {type(exc).__name__}") from exc
    finally:
        sys.stdin = old_stdin
    clean = lambda text: re.sub(r'("as_of": )"[^"]*"', r'\1"<NOW>"', text.replace(work, "<WORK>"))  # noqa: E731
    stderr = clean(err.getvalue())
    mode = "exact" if not stderr else "prefix" if stderr.startswith("easy-personal: ") else "none" if stderr.startswith("usage: ") else "exact"
    return {"code": code, "stdout": clean(out.getvalue()), "stderr": stderr, "stderr_compare": mode}


def build_tree(work: Path, tree: dict) -> None:
    sub = lambda text: text.replace("@W@", str(work))  # noqa: E731
    for d in ["agent-root/.claude", "agent-root/.claude-work", "nowhere", *tree["dirs"]]:
        (work / d).mkdir(parents=True, exist_ok=True)
    for rel, content in tree["files"].items():
        (work / rel).parent.mkdir(parents=True, exist_ok=True)
        (work / rel).write_text(content)
    for rel, link in tree.get("symlinks", {}).items():
        (work / rel).symlink_to(sub(link))


def run(out: Path) -> None:
    gen_golden.refuse_real_paths(out)
    done = []
    with tempfile.TemporaryDirectory(prefix="parity-cli-") as tmp:
        base = Path(tmp).resolve()
        gen_golden.isolate(base)
        gen_golden.guard_sqlite()
        from easy_personal.connectors import sync, writer
        from easy_personal.core import store
        from easy_personal.surfaces import cli

        gen_golden.guard_machine_state()
        real_which = shutil.which
        shutil.which = lambda name, *a, **k: None if name == "claude" else real_which(name, *a, **k)
        assert sync.shutil is shutil and writer
        number = 0
        specs = cases()
        names = [c["name"] for c in specs]
        assert len(names) == len(set(names)), [n for n in names if names.count(n) > 1]
        for spec in specs:
            results = {}
            for zone in spec["zones"]:
                number += 1
                work = base / f"case-{number}"
                work.mkdir()
                text = str(work)
                gen_golden.isolate(work)
                os.environ["TZ"] = zone
                time.tzset()
                build_tree(work, spec["tree"])
                path = work / "home" / "store.sqlite"
                if spec["store"] == "rich":
                    shutil.copy(SOURCE, path)
                    conn = sqlite3.connect(path)
                    conn.execute("PRAGMA foreign_keys = ON")
                    conn.execute("UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?), config_dir_env = replace(config_dir_env, '/work/rich', ?)", (text, text))
                    for sql in spec["setup"]:
                        conn.execute(sql.replace("@W@", text))
                    conn.commit()
                    conn.close()
                elif spec["store"] == "empty":
                    store.connect(path).close()
                    if spec["setup"]:
                        raise SystemExit(f"{spec['name']}: setup is for the rich store")
                try:
                    results[zone] = [run_step(cli, [a.replace("@W@", text) for a in s["args"]], s["stdin"], text) for s in spec["steps"]]
                except Crash as exc:
                    sys.exit(f"Python crashed in {spec['name']} ({exc}): such a case belongs in the README list, not in the oracle")
            done.append({**spec, "results": results})
        shutil.which = real_which
    leaks = [c["name"] for c in done for rs in c["results"].values() for r in rs if "/home/" in r["stdout"] + r["stderr"] or "must-not-appear" in r["stdout"] + r["stderr"]]
    if leaks:
        sys.exit(f"an answer holds a path of a real home or a planted secret: {leaks}")
    out.mkdir(parents=True, exist_ok=True)
    manifest = {"generator": "rust/parity/gen_cli.py", "source_store": "rust/parity/store-rich/store-v11.sqlite", "zones": ZONES, "cases": done}
    (out / "cases.json").write_text(json.dumps(manifest, indent=1, sort_keys=True, ensure_ascii=False) + "\n")
    steps = sum(len(rs) for c in done for rs in c["results"].values())
    print(f"{len(done)} cases, {steps} step runs -> {out}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=PARITY / "golden-cli")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if not args.check:
        run(args.out.resolve())
        return 0
    with tempfile.TemporaryDirectory() as tmp:
        fresh = Path(tmp) / "golden-cli"
        run(fresh)
        if (fresh / "cases.json").read_bytes() != (PARITY / "golden-cli" / "cases.json").read_bytes():
            print("different: golden-cli/cases.json")
            return 1
    print("golden-cli is equal to a fresh run")
    return 0


if __name__ == "__main__":
    sys.exit(main())
