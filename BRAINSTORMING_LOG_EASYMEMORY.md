# BRAINSTORMING_LOG_EASYMEMORY.md
**Generated**: 2026-11-02
**Task / Context**: easy-memory layer in the Memory view — PRD US-024, FR-051..FR-053, Open Questions 18–19
**Skill**: `Skill({ skill: "brainstorming" })` — tier **Standard** (shape already locked in requirement grilling; open choice = where the code lives)

---

## The Problem Space

The Memory view (FR-041, HUN-164) draws kit memory and Claude memory per project. It does not see the easy-memory store (`~/.easy-memory/<key>/`), which holds 861 notes over 34 store folders on the owner's machine (plus 50 folders with only `prompts.log`). The owner wants that store drawn as cited-file nodes with a stale mark, plus a 30-day panel, without any note text.

Non-negotiable constraints:
- Metadata only (PROJECT_SPEC.md Critical Constraints): `prompt`, `answer`, `session_id` never leave core code; `trash/` never read.
- No MCP call to easy-memory (DDR-0002 keeps easy-context as the only runtime plug-in call). Files are read directly.
- Read live, never into the store; read-only (NFR-003).
- NFR-010: memory graph for 50 projects < 1 s.

Verified facts (2026-11-02):
- `src/easy_personal/core/memgraph.py` — `project_list` / `project_graph`, node kinds `kit` / `claude` / `dangling`, rings 0–3 (3 = dangling).
- `src/easy_personal/surfaces/api.py:209` — `_memory_graph` serves both shapes on `GET /api/memory-graph`.
- `web/src/MemoryView.jsx` — hand-made SVG `RingsGraph`, `MAX_DRAWN = 300`, `KIND_NAMES`, golden `web/src/test/memory-graph.golden.svg`.
- `tests/leak/harness.py:109` — `/api/memory-graph` is in the leak suite already.
- easy-memory `repo_key` = `sha256(realpath(git common dir) or realpath(folder))[:16]` (`easy_memory.py:68-84`); `meta.json` `path` = that identity path (e.g. `<repo>/.git`).

## Questions for the User

1. ~~Which path?~~ Resolved 2026-11-02: Option A.

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | One graph response | New `core/easymem.py` reader; `memgraph` merges its nodes and a `easy_memory` panel block into the existing `/api/memory-graph` responses | Medium | ~220 py + ~120 jsx | Medium | ✅ Yes |
| B | Separate route | New `GET /api/easy-memory?project=`; the page loads both and merges nodes in the browser | Medium | ~260 py + ~170 jsx | Low–Medium | |
| C | Panel first | Ship FR-051 + FR-053 (reader + panel + list count) now; cited-file nodes (FR-052) in a second task | Low | ~150 py + ~70 jsx per slice | Low | |

### Option A — One graph response
**Approach**: `core/easymem.py` (core, stdlib only) finds the store folder for a project, reads notes keeping only `time` + `files`, reads `prompts.log` keeping `time` / `effective` / `injected`, and returns cited-file counts, newest hash per file, daily counts and hit rate. `memgraph.project_graph` adds up to 30 `easy-memory` nodes on a new ring 4 (or merges count + stale onto an existing kit/Claude node with the same path) and a `counts.easy_memory` / `easy_memory` panel block. `project_list` adds `easy_memory_notes` and includes easy-memory-only projects. The page adds one kind, one colour, the panel and the sparkline.
**Pros**: one request, one leak-suite route already covered, server owns the merge (one place to test the "same file = same node" rule), matches how FR-041 works.
**Cons**: `memgraph.py` grows; the golden SVG and existing API tests change; a slow store read slows the whole graph.
**Why it might fail**: hashing cited files on every graph load could miss NFR-010 on a repo with large cited files → cap hashed file size and measure. A merge bug could hide a kit node → keep kit node fields untouched, only add fields.

### Option B — Separate route
**Approach**: same reader, own route and response; the page fetches `/api/easy-memory` beside `/api/memory-graph` and merges nodes by path in `MemoryView.jsx`.
**Pros**: FR-041 code and tests untouched; easy-memory failure cannot break the existing graph.
**Cons**: a new route = new leak-suite entry, new guard tests, `API_CALLS` list update (learning from HUN-160..164); merge logic in the browser is harder to test; project list needs merging too (two sources of "which projects exist").
**Why it might fail**: two lists disagree on project keys (kit path vs easy-memory key) and the page shows a project twice.

### Option C — Panel first
**Approach**: split A into two tasks. Slice 1: reader, list count, panel. Slice 2: cited-file nodes + stale hashing in the graph.
**Pros**: smallest first merge; the stale-hash cost is measured before it lands in the graph.
**Cons**: two reviews and two golden updates; the owner's chosen shape (Q1: nodes) arrives later.
**Why it might fail**: slice 2 gets dropped and the feature stays half-built.

---

## 50% Rule Check

Half the code for Option A: drop the separate merge rule and always draw cited files as their own nodes even when a kit node has the same path, and show hit rate as a number without a sparkline. Rejected: the owner chose merging (FR-052) and a sparkline (FR-053). The reasonable saving kept: no layout library, no cache, no new route, reuse `RingsGraph` with one more ring.

---

## Recommended Path

**Option A — One graph response.** It meets FR-051..FR-053 with no new route, keeps the merge rule server-side where it is testable, and follows the FR-041 pattern the leak suite already covers. Task split for Stage 2: backend (reader + graph/list fields) then frontend (kind, ring, panel), so it is still two tracer slices without Option C's half-built risk.

---

## Surgical Scope

Files that **should** be touched:
- `src/easy_personal/core/easymem.py` (new) — store reader, metadata only.
- `src/easy_personal/core/memgraph.py` — merge easy-memory nodes and counts into `project_graph` / `project_list`.
- `web/src/MemoryView.jsx`, `web/src/app.css` — new kind, ring 4, panel with sparkline.
- `tests/core/test_easymem.py` (new), `tests/core/test_memgraph.py`, `tests/surfaces/test_api_memory_graph.py`, `tests/leak/harness.py` (plant `INVENTED_` prompt/answer/session text in a fixture store), `web/src/MemoryView.test.jsx`, `web/src/test/memory-graph.golden.svg`.
- `README.md` Memory view section; `site/index.html` only if a views stat changes.

Files that **must not** be touched:
- `../easy-memory-mcp/**` — another repo, read-only dependency (Open Question 19).
- `src/easy_personal/surfaces/api.py` route table — no new route in Option A.
- `src/easy_personal/core/store.py` / `core/migrations/` — nothing is stored.
- Anything under `~/.easy-memory/` — never written.

---

## Edge Case Checklist for TASK_GUIDE

- [ ] `EASY_MEMORY_HOME` set but empty → default `~/.easy-memory` (same as easy-memory's `store_home`).
- [ ] Store folder missing or unreadable → no easy-memory layer, graph still renders, no error.
- [ ] `meta.json` missing (50 of 84 folders today, `prompts.log` only) → match by computed key; compute git common dir without a `git` subprocess per project (read `<repo>/.git` dir, or the `gitdir:` line of a `.git` file for a worktree) to stay within NFR-010.
- [ ] `meta.json` `path` with symlinks → compare `realpath` on both sides.
- [ ] Note not a dict, `v` missing / bool / > 1, bad JSON, non-UTF-8, `files` not a dict → abstained and counted, never guessed.
- [ ] Cited path absolute, with `..`, or a symlink that resolves outside the project → never opened, counted as not drawn.
- [ ] Cited file gone, a directory, or unreadable → stale (gone) or abstained (unreadable); state which.
- [ ] Large cited file → cap bytes hashed (e.g. skip > 10 MB as "not checked"); measure NFR-010 with the real kit repo (288 notes, 109 files).
- [ ] Ties in citation count → order by path (stable golden SVG).
- [ ] Cited path equal to a kit/Claude node path → one node, kit fields unchanged, easy-memory fields added.
- [ ] Day buckets in local time, DST-aware (as `core/accounts.py:186`); a note `time` that does not parse is counted in total but not in a day.
- [ ] `prompts.log` large or with bad lines → read line by line, skip bad lines; only last 30 days counted.
- [ ] Hit rate with 0 prompts with memory on → show "no prompts", never 0% or a division error.
- [ ] Leak: `prompt`, `answer`, `session_id`, `trash/` content never in any API response, log or page (leak suite with `INVENTED_` markers).
- [ ] `trash/` folder present → never listed or counted.

---

## Next Actions

1. Owner picks a path.
2. Stage 1 checklist for this feature (env already set up; refresh `map-codebase` only if stale).
3. Stage 2: `PROJECT_SPEC.md` update (easy-memory store as a read-only source; one constraint line), Linear/Kanban tasks — backend (Backend-Implementer, C2 / Medium) then frontend (Frontend-Implementer, C2 / Medium, UI Evidence rows), TASK_GUIDEs with the checklist above.

---

## User Selection

> **Approved direction**: Option A — One graph response
> Approved by user on 2026-11-02.
