"""Memory graph (FR-041): memory notes and their `[[links]]`, read live and never stored.

Sources: `<repo>/memory/**/*.md` (kit memory) of every stored project, and
`<account>/projects/*/memory/**/*.md` (Claude memory) of every confirmed Claude account. Only the
frontmatter `name` and `description` lines, the relative path and the `[[link]]` targets leave this
module; a note body is scanned for link targets and dropped. The easy-memory layer (FR-051..053, `core/easymem.py`)
adds cited-file nodes and a panel block from the easy-memory store, by the same rules. Read-only: nothing is
written anywhere.
Core code: it must not import connectors.
"""

import json
import os
import re
import sqlite3
from pathlib import Path

from easy_personal.core import accounts, discovery, easymem, kit

MAX_NOTE_BYTES = 1024 * 1024  # a bigger note is abstained, never read
MAX_FILES = 5000  # per memory folder
MAX_TEXT = 200
_LINK = re.compile(r"\[\[([^\[\]|#\n]+?)(?:[|#][^\[\]\n]*)?\]\]")  # [[name]] [[name|alias]] [[name#anchor]]
_BLOCK_SCALAR = {"", ">", "|", ">-", "|-", ">+", "|+"}
_CWD_SCAN_LINES = 200
_CWD_SCAN_FILES = 3
MEMORY = "memory"


def _clean(value: str) -> str:
    """One line, quotes removed, at most MAX_TEXT characters; block scalars give nothing."""
    value = value.strip()
    if value in _BLOCK_SCALAR:
        return ""
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        value = value[1:-1]
    return " ".join(value.split())[:MAX_TEXT]


def _frontmatter(lines: list[str]) -> dict[str, str]:
    """`name` and `description` from a leading `---` block; no YAML parser, other keys are never kept."""
    found: dict[str, str] = {}
    if not lines or lines[0].strip() != "---":
        return found
    for line in lines[1:]:
        if line.strip() == "---":
            return found
        key, sep, value = line.partition(":")
        if sep and key in ("name", "description"):
            found[key] = _clean(value)
    return {}  # no closing fence: not frontmatter


def _read_note(real: Path) -> dict | None:
    """`{name, description, links}` of one note, or None when it is abstained (too big, unreadable, not UTF-8)."""
    try:
        if real.stat().st_size > MAX_NOTE_BYTES:
            return None
        text = real.read_bytes().decode("utf-8")
    except (OSError, UnicodeDecodeError):
        return None
    meta = _frontmatter(text.split("\n", 200)[:200])
    links = [target.strip() for target in _LINK.findall(text)]
    return {"name": meta.get("name", ""), "description": meta.get("description", ""), "links": links}


def _markdown(memory_dir: Path) -> tuple[list[tuple[str, Path]], bool]:
    """`(path relative to the memory folder, real path)` of its `.md` files, sorted; plus a cap flag.

    The folder itself may be a symlink. Symlinked subfolders are not entered (no loops) and a
    symlinked file that points outside the folder is skipped.
    """
    try:
        root = memory_dir.resolve(strict=True)
    except OSError:
        return [], False
    found: list[tuple[str, Path]] = []
    for here, dirs, files in os.walk(root, followlinks=False):
        dirs.sort()
        for name in sorted(files):
            if not name.endswith(".md"):
                continue
            real = Path(os.path.realpath(Path(here) / name))
            if not real.is_relative_to(root) or not real.is_file():
                continue
            found.append(((Path(here) / name).relative_to(root).as_posix(), real))
            if len(found) >= MAX_FILES:
                return found, True
    return found, False


def _encoded(path: str) -> str:
    """Claude Code's folder name for a cwd: every non-alphanumeric character becomes `-`."""
    return re.sub(r"[^A-Za-z0-9]", "-", path)


def _transcript_project(folder: Path) -> str | None:
    """Project of a Claude folder from the `cwd` of its first transcripts, resolved as ingest does."""
    try:
        files = sorted(p for p in folder.iterdir() if p.suffix == ".jsonl")[:_CWD_SCAN_FILES]
    except OSError:
        return None
    for path in files:
        try:
            with path.open(encoding="utf-8") as handle:
                for _, line in zip(range(_CWD_SCAN_LINES), handle, strict=False):
                    try:
                        record = json.loads(line)
                    except ValueError:
                        continue
                    cwd = record.get("cwd") if isinstance(record, dict) else None
                    if isinstance(cwd, str) and cwd:
                        return discovery.resolve_project(cwd)
        except (OSError, UnicodeDecodeError):
            continue
    return None


def _sources(conn: sqlite3.Connection, stored: list[str] | None = None) -> dict[str, dict]:
    """Project key -> `{"kit": memory folder or None, "claude": [(account id, label, memory folder)], "em": easy-memory folder or None}`.

    The key is the stored project path, or the Claude folder name when that folder maps to no project.
    A stored project with no memory folder is listed when its easy-memory store folder holds a note file.
    Folder checks only: no note is opened here. `stored` is the stored project list, when the caller already read it.
    """
    stored = kit.scoped_projects(conn) if stored is None else stored
    out: dict[str, dict] = {}
    for path in stored:
        if (Path(path) / MEMORY).is_dir():
            out.setdefault(path, {"kit": Path(path) / MEMORY, "claude": [], "em": None})
    by_folder_name = {_encoded(p): p for p in reversed(stored)}
    for account in accounts.list_accounts(conn):
        if account["provider"] != "claude" or account["status"] != "confirmed":
            continue
        try:
            folders = sorted((Path(account["folder"]) / "projects").iterdir())
        except OSError:
            continue
        for folder in folders:
            if not (folder / MEMORY).is_dir():
                continue
            key = by_folder_name.get(folder.name) or _transcript_project(folder) or folder.name
            entry = out.setdefault(key, {"kit": None, "claude": [], "em": None})
            entry["claude"].append((account["id"], account["label"], folder / MEMORY))
    store = easymem.load()
    for key in sorted(set(out) | set(stored)):
        folder = easymem.find(store, key) if key.startswith("/") else None
        if folder is None:
            continue
        if key in out:
            out[key]["em"] = folder
        elif easymem.has_notes(folder):
            out[key] = {"kit": None, "claude": [], "em": folder}
    return out


def _files(source: dict) -> tuple[list[dict], bool]:
    """Memory files of one project, each read once even when symlinks repeat it, in a stable order."""
    entries: list[dict] = []
    seen: set[Path] = set()
    capped = False
    folders = [("kit", None, None, source["kit"])] if source["kit"] else []
    folders += [("claude", i, label, folder) for i, label, folder in sorted(source["claude"], key=lambda c: c[0])]
    for kind, account_id, label, folder in folders:
        files, folder_capped = _markdown(folder)
        capped = capped or folder_capped
        for rel, real in files:
            if real in seen:
                continue
            seen.add(real)
            entries.append({"kind": kind, "account_id": account_id, "account": label, "rel": f"{MEMORY}/{rel}", "real": real})
    return entries, capped


def _display_name(key: str) -> str:
    return (Path(key).name or key) if key.startswith("/") else key


def project_list(conn: sqlite3.Connection) -> dict:
    """Projects that have at least one memory note: id, folder name and the note counts per origin.

    A listed project whose stored folder is gone (FR-054) is left out and counted in the top-level
    `gone`; a Claude-only key (a Claude folder that maps to no stored project) is kept.
    """
    stored = kit.scoped_projects(conn)
    stored_set, rows, gone = set(stored), [], 0
    for key, source in sorted(_sources(conn, stored).items()):
        files, _ = _files(source)
        kit_notes = sum(f["kind"] == "kit" for f in files)
        em_notes = easymem.note_count(source["em"]) if source["em"] else 0
        if not (files or em_notes):
            continue
        if key in stored_set and not Path(key).is_dir():
            gone += 1
            continue
        rows.append({
            "project": key, "name": _display_name(key), "kit_notes": kit_notes,
            "claude_notes": len(files) - kit_notes, "easy_memory_notes": em_notes,
        })  # fmt: skip
    return {"projects": rows, "gone": gone}


def _ring(entry: dict) -> int:
    """Layout ring: 0 kit MEMORY.md, 1 cold files and the Claude index, 2 learning records and Claude notes."""
    name = Path(entry["rel"]).name
    if entry["kind"] == "kit":
        if entry["rel"] == f"{MEMORY}/MEMORY.md":
            return 0
        return 1 if entry["rel"].count("/") == 1 else 2
    return 1 if name == "MEMORY.md" else 2


def _cited_nodes(nodes: list[dict], cited: list[dict]) -> list[dict]:
    """Cited files as ring 4 nodes. One that is also a kit memory node adds its three fields to that node instead.

    A cited path is relative to the repo, so it can only be a kit note: a Claude note's `memory/...` path is
    relative to its account folder and names another file even when the text is the same.
    """
    by_path: dict[str, list[dict]] = {}
    for node in nodes:
        if node["kind"] == "kit":
            by_path.setdefault(node["path"], []).append(node)
    extra = []
    for item in cited:
        fields = {"cited": item["cited"], "newest": item["newest"], "stale": item["stale"]}
        if item["path"] in by_path:
            for node in by_path[item["path"]]:
                node.update(fields)
            continue
        name = Path(item["path"]).name
        extra.append({
            "id": f"easymem:{item['path']}", "kind": "easy_memory", "ring": 4, "name": name, "stem": Path(name).stem,
            "description": "", "path": item["path"], "account": None, "abstained": False,
            "links_in": 0, "links_out": 0, **fields,
        })  # fmt: skip
    return extra


def project_graph(conn: sqlite3.Connection, project: str) -> dict:
    """Nodes and edges of one project's memory notes. An unknown project raises ValueError."""
    source = _sources(conn).get(project)
    if source is None:
        raise ValueError("unknown project")
    files, capped = _files(source)
    nodes: list[dict] = []
    links: list[list[str]] = []
    for entry in files:
        note = _read_note(entry["real"])
        stem = Path(entry["rel"]).stem
        prefix = "kit" if entry["kind"] == "kit" else f"claude:{entry['account_id']}"
        nodes.append({
            "id": f"{prefix}:{entry['rel']}",
            "kind": entry["kind"],
            "ring": _ring(entry),
            "name": (note and note["name"]) or stem,
            "stem": stem,
            "description": note["description"] if note else "",
            "path": entry["rel"],
            "account": entry["account"],
            "abstained": note is None,
            "links_in": 0,
            "links_out": 0,
        })  # fmt: skip
        links.append(note["links"] if note else [])

    by_name: dict[str, str] = {}
    by_stem: dict[str, str] = {}
    duplicates = 0
    for node in nodes:
        if node["abstained"]:
            continue
        key = node["name"].casefold()
        if key in by_name:
            duplicates += 1
        else:
            by_name[key] = node["id"]
        by_stem.setdefault(node["stem"].casefold(), node["id"])

    notes = {n["id"]: n for n in nodes}
    dangling: dict[str, dict] = {}
    edges: set[tuple[str, str]] = set()
    for node, targets in zip(list(nodes), links, strict=True):
        for target in targets:
            key = target.casefold()
            found = by_name.get(key) or by_stem.get(key)
            if found is None:
                found = f"dangling:{key}"
                dangling.setdefault(found, {
                    "id": found, "kind": "dangling", "ring": 3, "name": target, "stem": target,
                    "description": "", "path": None, "account": None, "abstained": False,
                    "links_in": 0, "links_out": 0,
                })  # fmt: skip
            if found != node["id"]:
                edges.add((node["id"], found))
    notes.update(dangling)
    for source_id, target_id in edges:
        notes[source_id]["links_out"] += 1
        notes[target_id]["links_in"] += 1
    panel, cited_nodes = None, []
    if source["em"]:
        panel, cited = easymem.summary(source["em"], project)
        cited_nodes = _cited_nodes(nodes, cited)
    return {
        "project": project,
        "name": _display_name(project),
        "nodes": nodes + sorted(dangling.values(), key=lambda n: n["id"]) + cited_nodes,
        "edges": [{"source": s, "target": t} for s, t in sorted(edges)],
        "counts": {
            "notes": len(nodes),
            "links": len(edges),
            "dangling": len(dangling),
            "abstained": sum(n["abstained"] for n in nodes),
            "duplicates": duplicates,
            "capped": capped,
        },
        "easy_memory": panel,
    }
