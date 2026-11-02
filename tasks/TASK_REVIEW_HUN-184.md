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
| Verification command run | ☑ pass | `67 passed, 390 deselected` (core) / `7 passed, 183 deselected` (surfaces) / `33 passed` (leak); full suite `1120 passed, 2 skipped, 6 deselected, 1 xfailed in 153.49s`; `ruff check src tests`: All checks passed |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

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
