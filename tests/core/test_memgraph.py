"""Memory graph reader (HUN-164, FR-041): names + [[links]] read live, bodies never returned."""

import json
import os
import re
import time

import pytest

from easy_personal.core import accounts, config, memgraph
from tests_helpers import add_event

SECRET = "SECRET_Y_body_text"


def note(path, name=None, description=None, body=""):
    path.parent.mkdir(parents=True, exist_ok=True)
    head = ""
    if name is not None or description is not None:
        head = "---\n"
        if name is not None:
            head += f"name: {name}\n"
        if description is not None:
            head += f"description: {description}\n"
        head += "---\n"
    path.write_text(head + body, encoding="utf-8")
    return path


def encoded(path) -> str:
    return re.sub(r"[^A-Za-z0-9]", "-", str(path))


@pytest.fixture
def world(conn, tmp_path):
    """A kit repo (events in the store), one confirmed Claude account with a note for that repo."""
    repo = tmp_path / "work" / "alpha"
    mem = repo / "memory"
    note(mem / "MEMORY.md", "memory-index", "Hot index", f"- [[decisions]]\n- [[learnings]] {SECRET}\n")
    note(mem / "decisions.md", "decisions", "Why we chose things", "see [[glossary|the glossary]] and [[ghost]]\n")
    note(mem / "glossary.md", None, None, "no frontmatter, links [[decisions#2026]] and [[glossary]]\n")
    note(mem / "learnings.md", "learnings", "Gotchas", "[[LR-0001]]\n")
    note(mem / "learning-records" / "LR-0001.md", "LR-0001", "A record", "back to [[learnings]]\n")
    note(mem / "learning-records" / "LR-0002.md", "LR-0002", "Another", "\n")

    root = config.agent_root()
    folder = root / ".claude-work"
    claude_mem = folder / "projects" / encoded(repo) / "memory"
    note(claude_mem / "feedback.md", "feedback-tone", "How the owner likes answers", "uses [[decisions]]\n")
    accounts.detect(conn)
    accounts.confirm(conn, str(folder), "Claude work")
    add_event(conn, 1, str(repo))
    conn.commit()
    return {"repo": repo, "mem": mem, "claude_mem": claude_mem, "folder": folder}


def graph(conn, repo):
    return memgraph.project_graph(conn, str(repo))


def by_path(g):
    return {n["path"]: n for n in g["nodes"] if n["path"]}


def test_nodes_and_edges_of_a_project(conn, world):
    g = graph(conn, world["repo"])
    names = sorted(n["name"] for n in g["nodes"])
    # 4 cold + 2 LRs + 1 Claude note + 1 dangling = 8 (glossary has no frontmatter -> stem)
    assert names == ["LR-0001", "LR-0002", "decisions", "feedback-tone", "ghost", "glossary", "learnings", "memory-index"]
    assert sum(n["kind"] == "dangling" for n in g["nodes"]) == 1
    ids = {n["name"]: n["id"] for n in g["nodes"]}
    edges = {(e["source"], e["target"]) for e in g["edges"]}
    assert (ids["memory-index"], ids["decisions"]) in edges
    assert (ids["decisions"], ids["glossary"]) in edges  # [[glossary|alias]]
    assert (ids["glossary"], ids["decisions"]) in edges  # [[decisions#anchor]]
    assert (ids["decisions"], ids["ghost"]) in edges
    assert (ids["feedback-tone"], ids["decisions"]) in edges
    assert (ids["LR-0001"], ids["learnings"]) in edges and (ids["learnings"], ids["LR-0001"]) in edges  # cycle ok
    assert g["counts"]["dangling"] == 1


def test_self_links_are_ignored(conn, world):
    g = graph(conn, world["repo"])
    assert all(e["source"] != e["target"] for e in g["edges"])
    glossary = by_path(g)["memory/glossary.md"]
    assert glossary["links_out"] == 1  # only decisions; [[glossary]] is itself


def test_link_forms_alias_and_anchor(conn, world):
    g = graph(conn, world["repo"])
    names = {n["name"] for n in g["nodes"]}
    assert not any("|" in n or "#" in n for n in names)


def test_note_without_frontmatter_uses_stem(conn, world):
    node = by_path(graph(conn, world["repo"]))["memory/glossary.md"]
    assert node["name"] == "glossary" and node["stem"] == "glossary" and node["description"] == ""


def test_in_and_out_counts(conn, world):
    nodes = by_path(graph(conn, world["repo"]))
    dec = nodes["memory/decisions.md"]
    assert dec["links_in"] == 3  # memory-index, glossary, feedback-tone
    assert dec["links_out"] == 2  # glossary, ghost


def test_rings(conn, world):
    nodes = by_path(graph(conn, world["repo"]))
    assert nodes["memory/MEMORY.md"]["ring"] == 0
    assert nodes["memory/decisions.md"]["ring"] == 1
    assert nodes["memory/learning-records/LR-0001.md"]["ring"] == 2
    claude = next(n for n in graph(conn, world["repo"])["nodes"] if n["kind"] == "claude")
    assert claude["ring"] == 2 and claude["account"] == "Claude work"
    dangling = next(n for n in graph(conn, world["repo"])["nodes"] if n["kind"] == "dangling")
    assert dangling["ring"] == 3 and dangling["path"] is None


def test_paths_are_relative(conn, world):
    g = graph(conn, world["repo"])
    for n in g["nodes"]:
        assert n["path"] is None or not os.path.isabs(n["path"])
    claude = next(n for n in g["nodes"] if n["kind"] == "claude")
    assert claude["path"] == "memory/feedback.md"


def test_response_never_holds_note_bodies(conn, world):
    text = json.dumps(graph(conn, world["repo"]))
    assert SECRET not in text
    text = json.dumps(memgraph.project_list(conn))
    assert SECRET not in text


def test_description_is_truncated_and_single_line(conn, world):
    note(world["mem"] / "long.md", "long", "x" * 500)
    note(world["mem"] / "folded.md", "folded", ">\n  second line SECRET_FOLD")
    nodes = by_path(graph(conn, world["repo"]))
    assert len(nodes["memory/long.md"]["description"]) <= 200
    assert "SECRET_FOLD" not in json.dumps(nodes["memory/folded.md"])


def test_duplicate_names_both_nodes_links_go_to_first_path(conn, world):
    note(world["mem"] / "a" / "dup.md", "twin", "first", "")
    note(world["mem"] / "b" / "dup.md", "twin", "second", "[[twin]]")
    note(world["mem"] / "linker.md", "linker", "", "[[twin]]")
    g = graph(conn, world["repo"])
    twins = [n for n in g["nodes"] if n["name"] == "twin"]
    assert len(twins) == 2 and g["counts"]["duplicates"] == 1
    first = by_path(g)["memory/a/dup.md"]
    assert first["links_in"] == 2  # linker + the second twin's link resolves to the first
    assert by_path(g)["memory/b/dup.md"]["links_in"] == 0


def test_abstained_files(conn, world):
    (world["mem"] / "bad.md").write_bytes(b"\xff\xfe not utf8 [[ghost2]]")
    (world["mem"] / "huge.md").write_text("[[ghost3]]" + "x" * (1024 * 1024 + 1))
    g = graph(conn, world["repo"])
    nodes = by_path(g)
    assert nodes["memory/bad.md"]["abstained"] and nodes["memory/huge.md"]["abstained"]
    assert g["counts"]["abstained"] == 2
    assert not any(n["name"] in ("ghost2", "ghost3") for n in g["nodes"])


def test_symlink_loops_and_repeats_are_read_once(conn, world):
    (world["mem"] / "loop").symlink_to(world["mem"], target_is_directory=True)
    (world["mem"] / "decisions-copy.md").symlink_to(world["mem"] / "decisions.md")
    (world["mem"] / "escape.md").symlink_to(world["repo"].parent / "outside.md")
    note(world["repo"].parent / "outside.md", "outside", "not in memory", "")
    g = graph(conn, world["repo"])
    assert sum(n["name"] == "decisions" for n in g["nodes"]) == 1
    assert not any(n["name"] == "outside" for n in g["nodes"])


def test_symlinked_memory_dir_is_followed_once(conn, world, tmp_path):
    real = tmp_path / "elsewhere"
    note(real / "extra.md", "extra", "via symlinked dir", "")
    (world["mem"] / "link-to-real").symlink_to(real, target_is_directory=True)
    names = {n["name"] for n in graph(conn, world["repo"])["nodes"]}
    assert "extra" not in names  # inside-the-memory symlinked dirs are not followed
    # but the memory dir itself may be a symlink
    repo2 = tmp_path / "work" / "beta"
    repo2.mkdir(parents=True)
    (repo2 / "memory").symlink_to(real, target_is_directory=True)
    add_event(conn, 2, str(repo2))
    conn.commit()
    assert {n["name"] for n in memgraph.project_graph(conn, str(repo2))["nodes"]} == {"extra"}


def test_unmapped_claude_folder_shows_under_its_folder_name(conn, world):
    folder = world["folder"] / "projects" / "-nowhere-else" / "memory"
    note(folder / "orphan.md", "orphan-note", "lonely", "")
    listed = {p["project"]: p for p in memgraph.project_list(conn)["projects"]}
    assert "-nowhere-else" in listed and listed["-nowhere-else"]["name"] == "-nowhere-else"
    g = memgraph.project_graph(conn, "-nowhere-else")
    assert [n["name"] for n in g["nodes"]] == ["orphan-note"]


def test_claude_folder_mapped_through_transcript_cwd(conn, world):
    """A worktree folder maps to the repo through the transcript's cwd, like ingest does."""
    wt = world["folder"] / "projects" / "-some-worktree-folder"
    note(wt / "memory" / "wt-note.md", "wt-note", "from a worktree", "")
    (wt / "s1.jsonl").write_text(json.dumps({"type": "user", "cwd": str(world["repo"])}) + "\n")
    names = {n["name"] for n in graph(conn, world["repo"])["nodes"]}
    assert "wt-note" in names


def test_ignored_account_notes_are_not_read(conn, world):
    accounts.ignore(conn, str(world["folder"]))
    names = {n["name"] for n in graph(conn, world["repo"])["nodes"]}
    assert "feedback-tone" not in names


def test_project_list(conn, world):
    listed = memgraph.project_list(conn)["projects"]
    row = next(p for p in listed if p["project"] == str(world["repo"]))
    assert row["name"] == "alpha" and row["kit_notes"] == 6 and row["claude_notes"] == 1


def test_project_without_memory_is_not_listed(conn, world, tmp_path):
    bare = tmp_path / "bare"
    bare.mkdir()
    add_event(conn, 3, str(bare))
    conn.commit()
    assert all(p["project"] != str(bare) for p in memgraph.project_list(conn)["projects"])


def test_unknown_project_is_a_value_error_without_echo(conn, world):
    with pytest.raises(ValueError) as exc:
        memgraph.project_graph(conn, "/etc/passwd")
    assert "passwd" not in str(exc.value)


def test_nothing_is_written(conn, world):
    files = [p for p in world["repo"].rglob("*") if p.is_file()] + list(world["folder"].rglob("*"))
    before = {p: p.stat().st_mtime_ns for p in files if p.is_file()}
    listing = sorted(str(p) for p in world["repo"].parent.rglob("*"))
    graph(conn, world["repo"])
    memgraph.project_list(conn)
    assert {p: p.stat().st_mtime_ns for p in before} == before
    assert sorted(str(p) for p in world["repo"].parent.rglob("*")) == listing


def test_fifty_projects_by_forty_notes_under_one_second(conn, tmp_path):
    root = tmp_path / "many"
    projects = []
    for i in range(50):
        repo = root / f"p{i:02d}"
        for j in range(40):
            note(repo / "memory" / f"n{j:02d}.md", f"p{i}-n{j}", f"note {j}", f"[[p{i}-n{(j + 1) % 40}]] [[missing-{j}]]\n")
        add_event(conn, i, str(repo))
        projects.append(repo)
    conn.commit()
    start = time.perf_counter()
    listed = memgraph.project_list(conn)
    list_time = time.perf_counter() - start
    assert len(listed["projects"]) == 50
    worst = 0.0
    for repo in projects:
        start = time.perf_counter()
        g = memgraph.project_graph(conn, str(repo))
        worst = max(worst, time.perf_counter() - start)
        assert g["counts"]["notes"] == 40
    assert list_time < 1 and worst < 1
