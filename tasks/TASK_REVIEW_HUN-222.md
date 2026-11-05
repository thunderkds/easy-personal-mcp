# TASK_REVIEW — HUN-222: Warnings & plan page: Saved reports section with before / now per rule (FR-078 UI)

> Sibling of `tasks/TASK_GUIDE_HUN-222.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/WarningsPlanView.test.jsx`: 17 new tests (AC 1–7: list order + meta, open/close one at a time, summary/fixes/before-now lines, abstained + not-measured text, exact empty text, warnings list + hint unchanged, literal `<b>` text); red first (17 failed, 27 passed), then 44 passed. Fixture `fakeFetch` in `web/src/test/fixtures.js` answers `/api/reports*`. |
| Verification command run | ☑ pass | Supervisor re-run 2026-11-05 in the worktree: `npx vitest run` → `Test Files 21 passed (21)`, `Tests 617 passed (617)`; `npm run build` → `✓ built in 453ms`. |
| Negative cases hold | ☑ pass | 404 on one report → "Report not found", list stays; other error → "Could not load the report: …"; list load error inline; abstained / not_measured show text, never a number or `0`; no write control in the section. |
| verify | ☑ pass | Runtime check on a copy of the owner store with 1 report saved over MCP stdio, real `easy-personal dashboard` (built page): Supervisor viewed `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/after-open-1280.png` and `after-open-375.png` — section under the hint, one report open, 7 before / now lines with real values (e.g. `Token use jumped 217.2% fired → 223.1% fired`, `Sessions grow too large 54,857 tokens clear → 55,283 tokens clear`, `Code index is out of date abstained → abstained`) — pass. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor `code-review` over `git diff main...HEAD` (7 files; lane check `FITS`, 410 lines) + its one caller `WarningsPlanView.jsx`. P0 0. P1 1 process: the agent's Startup reads line says `PROJECT_SPEC.md (skimmed …, not re-read in full)` and `agents/frontend.md (first 30 lines)` — two Permanent-Rule reads incomplete; no code defect traced to it, recorded for the owner, not fixable after the fact. P2 0. P3 2: `valueText` picks the unit by rule id (`session_huge_no_compact` tokens, `context_index_stale` count, else percent) — correct for all 7 rules in `rules.yaml` today, but a new custom rule would show as a percent (follow-up: a `unit` field from the server); the list's date uses the browser locale format (`11/5/2026, 10:04:33 AM`). Security: report text renders as React text (no `dangerouslySetInnerHTML`; `<b>` test). |
| Full smoke suite still green (no regression) | ☑ pass | `Tests 617 passed (617)` (whole web suite); no server file changed (`git diff main...HEAD --stat`: `web/` + review file only). |
| **Companion score (easy-verifier)** | ☑ N/A | Not run for this web-only diff: the HUN-221 `score` (scope=`changes`) rated 4 of 13 dimensions and abstained on the rest; a 410-line JSX/CSS diff gives it nothing more to rate. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/before-1280.png` (no Saved reports section, `has Saved reports: false`); AFTER `after-closed-1280.png`, `after-open-1280.png`: section under the investigate hint, warnings list above unchanged. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | New `.report-*` / `.ba-*` rules in `web/src/app.css` use `var(--…)` tokens only (0 hex / `rgb(` literals in the added lines); fired = existing `chip-warning`, clear = existing `chip-ok`; rows are native `<button>` with `aria-expanded`; before / now is a `<ul>`; arrow `aria-hidden` with `sr-only` "then" / "now". |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280×720: before / now in two aligned columns; 375×812: each line stacks then ↓ now (`after-open-375.png`); page scroll width 1265 = 1265 and 375 = 375 (no page-wide horizontal scroll). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-05T03:05Z, before any implementation commit; worktree branch at `26e5f6c`, web built from it):
Store copy (`sqlite3 ~/.easy-personal/store.sqlite ".backup <copy>"`, `EASY_PERSONAL_HOME=<copy>`) holds 1 report, saved over MCP stdio:
`tools/call save_report` -> `{"id": 1, "saved_utc": "2026-11-05T03:04:33Z"}`.
`BROWSER=true easy-personal dashboard` -> `dashboard: http://127.0.0.1:60589/#<token>`. Headless Chrome (CDP), 1280x720, Warnings & plan opened, 2026-11-05T03:05:55Z:
`nav: clicked / hscroll: 1265 vs 1265 / has Saved reports: false`; screenshot `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/before-1280.png` shows the page ending at "Not checked (1)" and the investigate hint, no Saved reports section, although the store holds 1 report.
Vitest baseline, 2026-11-05T03:05:17Z: `npx vitest run src/WarningsPlanView.test.jsx` -> `Test Files 1 passed (1) / Tests 27 passed (27)`.

**AFTER** (2026-11-05T03:11Z to 03:13Z, same store copy and dashboard): headless Chrome, Warnings & plan, `has Saved reports: true`, `hscroll: 1265 vs 1265` at 1280x720 and `hscroll: 375 vs 375` at 375x812 (no page-wide horizontal scroll). Screenshots (scratchpad): `after-closed-1280.png`, `after-open-1280.png`, `after-closed-375.png`, `after-open-375.png`, all under `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/`. The open report shows summary, 2 fixes (`session_outliers: 645,301`), and 7 before / now lines (e.g. `Token use jumped 217.2% fired -> 222.6% fired`, abstained rules show their reason).
Vitest, 2026-11-05T03:13:54Z: `npx vitest run src/WarningsPlanView.test.jsx` -> 44 passed (44); `npx vitest run` -> Test Files 21 passed, Tests 617 passed; `npm run build` -> built.

**DELTA**: the owner can now read each saved investigate report on the Warnings & plan page and see, rule by rule, how the saved number compares with the same rule measured now.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
