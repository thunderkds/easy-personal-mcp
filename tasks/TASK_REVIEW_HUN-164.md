# TASK_REVIEW — HUN-164: Work view: memory graph (names + links, read live)

> Sibling of `tasks/TASK_GUIDE_HUN-164.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_memgraph.py` (nodes/edges, link forms, self-links, Claude folder mapping, unmapped/ignored accounts, no bodies, truncation, abstained files, symlinks, duplicates, nothing written, 50×40 < 1 s), `tests/surfaces/test_api_memory_graph.py`, `tests/leak/{canaries,conftest,harness,test_leak_suite}.py` (memory_body canary), `web/src/MemoryView.test.jsx` (golden SVG `web/src/test/memory-graph.golden.svg`, order-independent, panel, keyboard, cap, tokens). Supervisor run 2026-10-31T15:37:11+07:00 on 5b954d9: `Tests  292 passed (292)`; `865 passed, 5 deselected, 1 xfailed, 1 warning in 146.10s (0:02:26)` |
| Verification command run | ☑ pass | `npm --prefix web test -- --run` → `Test Files  14 passed (14)` / `Tests  292 passed (292)`; build `✓ built in 441ms`; `uv run pytest -q` → `865 passed, 5 deselected, 1 xfailed, 1 warning in 146.10s (0:02:26)`; ruff `All checks passed!` |
| Negative cases hold | ☑ pass | runtime: `SECRET_Y_PLANTED` in a note body → `SECRET in response: 0`, not on the page; unknown project `/nope<script>` → `{"error":"unknown project"} 400` (input not echoed); dangling `[[ghost-note]]` → dangling node, path null |
| verify | ☑ pass | Supervisor runtime check 2026-10-31 on 5b954d9: dashboard on a temp git repo with 5 kit notes + fixture transcript (cwd mapped): `GET /api/memory-graph` → 1 project `demo-app`, `kit_notes 5`; `?project=` → `counts {'notes': 5, 'links': 10, 'dangling': 1, 'abstained': 0, 'duplicates': 0, 'capped': False}`, rings memory-index 0 · decisions/glossary/learnings 1 · lr-0001 2 · ghost-note 3 — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor code-review + inline security-review 2026-10-31 on `1c456d9...HUN-164` (5b954d9): 15 files + callers (`kit.scoped_projects`, `accounts.list_accounts`, `discovery.resolve_project`, `api._query`). lane_check: `lane: C2, 15 files, 1033 lines, EXCEEDS (files 15 > 14; lines 1033 > 500)` — ~600 lines tests + golden SVG. Findings: P0 0, P1 0. P2 2: (a) Memory tab is `scoped: true` (Supervisor's spawn instruction) but the view ignores Workspace/Account scope — either filter the project list by workspace or set `scoped: false` (follow-up); (b) `[[link]]` targets are body-derived and not length-capped (descriptions are capped at 200) — cap targets with `_clean`. P3 1: link resolution falls back to file stem (beyond glossary wording) — accepted, documented in API contract. Security/privacy (Medium): bodies never leave `_read_note` (only name/description/links), files > 1 MB / non-UTF-8 abstained, symlinks outside the memory folder skipped, no subfolder link-follow, unknown project 400 without echo, route behind `Guard`; leak suite green |
| Full smoke suite still green (no regression) | ☑ pass | baseline 839 → 865 passed; web 276 → 292 |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier MCP not run for this task (advisory, no hook); no report path. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | `reports/HUN-164/Memory_{1280,768,375}.png` (CDP, temp repo). Verdict vs sketch: pass — `// work › Memory`, project picker, rings SVG (memory-index centre, cold files ring 1, LR ring 2, dashed ghost-note outer), side panel prompt, legend `kit memory · Claude memory · unresolved link · 5 notes · 10 links · 1 unresolved` |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `MemoryView.test.jsx` 'styles the memory view with tokens only'; Plex Mono labels / Plex Sans body computed; console errors `[]`; request hosts only `127.0.0.1` |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280/768/375: no horizontal page scroll; graph scrolls inside its box |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, 2026-10-31T15:05:19+07:00, main a491d1d, before any implementation commit):
```
$ ls src/easy_personal/core/memgraph.py
ls: cannot access 'src/easy_personal/core/memgraph.py': No such file or directory
$ grep -nE "/api/(board|jobs|memory-graph)" src/easy_personal/surfaces/api.py
(no output — no /api/memory-graph route)
$ ls web/src | grep -E "Board|Jobs|Memory"
(no output — no MemoryView)
```

**AFTER** (Supervisor, 2026-10-31, HUN-164 5b954d9):
```
$ ls src/easy_personal/core/memgraph.py
src/easy_personal/core/memgraph.py
$ curl -G -H 'X-Dashboard-Token: …' --data-urlencode project=<repo> $BASE/api/memory-graph
counts {'notes': 5, 'links': 10, 'dangling': 1, 'abstained': 0, 'duplicates': 0, 'capped': False}
```

**DELTA**: The owner sees each project's memory notes as a graph of names and links, including unresolved links, without any note body leaving disk.

**WITNESS**: Supervisor, 2026-10-31 (runtime check + full suites with `active_task` = HUN-164).
