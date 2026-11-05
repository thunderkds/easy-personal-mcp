# TASK_REVIEW — HUN-226: Kit page hides the Code index part when easy-context is not in use (FR-080 UI)

> Sibling of `tasks/TASK_GUIDE_HUN-226.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/KitView.test.jsx` (+6: AC1 no part line/footer/stale chip/column/intro; AC2 projectHealth; AC3 in-use stale kept; mixed, missing key, stale flag edge cases), `web/src/WarningsPlanView.test.jsx` (+2: Now `not in use`, Other rules then/now not in use). Red before change (6 Kit tests failed) |
| Verification command run | ☑ pass | Supervisor rerun on dc2f606: `npx vitest run` -> `Test Files 21 passed (21)`, `Tests 640 passed (640)`; `npm run build` built (combined tree) |
| Negative cases hold | ☑ pass | in-use stale index still shows reason; project with part in use keeps column; missing `context` key behaves as before |
| verify | ☑ pass | Supervisor combined real run 2026-11-05T07:45-08:00Z: detached scratch tree of HUN-226 + HUN-225 src files, dashboard on a fresh backup of ~/.easy-personal/store.sqlite. /api/warnings abstained: []; /api/reports/1 rules = cache_hit_low, project_dominates, records_unreadable, session_huge_no_compact, tokens_jump, tool_errors_high (no context_index_stale); Kit page: 'Code index' in text false; screenshots /tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/combo-kit-1280.png, combo-kit-375.png, combo-report-1280.png — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor Stage 4 code-review inline: KitView.jsx diff (notInUse helper, hasData, Part, ProjectCard shownKeys, DetailsTable column filter, intro text) + tests. lane_check: `lane: C1, 3 files, 117 lines, FITS`. P0-P3: none. Agent decisions accepted: generic column drop, empty cell when mixed, intro text. Pass |
| Full smoke suite still green (no regression) | ☑ pass | 640/640 vitest (630 before + 10 new) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier score not run for this C1 Low-risk UI change |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | combined real data, 1280x720: /tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/combo-kit-1280.png (no Code index line, footer 'Not set up here: Verifier'); opened report 1: /tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/combo-report-1280.png (1 to watch then + Other rules (5), no Code index card) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | no CSS change, no new colours (diff touches only KitView.jsx logic) |
| **UI: Responsiveness at target viewports** | ☑ pass | 375x812: /tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/combo-kit-375.png, hscroll 375 vs 375; 1280: 1265 vs 1265 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-05, before any implementation commit; `main` at 18ae74b):

```
$ cd web && npx vitest run
      Tests  630 passed (630)
2026-11-05T05:21:16Z
```

Real run: web build of the unchanged sources, dashboard from this worktree's `src` on a fresh copy of the owner's store (`$SCRATCH/h226home/store.sqlite`), Kit page, headless Chrome 1280x720 (`h226-shoot.mjs`, 2026-11-05T07:40:59Z):

```
Context column header: true
Code index in text: true
"code index is out of date" in text: false
hscroll: 1265 vs 1265
```

Screenshot: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h226-before-1280.png`. It shows the footer line "Not set up here: Code index, Verifier" on the project cards and a "Context" column ("no data") in the details table. The real server still sends `context: {"status": "no data"}` (HUN-225 is not merged), so the part is shown as a "no data" Code index.

**AFTER** (implementer, 2026-11-05):

```
$ cd web && npx vitest run
 Test Files  21 passed (21)
      Tests  640 passed (640)      # 630 before + 10 new
```

Real run: `npm run build`, then the dashboard from this worktree on the same store copy. HUN-225 is not merged, so the real server still sends `context: {"status": "no data"}`. The run script (`h226-after-main.py`, outside the repo) wraps `kit.kit_summary` to send HUN-225's shape `context: {"status": "not in use"}` for every project. This simulates the server; the Supervisor re-shoots after HUN-225 merges.

```
1280x720 (2026-11-05T07:43:22Z)  Context column header: false | Code index in text: false | "code index is out of date" in text: false | hscroll: 1265 vs 1265
375x812  (2026-11-05T07:43:30Z)  Context column header: false | Code index in text: false | "code index is out of date" in text: false | hscroll: 375 vs 375
```

Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h226-after-1280.png`, `.../h226-after-375.png`. Cards read "Not set up here: Verifier" (no Code index); the details table has Kernel, Verifier, Git columns only; the intro reads "How the agent kit and verifier are doing in each project."

**DELTA**: a Kit page for a machine where easy-context is not in use no longer shows a Code index part, column, "not set up" entry or stale reason.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
