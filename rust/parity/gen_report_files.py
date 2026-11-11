"""T045 oracle: what Python answers for the report-folder routes over an attack tree.

Builds the tree of `TREE` in a temp folder, runs Python's own `kit.report_files` / `kit.report_file` (the functions behind
`GET /api/kit/report-files` and `GET /api/kit/report-file`, with `api._report_args` for the query) and writes
`rust/parity/golden-report-files/{tree.json,cases.json}`. The Rust test (`crates/core/tests/report_files_parity.rs`) builds
the same tree from `tree.json` and compares every case. Nothing outside the temp folder is read or written; the store is a
temp store; the "secret" is a canary file the generator creates beside the repository.

Run (read-only use of the parity venv):  PYTHONPATH=src rust/parity/.venv/bin/python rust/parity/gen_report_files.py
"""

import hashlib
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path

CAP = 10 * 1024 * 1024
FIXED_MTIME = 1768694400  # 2026-01-18T00:00:00Z
OUT_DIR = Path(__file__).resolve().parent / "golden-report-files"
CANARY = "CANARY-T045-OUTSIDE-THE-REPORT-FOLDER"
LONG = "a" * 251 + ".txt"  # 255 bytes

# `p`: path under the repository (`p_hex`: the same as raw bytes); `@OUT` in a target: the folder outside the repository.
TREE = [
    {"p": "reports/ok.html", "t": "file", "text": "<p>ok</p>\n"},
    {"p": "reports/up.HTML", "t": "file", "text": "<p>upper</p>\n"},
    {"p": "reports/.hidden.html", "t": "file", "text": "<p>dot</p>\n"},
    {"p": "reports/.html", "t": "file", "text": "<p>just an extension</p>\n"},
    {"p": "reports/sub/inner.json", "t": "file", "text": '{"a": 1}\n'},
    {"p": "reports/sub/deeper/deep.md", "t": "file", "text": "# deep\n"},
    {"p": "reports/sub/deeper/deepest/too-deep.md", "t": "file", "text": "# too deep\n"},
    {"p": "reports/link-out.html", "t": "symlink", "to": "@OUT/secret.txt"},
    {"p": "reports/link-dir", "t": "symlink", "to": "@OUT/dir"},
    {"p": "reports/link-in.html", "t": "symlink", "to": "ok.html"},
    {"p": "reports/hard.html", "t": "hardlink", "to": "@OUT/secret.txt"},
    {"p": "reports/linked.html", "t": "file", "text": "<p>has a second name</p>\n"},
    {"p": "reports/hard-in.html", "t": "hardlink", "to": "reports/linked.html"},
    {"p": "reports/fifo.html", "t": "fifo"},
    {"p": "reports/dir.html", "t": "dir"},
    {"p": "reports/cap.txt", "t": "fill", "size": CAP},
    {"p": "reports/cap1.txt", "t": "fill", "size": CAP + 1},
    {"p": "reports/big.txt", "t": "sparse", "size": 100 * 1000 * 1000},
    {"p": "bytes-reports/ok.txt", "t": "file", "text": "ok\n"},
    {"p_hex": b"bytes-reports/bad\xff.txt".hex(), "t": "file", "text": "not utf-8\n"},
    {"p": "names-reports/ok.txt", "t": "file", "text": "ok\n"},
    {"p": "names-reports/line\nbreak.txt", "t": "file", "text": "newline\n"},
    {"p": f"names-reports/{LONG}", "t": "file", "text": "long\n"},
    {"p": "reports-link", "t": "symlink", "to": "@OUT/dir"},
    {"p": "plain/x.html", "t": "file", "text": "not a report folder\n"},
]
OUTSIDE = [
    {"p": "outside/secret.txt", "t": "file", "text": CANARY + "\n"},
    {"p": "outside/dir/x.html", "t": "file", "text": CANARY + "\n"},
]


def build(root: Path) -> Path:
    """The tree under `root`; returns the repository folder (`root/repo`)."""
    for entry in OUTSIDE + TREE:
        rel = bytes.fromhex(entry["p_hex"]) if "p_hex" in entry else entry["p"].encode()
        outside = rel.startswith(b"outside/")
        base = root if outside else root / "repo"
        path = os.fsdecode(os.path.join(os.fsencode(base), rel))
        os.makedirs(os.path.dirname(path), exist_ok=True)
        kind = entry["t"]
        target = entry.get("to", "").replace("@OUT", str(root / "outside"))
        if kind == "file":
            with open(path, "wb") as f:
                f.write(entry["text"].encode())
        elif kind == "fill":
            with open(path, "wb") as f:
                f.write(b"a" * entry["size"])
        elif kind == "sparse":
            with open(path, "wb") as f:
                f.truncate(entry["size"])
        elif kind == "symlink":
            os.symlink(target, path)
        elif kind == "hardlink":
            os.link(target if target.startswith("/") else str(root / "repo" / target), path)
        elif kind == "fifo":
            os.mkfifo(path)
        elif kind == "dir":
            os.makedirs(path, exist_ok=True)
        if kind in ("file", "fill", "sparse"):
            os.utime(path, (FIXED_MTIME, FIXED_MTIME))
    return root / "repo"


CASES = [  # (id, route, query)
    ("files_reports", "files", {"workspace": "acme", "repo_no": "0", "folder": "reports"}),
    ("files_bytes_reports", "files", {"workspace": "acme", "repo_no": "0", "folder": "bytes-reports"}),
    ("files_names_reports", "files", {"workspace": "acme", "repo_no": "0", "folder": "names-reports"}),
    ("files_missing_folder", "files", {"workspace": "acme", "repo_no": "0", "folder": "nope-reports"}),
    ("files_symlink_folder", "files", {"workspace": "acme", "repo_no": "0", "folder": "reports-link"}),
    ("files_plain_folder", "files", {"workspace": "acme", "repo_no": "0", "folder": "plain"}),
    ("files_wrong_workspace", "files", {"workspace": "nope", "repo_no": "0", "folder": "reports"}),
    ("files_repo_no_1", "files", {"workspace": "acme", "repo_no": "1", "folder": "reports"}),
    ("files_repo_no_minus1", "files", {"workspace": "acme", "repo_no": "-1", "folder": "reports"}),
    ("files_repo_no_huge", "files", {"workspace": "acme", "repo_no": "99999999999", "folder": "reports"}),
    ("files_repo_no_exp", "files", {"workspace": "acme", "repo_no": "1e3", "folder": "reports"}),
    ("files_repo_no_arabic", "files", {"workspace": "acme", "repo_no": "١", "folder": "reports"}),
    ("files_repo_no_zeros", "files", {"workspace": "acme", "repo_no": "000", "folder": "reports"}),
    ("files_nothing", "files", {}),
    ("file_not_a_folder", "file", {"workspace": "acme", "repo_no": "0", "folder": "plain", "path": "x.html"}),
    ("file_bad_repo_no_arabic", "file", {"workspace": "acme", "repo_no": "١", "folder": "reports", "path": "ok.html"}),
    ("file_bad_repo_no_minus1", "file", {"workspace": "acme", "repo_no": "-1", "folder": "reports", "path": "ok.html"}),
    ("file_bad_repo_no_exp", "file", {"workspace": "acme", "repo_no": "1e3", "folder": "reports", "path": "ok.html"}),
    ("file_names_newline", "file", {"workspace": "acme", "repo_no": "0", "folder": "names-reports", "path": "line\nbreak.txt"}),
    ("file_names_long", "file", {"workspace": "acme", "repo_no": "0", "folder": "names-reports", "path": LONG}),
    ("file_bytes_replacement", "file", {"workspace": "acme", "repo_no": "0", "folder": "bytes-reports", "path": "bad�.txt"}),
] + [
    (f"file_{name}", "file", {"workspace": "acme", "repo_no": "0", "folder": "reports", "path": path})
    for name, path in [
        ("ok", "ok.html"), ("upper_ext", "up.HTML"), ("dotfile_with_ext", ".hidden.html"), ("dot_ext_only", ".html"),
        ("sub", "sub/inner.json"), ("depth3", "sub/deeper/deep.md"), ("depth4", "sub/deeper/deepest/too-deep.md"),
        ("dotdot", "../outside/secret.txt"), ("dotdot_inside", "sub/../ok.html"), ("absolute", "/etc/passwd"),
        ("backslash", "sub\\inner.json"), ("nul", "ok.html\0"), ("empty", ""), ("empty_part", "sub//inner.json"),
        ("dot_part", "sub/./inner.json"), ("trailing_slash", "ok.html/"), ("symlink_out", "link-out.html"),
        ("symlink_dir", "link-dir/x.html"), ("symlink_in", "link-in.html"), ("hardlink_out", "hard.html"),
        ("hardlink_in", "hard-in.html"), ("hardlink_in_origin", "linked.html"), ("fifo", "fifo.html"), ("dir_named_file", "dir.html"), ("cap", "cap.txt"),
        ("cap_plus_1", "cap1.txt"), ("sparse_100mb", "big.txt"), ("missing", "nope.html"), ("bad_ext", "ok.svg"),
        ("no_ext", "ok"), ("too_long_path", "a/" * 300 + "x.html"), ("no_path", None),
    ]
]


def main() -> None:
    root = Path(tempfile.mkdtemp(prefix="easy-personal-t045-gen-"))
    try:
        os.environ["EASY_PERSONAL_HOME"] = str(root / "home")
        from easy_personal.core import config, kit, store, workspaces
        from easy_personal.surfaces import api

        repo = build(root)
        with store.opened(config.store_path()) as conn:
            workspaces.add(conn, "acme", repos=[str(repo)])
            conn.commit()
            results = []
            for case_id, route, query in CASES:
                q = {k: v for k, v in query.items() if v is not None}
                args = api._report_args(q)
                entry = {"id": case_id, "route": route, "query": q}
                try:
                    if route == "files":
                        body = kit.report_files(conn, *args)
                        json.dumps(body, ensure_ascii=False).encode("utf-8")  # what Starlette does: a lone surrogate fails here
                        entry |= {"status": 200, "body": body}
                    else:
                        content_type, data = kit.report_file(conn, *args, q.get("path"))
                        entry |= {"status": 200, "content_type": content_type, "size": len(data),
                                  "sha256": hashlib.sha256(data).hexdigest(), "canary": CANARY.encode() in data}
                except kit.ReportNotFound:
                    entry |= {"status": 404, "body": {"error": "not found"}}
                except kit.ReportTooLarge:
                    entry |= {"status": 413, "body": {"error": "too large"}}
                except UnicodeEncodeError:
                    entry |= {"status": "python_crash", "note": "JSON encoding of a name that is not UTF-8: 500 with a traceback"}
                results.append(entry)
        OUT_DIR.mkdir(exist_ok=True)
        (OUT_DIR / "tree.json").write_text(json.dumps({"canary": CANARY, "cap": CAP, "outside": OUTSIDE, "tree": TREE}, indent=1, sort_keys=True) + "\n")
        (OUT_DIR / "cases.json").write_text(json.dumps({"python": sys.version.split()[0], "cases": results}, indent=1, sort_keys=True) + "\n")
        print(f"wrote {len(results)} cases to {OUT_DIR}")
    finally:
        shutil.rmtree(root, ignore_errors=True)


if __name__ == "__main__":
    main()
