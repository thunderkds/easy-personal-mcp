"""T043 oracle: what Python answers for the board and memory-graph routes over a hostile fixture tree.

Builds `fixtures-memgraph/` (plus the files a repository cannot hold: a note over the size cap, a non-UTF-8 note, symlinks,
600 and 5001 notes, a FIFO, board files of every kind) in a temp folder, seeds a temp copy of `store-v11.sqlite` with
`fixtures-memgraph/seed.sql`, runs Python's own `memgraph.project_list` / `project_graph` and `board.read_project` /
`board.board` / `board._parse`, and writes `rust/parity/golden-memgraph/cases.json` with the temp folder replaced by `<T>`.
The Rust test (`crates/core/tests/memgraph_parity.rs`) builds the same tree and compares every case. Nothing outside the
temp folder is read or written; `HOME` and `EASY_MEMORY_HOME` point at empty temp folders (no real home is opened).

Run (read-only use of the parity venv):  PYTHONPATH=src rust/parity/.venv/bin/python rust/parity/gen_memgraph.py
"""

import json
import os
import re
import shutil
import sqlite3
import sys
import tempfile
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
]


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


def main() -> None:
    t = Path(tempfile.mkdtemp(prefix="easy-personal-t043-gen-"))
    try:
        os.environ["EASY_PERSONAL_HOME"] = str(t / "home")
        os.environ["HOME"] = str(t / "home")
        os.environ["EASY_MEMORY_HOME"] = str(t / "em-none")
        sys.path.insert(0, str(PARITY.parent.parent / "src"))
        from easy_personal.core import board, config, memgraph, store

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
    finally:
        shutil.rmtree(t, ignore_errors=True)


if __name__ == "__main__":
    main()
