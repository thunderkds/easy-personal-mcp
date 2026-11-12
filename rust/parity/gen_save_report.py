#!/usr/bin/env python3
"""Oracle for the Rust `save_report` tool (T047): Python's `reports.save` over copies of `store-rich`, and the secret corpus.

    rust/parity/.venv/bin/python rust/parity/gen_save_report.py            # writes rust/parity/golden-save-report/
    rust/parity/.venv/bin/python rust/parity/gen_save_report.py --out DIR  # same, elsewhere (a re-run must diff empty)

Two files, both sorted-key JSON:

* `secret_corpus.json`: strings and Python's `SECRET.search` answer for each (`core/secretlike.py`, read-only).
* `cases.json`: per case the arguments and what Python did on a **temp copy** of `store-rich/store-v11.sqlite` at the
  frozen clock (`gen_golden.FROZEN_NOW`): either `ok` (`id`, `saved_utc`, the whole new row, the `get_report` answer for it)
  or `error` (the refusal text), plus the row count after the call. `server_version` is Python's package version: the one
  value a comparison normalises. The accounts of the copy are pointed at temp folders (as `reports_parity.rs` does), because
  the rich store names `/work/rich/...` folders that only Python's golden run had.

Python is only read; the real store and the owner's folders are never opened (`gen_golden.refuse_real_paths`).
"""

import argparse
import json
import shutil
import sqlite3
import sys
import tempfile
from pathlib import Path

PARITY = Path(__file__).resolve().parent
sys.path.insert(0, str(PARITY))
import gen_golden  # noqa: E402

SOURCE = PARITY / "store-rich" / "store-v11.sqlite"
FROZEN = "2026-01-20T00:00:00+00:00"

CORPUS = [
    # keys and tokens that must match
    "sk-abc", "sk-ant-api03-abcdef", "my key is sk-1 ok", "(sk-Z)", "xsk-abc", "_sk-abc", "9sk-abc", "ésk-abc", "日sk-abc",
    "ghp_abc", "gho_A", "ghu_9", "ghs_x", "ghr_x", "ghx_abc", "ghp_", "ghp-abc", "github_pat_", "github_pat", "xgithub_pat_11",
    "xoxb-1234", "xoxb1234", "xoxp_abcd", "xox-1234", "xox_abcd5", "xoxa-123", "xoxb-123", "xox1234", "xoxb", "xoxbcdef",
    "xoxbc-12345", "xoxb--1234", "axoxb-1234",
    "AKIAABCD", "ASIA1234", "AKIAABC", "AKIA abcd", "xAKIAABCDE", "akiaabcdef", "ASIAZZZZZZZZZZZZ",
    "-----BEGIN", "x-----BEGIN PRIVATE KEY-----", "-----BEGINNING", "----BEGIN", "-----begin", "text -----BEGIN RSA",
    # long runs
    "a" * 32, "a" * 31, "A1" * 16, "A1" * 15 + "A", "a" * 31 + "-" + "a" * 31, "a" * 16 + " " + "b" * 16, "a" * 31 + "é" + "a" * 31,
    "0123456789" * 4, "/" + "a" * 40, "b" * 32 + "-tail", "é" * 40, "日本語" * 20, "a" * 31 + "_" + "a",
    "550e8400-e29b-41d4-a716-446655440000", "/home/user/projects/very-long-folder-name/sub/dir/file.py",
    # plain text, near misses, unicode, empty
    "", " ", "ordinary words about cache hits", "ask-me later", "task-1234", "disk-usage", "sk", "sk-", "sk--", "SK-abc",
    "ghp", "gh_abc", "ＡＫＩＡABCD", "Ａ" * 40, "٣" * 40, "ⅷ" * 40, "sk-é", "sk-٣", "emoji 🔑 sk-abc", "sk-́abc",
    "tab\tsk-abc", "line\nsk-abc", "line\r\n-----BEGIN", " sk-abc", " ghp_abc",
    # T052: Other_Alphabetic characters that are not letters or digits are not `\w` in Python, so `\b` holds before `sk-`
    "Ⓐsk-abc", "ⓩsk-abc", "ⓐghp_abc", "ͅsk-abc", "िsk-abc", "ःxoxb-1234", "̀sk-abc",
    "\U0001f130sk-abc", "\U0001f150sk-abc", "\U0001f170sk-abc", "Ⅰsk-abc", "²sk-abc", "½sk-abc", "①sk-abc",
    "xͅsk-abc", "ⒶAKIAABCD", "Ⓐgithub_pat_", "ͅ" + "a" * 32, "Ⓐ-----BEGIN",
]


def row_of(conn: sqlite3.Connection, report_id: int) -> dict:
    cur = conn.execute("SELECT * FROM investigate_reports WHERE id = ?", (report_id,))
    names = [d[0] for d in cur.description]
    return dict(zip(names, cur.fetchone(), strict=True))


def cases() -> list[dict]:
    def one(name, summary="summary text", fixes=None, workspace=None, account=None):
        args = {"summary": summary, "fixes": [] if fixes is None else fixes}
        if workspace is not None:
            args["workspace"] = workspace
        if account is not None:
            args["account"] = account
        return {"name": name, "args": args}

    cite = [{"text": "Trim the prompt prefix.", "tool": "usage_breakdown", "number": 42}]
    return [
        one("plain"),
        one("scoped_workspace", workspace="acme"),
        one("scoped_account_label", account="Claude work"),
        one("scoped_account_id", account="2"),
        one("scoped_both", workspace="acme", account="Claude work"),
        one("scope_empty_strings", workspace="", account=""),
        one("fixes_with_citations", fixes=cite + [{"text": "Use a smaller model.", "tool": "warnings", "number": "3.5 %"}]),
        one("fixes_number_float", fixes=[{"text": "a", "number": 3.5}, {"text": "b", "number": 1e-07}, {"text": "c", "number": 1e16}, {"text": "d", "number": 0.0001}, {"text": "e", "number": 5.0}]),
        one("fixes_number_big_int", fixes=[{"text": "a", "number": 12345678901234567}, {"text": "b", "number": -7}]),
        one("fixes_empty_optionals", fixes=[{"text": "a", "tool": "", "number": ""}, {"text": "b", "tool": None, "number": None}]),
        one("five_fixes", fixes=[{"text": f"fix {n}", "tool": "warnings", "number": n} for n in range(5)]),
        one("no_fixes"),
        one("unicode_text", summary="résumé: é中文 \U0001f600 line\nsecond", fixes=[{"text": "naïve \U0001f680", "tool": "té"}]),
        one("summary_2000", summary="x " * 1000),
        one("summary_2000_multibyte", summary="é" * 2000),
        one("summary_2000_emoji", summary="\U0001f600" * 2000),
        one("summary_2001", summary="x " * 1000 + "y"),
        one("summary_2001_multibyte", summary="é" * 2001),
        one("summary_empty", summary=""),
        one("summary_blank_unicode", summary=" \t 　\x1c\x1f"),
        one("summary_not_text", summary=5),
        one("summary_secret", summary="the key is sk-ant-abcdef"),
        one("summary_secret_long_run", summary="token " + "a1" * 20),
        one("summary_secret_pem", summary="-----BEGIN PRIVATE KEY-----"),
        one("fixes_not_list", fixes="nope"),
        one("fixes_dict", fixes={"text": "a"}),
        one("fixes_six", fixes=[{"text": "a"}] * 6),
        one("fix_not_object", fixes=[{"text": "ok"}, "plain string"]),
        one("fix_unknown_key", fixes=[{"text": "ok", "extra": 1}]),
        one("fix_missing_text", fixes=[{"tool": "warnings"}]),
        one("fix_text_not_text", fixes=[{"text": 5}]),
        one("fix_text_empty", fixes=[{"text": "  "}]),
        one("fix_text_300", fixes=[{"text": "y " * 150}]),
        one("fix_text_301", fixes=[{"text": "y " * 150 + "y"}]),
        one("fix_text_secret", fixes=[{"text": "use ghp_abc123"}]),
        one("fix_tool_100", fixes=[{"text": "a", "tool": "t " * 50}]),
        one("fix_tool_101", fixes=[{"text": "a", "tool": "t " * 50 + "x"}]),
        one("fix_tool_not_text", fixes=[{"text": "a", "tool": 7}]),
        one("fix_tool_secret", fixes=[{"text": "a", "tool": "AKIAABCDEFGH"}]),
        one("fix_number_bool", fixes=[{"text": "a", "number": True}]),
        one("fix_number_list", fixes=[{"text": "a", "number": [1]}]),
        one("fix_number_object", fixes=[{"text": "a", "number": {"v": 1}}]),
        one("fix_number_text_101", fixes=[{"text": "a", "number": "1 " * 50 + "x"}]),
        one("fix_number_text_secret", fixes=[{"text": "a", "number": "sk-abc"}]),
        one("second_fix_fails", fixes=[{"text": "ok"}, {"text": ""}]),
        one("unknown_workspace", workspace="nope"),
        one("unknown_account", account="nope"),
        one("unknown_scope_and_bad_text", summary="", workspace="nope"),
        one("secret_then_unknown_scope", summary="sk-abc", workspace="nope"),
        # T052: DEL (0x7f) is escaped as \u007f by `json.dumps(ensure_ascii=True)`; the stored bytes must equal Python's
        one("del_in_summary", summary="before\x7fafter"),
        one("del_in_fix", fixes=[{"text": "trim\x7fthe prefix", "tool": "usage\x7fbreakdown", "number": "1\x7f2"}]),
    ]


def point_accounts_at(path: Path, base: Path) -> None:
    with sqlite3.connect(path) as conn:
        for (id_,) in conn.execute("SELECT id FROM agent_accounts").fetchall():
            folder = base / f"account-{id_}"
            folder.mkdir(parents=True, exist_ok=True)
            conn.execute("UPDATE agent_accounts SET folder = ? WHERE id = ?", (str(folder), id_))


def run(out: Path) -> None:
    from datetime import datetime

    gen_golden.refuse_real_paths(out)
    with tempfile.TemporaryDirectory(prefix="parity-save-") as tmp:
        work = Path(tmp).resolve()
        gen_golden.isolate(work)
        gen_golden.guard_sqlite()
        gen_golden.freeze_clock()
        from easy_personal.connectors import suggestions
        from easy_personal.core import reports, store
        from easy_personal.core.secretlike import SECRET
        from easy_personal.surfaces import api

        gen_golden.unplug_machine_state(api, suggestions)
        gen_golden.guard_machine_state()
        now = datetime.fromisoformat(FROZEN)
        done = []
        for number, case in enumerate(cases()):
            path = work / f"case-{number}.sqlite"
            shutil.copy(SOURCE, path)
            point_accounts_at(path, work / f"accounts-{number}")
            args = case["args"]
            with store.opened(path) as conn:
                before = conn.execute("SELECT count(*) FROM investigate_reports").fetchone()[0]
                try:
                    saved = reports.save(
                        conn, args["summary"], args["fixes"], args.get("workspace"), args.get("account"), now
                    )
                except ValueError as error:
                    result = {"error": str(error)}
                else:
                    result = {
                        "ok": saved,
                        "row": row_of(conn, saved["id"]),
                        "report": reports.get_report(conn, saved["id"], now),
                    }
                result["rows_before"] = before
                result["rows_after"] = conn.execute("SELECT count(*) FROM investigate_reports").fetchone()[0]
            done.append({**case, "python": result})
        corpus = [{"text": text, "secret": SECRET.search(text) is not None} for text in CORPUS]
    out.mkdir(parents=True, exist_ok=True)
    manifest = {
        "generator": "rust/parity/gen_save_report.py",
        "frozen_now": FROZEN,
        "source_store": "rust/parity/store-rich/store-v11.sqlite",
        "normalised": ["server_version"],
        "cases": done,
    }
    (out / "cases.json").write_text(json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
    (out / "secret_corpus.json").write_text(json.dumps(corpus, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
    errors = sum("error" in c["python"] for c in done)
    print(f"{len(done)} cases ({len(done) - errors} saved, {errors} refused), {len(corpus)} corpus strings -> {out}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=PARITY / "golden-save-report")
    run(parser.parse_args().out.resolve())
