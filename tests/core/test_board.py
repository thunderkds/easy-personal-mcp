"""kanban reader (HUN-162, FR-037): board tasks from each kit project's PROJECT_KANBAN.md, read live."""

import json
import os
import time

import pytest

from easy_personal.core import board, workspaces
from tests_helpers import add_event

FULL = """# PROJECT_KANBAN.md

## Board

### Todo
- [ ] **HUN-1** — First task | Agent | C2 | Risk: Low | P0
- [ ] **HUN-2** — Second task | Agent | C1

### In Progress
- [ ] **HUN-3** — Doing it | Agent

### Ready for Review
- [ ] **HUN-4** — Review me | Agent

### Done
- [x] **HUN-5** — Finished | Agent
- [x] **HUN-6** — Finished too | Agent

### Closed (will not do)
- [x] **HUN-7** — Dropped | Agent

### Parked
- [ ] **HUN-8** — Not a known column | Agent
- [ ] **HUN-9** — Also not | Agent

## Blocked

| Task | Reason | Waiting on |
|------|--------|-----------|
| XR-1 | Needs the other repo | someone |

---

## Stage Tracker

| Stage | Status |
|-------|--------|
| 1 Setup | done |
"""


def repo(tmp_path, name, kanban: str | bytes | None = FULL, board_json=None):
    root = tmp_path / "repos" / name
    (root / ".claude").mkdir(parents=True)
    if kanban is not None:
        (root / "PROJECT_KANBAN.md").write_bytes(kanban if isinstance(kanban, bytes) else kanban.encode())
    if board_json is not None:
        (root / ".claude" / "board.json").write_text(board_json if isinstance(board_json, str) else json.dumps(board_json))
    return root


def parse(tmp_path, kanban, name="p", board_json=None):
    return board.read_project(repo(tmp_path, name, kanban, board_json))


def by_id(entry):
    return {t["id"]: t for t in entry["tasks"]}


# AC1: sections, statuses, abstained unknown subsections

def test_every_known_section_and_the_blocked_table_map_to_their_status(tmp_path):
    entry = parse(tmp_path, FULL)
    got = {t["id"]: t["status"] for t in entry["tasks"]}
    assert got == {
        "HUN-1": "Todo", "HUN-2": "Todo", "HUN-3": "In Progress", "HUN-4": "Ready for Review",
        "HUN-5": "Done", "HUN-6": "Done", "HUN-7": "Closed (will not do)", "XR-1": "Blocked",
    }  # fmt: skip
    assert entry["status"] == "ok"
    assert by_id(entry)["HUN-1"]["title"] == "First task"
    assert by_id(entry)["XR-1"]["title"] == "Needs the other repo"


def test_unknown_subsection_is_counted_abstained_never_coerced(tmp_path):
    entry = parse(tmp_path, FULL)
    assert entry["abstained"] == 2
    assert "HUN-8" not in by_id(entry) and "HUN-9" not in by_id(entry)


def test_table_rows_in_a_board_section_work_and_header_rows_are_dropped(tmp_path):
    text = (
        "## Board\n\n### Todo\n| ID | Title | Agent |\n|----|-------|-------|\n"
        "| T001 | Table task | a |\n| T002 | `Code` and [link](http://x.test) | b |\n\n### Done\n"
    )
    entry = parse(tmp_path, text)
    assert [(t["id"], t["title"], t["status"]) for t in entry["tasks"]] == [
        ("T001", "Table task", "Todo"),
        ("T002", "Code and link", "Todo"),
    ]
    assert entry["abstained"] == 0


def test_titles_are_plain_text_with_escapes_removed(tmp_path):
    text = (
        "## Board\n### Todo\n"
        "- [ ] **HUN-1** — Tracer: \\`claude_code\\` source \\| store | Agent | C3\n"
        "- [ ] **HUN-2** — See [the doc](https://x.test/a|b) now | Agent\n"
        "- [ ] **HUN-3** — <b>bold</b> & `tick` | Agent\n"
    )
    titles = {t["id"]: t["title"] for t in parse(tmp_path, text)["tasks"]}
    assert titles == {
        "HUN-1": "Tracer: claude_code source | store",
        "HUN-2": "See the doc now",
        "HUN-3": "<b>bold</b> & tick",
    }


def test_plain_bullets_without_checkbox_or_bold_are_read(tmp_path):
    text = "## Board\n### In Progress\n- T120 - Plain bullet | agent\n* **T121** - Star bullet\n"
    assert [(t["id"], t["title"]) for t in parse(tmp_path, text)["tasks"]] == [("T120", "Plain bullet"), ("T121", "Star bullet")]


def test_a_row_without_an_id_in_a_known_section_is_abstained(tmp_path):
    text = "## Board\n### Todo\n- [ ] no id here at all\n- [ ] **HUN-1** — ok\n"
    entry = parse(tmp_path, text)
    assert [t["id"] for t in entry["tasks"]] == ["HUN-1"]
    assert entry["abstained"] == 1


def test_struck_through_blocked_rows_are_closed_and_skipped(tmp_path):
    text = "## Board\n### Todo\n\n## Blocked\n\n| Task | Reason |\n|---|---|\n| ~~T004~~ | **CLOSED** old |\n| T005 | open one |\n"
    entry = parse(tmp_path, text)
    assert [(t["id"], t["status"]) for t in entry["tasks"]] == [("T005", "Blocked")] and entry["abstained"] == 0


def test_blank_lines_notes_and_placeholder_table_cells_are_not_tasks(tmp_path):
    text = "## Board\n> a note\n\n### Todo\n\nsome prose line\n- [ ] **HUN-1** — ok\n"
    entry = parse(tmp_path, text)
    assert [t["id"] for t in entry["tasks"]] == ["HUN-1"]


def test_a_huge_done_section_is_counted_exactly(tmp_path):
    text = "## Board\n### Done\n" + "".join(f"- [x] **HUN-{n}** — t{n} | a\n" for n in range(1, 501))
    assert len(parse(tmp_path, text)["tasks"]) == 500


def test_a_very_long_title_is_cut(tmp_path):
    text = f"## Board\n### Todo\n- [ ] **HUN-1** — {'x' * 5000}\n"
    assert len(parse(tmp_path, text)["tasks"][0]["title"]) <= 300


# AC2: no data

def test_missing_file_is_no_data(tmp_path):
    entry = parse(tmp_path, None)
    assert entry == {"project": "p", "status": "no data", "board": None, "last_pull": None, "abstained": 0, "tasks": []}


@pytest.mark.parametrize(
    "raw",
    [b"\xff\xfe\x00 not utf-8 \x80", b"", b"just prose\nno board section here\n", b"## Stage Tracker\n| a | b |\n"],
    ids=["non-utf8", "empty", "no-board-heading", "other-heading-only"],
)
def test_unreadable_or_unparsable_file_is_no_data_not_a_crash(tmp_path, raw):
    entry = parse(tmp_path, raw)
    assert entry["status"] == "no data" and entry["tasks"] == []


def test_a_board_heading_with_no_tasks_is_ok_and_empty(tmp_path):
    entry = parse(tmp_path, "## Board\n### Todo\n### Done\n")
    assert entry["status"] == "ok" and entry["tasks"] == []


def test_a_directory_named_like_the_file_is_no_data(tmp_path):
    root = repo(tmp_path, "p", None)
    (root / "PROJECT_KANBAN.md").mkdir()
    assert board.read_project(root)["status"] == "no data"


def test_an_oversized_file_is_no_data(tmp_path):
    entry = parse(tmp_path, "## Board\n### Todo\n" + "- [ ] **HUN-1** — " + "x" * 6_000_000)
    assert entry["status"] == "no data"


# board kind and last pull

def test_board_json_naming_a_board_tags_it_and_gives_the_pull_time(tmp_path):
    root = repo(tmp_path, "p", FULL, {"board": "linear", "team": "SECRET-TEAM", "me": "someone"})
    os.utime(root / "PROJECT_KANBAN.md", (1_767_225_600, 1_767_225_600))
    entry = board.read_project(root)
    assert entry["board"] == "linear"
    assert entry["last_pull"] == "2026-01-01T00:00:00Z"


@pytest.mark.parametrize(
    "board_json",
    [None, "{not json", "[]", {"board": ""}, {"board": 5}, {"other": "x"}, {"board": None}],
    ids=["absent", "bad-json", "list", "empty", "number", "no-key", "null"],
)
def test_no_named_board_means_no_tag_and_no_pull_time(tmp_path, board_json):
    entry = parse(tmp_path, FULL, board_json=board_json)
    assert entry["board"] is None and entry["last_pull"] is None and entry["status"] == "ok"


# AC3: only the allowed keys leave the reader

def test_the_response_holds_only_the_allowed_keys(tmp_path):
    root = repo(tmp_path, "p", FULL + "\nSECRET-BODY-TEXT\n", {"board": "linear", "team": "SECRET-TEAM"})
    entry = board.read_project(root)
    assert set(entry) == {"project", "status", "board", "last_pull", "abstained", "tasks"}
    assert all(set(t) == {"id", "title", "status"} for t in entry["tasks"])
    blob = json.dumps(entry)
    assert "SECRET" not in blob and str(tmp_path) not in blob
    assert entry["project"] == "p"


# AC2: scope = scoped_projects that exist; worktrees skipped

def seed(conn, *paths):
    for n, p in enumerate(paths):
        add_event(conn, n, str(p))
    conn.commit()


def test_projects_come_from_usage_and_a_deleted_folder_is_no_data(conn, tmp_path):
    a, b = repo(tmp_path, "alpha"), repo(tmp_path, "beta")
    gone = tmp_path / "repos" / "gone"
    seed(conn, a, b, gone)
    got = {p["project"]: p for p in board.board(conn)["projects"]}
    assert set(got) == {"alpha", "beta", "gone"}
    assert got["alpha"]["status"] == "ok" and len(got["alpha"]["tasks"]) == 8
    assert got["gone"]["status"] == "no data" and got["gone"]["tasks"] == []


def test_worktree_folders_are_not_projects(conn, tmp_path):
    main = repo(tmp_path, "main-repo")
    wt = main / ".claude" / "worktrees" / "HUN-1"
    wt.mkdir(parents=True)
    (wt / "PROJECT_KANBAN.md").write_text(FULL)
    seed(conn, main, wt)
    assert [p["project"] for p in board.board(conn)["projects"]] == ["main-repo"]


def test_same_task_id_in_two_projects_stays_two_tasks(conn, tmp_path):
    seed(conn, repo(tmp_path, "one"), repo(tmp_path, "two"))
    projects = board.board(conn)["projects"]
    assert [p["project"] for p in projects] == ["one", "two"]
    assert all("HUN-1" in by_id(p) for p in projects)


def test_workspace_scope_keeps_only_its_projects(conn, tmp_path):
    a, b = repo(tmp_path, "alpha"), repo(tmp_path, "beta")
    seed(conn, a, b)
    workspaces.add(conn, "acme", repos=[str(a)])
    assert [p["project"] for p in board.board(conn, workspace="acme")["projects"]] == ["alpha"]
    with pytest.raises(ValueError):
        board.board(conn, workspace="nope")


# AC5: read-only

def test_nothing_is_written(conn, tmp_path):
    a = repo(tmp_path, "alpha", FULL, {"board": "linear"})
    seed(conn, a)
    before = {p: (p.stat().st_mtime_ns, p.read_bytes()) for p in (tmp_path / "repos").rglob("*") if p.is_file()}
    board.board(conn)
    after = {p: (p.stat().st_mtime_ns, p.read_bytes()) for p in (tmp_path / "repos").rglob("*") if p.is_file()}
    assert before == after
    assert {p for p in (tmp_path / "repos").rglob("*") if p.is_file()} == set(before)  # no file created or removed


# AC6 / NFR-010: 50 projects x 60 tasks

def test_fifty_projects_of_sixty_tasks_read_in_under_a_second(conn, tmp_path):
    rows = "".join(f"- [ ] **HUN-{n}** — Task {n} with `code` and a [link](http://x.test) | Agent | C2 | Risk: Low | P1\n" for n in range(60))
    text = "## Board\n### Todo\n" + rows
    roots = [repo(tmp_path, f"proj{n:02d}", text, {"board": "linear"}) for n in range(50)]
    seed(conn, *roots)
    start = time.perf_counter()
    result = board.board(conn)
    elapsed = time.perf_counter() - start
    assert sum(len(p["tasks"]) for p in result["projects"]) == 3000
    assert elapsed < 1.0
