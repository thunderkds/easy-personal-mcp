# TASK_REVIEW — HUN-213: Live view session rows with bars + one session panel (FR-072, NFR-015)

> Sibling of `tasks/TASK_GUIDE_HUN-213.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/LiveView.test.jsx` (68 tests; AC1 rows, AC2 widths, AC3 unknown price / $0, AC4 open/close/switch/aria, AC5 live into panel + leaving, AC6 attention tests adapted, AC7 batching + memo via Profiler and `ago` spy, AC8 one `.live-feed`). AC1 and AC8 failed before the change (see BEFORE). Enter/Space: jsdom cannot press keys on a native button, so the test asserts the row is a `<button type=button>`; the keys themselves were pressed in the real run (see Visual regression). |
| Verification command run | ☑ pass | 2026-11-04T11:24:08Z `cd web && npx vitest run src/LiveView.test.jsx && npx vitest run && npm run build`: LiveView.test.jsx "Test Files 1 passed (1), Tests 68 passed (68)"; full suite "Test Files 21 passed (21), Tests 589 passed (589)"; build "✓ built in 422ms" and check_static stamp written. |
| Negative cases hold | ☑ pass | tests: malformed message ignored; duplicate id inside one batch counted once; snapshot flushes queued events first; unmount drops the queued batch (no timers left); planted SECRET text in an event and in an attention state never on the page, panel open or closed; all-zero measures give 0% bars (no NaN); last session leaving still shows the panel notice. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real run (`easy-personal dashboard`, throwaway store, 7 fabricated metadata-only sessions in 2 accounts, headless Chrome). BEFORE `reports/HUN-213/before-1280x720.png`. AFTER `reports/HUN-213/after-rows-1280x720.png`, `reports/HUN-213/after-panel-1280x720.png`, `reports/HUN-213/after-panel-feed-1280x900.png` (live tool calls appended while the panel was open: 6 feed items, 1 `.live-feed` in the DOM), `reports/HUN-213/after-rows-375x812.png`, `reports/HUN-213/after-panel-375x812.png`. Keyboard in that browser: Enter on row 1 opened its panel; Space on row 2 closed row 1 and opened row 2 (1 panel); Close returned focus to a row. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Tests in LiveView.test.jsx `design`: live rules have no hex/rgb/hsl; bars use `--color-track` / `--color-accent`; text colours ≥ 4.5:1 on bg, raised and selected; row focus ring. Rows are buttons with aria-expanded/aria-controls; bars are `role=img` with "tokens 1.2M"-style names; one `aria-live` region on the page (test). |
| **UI: Responsiveness at target viewports** | ☑ pass | Real run, `document.documentElement.scrollWidth` vs `clientWidth`: 1280 = 1280/1280; 768 = 768/768 (panel right of rows); 767 = 767/767 (panel below rows); 375 = 375/375 (panel below rows at y 1022 vs rows end 1006). Long project name is cut with an ellipsis at 375 (full name in `title`). Bars stack one per line in a row narrower than 28rem (container query). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-04T11:11:09Z, before any implementation commit; only the two new failing tests existed):

```
$ cd web && npx vitest run src/LiveView.test.jsx -t "rows"
 FAIL  src/LiveView.test.jsx > rows (FR-072) > AC1: three sessions give three rows with project, 8-char id, active/idle text and three bars; no feed is drawn
AssertionError: expected [] to have a length of 3 but got +0
    553|     ]))
    554|     expect(rowsOf()).toHaveLength(3)

 FAIL  src/LiveView.test.jsx > rows (FR-072) > AC8: at most one .live-feed list is in the DOM, and only for the open session
AssertionError: expected <ol class="live-feed" …(3)></ol>
…(1) to have a length of +0 but got 2
    566|     expect(document.querySelectorAll('.live-feed')).toHaveLength(0)

 Test Files  1 failed (1)
      Tests  2 failed | 2 passed | 43 skipped (47)
   Start at  18:11:11
```

BEFORE screenshot of the current Live view (real `easy-personal dashboard` run on the old build, throwaway store, 7 fabricated metadata-only sessions in 2 accounts, taken 2026-11-04T11:14:58Z): `reports/HUN-213/before-1280x720.png` in the task worktree. It shows one tall card per session with its own feed area ("No tool calls seen since this page opened."): about 3 sessions fit on screen.

**AFTER** (2026-11-04T11:24:08Z, same suite after the change): `npx vitest run src/LiveView.test.jsx` gives "Test Files 1 passed (1) / Tests 68 passed (68)", including the two tests that failed in BEFORE. AFTER screenshots: `reports/HUN-213/after-rows-1280x720.png` (7 rows in 2 accounts, 3 bars each, no feed), `reports/HUN-213/after-panel-feed-1280x900.png` (one open session: totals + live feed, panel right of the rows), `reports/HUN-213/after-panel-375x812.png` (panel below the rows).

**DELTA**: The Live view shows every active session as one short row with token, cost and tool-call bars (instead of a tall card with its own feed), and opens one session's totals and feed on click, so the page draws one feed and updates at most four times a second.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-213.jsonl`, never the
implementing agent alone]
