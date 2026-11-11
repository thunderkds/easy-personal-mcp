#!/usr/bin/env python3
"""Accounts oracle (T041 / RS-003): freeze what the Python app answers for `GET /api/accounts` and `GET /api/sync`
over a hand-made store, under three time zones, as golden JSON in `rust/parity/golden-accounts/`.

    rust/parity/.venv/bin/python rust/parity/gen_accounts.py           # (re)write rust/parity/golden-accounts/
    rust/parity/.venv/bin/python rust/parity/gen_accounts.py --check   # regenerate in a temp folder and compare: exit 0 equal

The store (`store.sqlite`, schema v11) holds `@W@` where a work folder goes; `tree.json` lists the folders and files
the accounts need under it. Python runs over a copy whose `@W@` is a throw-away work folder, with `EASY_PERSONAL_AGENT_ROOT`
inside it, the clock frozen at 2026-01-20T00:00:00Z, and `shutil.which` answering "no `claude`" (the Rust port never
starts a process). In every golden the work folder is the token `<WORK>`. Python's `detect` runs inside the handlers: the
generator fails if it inserts a row, so each golden is what Python answers without any insertion.

Not in the oracle, because Python crashes there (500): an `account` of 20 digits (sqlite3 OverflowError) and a non-ASCII
digit such as `²` (`int()` ValueError). The Rust port answers 404 and 400; `rust/README.md` lists both.
"""

import argparse
import json
import os
import shutil
import sqlite3
import sys
import tempfile
import time
from datetime import UTC, datetime
from pathlib import Path

sys.dont_write_bytecode = True
PARITY = Path(__file__).resolve().parent
REPO = PARITY.parents[1]
sys.path.insert(0, str(REPO / "src"))
OUT = PARITY / "golden-accounts"
FROZEN_NOW = "2026-01-20T00:00:00+00:00"
ZONES = ["UTC", "Pacific/Kiritimati", "Pacific/Pago_Pago"]
NOW_EPOCH = datetime.fromisoformat(FROZEN_NOW).timestamp()

LIMITS = {  # five_hour not yet reset (float epoch), seven_day reset an hour ago (int percentage)
    "five_hour": {"used_percentage": 42.5, "resets_at": NOW_EPOCH + 3600.5},
    "seven_day": {"used_percentage": 10, "resets_at": NOW_EPOCH - 3600},
    "written_at": NOW_EPOCH - 60,
    "secret_token": "must-not-appear",
}
TREE = {
    "dirs": ["agent-root/.claude", "agent-root/.claude-work", "agent-root/.claude-old", "agent-root/.claude-new",
             "agent-root/.codex"],
    "files": {  # path under the work folder -> file content
        "agent-root/.claude.json": json.dumps({"oauthAccount": {"emailAddress": "default@example.invalid",
                                                                "billingType": "stripe_subscription",
                                                                "organizationName": "Default Org", "accessToken": "x"}}),
        "agent-root/.claude-work/.claude.json": json.dumps({"oauthAccount": {"emailAddress": "work@example.invalid",
                                                                             "billingType": "api", "organizationName": "Acme ☕"}}),
        "agent-root/.claude-work/easy-personal-limits.json": json.dumps(LIMITS),
        "agent-root/.claude-old/.claude.json": "{ not json",
        "agent-root/.claude-new/projects/.keep": "",
        "agent-root/.codex/config.toml": "",
    },
}


def build_store(path: Path) -> None:
    from easy_personal.core import store

    conn = store.connect(path)
    with conn:
        conn.executemany("INSERT INTO agent_accounts (id, provider, folder, label, status, config_dir_env) VALUES (?, ?, ?, ?, ?, ?)", [
            (1, "claude", "@W@/agent-root/.claude", "Claude default", "confirmed", None),
            (2, "claude", "@W@/agent-root/.claude-work", 'Café ☕ "work"', "confirmed", "@W@/agent-root/.claude-work"),
            (3, "claude", "@W@/gone/.claude-gone", "Gone", "confirmed", "@W@/gone/.claude-gone"),
            (4, "claude", "@W@/agent-root/.claude-old", "Old\nname", "confirmed", "@W@/agent-root/.claude-old"),
            (5, "codex", "@W@/agent-root/.codex", "Codex main", "confirmed", None),
            (6, "claude", "@W@/agent-root/.claude-new", None, "detected", "@W@/agent-root/.claude-new"),
        ])  # fmt: skip
        conn.execute("INSERT INTO workspaces (id, name, is_orphan) VALUES (1, 'orphan', 1) ON CONFLICT DO NOTHING")
        conn.executemany("INSERT INTO workspaces (id, name, is_orphan) VALUES (?, ?, 0)", [(2, "acme"), (3, "beta")])
        conn.executemany("INSERT INTO connectors (id, alias, provider, workspace_id) VALUES (?, ?, ?, ?)", [
            ("slack-acme", "acme-slack", "slack", 2), ("gmail-acme", "acme-gmail", "gmail", 2),
            ("gone-conn", "gone-alias", "discord", 3), ("slack-old", "old-slack", "slack", 3),
        ])  # fmt: skip
        conn.executemany("INSERT INTO account_workspaces VALUES (?, ?)", [(2, 2), (4, 2)])  # 4 picked while confirmed
        conn.executemany("INSERT INTO account_registrations VALUES (?, ?, ?)", [
            (2, "acme-slack", "slack-acme"), (2, "stale-one", "gone-conn"), (4, "old-slack", "slack-old"),
        ])  # fmt: skip
        conn.execute("UPDATE agent_accounts SET status = 'ignored' WHERE id = 4")  # an ignored account keeps its picks
        rows = [  # (account, ts, model, message id, tokens in/out/cache write/cache read, abstained)
            (1, "2026-01-18T10:00:00.000000Z", "model-a", "m1", (100, 50, 0, 25), 0),
            (1, "2026-01-13T12:00:00.000000Z", "model-a", "m0", (3, 0, 0, 0), 0),  # inside 7 days, outside 6
            (None, "2026-01-19T09:00:00.000000Z", "model-null-account", "m2", (1000, 500, 0, 0), 0),
            (2, "2026-01-19T23:30:00.000000Z", "model-work", "w1", (10, 20, 30, 40), 0),
            (2, "2026-01-19T23:31:00.000000Z", "model-work", "w1", (11, 22, 33, 44), 0),  # same message: the larger figures
            (2, "2026-01-12T23:59:00.000000Z", "model-old", "w2", (999, 999, 0, 0), 0),  # starts before the 7-day window
            (2, "2026-01-13T00:30:00.000000Z", "model-old", "w2", (999, 999, 0, 0), 0),
            (2, "2026-01-19T20:00:00.000000Z", "model-work-2", "w3", (1, 1, 1, 1), 0),
            (2, "2026-01-19T22:00:00.000000Z", "model-abstained", "w4", (5, 5, 5, 5), 1),  # abstained: not counted
            (2, "2026-01-19T22:30:00.000000Z", None, None, (7, 7, 7, 7), 0),  # no model, no message id
        ]
        for n, (acct, ts, model, mid, t, abstained) in enumerate(rows):
            conn.execute(
                "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type, ts_utc, "
                "session_id, model, message_id, input_tokens, output_tokens, cache_creation_input_tokens, "
                "cache_read_input_tokens, agent_account_id) VALUES ('m', 'claude_code', ?, 1, ?, ?, 'assistant', ?, 's', ?, ?, ?, ?, ?, ?, ?)",
                (f"/p/{n}.jsonl", n, abstained, ts, model, mid, *t, acct),
            )  # fmt: skip
    conn.close()


def make_tree(work: Path) -> None:
    for d in TREE["dirs"]:
        (work / d).mkdir(parents=True, exist_ok=True)
    for rel, content in TREE["files"].items():
        (work / rel).parent.mkdir(parents=True, exist_ok=True)
        (work / rel).write_text(content)


CASES = [  # (name, route, params, expected status when Python answers)
    ("accounts__by_account", "/api/accounts", {}),
    ("accounts__by_provider", "/api/accounts", {"by": "provider"}),
    ("accounts__bad_by", "/api/accounts", {"by": "bad"}),
    ("accounts__empty_by", "/api/accounts", {"by": ""}),
    ("sync__all", "/api/sync", {}),
    ("sync__empty_account", "/api/sync", {"account": ""}),
    ("sync__account_2", "/api/sync", {"account": "2"}),
    ("sync__account_02", "/api/sync", {"account": "02"}),
    ("sync__account_3_missing_folder", "/api/sync", {"account": "3"}),
    ("sync__account_4_ignored_with_picks", "/api/sync", {"account": "4"}),
    ("sync__account_5_codex", "/api/sync", {"account": "5"}),
    ("sync__account_6_detected", "/api/sync", {"account": "6"}),
    ("sync__account_unknown", "/api/sync", {"account": "99"}),
    ("sync__account_zero", "/api/sync", {"account": "0"}),
    ("sync__account_word", "/api/sync", {"account": "abc"}),
    ("sync__account_negative", "/api/sync", {"account": "-1"}),
    ("sync__account_float", "/api/sync", {"account": "1.5"}),
    ("sync__account_true", "/api/sync", {"account": "true"}),
    ("sync__account_space", "/api/sync", {"account": " 2"}),
]


def answer(conn, route: str, params: dict) -> tuple[int, dict]:
    """What the Python handlers return: status and JSON body (`api.sync_plan` and the `/api/accounts` lambda)."""
    from easy_personal.connectors import sync
    from easy_personal.core import accounts
    from easy_personal.surfaces import api

    try:
        if route == "/api/accounts":
            accounts.detect(conn)  # what Python's handler does first, through `grouped`
            return 200, accounts.grouped(conn, params.get("by", "account"))
        account = api._sync_account(params.get("account"))
        accounts.detect(conn)
        return 200, {"steps": [s.as_dict() for s in sync.plan_all(conn, account)]}
    except accounts.NotFound as exc:
        return 404, {"error": str(exc)}
    except accounts.AccountError as exc:
        return 400, {"error": str(exc)}


def freeze_clock() -> None:
    now = datetime.fromisoformat(FROZEN_NOW).astimezone(UTC)

    class Frozen(datetime):
        @classmethod
        def now(cls, tz=None):
            return now.astimezone(tz) if tz else now.replace(tzinfo=None)

    from easy_personal.core import accounts

    accounts.datetime = Frozen


def generate(out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="easy-personal-accounts-oracle-")).resolve()
    try:
        saved = out / "store.sqlite"
        saved.unlink(missing_ok=True)
        build_store(saved)
        live = work / "store.sqlite"
        shutil.copy(saved, live)
        make_tree(work)
        conn = sqlite3.connect(live)
        with conn:
            conn.execute("UPDATE agent_accounts SET folder = replace(folder, '@W@', ?), "
                         "config_dir_env = replace(config_dir_env, '@W@', ?)", (str(work), str(work)))  # fmt: skip
        conn.close()
        os.environ["EASY_PERSONAL_AGENT_ROOT"] = str(work / "agent-root")
        os.environ["HOME"] = str(work / "nowhere")  # nothing here can reach a real home
        from easy_personal.connectors import sync
        from easy_personal.core import store

        sync.shutil.which = lambda *_: None  # no `claude` on PATH: the state the Rust port answers as
        freeze_clock()
        manifest = {"generator": "rust/parity/gen_accounts.py", "frozen_now": FROZEN_NOW, "zones": {}, "tree": TREE,
                    "note": "store.sqlite holds @W@ for the work folder; goldens hold <WORK>"}  # fmt: skip
        for zone in ZONES:
            os.environ["TZ"] = zone
            time.tzset()
            entries = []
            folder = out / zone.replace("/", "_")
            folder.mkdir(exist_ok=True)
            with store.opened(live) as conn:
                for name, route, params in CASES:
                    before = conn.execute("SELECT COUNT(*) FROM agent_accounts").fetchone()[0]
                    status, body = answer(conn, route, params)
                    after = conn.execute("SELECT COUNT(*) FROM agent_accounts").fetchone()[0]
                    if before != after:
                        raise SystemExit(f"{name}: Python's detect inserted a row; the oracle store must already hold it")
                    text = json.dumps(body, sort_keys=True, indent=2, ensure_ascii=False).replace(str(work), "<WORK>")
                    (folder / f"api__{name}.json").write_text(text + "\n")
                    entries.append({"file": f"api__{name}.json", "route": route, "params": params, "status": status})
            manifest["zones"][zone] = {"dir": folder.name, "entries": entries}
        (out / "manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
        leaked = [p for p in out.rglob("*") if p.is_file() and p.name not in ("store.sqlite", "manifest.json")
                  and (str(work) in p.read_text() or "must-not-appear" in p.read_text() or "/home/" in p.read_text())]  # fmt: skip
        if leaked:
            raise SystemExit(f"a golden holds the work folder, a path of the real home or a planted secret: {leaked}")
    finally:
        shutil.rmtree(work, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if not args.check:
        generate(OUT)
        print(f"wrote {OUT}")
        return 0
    with tempfile.TemporaryDirectory() as tmp:
        fresh = Path(tmp) / "golden-accounts"
        generate(fresh)
        names = sorted(p.relative_to(fresh) for p in fresh.rglob("*") if p.is_file() and p.name != "store.sqlite")
        stored = sorted(p.relative_to(OUT) for p in OUT.rglob("*") if p.is_file() and p.name != "store.sqlite")
        bad = [str(n) for n in names if n not in stored or (fresh / n).read_bytes() != (OUT / n).read_bytes()]
        if bad or names != stored:
            print("different:", bad or "file lists differ")
            return 1
    print("golden-accounts is equal to a fresh run")
    return 0


if __name__ == "__main__":
    sys.exit(main())
