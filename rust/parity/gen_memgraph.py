"""T043 oracle: what Python answers for the board and memory-graph routes over a hostile fixture tree.

Builds `fixtures-memgraph/` (plus the files a repository cannot hold: a note over the size cap, a non-UTF-8 note, symlinks,
600 and 5001 notes, a FIFO, board files of every kind) in a temp folder, seeds a temp copy of `store-v11.sqlite` with
`fixtures-memgraph/seed.sql`, runs Python's own `memgraph.project_list` / `project_graph` and `board.read_project` /
`board.board` / `board._parse`, and writes `rust/parity/golden-memgraph/cases.json` with the temp folder replaced by `<T>`.
The Rust test (`crates/core/tests/memgraph_parity.rs`) builds the same tree and compares every case. Nothing outside the
temp folder is read or written; `HOME` and `EASY_MEMORY_HOME` point at empty temp folders (no real home is opened).

T053 adds a second tree, built by `build_t053` under `<T>/t053` (kept apart so `cases.json` only gains parse cases): a project
with hostile notes, two Claude folders that resolve to one project (duplicate node ids), a git worktree identity and an
easy-memory store with notes (a cited file inside and outside the project, a stale and a gone one, `prompts.log`, `report.json`,
a `trash/` note, BOMs). It writes `cases-t053.json`: Python's answers for it, the easy-memory panel under four time zones
(`TZ` set before the run, "now" frozen at 2026-01-20T00:00:00Z) and the code points of Python's `\\d`.

Run (read-only use of the parity venv):  PYTHONPATH=src rust/parity/.venv/bin/python rust/parity/gen_memgraph.py
"""

import json
import os
import re
import shutil
import sqlite3
import sys
import tempfile
import time
from datetime import UTC, datetime
from pathlib import Path

PARITY = Path(__file__).resolve().parent
FIX = PARITY / "fixtures-memgraph"
OUT = PARITY / "golden-memgraph"
MTIME = 1768694400  # 2026-01-18T00:00:00Z

# kanban texts for `board._parse`: (name, text). The same strings are in the golden; Rust reads them from there.
PARSE = [
    ("basic", "## Board\n### Todo\n- **T-1** — first task\n- [ ] T-2: second\n- [x] ABC3 third | not part\n### In Progress\n| ID | Title |\n|----|-------|\n| T-4 | four |\n### Done\n- ~~T-5~~ closed\n~~| T-6 | x |~~\n"),
    ("crlf_bom_tabs", "﻿## Board\r\n### Todo\r\n- T-1 one\r\n## Board\r\n### Todo\r\n\t- T-2\ttwo\r\n-\tT-3\tthree\r\n"),
    ("no_board", "# Title\n### Todo\n- T-1 x\n"),
    ("blocked_and_unknown", "## Board\n### Weird\n- T-1 x\n- no id here\n### Todo\n- plain prose\n## Blocked\n| T-9 | waits |\n| Task | Why |\n|---|---|\n## Other\n- T-2 y\n"),
    ("unbalanced_pipes", "## Board\n### Todo\n| T-1 | a | b\n|| T-2 ||\n| | |\n| T-3 \\| escaped | x |\n- T-4 title | a | b\n- T-5 title \\| esc | x\n- T-6 a | b\n"),
    ("unicode_ids", "## Board\n### Todo\n- **R-١٢** arabic digits\n- AB１２ fullwidth\n- é-1 accent\n- **T-²** superscript\n- **T 1** spaced\n"),
    ("duplicates", "## Board\n### Todo\n- T-1 a\n- T-1 b\n### Done\n- T-1 c\n"),
    ("links_escapes", "## Board\n### Todo\n- T-1 [link](http://x) and `code` \\*star\\* \\\\ back [a](b [c](d) [e]\n- T-2 [[wiki]] [x]( y\n- T-3 " + "[" * 200 + "\n"),
    ("splitlines", "## Board\n### Todo\n- T-1 a\x0b- T-2 b\x0c- T-3 c\x1c- T-4 d\x85- T-5 e - T-6 f - T-7 g\r- T-8 h\n"),
    ("separators", "## Board\n### Todo\n| a | b |\n|:--|--:|\n| T-1 | x |\n|---|---|---|\n| - | - |\n|--\n| --- | --- |\n"),
    ("long_title", "## Board\n### Todo\n- T-1 " + "x" * 20000 + "\n| T-2 | " + "y" * 20000 + " |\n"),
    ("headings", "##   Board  \n###  in progress \n- T-1 a\n### READY FOR REVIEW\n- T-2 b\n### Closed (Will Not Do)\n- T-3 c\n### Todo extra\n- T-4 d\n## BLOCKED\n- T-5 e\n"),
    ("dashes_after_id", "## Board\n### Todo\n- T-1 —–:- - dashes\n- **T-2**:: x\n- ***T-3*** y\n- ** T-4 ** z\n- **T-5\n"),
    ("empty", ""),
    # T053: `_ESCAPE.sub(r"\1", ..)` keeps the escaped character and does not overlap the next match
    ("escaped_backslash_pairs", "## Board\n### Todo\n- T-1 \\\\* hi\n- T-2 \\\\\\\\y\n| T-3 | \\\\_u \\\\\\* |\n"),
    ("escaped_backslash_runs", "## Board\n### Todo\n- T-1 \\\\\\\\\\\\ end\n- T-2 \\\\ \\\\\\ \\\n- T-3 \\\\[a](b) \\\\`x\\\\`\n"),
    ("backslash_before_plain", "## Board\n### Todo\n- T-1 a\\b \\z \\é \\\n- T-2 \\1 \\ x \\\\\n"),
]
NOW_S = 1768867200  # 2026-01-20T00:00:00Z, the instant the Rust tests freeze
ZONES = ["UTC", "Pacific/Kiritimati", "Pacific/Pago_Pago", "America/Los_Angeles"]


def enc(path: str) -> str:
    return re.sub(r"[^A-Za-z0-9]", "-", path)


def build(t: Path) -> None:
    """The tree under `t` (the Rust test builds the same)."""
    shutil.copytree(FIX / "proj", t / "proj")
    claude = t / "acct" / "projects"
    shutil.copytree(FIX / "claude-memory", claude / enc(str(t / "proj")) / "memory")
    shutil.copytree(FIX / "claude-other", claude / "zzother")
    shutil.copytree(FIX / "claude-bare", claude / "claude-bare")
    memory = t / "proj" / "memory"
    (memory / "big.md").write_bytes(b"x" * (1024 * 1024 + 1))
    (memory / "bad.md").write_bytes(b"---\nname: bad\n---\n\xff\xfe\n")
    (t / "outside" / "dir").mkdir(parents=True)
    (t / "outside" / "secret.md").write_text("---\nname: secret\n---\n")
    (t / "outside" / "dir" / "d.md").write_text("---\nname: d\n---\n")
    os.symlink(t / "outside" / "secret.md", memory / "link-out.md")
    os.symlink(t / "outside" / "dir", memory / "link-dir")
    (t / "proj600" / "memory").mkdir(parents=True)
    for i in range(600):
        (t / "proj600" / "memory" / f"n{i:03}.md").write_text(f"---\nname: note-{i}\n---\n[[note-{(i + 1) % 600}]] [[gone-{i % 7}]]\n")
    (t / "projcap" / "memory").mkdir(parents=True)
    for i in range(5001):
        (t / "projcap" / "memory" / f"c{i:04}.md").write_text("x\n")
    boards = t / "boards"
    kinds = {
        "ok": ("## Board\n### Todo\n- T-1 one\n", ' {"board": "  linear  "} '),
        "bad_json": ("## Board\n### Todo\n- T-1 one\n", "{not json"),
        "board_not_str": ("## Board\n### Todo\n- T-1 one\n", '{"board": 5}'),
        "board_long": ("## Board\n### Todo\n- T-1 one\n", json.dumps({"board": "b" * 60})),
        "no_board_section": ("# nothing\n", '{"board": "linear"}'),
    }
    for name, (kanban, board_json) in kinds.items():
        (boards / name / ".claude").mkdir(parents=True)
        (boards / name / "PROJECT_KANBAN.md").write_text(kanban)
        (boards / name / ".claude" / "board.json").write_text(board_json)
    (boards / "over_cap").mkdir()
    (boards / "over_cap" / "PROJECT_KANBAN.md").write_bytes(b"x" * 5_000_001)
    (boards / "at_cap").mkdir()
    (boards / "at_cap" / "PROJECT_KANBAN.md").write_bytes(b"## Board\n### Todo\n- T-1 a\n" + b" " * (5_000_000 - 26))
    (boards / "bad_utf8").mkdir()
    (boards / "bad_utf8" / "PROJECT_KANBAN.md").write_bytes(b"## Board\n### Todo\n- T-1 \xff\n")
    (boards / "none").mkdir()
    (boards / "dir_file" / "PROJECT_KANBAN.md").mkdir(parents=True)
    (boards / "fifo").mkdir()
    os.mkfifo(boards / "fifo" / "PROJECT_KANBAN.md")
    for path in boards.rglob("PROJECT_KANBAN.md"):
        if path.is_file():
            os.utime(path, (MTIME, MTIME))


def log_line(total: int) -> bytes:
    """A `prompts.log` line of exactly `total` bytes, newline included (the cap is 64 KiB per line)."""
    head, tail = b'{"time":"2026-01-19T12:00:00Z","effective":"on","injected":true,"pad":"', b'"}\n'
    return head + b"x" * (total - len(head) - len(tail)) + tail


def build_t053(t: Path) -> None:
    """The T053 tree under `t / "t053"` (the Rust test builds the same)."""
    t2 = t / "t053"
    projh = t2 / "projh" / "memory"
    projh.mkdir(parents=True)
    (projh / "plain.md").write_text("---\nname: plain\ndescription: ok\n---\n[[nul-note]] [[big]] [[nested]]\n")
    (projh / "nul.md").write_text("---\nname: a\0b\ndescription: nul\n---\nbody [[plain]]\n")
    (projh / "bigdesc.md").write_text("---\nname: big\ndescription: " + "d" * 100_000 + "\n---\n")
    (projh / "nested.md").write_text("---\nname:\n  first: 1\n  second: [a, b]\ndescription:\n  deep:\n    deeper: x\n---\n[[plain]]\n")
    os.mkfifo(projh / "fifo.md")
    dup = t2 / "projdup"  # two Claude folders of one account that resolve to one project: duplicate node ids
    dup.mkdir()
    projects = t2 / "acct" / "projects"
    first = projects / enc(str(dup)) / "memory"
    first.mkdir(parents=True)
    (first / "MEMORY.md").write_text("---\nname: idx-a\n---\n[[c1]]\n")
    (first / "c1.md").write_text("---\nname: c1\n---\n")
    (projects / "zzsub" / "memory").mkdir(parents=True)
    (projects / "zzsub" / "memory" / "MEMORY.md").write_text("---\nname: idx-b\n---\n")
    (projects / "zzsub" / "t.jsonl").write_text(json.dumps({"cwd": str(dup)}) + "\n")
    shutil.copytree(FIX / "projem", t2 / "projem")
    root = t2 / "projem"
    os.symlink("missing.txt", root / "dang")
    os.symlink("/etc/nowhere", root / "out")
    os.symlink("src/a.py", root / "ln")
    os.symlink("src", root / "linkdir")
    os.mkfifo(root / "pipe")
    with open(root / "big.bin", "wb") as big:
        big.truncate(10 * 1024 * 1024 + 1)
    (t2 / "projwt").mkdir()  # a worktree: `.git` is a file, its `commondir` names the repo the store is keyed by
    (t2 / "projwt" / ".git").write_text(f"gitdir: {t2}/gd\n")
    (t2 / "gd").mkdir()
    (t2 / "gd" / "commondir").write_text("../common\n")
    (t2 / "common").mkdir()
    (t2 / "projdd" / "sub").mkdir(parents=True)  # the store's path is `lnk/..`: the link is resolved first, so it names projdd
    os.symlink("projdd/sub", t2 / "lnk")
    shutil.copytree(FIX / "em-store", t2 / "em")
    for meta in (t2 / "em").glob("*/meta.json"):
        meta.write_bytes(meta.read_bytes().replace(b"@T@", str(t).encode()))
    (t2 / "em" / "k1" / "notes" / "x.json").mkdir()  # a folder named like a note: abstained
    with open(t2 / "em" / "k1" / "prompts.log", "ab") as log:
        for total in (65537, 65538, 200_000):  # a line of 64 KiB + 1 with its newline is read, a longer one is skipped
            log.write(log_line(total))
        log.write(b'{"time":"2026-01-19T15:00:00Z","effective":"on","injected":true}\n')


def digits() -> list[list[int]]:
    """The code points where Python's `re.match(r"\\d", c)` holds, as `[first, last]` runs."""
    runs: list[list[int]] = []
    for point in range(0x110000):
        if re.match(r"\d", chr(point)):
            if runs and runs[-1][1] == point - 1:
                runs[-1][1] = point
            else:
                runs.append([point, point])
    return runs


def run_t053(t: Path, memgraph, easymem, store, config) -> dict:
    """Python's answers over the T053 tree, the easy-memory panel under each zone, and the `\\d` table."""
    t2 = t / "t053"
    build_t053(t)
    (t / "home053").mkdir()
    shutil.copy(PARITY / "store-v11.sqlite", t / "home053" / "store.sqlite")
    seed = sqlite3.connect(t / "home053" / "store.sqlite")
    seed.executescript((FIX / "seed-t053.sql").read_text().replace("@T@", str(t)))
    seed.commit()
    seed.close()
    os.environ["EASY_PERSONAL_HOME"] = str(t / "home053")
    os.environ["EASY_MEMORY_HOME"] = str(t2 / "em")
    real_summary = easymem.summary

    def frozen(folder, project, today=None):  # "now" is NOW_S in the zone of the process
        return real_summary(folder, project, today or datetime.fromtimestamp(NOW_S, UTC).astimezone().date())

    easymem.summary = frozen
    out: dict = {"python": sys.version.split()[0], "graphs": {}, "zones": {}}
    try:
        project = str(t2 / "projem")
        for zone in ZONES:
            os.environ["TZ"] = zone
            time.tzset()
            with store.opened(config.store_path()) as conn:
                graph = memgraph.project_graph(conn, project)
                if zone == "UTC":
                    out["list"] = memgraph.project_list(conn)
                    out["graphs"][project] = graph
                    for name in ["projh", "projdup", "projwt", "projdd"]:
                        out["graphs"][str(t2 / name)] = memgraph.project_graph(conn, str(t2 / name))
                out["zones"][zone] = graph["easy_memory"]
    finally:
        easymem.summary = real_summary
        os.environ["TZ"] = "UTC"
        time.tzset()
    out["digits"] = digits()
    return out


def main() -> None:
    t = Path(tempfile.mkdtemp(prefix="easy-personal-t043-gen-"))
    try:
        os.environ["EASY_PERSONAL_HOME"] = str(t / "home")
        os.environ["HOME"] = str(t / "home")
        os.environ["EASY_MEMORY_HOME"] = str(t / "em-none")
        sys.path.insert(0, str(PARITY.parent.parent / "src"))
        from easy_personal.core import board, config, easymem, memgraph, store

        build(t)
        (t / "home").mkdir()
        shutil.copy(PARITY / "store-v11.sqlite", t / "home" / "store.sqlite")
        seed = sqlite3.connect(t / "home" / "store.sqlite")
        seed.executescript((FIX / "seed.sql").read_text().replace("@T@", str(t)))
        seed.commit()
        seed.close()
        out: dict = {"python": sys.version.split()[0], "graphs": {}, "boards": {}, "parse": []}
        with store.opened(config.store_path()) as conn:
            out["list"] = memgraph.project_list(conn)
            out["board_all"] = board.board(conn)
            for name in ["proj", "proj600"]:
                out["graphs"][str(t / name)] = memgraph.project_graph(conn, str(t / name))
            for key in ["claude-bare", "/nonexistent-t043/zz"]:
                out["graphs"][key] = memgraph.project_graph(conn, key)
            cap = memgraph.project_graph(conn, str(t / "projcap"))
            out["cap"] = {"counts": cap["counts"], "nodes": len(cap["nodes"]), "edges": len(cap["edges"])}
            try:
                memgraph.project_graph(conn, "nope")
            except ValueError as exc:
                out["unknown"] = str(exc)
        for path in sorted((t / "boards").iterdir()):
            out["boards"][str(path)] = board.read_project(path)
        for name, text in PARSE:
            parsed = board._parse(text)
            out["parse"].append({"name": name, "text": text, "result": None if parsed is None else {"tasks": parsed[0], "abstained": parsed[1]}})
        text = json.dumps(out, indent=1, sort_keys=True, ensure_ascii=False).replace(str(t), "<T>")
        OUT.mkdir(exist_ok=True)
        (OUT / "cases.json").write_text(text + "\n")
        print(f"wrote {OUT / 'cases.json'}: {len(out['graphs'])} graphs, {len(out['boards'])} boards, {len(PARSE)} parse cases")
        more = run_t053(t, memgraph, easymem, store, config)
        text = json.dumps(more, indent=1, sort_keys=True, ensure_ascii=False).replace(str(t), "<T>")
        (OUT / "cases-t053.json").write_text(text + "\n")
        print(f"wrote {OUT / 'cases-t053.json'}: {len(more['graphs'])} graphs, {len(more['zones'])} zones, {len(more['digits'])} digit runs")
    finally:
        shutil.rmtree(t, ignore_errors=True)


if __name__ == "__main__":
    main()
