# TASK_REVIEW — HUN-162: Work view: read-only cross-project board

> Sibling of `tasks/TASK_GUIDE_HUN-162.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_board.py` (sections + Blocked table, unknown subsection abstained, table rows, plain bullets, id-less rows, struck-through Blocked, plain titles, huge Done, projects from usage, missing/unparsable/oversized/dir file → no data, worktrees skipped, same id in two projects, workspace scope, board kind + pull time, only allowed keys, nothing written, 50×60 < 1 s), `tests/surfaces/test_api_board.py`, `tests/leak/harness.py`, `web/src/BoardView.test.jsx`, `web/src/App.test.jsx`. Supervisor run 2026-10-31T15:41:07+07:00 on 2c504c8: `Tests  293 passed (293)`; `876 passed, 5 deselected, 1 xfailed, 1 warning in 136.80s (0:02:16)` |
| Verification command run | ☑ pass | `npm --prefix web test -- --run` → `Test Files  14 passed (14)` / `Tests  293 passed (293)`; build `✓ built in 176ms`; `uv run pytest -q` → `876 passed, 5 deselected, 1 xfailed, 1 warning in 136.80s (0:02:16)`; ruff `All checks passed!` |
| Negative cases hold | ☑ pass | runtime: project with no `PROJECT_KANBAN.md` → `no data`; repo without `.claude/board.json` → board None, no pull time; `find <repos> -newer <dash.log>` after the run → no file changed (read-only); token-less request 403 (test) |
| verify | ☑ pass | Supervisor runtime check 2026-10-31 on 2c504c8: dashboard on 3 temp git repos mapped from fixture transcripts: `GET /api/board` → alpha-app ok linear `2026-10-31T06:40:48Z` [HUN-201 Todo, HUN-202 Todo, HUN-198 In Progress, HUN-197 Ready for Review, HUN-190/191 Done, HUN-199 Blocked]; beta-api ok None [T014 Todo, T012 In Progress, T010 Done]; gamma-site no data — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor code-review + inline security-review 2026-10-31 on `1c456d9...HUN-162` (2c504c8): 11 files + callers (`kit.scoped_projects`, `api._query/_who`). No HUN-163/164 files on the branch after the stash incident (verified name-only diff). lane_check: `lane: C2, 11 files, 910 lines, EXCEEDS (lines 910 > 500)` — ~550 lines tests. Findings: P0 0, P1 0. P2 2: (a) at 1280 px only ~3.5 of 6 columns fit; Done / Closed / Blocked sit behind a horizontal scroll — narrower columns or a wrap layout (follow-up); (b) project dot colour cycles by order, not workspace colour (cut, documented). P3: NFR-010 timing test times `board.board()`, not the HTTP round trip. Security (Low): title only (text before the first ` | `), links reduced to labels, 300-char cap, 5 MB file cap, non-UTF-8 → no data, `board.json` read for `board` only (team id never returned — tested), route behind `Guard` |
| Full smoke suite still green (no regression) | ☑ pass | baseline 839 → 876 passed; web 276 → 293 |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier MCP not run for this task (advisory, no hook); no report path. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | `reports/HUN-162/Board_{1280,768,375}.png` (CDP, temp repos). Verdict vs sketch: pass with P2 — header `3 projects · 1 synced (last pull 2 h ago) · 1 no data`, project list with `linear` tag / `not synced` / `no data`, project filter, columns with counts, cards id · title · project dot · board tag; only the first 3.5 columns visible at 1280 (P2 above) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `BoardView.test.jsx` 'styles the board with tokens only'; Plex Sans / Plex Mono computed; console errors `[]`; request hosts only `127.0.0.1` |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280/768/375: no horizontal page scroll; the board scrolls inside its box |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, 2026-10-31T15:05:19+07:00, main a491d1d, before any implementation commit):
```
$ ls src/easy_personal/core/board.py
ls: cannot access 'src/easy_personal/core/board.py': No such file or directory
$ grep -nE "/api/(board|jobs|memory-graph)" src/easy_personal/surfaces/api.py
(no output — no /api/board route)
$ ls web/src | grep -E "Board|Jobs|Memory"
(no output — no BoardView)
```

**AFTER** (Supervisor, 2026-10-31, HUN-162 2c504c8):
```
$ ls src/easy_personal/core/board.py
src/easy_personal/core/board.py
$ curl -H 'X-Dashboard-Token: …' $BASE/api/board
alpha-app ok linear 2026-10-31T06:40:48Z [('HUN-201', 'Todo'), …, ('HUN-199', 'Blocked')]
beta-api ok None None [('T014', 'Todo'), ('T012', 'In Progress'), ('T010', 'Done')]
gamma-site no data
```

**DELTA**: The owner sees the tasks of every kit project on one read-only board, grouped by status, with how fresh each Linear pull is.

**WITNESS**: Supervisor, 2026-10-31 (runtime check + full suites with `active_task` = HUN-162).
