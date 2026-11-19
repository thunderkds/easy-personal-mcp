# TASK_REVIEW — T119: Live like Claude 5 — filters, names, past sessions

> Sibling of `tasks/TASK_GUIDE_T119.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` or `[admin]` tag; with `[admin]`, the `Not done:` line names `merge (admin: after the auto verify pass)`,
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
`Memory:` records the diff-driven memory pass you run before you stop: each entry written, or `nothing new`.
After an auto verify pass the hook, never you, adds `Closed: row moved to Done <date> — …` and moves the row,
or adds `Paused: done not moved — <reason>`. It moves the row only when this block holds `Mode: auto`, a
numbered decision, `Checks:`, `Memory:`, the `Not done:` auto verify slot with its result, and `Not done:`
naming the merge — no `<...>` left (`docs/claude-md/tracks.md` `### Auto close-out`).
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/LiveFilters.test.jsx` (19 tests, AC 1), `web/src/LiveRename.test.jsx` (AC 2), `web/src/PastSessions.test.jsx` (30 tests, AC 3: date range, Load more, 30-day note + Show older, read-only conversation, Resume / Fork). Filters: needs me / working / idle, project, kind, counts, `localStorage` (filters only, throwing storage, bad shapes). |
| Verification command run | ☑ pass | 2026-11-19 03:42 UTC at `ae8397e`: `Test Files  42 passed (42)`, `Tests  1410 passed (1410)`; `npm run build` `✓ built in 150ms`; `check_static: ok`. (Baseline before: 39 files, 1347 tests.) |
| Negative cases hold | ☑ pass | Mutations on the code, each caught by a named test, code restored after (`git status` clean of the mutated file): `to` sent without +1 day (date range test fails), no stale-answer guard (older filter test fails), no explicit `from` on Show older (2 tests fail), window note always off (3 fail), no de-dup on Load more (1 fails), filters not written to `localStorage` (2 fail), a kind count that does not ignore its own filter (1 fails, test added for it). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed: `LiveView.jsx` (hub: App mounts it; all Live tests ran: 42 files), `SessionPanel.jsx`, `api.js`, `appsession.css`, the four new components. Not touched: `rust/**`, `src/easy_personal/**`. Edge Case Checklist: gone file shown as the route's one line (Transcript error branch, tested); names and text are React text, never HTML (tested with `<b>`); 375 px wraps, code blocks scroll inside themselves (screenshots); names file limits are the server's (80 chars `maxLength`, refusal line shown). |
| Full smoke suite still green (no regression) | ☑ pass | Full web suite 1410/1410; build and `check_static.py` ok. |
| **Companion score (easy-verifier)** | ☑ N/A | Not run by the implementing agent. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | ux-evaluate on real headless-Chrome screenshots at 375 and 1280 (table below). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | New CSS (`web/src/appsession.css`, T119 block) uses only `var(--...)` tokens (`grep -n "#[0-9a-f]\{3,6\}" ` on the block: none). Reused `.periods`, `.board-filter`, `.link-button`, `.live-row`, `.session-*`. |
| **UI: Responsiveness at target viewports** | ☑ pass | 375: `scrollWidth 375 == clientWidth 375` on live list, live panel, Past list, Past panel, rename field; 1280: no overflow. Touch targets measured in the page at 375: none under 44 px (after one fix, below). |

### ux-evaluate (no UX rows in the guide: baseline checks only)

Screenshots (scratch, not committed): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t119/shots/` (`live-list`, `live-panel`, `live-filter-working`, `live-nomatch`, `past-list`, `past-older`, `past-panel`, `past-rename`, each `-375.png` and `-1280.png`). Served by a fixture mock of the Rust routes (no real sessions); driven by Chrome DevTools.

First run failed B-d at 375: Live / Past switch, Show older, Clear filters, Load more were 32 px high. Fixed in CSS (`ae8397e`), screenshots taken again, in-page measure now empty.

| # | Flow / state | Width | Verdict | Reason |
|---|---|---|---|---|
| B-a | Live list, Past list | 375 / 1280 | pass | The rows (whole-card targets) and the Live/Past switch are in view at the top; Resume in app is the first action in a past panel |
| B-b | all | 375 / 1280 | pass | Title and switch, then filters, then list, then panel; one reading order |
| B-c | all | 375 / 1280 | pass | Filters group on one row at 1280 and stack at 375; same card gaps as T118 |
| B-d | all | 375 | pass | In-page measure of buttons, selects, inputs at 375 (rows excluded: the whole card is their target): none under 44 px |
| B-e | all | 375 | fail (older) | Body text measures 12.5 px at 375; the app's `--fs-*` tokens are this size on every page, so it is not caused by T119 |
| B-f | list empty, no match, loading, error | 375 / 1280 | pass | "No sessions match these filters." with Clear filters is on screen (`live-nomatch-375.png`); Past empty, loading, error and "cannot be listed" states are asserted in `PastSessions.test.jsx`, not shot |
| B-g | rename, Load more, Show older | 375 / 1280 | pass | Show older changes the note to "Showing sessions since 2000-01-01. Last 30 days" (`past-older-*.png`); the rename field says "Enter saves. Esc cancels. Empty removes the name." |
| B-h | all | 375 | pass | No horizontal scroll, nothing clipped; the panel takes the full width with Back |
| B-i | all | 375 / 1280 | pass | Same accent, buttons and radius as the rest of Live |

ux-evaluate: 8 pass, 1 fail (older), 0 not checked, 0 not checkable. Overall: pass for T119's change; B-e is app-wide.

### Cut list (server shapes, accepted by the Supervisor 2026-11-19)
- No *App* kind and no *needs me / working / idle* in Past: `GET /api/sessions/past` takes `kind=terminal|headless` only (the store cannot tell an app session from a headless run); those states are live.
- The past row has the owner's name only, no Claude name (the store holds none).
- No per-filter counts in Past: the list is paged on the server.
- The project and kind filters are shared by Live and Past (one `localStorage` key, three values); the date range is not kept.
- Rename shows on a server with no names route too; it answers its one refusal line.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (frontend-developer, before any T119 implementation commit; `main` at `8c8c121`, the worktree's branch point):
```
$ date -u; git log -1 --oneline; grep -ln 'PastSessions\|renameSession' web/src/*.jsx web/src/api.js | wc -l; ls web/src/PastSessions.jsx; (cd web && npm test)
Thu Nov 19 03:20:44 AM UTC 2026
8c8c121 Board + memory: T114 merged; T102-T112 rows moved to Done; T114 guide
0
ls: cannot access 'web/src/PastSessions.jsx': No such file or directory
 Test Files  39 passed (39)
      Tests  1347 passed (1347)
```
(No filters, names or past sessions in the Live view; the baseline suite is 39 files, 1347 tests, all green.)

**AFTER** (frontend-developer, `T119` at `ae8397e`):
```
$ date -u; git log -1 --oneline; ls web/src/PastSessions.jsx web/src/LiveFilters.jsx web/src/SessionName.jsx; (cd web && npm test) | grep -E "Test Files|Tests "; (cd web && npm run build) | tail -1; python3 scripts/check_static.py
Thu Nov 19 03:42:45 AM UTC 2026
ae8397e T119: 44px touch targets at 375 for the Live/Past switch, Show older, Clear filters
 Test Files  42 passed (42)
      Tests  1410 passed (1410)
✓ built in 150ms
check_static: ok
```
(The three components exist; the Live view has the filter bar with counts, the Live | Past switch and the inline rename.)

**DELTA**: In the Live view an owner can filter by needs me / working / idle, project and kind (kept across reloads), name a session inline, and switch to Past to find an older session by project, kind and date range, open its read-only conversation and Resume or Fork it.

**WITNESS**: [filled by the hook from `memory/event-trace/T119.jsonl`, not by the implementing agent]
