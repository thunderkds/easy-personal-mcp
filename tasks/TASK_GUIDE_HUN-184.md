**Date**: 2026-11-02
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Memory note, easy-memory note, Cited file, Abstained; Critical Constraints: read-only, metadata only, DDR-0002)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `BRAINSTORMING_LOG_EASYMEMORY.md` (Option A) and `PRD.md` US-024, FR-041, FR-051, FR-052, FR-053, NFR-002, NFR-003, NFR-010, Open Questions 18–19

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-11-02: "show memory visualize from easy-memory mcp with the kit family" → owner locked the shape in a grill (cited files as nodes, stale = newest citing note's hash ≠ disk, top 30, 30-day panel, list easy-memory-only projects) and chose Option A (one graph response).

**Restated intent**:

> Add `src/easy_personal/core/easymem.py`, a read-only, metadata-only reader of the easy-memory store (`$EASY_MEMORY_HOME`, else `~/.easy-memory/`). For a project it finds the store folder (by `meta.json` `path`, else by the computed easy-memory key), keeps only each note's `time` and `files` and each `prompts.log` line's `time` / `effective` / `injected`, and returns citation counts, the newest hash per cited file, notes per local day for 30 days and the 30-day hit rate. `memgraph.project_graph` adds up to 30 `easy_memory` cited-file nodes (or adds the fields to an existing kit / Claude node with the same path) and an `easy_memory` panel block; `memgraph.project_list` adds `easy_memory_notes` and lists easy-memory-only projects. Same route, no new route.

**Out of scope**:

* The page (next task)
* Any note text, `session_id`, `trash/`, the easy-memory MCP, any write to the store
* Storing anything (no migration), any cache (only if NFR-010 is measured to miss)
* Any change in `../easy-memory-mcp`

**Requirement Refs**: US-024, FR-051, FR-052, FR-053, NFR-002, NFR-003, NFR-010

### Shared rules (easy-memory layer)

* Direction: `BRAINSTORMING_LOG_EASYMEMORY.md` Option A and `memory/decisions.md` "easy-memory layer = Option A". Core code: `easymem.py` imports stdlib only and never imports connectors.
* Metadata only (NFR-002): `prompt`, `answer` and `session_id` may be parsed by `json.loads` but must never be kept in a returned value, a log line or an exception message. Extend the leak suite (`tests/leak/harness.py`) with a fixture store whose notes carry `INVENTED_` prompt / answer / session text and an `INVENTED_` file in `trash/`.
* Cited files are opened only to hash them (SHA-256, streamed), only inside the project folder (`realpath` check), at most 10 MB each; nothing of their content is kept.
* Never use `git stash` (shared across worktrees); refresh `.claude/hooks/.state/active_task` right before every full test run.

### Response contract (the frontend task builds on this)

* `project_list` rows: add `easy_memory_notes` (int, 0 when none). A project with only easy-memory notes is listed with `kit_notes: 0`, `claude_notes: 0`.
* `project_graph`:
  * new node kind `easy_memory`: `id` = `easymem:<path>`, `ring` = 4, `name` = file basename, `stem`, `description` = "", `path` = the repo-relative path, `account` = null, `abstained` = false, `links_in` / `links_out` = 0, plus `cited` (int), `newest` (`YYYY-MM-DD`, local), `stale` (`"fresh"` | `"stale"` | `"gone"` | `"unchecked"`).
  * a kit / Claude node whose `path` equals a cited path gets the same three fields (`cited`, `newest`, `stale`) and keeps every other field unchanged; no second node is drawn.
  * top-level `easy_memory`: `null` when the project has no store folder, else `{notes, abstained, days: [{date, count}] × 30 oldest first, prompts_on, injected, files_cited, files_not_drawn, outside}` (`outside` = cited paths not opened because they resolve outside the project; counted in `files_not_drawn`).

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved grill + Option A, 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: none (builds on HUN-164 memory graph, merged)

**Entry point**: `easy-personal dashboard` → `GET /api/memory-graph` (`surfaces/api.py` `_memory_graph`) → `core/memgraph.py` → `core/easymem.py`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `EASY_MEMORY_HOME` set → that folder is read; unset or empty → `~/.easy-memory`; folder missing → `easy_memory: null`, the graph is otherwise unchanged, no error | FR-051 |
| 2 | A store folder with `meta.json` `path` = `<repo>/.git` (also via a symlink) belongs to `<repo>`; a folder with no `meta.json` whose name equals the computed key (git common dir, a worktree's `.git` file followed, else real path; no `git` subprocess) gets its `prompts.log` counted for that project | FR-051 |
| 3 | A note that is not a dict, has `v` missing / bool / > 1, bad JSON, non-UTF-8, or `files` not a dict → counted in `abstained`, never in `notes` | FR-051 |
| 4 | The 30 most-cited paths become nodes (ties by path); the 31st onward and `outside` paths count in `files_not_drawn` | FR-052 |
| 5 | `stale`: hash equal to the newest citing note → `fresh`; different → `stale`; file missing → `gone`; > 10 MB, a directory or unreadable → `unchecked` | FR-052 |
| 6 | A cited path that is absolute, contains `..` or resolves through a symlink outside the project is never opened (assert with a file outside that would raise if read) and counts in `outside` | FR-052 |
| 7 | A cited path equal to a kit node's `path` (e.g. `memory/MEMORY.md`) adds `cited` / `newest` / `stale` to that node; node count and every other field of that node are unchanged | FR-052 |
| 8 | `days` has exactly 30 entries in local time (DST-aware); a note `time` that does not parse counts in `notes` but in no day; notes older than 30 days count in `notes` only | FR-053 |
| 9 | Hit rate: `prompts_on` = last-30-day `prompts.log` lines with `effective` `on`; `injected` = those with `injected` true; bad lines skipped; 0 prompts → `prompts_on: 0` | FR-053 |
| 10 | `project_list` includes an easy-memory-only project and every row has `easy_memory_notes` | FR-053 |
| 11 | `INVENTED_` prompt / answer / session text and `trash/` content never appear in any API response, log or exception text (leak suite) | NFR-002 |
| 12 | Nothing under the store or any repo is written (mtimes + file list unchanged after a call) | NFR-003 |
| 13 | Real run on the owner's machine: `/api/memory-graph` list and this repo's graph each return in < 1 s; for the 50-project list < 1 s (NFR-010); paste times and the `easy_memory` block for this repo | NFR-010, US-024 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core -q -k "easymem or memgraph" && uv run pytest tests/surfaces -q -k memory_graph && uv run pytest tests/leak -q
```

## Approach

**Vital slice**: store folder lookup → note + log reader (metadata only) → counts / newest hash / days / hit rate → merge into `project_graph` and `project_list`.
**Cut list**: no cache; no per-account split of easy-memory; no config key for the store path beyond `EASY_MEMORY_HOME`; no edges for cited-file nodes.

`easymem.py` exposes plain functions returning dicts (no classes needed). Hash each cited file once per call. `memgraph._sources` must learn easy-memory-only projects (stored project paths whose store folder has notes).

---

## Edge Case Checklist

- [ ] `EASY_MEMORY_HOME` empty string → default
- [ ] Store folder or `notes/` unreadable → `easy_memory: null` or counts 0, never an exception to the page
- [ ] `meta.json` missing (50 of 84 folders today hold only `prompts.log`) → match by computed key
- [ ] Worktree project path (`.git` is a file with `gitdir:`) → its common dir
- [ ] `meta.json` `path` through symlinks → compare `realpath`
- [ ] Bad note shapes → abstained
- [ ] Cited path absolute / `..` / symlink escape → never opened
- [ ] Cited file gone / directory / unreadable / > 10 MB
- [ ] Citation ties → order by path
- [ ] Cited path equal to a kit or Claude node path → one node
- [ ] Local-day buckets across a DST change; unparsable `time`
- [ ] `prompts.log` large or with bad lines → line by line, skip bad
- [ ] `trash/` present → never listed or read
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/easymem.py` (new) | store reader, metadata only |
| `src/easy_personal/core/memgraph.py` | merge nodes, `easy_memory` block, list count + easy-memory-only projects |
| `tests/core/test_easymem.py` (new), `tests/core/test_memgraph.py`, `tests/surfaces/test_api_memory_graph.py`, `tests/leak/harness.py` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/surfaces/api.py` route table | Option A: no new route |
| `src/easy_personal/core/store.py`, `core/migrations/` | Nothing is stored |
| `src/easy_personal/sources/` | easy-memory is not an ingest source |
| `web/` | Next task |
| `~/.easy-memory/**`, `../easy-memory-mcp/**` | Read-only dependency; another repo |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Temp-dir fixture store (`EASY_MEMORY_HOME`) + temp project repos (one normal, one worktree, one easy-memory-only); notes with planted `INVENTED_` text; cited files that are fresh, changed, gone, oversize, outside via symlink; a fake clock / fixed "today" for the 30 days; assert the contract fields; leak suite run; mtime check for no writes; real-run timing on the owner's store.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review