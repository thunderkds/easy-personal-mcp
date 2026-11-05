# TASK_REVIEW — HUN-229: Saved reports loading indicator + random funny line

> Sibling of `tasks/TASK_GUIDE_HUN-229.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/Loading.test.jsx` (AC4, AC5: label first, exactly one `.loading-line` from the list, stubbed `Math.random` 0 / 0.999999, line kept on re-render with one `Math.random` call, 12 lines each ≤ 40 chars, CSS rule uses tokens only; view table: each pill starts with its label and has one line). `web/src/WarningsPlanView.test.jsx`, describe `saved reports loading indicator (HUN-229)` (AC1, AC2, AC3 and the edge cases: close before load, switch between two reports, list error, two pills together). Written first: 7 of the new WarningsPlanView tests failed and Loading.test.jsx failed to import `loadingLines.js` before the implementation. |
| Verification command run | ☑ pass | 2026-11-05T11:23:28Z `cd web && npx vitest run src/WarningsPlanView.test.jsx src/Loading.test.jsx && npx vitest run && npm run build`: `Test Files 2 passed (2) / Tests 82 passed (82)`; full run `Test Files 21 passed (21) / Tests 663 passed (663)`; build `built in 531ms` (JS 325.90 kB before, 326.45 kB after). |
| Negative cases hold | ☑ pass | Failed list and failed report (404 and 500) remove the pill and show the inline error line; closing a row before load leaves no pill; a late answer of a closed or replaced report shows nothing; the old `.report-wait` line is gone. Changed existing test: the `Loading warnings` row of the view table in `Loading.test.jsx` used `getByRole('status')` (one pill); Warnings & plan now has two pills when all requests hang (warnings + saved reports list), so it takes the pill whose text starts with the label. No test in `WarningsPlanView.test.jsx` asserted the old text lines. |
| verify | ☐ N/A | Not run by the implementer: left to the Supervisor's Stage 4/5 `code-review` and `verify`. Implementer's real run: see below (pass). |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ N/A | `Loading` is a hub component (11 uses). All 21 web test files run (663 pass); `Loading` consumers are covered by the view table in `Loading.test.jsx`. `code-review` is left to the Supervisor. |
| Full smoke suite still green (no regression) | ☑ pass | `npx vitest run`: 21 files, 663 tests passed. No Python change. |
| **Companion score (easy-verifier)** | ☑ N/A | Not run by the implementer (no easy-verifier MCP call made); the Supervisor scores at Stage 4. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real run, dashboard from this worktree on a store copy, headless Chrome with `GET /api/reports/1` held. AFTER pending 1280x720: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-after-1280-pending.png` (floating pill "Loading report" + line "Stacking tokens into tidy piles…", centred; no text line; the report row and list do not move). Console: `pills: [{"text":"Loading reportStacking tokens into tidy piles…"...}]`, `old line: []`. Opened report, BEFORE vs AFTER: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-before-1280-done.png` and `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-after-1280-done.png`: same layout, only live-ingest numbers differ (store copy ingests the owner's real transcripts). BEFORE pending: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-before-1280-pending.png` (text line, no pill). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Reuses `Loading`; one new rule `.loading-line { display: block; color: var(--color-muted); font-size: var(--fs-small); }` plus `.loading-text { min-width: 0; }`; no colour literal (asserted in `Loading.test.jsx`). Pill keeps `role="status"` and starts with its label (asserted). |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280: pill centred in the content area (x 635..861 of the main column; `hscroll: 1265 vs 1265`, the page scrollbar). 375x700: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-after-375-pending.png`, pill 187 px wide, line fits on its own row, `hscroll: 375 vs 375` (no horizontal scroll). Overlap finding (not fixed, see report): Warnings request and saved-reports list request pending together show two pills at the same fixed spot, drawn on top of each other: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-both-1280-pending.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-both-375-pending.png`. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (main code, worktree `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/worktrees/agent-a0483c7617b4d9479`, no implementation commit yet; captured 2026-11-05 UTC):

Verification command part 1 on unchanged code (`cd web && npx vitest run src/WarningsPlanView.test.jsx src/Loading.test.jsx`, started 2026-11-05T11:13:36Z):
```
 Test Files  2 passed (2)
      Tests  70 passed (70)
```
Real run (dashboard from the worktree source on a store copy; `GET /api/reports/1` held with headless Chrome CDP `Fetch` while the report row was clicked, 2026-11-05T11:17:00Z):
```
nav: clicked
row: clicked
held http://127.0.0.1:49309/api/reports/1
pills: []                                   <- no floating loading indicator
old line: ["Loading report"]                <- the plain text line `p.report-wait` pushes under the row
hscroll: 1265 vs 1265
saved h229-before-1280-pending.png 2026-11-05T11:17:00.201Z
```
Screenshot: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-before-1280-pending.png` (final take, 2026-11-05T11:17:31Z; shows the plain "Loading report" text line under the row, no floating pill. An earlier take at 11:16:29Z, taken while the real `/api/warnings` was still slow, also showed a `Loading warnings` pill next to the text line.)

**AFTER** (same real run after the change, 2026-11-05T11:21:25Z, `GET /api/reports/1` held):
```
row: clicked
held http://127.0.0.1:49309/api/reports/1
pills: [{"text":"Loading reportStacking tokens into tidy piles…","rect":{"x":635.6,"y":334.5,"width":225.8,"height":51}}]
old line: []
hscroll: 1265 vs 1265
saved h229-after-1280-pending.png 2026-11-05T11:21:25.888Z
```
Verification command part 1 after the change (2026-11-05T11:23:28Z): `Test Files 2 passed (2) / Tests 82 passed (82)` (was 70).

**DELTA**: Clicking a saved report (or opening Warnings & plan) now shows the same floating loading indicator as every other page, and every indicator shows one random funny line under its label.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-229.jsonl`, never the
implementing agent alone]
