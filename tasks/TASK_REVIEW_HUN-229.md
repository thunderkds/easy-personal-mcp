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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/Loading.test.jsx` (AC4, AC5, aria-hidden on `.loading-line`) and `web/src/WarningsPlanView.test.jsx`, describe `saved reports loading indicator (HUN-229)` (AC1, AC2, AC3, edge cases, and AC 3b: Warnings + list pending = one pill; Warnings pending + report clicked = one pill, then the report pill after Warnings loads; failed Warnings no longer holds the pills back). Written before the implementation; the first-pass tests were seen red. The AC 3b tests and the aria-hidden test were written before the `waiting` code, but I did not run them red on their own (the step-limit hook interrupted that run): they passed on the first run after the implementation. |
| Verification command run | ☑ pass | 2026-11-05T11:26:43Z `cd web && npx vitest run src/WarningsPlanView.test.jsx src/Loading.test.jsx && npx vitest run && npm run build`: `Test Files 2 passed (2) / Tests 85 passed (85)`; full `Test Files 21 passed (21) / Tests 666 passed (666)`; build `built in 548ms` (JS 326.54 kB; 325.90 kB before HUN-229). |
| Negative cases hold | ☑ pass | Failed list and failed report (404 and 500) remove the pill and show the inline error line; closing a row before load leaves no pill; a late answer of a closed or replaced report shows nothing; the old `.report-wait` line is gone. Changed existing test: the `Loading warnings` row of the view table in `Loading.test.jsx` used `getByRole('status')` (one pill); Warnings & plan now has two pills when all requests hang (warnings + saved reports list), so it takes the pill whose text starts with the label. No test in `WarningsPlanView.test.jsx` asserted the old text lines. |
| verify | ☑ pass | Supervisor `/verify` 2026-11-05T11:33Z, temp `uv tool install` of the branch (fresh `npm run build`): `--version` → `0.4.0`; `ping` → `easy-personal core ok`; MCP stdio `tools/call ping` → `"isError":false`; installed bundle contains `cache nicely`. Installed `easy-personal dashboard` on a store copy, headless Chrome holding `/api/warnings` + `/api/reports/1`: 1280 → `pills: ["Loading warningsUntangling the session timeline…"]` (one), after Warnings released → `["Loading reportSorting receipts from your robots…"]`, then `pills after: 0`; 375 with warnings + list held → one pill `Loading warningsTeaching workspaces to share…`, `hscroll: 375 vs 375`. Screenshots: scratchpad `v229/`. Dashboard stopped. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor `code-review` 2026-11-05: scope = 8 changed files + caller `WarningsPlanView.jsx:175` (`SavedReports`) + the 11 `Loading` consumers via the `Loading.test.jsx` view table. Lane: `lane: C2, 8 files, 291 lines, FITS; unpredicted: tasks/TASK_GUIDE_HUN-229.md, web/src/WarningsPlanView.jsx` (WarningsPlanView added by Supervisor AC 3b). Entry point reachable. Startup reads: all 3 Permanent-Rule reads present. Findings: P0 0, P1 0, P2 0, P3 1 (guide Files table omits `web/src/WarningsPlanView.jsx`, added by AC 3b). 0 fixes applied. Supervisor mutation check (red-gap): with the 3 impl files reset to 5513078, `vitest run src/WarningsPlanView.test.jsx src/Loading.test.jsx` → `Tests  3 failed | 82 passed (85)` (the aria-hidden test and both AC3b tests); restored → full `Tests  666 passed (666)`, build ✓ 858ms. |
| Full smoke suite still green (no regression) | ☑ pass | `npx vitest run`: 21 files, 666 tests passed. No Python change. |
| **Companion score (easy-verifier)** | ☑ advisory | Supervisor `score(scope="changes", base="main")` 2026-11-05: `rating_abstention` / `no_dimension_rated` on all 13 dimensions — "achieved coverage is below the declared floor". No number; advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real run (worktree source, store copy, headless Chrome holding requests). Report pill, 1280x720: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-after-1280-pending.png` (pill "Loading report" + line, centred, no text line, list does not move; taken before the AC 3b change, same markup). After the AC 3b change: report pill shows once Warnings has loaded: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-afterwarn-1280-afterwarn.png` (console: `pills after warnings loaded: ["Loading reportStacking tokens into tidy piles…"]`). Opened report BEFORE vs AFTER `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-before-1280-done.png` / `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-after-1280-done.png`: same layout. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Reuses `Loading`; one new rule `.loading-line { display: block; color: var(--color-muted); font-size: var(--fs-small); }` plus `.loading-text { min-width: 0; }`; no colour literal (asserted in `Loading.test.jsx`). Pill keeps `role="status"` and starts with its label (asserted). |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280: pill centred in the content area, `hscroll: 1280 vs 1280` (this run). 375x700: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-after-375-pending.png` pill 187 px, line fits, no horizontal scroll. Overlap re-check after AC 3b (Warnings and saved-reports list both pending): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-both-1280-pending.png` and `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-both-375-pending.png` show exactly one pill (`pills: [{"text":"Loading warnings…"}]` at both widths, `hscroll: 375 vs 375`). Report clicked while Warnings pending: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h229-afterwarn-1280-pending.png`, one pill ("Loading warnings"). |

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

**DELTA**: Clicking a saved report (or opening Warnings & plan) now shows the same floating loading indicator as every other page, and every indicator shows one random funny line under its label (hidden from a screen reader). On Warnings & plan only one indicator shows at a time: the saved-report pills wait until the Warnings list has loaded.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-229.jsonl`, never the
implementing agent alone]
