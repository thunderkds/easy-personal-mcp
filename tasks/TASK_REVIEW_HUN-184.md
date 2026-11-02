# TASK_REVIEW — HUN-184: easy-memory store reader + cited-file nodes and panel data in /api/memory-graph (FR-051..053 backend)

> Sibling of `tasks/TASK_GUIDE_HUN-184.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_easymem.py` (new, 37 tests), `tests/core/test_memgraph.py`, `tests/surfaces/test_api_memory_graph.py`, `tests/leak/test_leak_suite.py` + `canaries.py` / `conftest.py` / `harness.py` |
| Verification command run | ☑ pass | Supervisor rerun after fix f0343d1 (2026-11-02): `uv run pytest tests/core -q -k "easymem or memgraph"` -> `68 passed, 390 deselected`; `uv run pytest tests/surfaces -q -k memory_graph` -> `7 passed, 183 deselected`; `uv run pytest tests/leak -q` -> `33 passed`; `uv run ruff check src tests` -> `All checks passed!` — pass |
| Negative cases hold | ☑ pass | Store missing / `EASY_MEMORY_HOME` empty → `easy_memory: null`, graph unchanged; bad note shapes (not dict, `v` bool / missing / >1, bad JSON, non-UTF-8, `files` missing or not a dict) → abstained; absolute / `..` / symlinked file or folder escaping the repo → never opened (open spy), counted in `outside`; gone / directory / >10 MB → `gone` / `unchecked`; `trash/` never listed; nothing written (mtime check). Stage 4 added: a cited `memory/feedback.md` that exists only as a Claude note is NOT stamped on the Claude node (test fails before fix f0343d1) — pass |
| verify | ☑ pass | Stage 5 2026-11-02 @ 5cf8d0b, `/verify` recipe: page built, temp `uv tool install` (`0.3.0`, `easy-personal core ok`), real `easy-personal ingest` of a fixture transcript (cwd = fixture repo) and real `easy-personal dashboard` on temp `EASY_PERSONAL_HOME`, fixture `EASY_MEMORY_HOME` (2 notes + `trash/` note + 3-line `prompts.log`, all planted with a canary), `GET /api/memory-graph` over HTTP with the dashboard token (`/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/v184/drive.py`): PASS list row `easy_memory_notes: 2` (28 ms); PASS `easymem:src/app.py` cited 2 `stale`; PASS `easymem:gone.py` `gone`; PASS `memory/decisions.md` merged into the kit node, `fresh`, no second node; PASS `../../etc/passwd` not drawn, `outside: 1`; PASS panel `notes 2`, 30 days, today 2, `prompts_on 3`, `injected 2`; PASS no canary in any response; PASS store mtimes unchanged; PASS graph 23 ms (< 1 s); dashboard exit 0. Owner-store real run (AC 13, agent): list 204 ms, this repo graph 166 ms — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `core/easymem.py` (full), `core/memgraph.py` diff, the route caller `surfaces/api.py` `_memory_graph` (unchanged), and the test/leak changes. `lane: C2, 10 files, 929 lines, EXCEEDS (lines 929 > 500); unpredicted: tests/conftest.py, tests/leak/canaries.py, tests/leak/conftest.py, tests/leak/test_leak_suite.py` — ~600 lines are tests and the unpredicted files are test-only; kept C2. Startup reads: all 3 Permanent-Rule reads present. Entry point `/api/memory-graph` reachable (route unchanged, calls `memgraph`). Stage 4 code-review: P1 cited path merged into a Claude note with the same relative `memory/...` path (wrong count/stale on another file) — fixed f0343d1 + regression test; P0/P2/P3 none. Security pass (manual; built-in `security-review` cannot run: remote is `github`, no `origin/HEAD`): metadata only (`_load_note` keeps `time` + `files`; `prompt`/`answer`/`session_id` never returned, logged or in exceptions; leak suite plants 4 `INVENTED_easymem-*` canaries incl. `trash/`, none in any output); path containment by `realpath` + `..`/absolute refusal, tested with an open spy; size caps (note 1 MB, meta 64 KB, log line 64 KB, hash 10 MB); regular files only; JSON `RecursionError` caught; read-only. Accepted: symlink swap between `realpath` and `open` needs write access to the owner's own repo (local single-user). No findings — pass |
| Full smoke suite still green (no regression) | ☑ pass | After fix f0343d1: `uv run pytest -q` -> `1121 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 151.66s` — pass |
| **Companion score (easy-verifier)** | ☑ N/A | Not run: learnings 2026-11-01 — easy-verifier refused every repo path |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Backend only; the page is HUN-185 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Backend only; the page is HUN-185 |
| **UI: Responsiveness at target viewports** | ☑ N/A | Backend only; the page is HUN-185 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-11-02T04:35:54Z, `uv run python <scratchpad>/demo.py` on main @ 16eb0c2; fixture = temp repo `alpha` with `memory/MEMORY.md`, plus a temp easy-memory store holding one note citing `src/a.py` and `memory/MEMORY.md`, one prompts.log line, and one easy-memory-only folder):
```
{"projects": [{"project": "<tmp>/alpha", "name": "alpha", "kit_notes": 1, "claude_notes": 0}]}
{"project": "<tmp>/alpha", "name": "alpha",
 "nodes": [{"id": "kit:memory/MEMORY.md", "kind": "kit", "ring": 0, "name": "index", "stem": "MEMORY", "description": "hot",
            "path": "memory/MEMORY.md", "account": null, "abstained": false, "links_in": 0, "links_out": 0}],
 "edges": [], "counts": {"notes": 1, "links": 0, "dangling": 0, "abstained": 0, "duplicates": 0, "capped": false}}
```
No `easy_memory_notes` on the list row, no `cited`/`newest`/`stale` on the node, no `easy_memory` block, no cited-file node for `src/a.py`, easy-memory-only project not listed.

**AFTER** (2026-11-02T04:44:13Z, same command on branch HUN-184; `days` shortened to its non-zero entry):
```
{"projects": [{"project": "<tmp>/alpha", "name": "alpha", "kit_notes": 1, "claude_notes": 0, "easy_memory_notes": 1}]}
{"project": "<tmp>/alpha", "name": "alpha",
 "nodes": [{"id": "kit:memory/MEMORY.md", "kind": "kit", "ring": 0, ..., "links_out": 0, "cited": 1, "newest": "2026-11-01", "stale": "stale"},
           {"id": "easymem:src/a.py", "kind": "easy_memory", "ring": 4, "name": "a.py", "stem": "a", "description": "", "path": "src/a.py",
            "account": null, "abstained": false, "links_in": 0, "links_out": 0, "cited": 1, "newest": "2026-11-01", "stale": "stale"}],
 "edges": [], "counts": {...unchanged...},
 "easy_memory": {"notes": 1, "abstained": 0, "days": [30 x {date, count}; 2026-11-01 = 1], "prompts_on": 1, "injected": 1,
                 "files_cited": 2, "files_not_drawn": 0, "outside": 0}}
```
Real run on the owner's machine (read-only; the store was copied with the sqlite backup API, then `GET /api/memory-graph` through the guarded app, 2026-11-02T04:44:06Z): list 200, 40 projects, 204 ms; graph of easy-personal-mcp 200, 166 ms, 32 nodes (27 easy_memory; fresh 20, stale 10 across all node kinds); block `{"notes": 102, "abstained": 0, "prompts_on": 12, "injected": 5, "files_cited": 44, "files_not_drawn": 14, "outside": 0}`, days (oldest first, 30 entries) ending `... 0, 49, 0, 26, 11, 16`.

**DELTA**: `GET /api/memory-graph` now returns cited-file nodes, stale marks, the 30-day easy-memory panel and per-project easy-memory note counts (including easy-memory-only projects), read from the easy-memory store files with no note text.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
