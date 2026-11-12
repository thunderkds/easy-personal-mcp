#!/usr/bin/env python3
"""Oracle for the Rust write routes (T048): the Python app's own handlers over copies of `store-rich`.

    rust/parity/.venv/bin/python rust/parity/gen_writes.py            # writes rust/parity/golden-writes/
    rust/parity/.venv/bin/python rust/parity/gen_writes.py --out DIR  # same, elsewhere (a re-run must diff empty)

For each case this runs the Python app (`api.create_app`, Starlette's `TestClient`) on a **temp copy** of
`store-rich/store-v11.sqlite` and records, for every request, the status, the content type and the body, and then the
store: which tables changed and their rows after the requests (`changed`), plus the name of every table (`tables`). The
Rust test (`crates/app/tests/writes.rs`) builds the same copy, folder tree and setup SQL, sends the same bytes to the real
`serve` binary, and requires the same answers and, for **every table**, the same rows: a table not named in `changed` must
be as it was before the requests.

Machine state: `@W@` in a request, a setup statement or the tree is the case's work folder; in the answers and the rows it
is `<WORK>`. The rich store's accounts name `/work/rich/agent-root/...`; the copy's `agent_accounts` rows point at the work
folder instead (as `accounts_parity.rs` does) so the folders exist. `HOME` is `@W@/nowhere`. There is no `claude` on PATH
(`shutil.which` answers none) and the keychain and `claude` calls raise: a plan that needs them is not asked to apply.

What is **not** here, because Python crashes (500), cannot be asked of it, or differs by design (the Rust tests and the
README list each): a body over 1 MiB (Python has no cap), a body nested past 128 levels or with `NaN` / `Infinity` / a lone
surrogate / UTF-16 text (Python's parser takes or crashes on them), an id in non-ASCII digits, an id past 18 digits, a label
that starts with NUL (a store CHECK: Python raises), a `confirm: true` on a plan that holds an `add` or `remove` step (Python
would run `claude`; Rust refuses with 409), and a `~name` repo of another user.

Python is only read; the real store and the owner's folders are never opened (`gen_golden.refuse_real_paths`).
"""

import argparse
import json
import os
import shutil
import sqlite3
import sys
import tempfile
from pathlib import Path

PARITY = Path(__file__).resolve().parent
sys.path.insert(0, str(PARITY))
import gen_golden  # noqa: E402

SOURCE = PARITY / "store-rich" / "store-v11.sqlite"
TOKEN, PORT = "parity-token", 5555
JSON = "application/json"
CONNECTORS = [  # two connectors in `acme` (workspace 2), whose account 2 picks it
    "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('slack-acme', 'acme-slack', 0, 'slack', 2)",
    "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('gmail-acme', 'acme-gmail', 0, 'gmail', 2)",
]
UNPICK = ["DELETE FROM account_workspaces"]  # nobody picks `acme`: its plan has no step
REGISTERED = ["INSERT INTO account_registrations (account_id, alias, connector_id) VALUES (2, 'acme-slack', 'slack-acme')"]
NEW_FOLDERS = {"dirs": ["agent-root/.claude-new/projects", "agent-root/.codex"], "files": {"agent-root/.codex/config.toml": ""}}


def req(method, path, body=None, *, raw=None, hexed=None, type=JSON, compare="full"):
    """One request. `body` is a JSON value, `raw` text, `hexed` bytes as hex; `type` None sends no content type."""
    if hexed is not None:
        data = bytes.fromhex(hexed)
    elif raw is not None:
        data = raw.encode()
    elif body is not None:
        data = json.dumps(body, ensure_ascii=False).encode()
    else:
        data = b""
    return {"method": method, "path": path, "type": type, "body_hex": data.hex(), "compare": compare}


def post(path, body=None, **kw):
    return req("POST", path, body, **kw)


def patch(ws, body=None, **kw):
    return req("PATCH", f"/api/workspaces/{ws}", body, **kw)


def delete(ws, body=None, **kw):
    return req("DELETE", f"/api/workspaces/{ws}", body, **kw)


def acct(route, ident, body=None, method="POST", **kw):
    return req(method, f"/api/accounts/{ident}/{route}", body, **kw)


def case(name, requests, setup=(), tree=None):
    return {"name": name, "setup": list(setup), "tree": tree or {"dirs": [], "files": {}, "symlinks": {}}, "requests": requests}


def cases():
    out = []
    add = out.append
    ws = "/api/workspaces"

    # ---- POST /api/workspaces
    add(case("ws_add_minimal", [post(ws, {"name": "newone"})]))
    add(case("ws_add_full", [post(ws, {"name": "full-1", "color": "#aBcDeF", "icon": "\U0001f680", "description": "né ☕ line\nnext",
                                       "email": "me@example.invalid", "repos": ["/work/a/", "/work/b/../c", "/work/a", "~/proj"]})]))
    add(case("ws_add_repos_sorted_deduped", [post(ws, {"name": "sorted", "repos": ["/z", "/a", "/a/", "//a", "/a/./b", "/a/b"]})]))
    add(case("ws_add_repo_symlink", [post(ws, {"name": "linked", "repos": ["@W@/link/x", "@W@/link"]})],
             tree={"dirs": ["real"], "files": {}, "symlinks": {"link": "@W@/real"}}))
    add(case("ws_add_repo_root", [post(ws, {"name": "rooted", "repos": ["/", "//"]})]))
    add(case("ws_add_1000_repos", [post(ws, {"name": "many", "repos": [f"/r/{n:04d}" for n in range(1000)]})]))
    add(case("ws_add_repo_long_path", [post(ws, {"name": "longpath", "repos": ["/" + "a" * 300 + "/b"]})]))
    add(case("ws_add_repo_unicode", [post(ws, {"name": "uni", "repos": ["/é/ü/日本", "  /spaced/path  "]})]))
    for label, repos in {
        "relative": ["rel/path"], "empty": [""], "blank": ["   "], "int": [5], "null_item": [None], "nul": ["/a\u0000b"],
        "not_list": "/a", "null": None, "object": {"a": 1}, "dot": ["."], "tilde_unset_user": ["~nosuchuser-x/p"],
        "second_bad": ["/ok", "bad"], "first_of_two_bad": [5, "bad"],
    }.items():
        add(case(f"ws_add_repos_{label}", [post(ws, {"name": "x", "repos": repos})]))
    for label, name in {
        "missing": None, "empty": "", "upper": "Bad", "orphan": "orphan", "long41": "a" * 41, "long40": "a" * 40,
        "leading_dash": "-x", "underscore": "x_y", "unicode": "ünï", "int": 5, "list": ["x"], "bool": True, "newline": "x\n",
        "trailing_space": "x ", "digit_first": "1abc", "dash_inside": "a-b-c", "single": "a", "taken": "acme",
    }.items():
        add(case(f"ws_add_name_{label}", [post(ws, {} if name is None else {"name": name})]))
    add(case("ws_add_name_null", [post(ws, {"name": None})]))
    for label, fields in {
        "color_red": {"color": "red"}, "color_short": {"color": "#12345"}, "color_long": {"color": "#1234567"},
        "color_nohash": {"color": "123456"}, "color_nonhex": {"color": "#12345g"}, "color_int": {"color": 5},
        "color_null": {"color": None}, "color_ok_lower": {"color": "#abcdef"}, "icon_32": {"icon": "i" * 32},
        "icon_33": {"icon": "i" * 33}, "icon_int": {"icon": 5}, "icon_float": {"icon": 1e2}, "icon_bigint": {"icon": 123456789012345678901234567890},
        "icon_emoji_32": {"icon": "\U0001f600" * 32}, "icon_emoji_33": {"icon": "\U0001f600" * 33},
        "description_500": {"description": "d" * 500}, "description_501": {"description": "d" * 501}, "description_list": {"description": ["x"]},
        "description_nul": {"description": "a\u0000b"}, "email_254": {"email": "e" * 254}, "email_255": {"email": "e" * 255},
        "email_bool": {"email": False}, "color_then_icon_bad": {"icon": "i" * 40, "color": "red"},
        "unknown_bogus": {"bogus": 1}, "unknown_id": {"id": 5}, "unknown_confirm": {"confirm": True}, "unknown_is_orphan": {"is_orphan": True},
    }.items():
        add(case(f"ws_add_{label}", [post(ws, {"name": "x", **fields})]))
    add(case("ws_add_bad_repos_and_bad_name", [post(ws, {"name": "BAD", "repos": ["rel"]})]))
    add(case("ws_add_bad_color_and_bad_name", [post(ws, {"name": "BAD", "color": "red"})]))
    add(case("ws_add_unknown_beats_everything", [post(ws, {"name": "BAD", "color": "red", "bogus": 1})]))
    add(case("ws_add_double_submit", [post(ws, {"name": "twice"}), post(ws, {"name": "twice"})]))
    add(case("ws_add_two_then_list_order", [post(ws, {"name": "zeta"}), post(ws, {"name": "alpha"})]))
    for label, r in {
        "not_json": req("POST", ws, raw="{nope"), "empty": req("POST", ws, raw=""), "blank": req("POST", ws, raw="  \n "),
        "list": req("POST", ws, raw="[]"), "null": req("POST", ws, raw="null"), "number": req("POST", ws, raw="5"),
        "string": req("POST", ws, raw='"x"'), "text_type": req("POST", ws, raw='{"name":"x"}', type="text/plain"),
        "no_type": req("POST", ws, raw='{"name":"x"}', type=None), "type_charset": req("POST", ws, raw='{"name":"charset"}', type="application/json; charset=utf-8"),
        "type_upper": req("POST", ws, raw='{"name":"upper"}', type="APPLICATION/JSON"), "type_spaced": req("POST", ws, raw='{"name":"spaced"}', type="  application/json  "),
        "type_suffix": req("POST", ws, raw='{"name":"x"}', type="application/jsonx"), "type_form": req("POST", ws, raw="name=x", type="application/x-www-form-urlencoded"),
        "dup_keys": req("POST", ws, raw='{"name":"first","name":"second"}'), "bom": req("POST", ws, hexed=(b"\xef\xbb\xbf" + b'{"name":"bom"}').hex()),
        "bad_utf8": req("POST", ws, hexed=b'{"name":"\xff\xfe"}'.hex()), "trailing_garbage": req("POST", ws, raw='{"name":"x"} x'),
        "trailing_ws": req("POST", ws, raw='{"name":"padded"} \n\t '), "single_quotes": req("POST", ws, raw="{'name':'x'}"),
        "nested_value": req("POST", ws, raw='{"name":"x","description":{"a":[1,2,{"b":null}]}}'),
        "escaped_key": req("POST", ws, raw='{"\\u006eame":"escaped"}'),
    }.items():
        add(case(f"ws_add_body_{label}", [r]))

    # ---- PATCH /api/workspaces/{id}
    add(case("ws_edit_rename_no_connectors", [patch(2, {"name": "company"})]))
    add(case("ws_edit_rename_then_list", [patch(2, {"name": "company"}), patch(2, {"name": "company2", "confirm": True})]))
    add(case("ws_edit_fields", [patch(2, {"description": "new text", "color": None, "email": "a@b.invalid", "icon": "I"})]))
    add(case("ws_edit_repos_replace", [patch(2, {"repos": ["/n/one", "/n/two", "/n/one/"]})]))
    add(case("ws_edit_repos_clear", [patch(2, {"repos": []})]))
    add(case("ws_edit_empty_object", [patch(2, {})]))
    add(case("ws_edit_same_name_other_field", [patch(2, {"name": "acme", "description": "still acme"})]))
    add(case("ws_edit_same_name_only", [patch(2, {"name": "acme"})]))
    add(case("ws_edit_name_taken", [patch(2, {"name": "personal"})]))
    add(case("ws_edit_name_orphan", [patch(2, {"name": "orphan"})]))
    add(case("ws_edit_name_bad", [patch(2, {"name": "Bad Name"})]))
    add(case("ws_edit_name_null", [patch(2, {"name": None})]))
    add(case("ws_edit_orphan_403", [patch(1, {"description": "x"}), patch(1, {"name": "z"}), patch(1, {"name": "orphan"}), patch(1, {"bogus": 1})]))
    add(case("ws_edit_unknown_404", [patch(999, {"description": "x"}), patch(0, {}), patch("abc", {}), patch("unassigned", {}), patch("2x", {}), patch("-1", {})]))
    add(case("ws_edit_unknown_beats_bad_fields", [patch(999, {"bogus": 1, "color": "red"})]))
    add(case("ws_edit_unknown_field", [patch(2, {"id": 5}), patch(2, {"bogus": 1}), patch(2, {"description": "ok", "bogus": 1})]))
    add(case("ws_edit_error_order_icon_then_color", [patch(2, {"icon": "i" * 40, "color": "red"})]))
    add(case("ws_edit_error_order_color_then_icon", [patch(2, {"color": "red", "icon": "i" * 40})]))
    add(case("ws_edit_error_order_email_then_description", [patch(2, {"email": "e" * 300, "description": "d" * 600})]))
    add(case("ws_edit_error_order_description_then_email", [patch(2, {"description": "d" * 600, "email": "e" * 300})]))
    add(case("ws_edit_error_order_text_before_name", [patch(2, {"name": "BAD", "color": "red"})]))
    add(case("ws_edit_error_order_name_before_repos", [patch(2, {"repos": ["rel"], "name": "BAD"})]))
    add(case("ws_edit_error_order_repos_after_text", [patch(2, {"repos": ["rel"], "icon": "i" * 40})]))
    add(case("ws_edit_error_order_dup_keys", [patch(2, raw='{"color":"red","icon":"' + "i" * 40 + '","color":"#112233"}')]))
    add(case("ws_edit_error_order_nested_before", [patch(2, raw='{"x":{"color":1},"icon":"' + "i" * 40 + '"}')]))
    for label, value in {"yes": "yes", "null": None, "one": 1, "list": [True], "zero": 0, "string_true": "true"}.items():
        add(case(f"ws_edit_confirm_{label}", [patch(2, {"description": "x", "confirm": value})]))
    add(case("ws_edit_confirm_false_true", [patch(2, {"description": "a", "confirm": False}), patch(2, {"description": "b", "confirm": True})]))
    add(case("ws_edit_unknown_id_beats_bad_confirm", [patch(999, {"confirm": "yes"})]))
    add(case("ws_edit_body_not_json", [patch(2, raw="{nope"), patch(2, raw=""), patch(2, raw="[1]"), patch(2, raw='{"description":"x"}', type="text/plain")]))
    add(case("ws_edit_body_beats_id", [patch("abc", raw="{nope"), patch("abc", raw='{"a":1}', type="text/plain")]))
    add(case("ws_edit_repos_not_list", [patch(2, {"repos": "/x"}), patch(2, {"repos": None}), patch(2, {"repos": [1]})]))
    add(case("ws_edit_text_types", [patch(2, {"description": 5}), patch(2, {"email": []}), patch(2, {"icon": {}})]))
    # renaming a workspace that holds connectors: the plan first
    held = dict(setup=CONNECTORS)
    add(case("ws_rename_connectors_plan", [patch(2, {"name": "firm"})], **held))
    add(case("ws_rename_connectors_plan_confirm_false", [patch(2, {"name": "firm", "confirm": False})], **held))
    add(case("ws_rename_connectors_unpicked_applies", [patch(2, {"name": "firm", "confirm": True})], setup=CONNECTORS + UNPICK))
    add(case("ws_rename_connectors_unpicked_plan", [patch(2, {"name": "firm"})], setup=CONNECTORS + UNPICK))
    add(case("ws_rename_connectors_custom_alias_stays", [patch(2, {"name": "firm", "confirm": True})],
             setup=CONNECTORS + UNPICK + ["UPDATE connectors SET alias = 'my-chat', alias_custom = 1 WHERE id = 'slack-acme'"]))
    add(case("ws_rename_connectors_alias_clash", [patch(2, {"name": "firm"})],
             setup=CONNECTORS + UNPICK + ["INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('other-slack', 'firm-slack', 1, 'slack', 3)"]))
    add(case("ws_rename_connectors_alias_too_long", [patch(2, {"name": "n" * 40})],
             setup=UNPICK + ["INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('long-one', 'acme-" + "p" * 30 + "', 0, '" + "p" * 30 + "', 2)"]))
    add(case("ws_rename_connectors_missing_folder_skipped", [patch(2, {"name": "firm"}), patch(2, {"name": "firm", "confirm": True})],
             setup=CONNECTORS + ["UPDATE agent_accounts SET folder = '@W@/gone/.claude-work', config_dir_env = '@W@/gone/.claude-work' WHERE id = 2"]))
    add(case("ws_rename_connectors_registered_plan", [patch(2, {"name": "firm"})], setup=CONNECTORS + REGISTERED))
    add(case("ws_rename_connectors_other_fields_apply", [patch(2, {"description": "kept", "confirm": False})], **held))
    add(case("ws_rename_connectors_bad_name_before_plan", [patch(2, {"name": "BAD"}), patch(2, {"name": "personal"})], **held))
    add(case("ws_rename_connectors_orphan_provider_clash_ok", [patch(2, {"name": "firm", "confirm": True})],
             setup=CONNECTORS + UNPICK + ["INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES ('orph-slack', 'firm-slack', 1, 'slack', 1)"]))

    # ---- DELETE /api/workspaces/{id}
    add(case("ws_delete_plan_no_body", [delete(3)]))
    add(case("ws_delete_plan_empty_object", [delete(3, {})]))
    add(case("ws_delete_plan_confirm_false", [delete(3, {"confirm": False})]))
    add(case("ws_delete_confirm", [delete(3, {"confirm": True})]))
    add(case("ws_delete_confirm_with_picks", [delete(2, {"confirm": True})]))
    add(case("ws_delete_picked_plan_counts", [delete(2)]))
    add(case("ws_delete_blank_body", [delete(3, raw="  \n\t\x0b\x0c ")]))
    add(case("ws_delete_double", [delete(3, {"confirm": True}), delete(3, {"confirm": True})]))
    add(case("ws_delete_extra_fields_ignored", [delete(3, {"confirm": True, "anything": [1, 2]})]))
    add(case("ws_delete_confirm_types", [delete(3, {"confirm": "yes"}), delete(3, {"confirm": None}), delete(3, {"confirm": 1})]))
    add(case("ws_delete_body_errors", [delete(3, raw="{nope"), delete(3, raw="[]"), delete(3, raw='{"confirm":true}', type="text/plain"), delete(3, raw="x", type=None)]))
    add(case("ws_delete_orphan_403", [delete(1), delete(1, {"confirm": True})]))
    add(case("ws_delete_unknown_404", [delete(999), delete("abc"), delete("unassigned"), delete(0, {"confirm": True})]))
    add(case("ws_delete_body_beats_id", [delete("abc", raw="{nope")]))
    add(case("ws_delete_connectors_unpicked_plan", [delete(2)], setup=CONNECTORS + UNPICK))
    add(case("ws_delete_connectors_unpicked_confirm", [delete(2, {"confirm": True})], setup=CONNECTORS + UNPICK))
    add(case("ws_delete_connectors_picked_missing_folder", [delete(2), delete(2, {"confirm": True})],
             setup=CONNECTORS + ["UPDATE agent_accounts SET folder = '@W@/gone/.claude-work', config_dir_env = '@W@/gone/.claude-work' WHERE id = 2"]))
    add(case("ws_delete_connectors_registered_plan", [delete(2)], setup=CONNECTORS + REGISTERED))
    add(case("ws_delete_then_recreate_same_name", [delete(3, {"confirm": True}), post(ws, {"name": "personal"})]))
    add(case("ws_delete_connectors_move_keeps_alias_in_orphan", [delete(2, {"confirm": True}), post(ws, {"name": "acme"})], setup=CONNECTORS + UNPICK))

    # ---- accounts: the agent folder detection runs before each write (Python's `accounts.detect`)
    base = "/api/accounts"
    cfg = {"dirs": ["cfg/projects", "cfg-settings", "cfg-codex", "cfg-empty", "nowhere/cfg/projects", "link-target/projects", "cfg trailing /projects"],
           "files": {"cfg-settings/settings.json": "{}", "cfg-codex/config.toml": "", "a-file": "x"},
           "symlinks": {"link-to-cfg": "@W@/link-target"}}
    for label, path in {
        "projects": "@W@/cfg", "settings": "@W@/cfg-settings", "codex": "@W@/cfg-codex", "empty_dir": "@W@/cfg-empty", "missing": "@W@/nope",
        "file": "@W@/a-file", "tilde": "~/cfg", "symlink": "@W@/link-to-cfg", "dotdot": "@W@/cfg/../cfg-codex", "default_folder": "@W@/agent-root/.claude",
        "work_folder": "@W@/agent-root/.claude-work", "trailing_slash": "@W@/cfg/", "spaces_around": "  @W@/cfg  ", "trailing_space_dir": "@W@/cfg trailing ",
    }.items():
        add(case(f"acct_add_{label}", [post(base, {"path": path})], tree=cfg))
    add(case("acct_add_twice", [post(base, {"path": "@W@/cfg"}), post(base, {"path": "@W@/cfg"})], tree=cfg))
    add(case("acct_add_then_confirm", [post(base, {"path": "@W@/cfg"}), acct("confirm", 3, {"label": "Added"})], tree=cfg))
    for label, value in {"int": 3, "null": None, "empty": "", "blank": "   ", "list": ["@W@/cfg"], "bool": True}.items():
        add(case(f"acct_add_path_{label}", [post(base, {"path": value})], tree=cfg))
    add(case("acct_add_no_path", [post(base, {})], tree=cfg))
    add(case("acct_add_detects_new_folders", [post(base, {"path": "@W@/cfg"})], tree={**cfg, **NEW_FOLDERS, "dirs": cfg["dirs"] + NEW_FOLDERS["dirs"], "files": {**cfg["files"], **NEW_FOLDERS["files"]}}))
    add(case("acct_refused_still_detects", [acct("confirm", 1, {"label": ""})], tree={"dirs": NEW_FOLDERS["dirs"], "files": NEW_FOLDERS["files"], "symlinks": {}}))
    for label, r in {
        "not_json": post(base, raw="{nope"), "empty": post(base, raw=""), "list": post(base, raw="[]"), "null": post(base, raw="null"),
        "confirm_string": post(base, {"path": "@W@/cfg", "confirm": "yes"}), "confirm_null": post(base, {"path": "@W@/cfg", "confirm": None}),
        "confirm_true_ignored": post(base, {"path": "@W@/cfg", "confirm": True}), "text_type": post(base, raw='{"path":"@W@/cfg"}', type="text/plain"),
        "no_type": post(base, raw='{"path":"@W@/cfg"}', type=None), "blank": post(base, raw="  "), "dup_keys": post(base, raw='{"path":"@W@/nope","path":"@W@/cfg"}'),
        "bom": post(base, hexed=(b"\xef\xbb\xbf" + b'{"path":"@W@/cfg"}').hex()), "extra_fields": post(base, {"path": "@W@/cfg", "label": "ignored", "x": [1]}),
    }.items():
        add(case(f"acct_add_body_{label}", [r], tree=cfg))

    two = {"dirs": ["agent-root/.claude-new/projects", "agent-root/.codex"], "files": {"agent-root/.codex/config.toml": ""}, "symlinks": {}}
    add(case("acct_confirm_new", [acct("confirm", 3, {"label": "Claude new"})], tree=two))
    add(case("acct_confirm_codex", [acct("confirm", 4, {"label": "Codex main"})], tree=two))
    add(case("acct_confirm_rename", [acct("confirm", 2, {"label": "Renamed"})]))
    add(case("acct_confirm_default_rename", [acct("confirm", 1, {"label": "Mine"})]))
    add(case("acct_confirm_twice", [acct("confirm", 2, {"label": "One"}), acct("confirm", 2, {"label": "Two"})]))
    add(case("acct_confirm_stripped", [acct("confirm", 2, {"label": "  spaced  "}), acct("confirm", 1, {"label": "\x1f x \x1f"})]))
    add(case("acct_confirm_unicode", [acct("confirm", 2, {"label": "Café ☕ \"work\""})]))
    add(case("acct_confirm_60", [acct("confirm", 2, {"label": "l" * 60})]))
    add(case("acct_confirm_61", [acct("confirm", 2, {"label": "l" * 61})]))
    add(case("acct_confirm_60_after_strip", [acct("confirm", 2, {"label": " " + "l" * 60 + " "})]))
    add(case("acct_confirm_emoji_60", [acct("confirm", 2, {"label": "\U0001f600" * 60}), acct("confirm", 1, {"label": "\U0001f600" * 61})]))
    for label, value in {"empty": "", "blank": "   ", "int": 5, "null": None, "list": ["x"], "bool": True, "tabs": "\t\n"}.items():
        add(case(f"acct_confirm_label_{label}", [acct("confirm", 2, {"label": value})]))
    add(case("acct_confirm_no_label", [acct("confirm", 2, {})]))
    add(case("acct_confirm_unknown", [acct("confirm", 999, {"label": "L"}), acct("confirm", 0, {"label": "L"}), acct("confirm", "abc", {"label": "L"}), acct("confirm", "1x", {"label": "L"})]))
    add(case("acct_confirm_bad_label_beats_unknown_id", [acct("confirm", 999, {"label": ""})]))
    add(case("acct_confirm_body_beats_id", [acct("confirm", "abc", raw="{nope"), acct("confirm", "abc", {"label": "L", "confirm": 1})]))
    add(case("acct_confirm_bad_confirm", [acct("confirm", 2, {"label": "L", "confirm": "yes"}), acct("confirm", 2, {"label": "L", "confirm": None})]))
    add(case("acct_confirm_confirm_true_ok", [acct("confirm", 2, {"label": "L", "confirm": True})]))
    add(case("acct_confirm_ignored_comes_back", [acct("ignore", 4, {}), acct("confirm", 4, {"label": "Back"})], tree=two))
    add(case("acct_confirm_body_errors", [acct("confirm", 2, raw="{nope"), acct("confirm", 2, raw="[]"), acct("confirm", 2, raw='{"label":"x"}', type="text/plain")]))
    add(case("acct_confirm_missing_folder_status", [acct("confirm", 2, {"label": "Gone"})], setup=["UPDATE agent_accounts SET folder = '@W@/gone/.claude-work', config_dir_env = '@W@/gone/.claude-work' WHERE id = 2"]))
    add(case("acct_ignore_codex", [acct("ignore", 4, {})], tree=two))
    add(case("acct_ignore_twice", [acct("ignore", 4, {}), acct("ignore", 4, {})], tree=two))
    add(case("acct_ignore_work_keeps_picks", [acct("ignore", 2, {})]))
    add(case("acct_ignore_default_refused", [acct("ignore", 1, {})]))
    add(case("acct_ignore_detected", [acct("ignore", 3, {})], tree=two))
    add(case("acct_ignore_unknown", [acct("ignore", 999, {}), acct("ignore", "abc", {}), acct("ignore", 0, {})]))
    add(case("acct_ignore_body_errors", [acct("ignore", 2, raw="{nope"), acct("ignore", 2, raw=""), acct("ignore", 2, raw="[]"), acct("ignore", 2, {"confirm": "x"})]))
    add(case("acct_ignore_body_beats_id", [acct("ignore", "abc", raw="{nope")]))
    add(case("acct_ignore_extra_body", [acct("ignore", 2, {"label": "x", "anything": 1})]))
    add(case("acct_ignore_missing_folder", [acct("ignore", 2, {})], setup=["UPDATE agent_accounts SET folder = '@W@/gone/.claude-work', config_dir_env = '@W@/gone/.claude-work' WHERE id = 2"]))

    put = lambda ident, body=None, **kw: acct("workspaces", ident, body, method="PUT", **kw)  # noqa: E731
    add(case("acct_use_set_two", [put(2, {"workspaces": ["acme", "personal"]})]))
    add(case("acct_use_replace", [put(2, {"workspaces": ["personal"]})]))
    add(case("acct_use_clear", [put(2, {"workspaces": []})]))
    add(case("acct_use_duplicates", [put(2, {"workspaces": ["acme", "acme", "personal", "acme"]})]))
    add(case("acct_use_default_account", [put(1, {"workspaces": ["personal"]})]))
    add(case("acct_use_unknown_workspace", [put(2, {"workspaces": ["acme", "nope"]})]))
    add(case("acct_use_orphan", [put(2, {"workspaces": ["orphan"]})]))
    add(case("acct_use_not_list", [put(2, {"workspaces": "acme"}), put(2, {"workspaces": None}), put(2, {}), put(2, {"workspaces": {"acme": 1}})]))
    add(case("acct_use_item_not_text", [put(2, {"workspaces": ["acme", 5]}), put(2, {"workspaces": [None]}), put(2, {"workspaces": [["acme"]]})]))
    add(case("acct_use_unconfirmed_claude", [put(3, {"workspaces": ["acme"]})], tree=two))
    add(case("acct_use_codex", [put(4, {"workspaces": ["acme"]})], tree=two))
    add(case("acct_use_confirmed_codex", [acct("confirm", 4, {"label": "Codex main"}), put(4, {"workspaces": ["acme"]})], tree=two))
    add(case("acct_use_ignored", [acct("ignore", 2, {}), put(2, {"workspaces": ["personal"]})]))
    add(case("acct_use_after_confirm", [acct("confirm", 3, {"label": "New"}), put(3, {"workspaces": ["acme", "personal"]})], tree=two))
    add(case("acct_use_unknown_account", [put(999, {"workspaces": ["acme"]}), put("abc", {"workspaces": ["acme"]}), put(0, {})]))
    add(case("acct_use_unknown_beats_bad_list", [put(999, {"workspaces": 5})]))
    add(case("acct_use_provider_beats_bad_list", [put(4, {"workspaces": 5})], tree=two))
    add(case("acct_use_unknown_ws_before_orphan", [put(2, {"workspaces": ["orphan", "nope"]}), put(2, {"workspaces": ["nope", "orphan"]})]))
    add(case("acct_use_body_errors", [put(2, raw="{nope"), put(2, raw="[]"), put(2, raw='{"workspaces":[]}', type="text/plain"), put(2, {"workspaces": [], "confirm": "x"})]))
    add(case("acct_use_body_beats_id", [put("abc", raw="{nope")]))
    add(case("acct_use_confirm_true_ignored", [put(2, {"workspaces": ["personal"], "confirm": True})]))
    add(case("acct_use_after_workspace_delete", [delete(2, {"confirm": True}), put(2, {"workspaces": ["acme"]}), put(2, {"workspaces": ["personal"]})]))
    add(case("acct_use_ws_renamed", [patch(2, {"name": "company"}), put(2, {"workspaces": ["acme"]}), put(2, {"workspaces": ["company"]})]))

    # ---- methods and paths that are not routes
    status_only = {"compare": "status"}
    add(case("methods", [
        req("GET", "/api/workspaces/2", **status_only), req("POST", "/api/workspaces/2", {}, **status_only), req("PUT", "/api/workspaces", {}, **status_only),
        req("DELETE", "/api/workspaces", {}, **status_only), req("PATCH", "/api/workspaces", {}, **status_only), req("GET", "/api/accounts/1/confirm", **status_only),
        req("PUT", "/api/accounts/1/ignore", {}, **status_only), req("POST", "/api/accounts/1/workspaces", {}, **status_only),
        req("GET", "/api/accounts/1/workspaces", **status_only), req("DELETE", "/api/accounts/1/confirm", **status_only),
        req("PATCH", "/api/accounts", {}, **status_only), req("PUT", "/api/accounts", {}, **status_only), req("DELETE", "/api/accounts", **status_only),
        post("/api/accounts/1", {}, **status_only), post("/api/accounts/1/nothing", {}, **status_only), post("/api/workspaces/2/x", {}, **status_only),
        req("PATCH", "/api/workspaces/unassigned/x", {}, **status_only), req("POST", "/api/workspaces/unassigned", {}, **status_only),
        req("PATCH", "/api/workspaces/unassigned", {"x": 1}), req("DELETE", "/api/workspaces/unassigned"),
    ]))
    return out


def scrub(value, work: str):
    if isinstance(value, str):
        return value.replace(work, "<WORK>")
    if isinstance(value, list):
        return [scrub(v, work) for v in value]
    if isinstance(value, dict):
        return {k: scrub(v, work) for k, v in value.items()}
    return value


def dump(path: Path, work: str) -> dict:
    """Every table: its rows as JSON-able lists, sorted by their compact JSON text (`<WORK>` for the work folder)."""
    conn = sqlite3.connect(path)
    try:
        names = [r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
        out = {}
        for name in names:
            rows = []
            for row in conn.execute(f'SELECT * FROM "{name}"'):
                rows.append(scrub([v.hex() if isinstance(v, bytes) else v for v in row], work))
            out[name] = sorted(rows, key=lambda r: json.dumps(r, ensure_ascii=False, separators=(",", ":")))
        return out
    finally:
        conn.close()


def build_tree(work: Path, tree: dict) -> None:
    sub = lambda text: text.replace("@W@", str(work))  # noqa: E731
    for d in ["agent-root/.claude", "agent-root/.claude-work", "nowhere", *tree["dirs"]]:
        (work / d).mkdir(parents=True, exist_ok=True)
    for rel, content in tree["files"].items():
        target = work / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)
    for rel, link in tree["symlinks"].items():
        (work / rel).symlink_to(sub(link))


PATHS = [
    "/a/b", "//a", "///a/b", "/a//b/", "/a/./b", "/a/../b", "/../..", "/", "//", "/a/../../x", "/a/b/..", "/a/b/.", "/a/b/\\", "  /a/b  ",
    "/é/ü", "/a/ b /c", "/" + "a" * 300, "@W@/link", "@W@/link/x/../y", "@W@/link/..", "@W@/rel", "@W@/rel/z", "@W@/dangle", "@W@/dangle/x",
    "@W@/loop-a", "@W@/loop-a/x", "@W@/file", "@W@/file/x", "@W@/deep/../link", "@W@/chain1", "@W@/dir-link/real", "relative", ".", "", "   ", "a\u0000b", 5,
    # T054: a chain of 30 links that doubles at every hop, a loop of three links, a link to itself
    "@W@/hop0", "@W@/hop0/real", "@W@/tri-a", "@W@/tri-a/x", "@W@/tri-c/x", "@W@/selfie", "@W@/selfie/x",
]


def path_corpus(base: Path) -> dict:
    """`workspaces.normalise_repo` over a folder with links (`@W@` = that folder): the answer or the refusal text."""
    from easy_personal.core import workspaces

    work = base / "paths"
    (work / "real").mkdir(parents=True)
    (work / "file").write_text("")
    (work / "deep").mkdir()
    (work / "link").symlink_to(work / "real")
    (work / "rel").symlink_to("real")
    (work / "dangle").symlink_to(work / "nowhere")
    (work / "loop-a").symlink_to(work / "loop-b")
    (work / "loop-b").symlink_to(work / "loop-a")
    (work / "chain1").symlink_to(work / "link")
    (work / "dir-link").symlink_to(work / "deep" / "..")
    for i in range(30):  # hop0 -> hop1/hop1 -> ... -> hop30 -> `.`: 2**30 expansions without a cache of resolved links
        (work / f"hop{i}").symlink_to(f"hop{i + 1}/hop{i + 1}")
    (work / "hop30").symlink_to(".")
    (work / "tri-a").symlink_to("tri-b")
    (work / "tri-b").symlink_to("tri-c")
    (work / "tri-c").symlink_to("tri-a")
    (work / "selfie").symlink_to("selfie")
    out = []
    for text in PATHS:
        arg = text.replace("@W@", str(work)) if isinstance(text, str) else text
        try:
            result = {"ok": scrub(workspaces.normalise_repo(arg), str(work))}
        except workspaces.WorkspaceError as error:
            result = {"error": str(error)}
        out.append({"input": text, **result})
    return {"cases": out}


def run(out: Path) -> None:
    gen_golden.refuse_real_paths(out)
    done = []
    with tempfile.TemporaryDirectory(prefix="parity-writes-") as tmp:
        base = Path(tmp).resolve()
        gen_golden.isolate(base)
        gen_golden.guard_sqlite()
        from starlette.testclient import TestClient

        from easy_personal.connectors import suggestions, sync, writer
        from easy_personal.surfaces import api

        gen_golden.unplug_machine_state(api, suggestions)
        gen_golden.guard_machine_state()
        real_which = shutil.which
        shutil.which = lambda name, *a, **k: None if name == "claude" else real_which(name, *a, **k)
        assert sync.shutil is shutil and writer  # the plan asks `shutil.which("claude")`: no such program
        for number, spec in enumerate(cases()):
            work = base / f"case-{number}"
            work.mkdir()
            wtext = str(work)
            gen_golden.isolate(work)
            build_tree(work, spec["tree"])
            store = work / "home" / "store.sqlite"
            store.parent.mkdir(exist_ok=True)
            shutil.copy(SOURCE, store)
            conn = sqlite3.connect(store)
            conn.execute("PRAGMA foreign_keys = ON")
            conn.execute("UPDATE agent_accounts SET folder = replace(folder, '/work/rich', ?), config_dir_env = replace(config_dir_env, '/work/rich', ?)", (wtext, wtext))
            for sql in spec["setup"]:
                conn.execute(sql.replace("@W@", wtext))
            conn.commit()
            conn.close()
            before = dump(store, wtext)
            client = TestClient(api.create_app(TOKEN, PORT, work / "absent"), base_url=f"http://127.0.0.1:{PORT}", raise_server_exceptions=False)
            answers = []
            for r in spec["requests"]:
                headers = {api.TOKEN_HEADER: TOKEN}
                if r["type"] is not None:
                    headers["content-type"] = r["type"]
                body = bytes.fromhex(r["body_hex"]).replace(b"@W@", wtext.encode())
                path = r["path"].replace("@W@", wtext)
                res = client.request(r["method"], path, headers=headers, content=body)
                kind = res.headers.get("content-type", "")
                answers.append({"status": res.status_code, "json": scrub(res.json(), wtext) if kind.startswith("application/json") else None})
            after = dump(store, wtext)
            done.append({
                "name": spec["name"], "setup": spec["setup"], "tree": spec["tree"],
                "requests": [{k: v for k, v in r.items()} for r in spec["requests"]],
                "answers": answers,
                "changed": {t: rows for t, rows in after.items() if rows != before[t]},
                "tables": sorted(after),
            })
        shutil.which = real_which
        paths = path_corpus(base)
    crashed = [(c["name"], a["status"]) for c in done for a in c["answers"] if a["status"] >= 500]
    if crashed:
        sys.exit(f"Python answered 5xx in: {crashed} (such a case belongs in the README list, not in the oracle)")
    out.mkdir(parents=True, exist_ok=True)
    manifest = {
        "generator": "rust/parity/gen_writes.py",
        "source_store": "rust/parity/store-rich/store-v11.sqlite",
        "token_header": "x-dashboard-token",
        "cases": done,
    }
    (out / "cases.json").write_text(json.dumps(manifest, indent=1, sort_keys=True, ensure_ascii=False) + "\n")
    (out / "paths.json").write_text(json.dumps(paths, indent=1, sort_keys=True, ensure_ascii=False) + "\n")
    requests = sum(len(c["answers"]) for c in done)
    changed = sum(1 for c in done if c["changed"])
    print(f"{len(done)} cases, {requests} requests ({changed} cases change the store) -> {out}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=PARITY / "golden-writes")
    run(parser.parse_args().out.resolve())
