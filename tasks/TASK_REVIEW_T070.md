# TASK_REVIEW — T070: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T070.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/AgentsView.test.jsx`, `KitView.test.jsx`, `WarningsPlanView.test.jsx`, `WorkspacesView.test.jsx`, `shell.test.js` (red token + First run tokens). Every fixed row that adds text or an element has an assertion (column "Test" below). Full suite: `Test Files  25 passed (25)`, `Tests  838 passed (838)` (828 before, +10). |
| Verification command run | ☑ pass | See AFTER: `cd web && npm test 2>&1 | tail -5 && npm run build 2>&1 | tail -3` ran 2026-11-15T02:45Z; 25 files, 838 tests pass, build `✓ built in 150ms`. |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE `tasks/evidence-T070/before-<view>-1440.png`, AFTER `after-<view>-1440.png` for usage, warnings, kit, workspaces, agents. Vision verdict against each artboard: every "fix" row in the Difference table is closed in the AFTER image (banner row, lit Changes block, dashed orphan card, raised connector boxes, red Delete, letter tile, alias chips; Agents status block, missing-card note, lit detected card, person icon; Kit cell strip with bar; Warnings note). "keep" and "needs server" rows are listed below. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `git diff 8a9ca6e..T070 -- web/src/app.css` added lines grep for `#[0-9a-f]{3,8}` : 0 hits (only `tokens.css` defines `--color-red: #e8968f`). Raw `px` left only for gaps/paddings (6, 10, 14 px) and the 5/6 px bars; no raw font sizes (the First run `12.5px` / `13.5px` became `--fs-note` / `--fs-control`, test in `shell.test.js`). |
| **UI: Responsiveness at target viewports** | ☑ pass | 1440, 1024 and 375 px for all five views, CDP screenshots `after-<view>-<width>.png`. `scrollWidth == innerWidth` in all 15 (375: usage/warnings/kit/workspaces/agents 375/375; 1024: 1024/1024). No horizontal scroll. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-14T17:18Z, branch `T070` at main `ea872f7`, before any implementation commit):

Command `cd web && npm test 2>&1 | tail -5 && npm run build 2>&1 | tail -3`, then `npm test | grep -E "Test Files|Tests "`:

```
../src/easy_personal/dashboard/static/assets/index-DdvNImUp.js   345.52 kB │ gzip: 103.67 kB
✓ built in 197ms
 Test Files  25 passed (25)
      Tests  828 passed (828)
```

BEFORE screenshots, 1440 px, built page served by a fixture server (canned JSON from `web/src/test/fixtures.js`; the
`store-v11.sqlite` store has no usage data, so Usage there reads 0 everywhere):
`tasks/evidence-T070/before-usage-1440.png`, `before-warnings-1440.png`, `before-kit-1440.png`,
`before-workspaces-1440.png`, `before-agents-1440.png`.
What is absent before: no "Not measured is not the same as passed." note on Warnings; Agents `.claude-old` (missing) card shows
"last active never / Tokens: 0 today / Plan limits: add the status-line snippet"; `--color-red: #ef7b7b`; First run CSS has raw
`12.5px` / `13.5px`; Kit "easy-ui checks (overall)" is a plain list; Workspaces connector banner stacks Adopt under the text.

**AFTER** (2026-11-15T02:45Z, branch `T070`):

```
$ cd web && npm test 2>&1 | tail -5 && npm run build 2>&1 | tail -3
 Test Files  25 passed (25)
      Tests  838 passed (838)
../src/easy_personal/dashboard/static/assets/index-CtkuaSHE.css    50.60 kB │ gzip:   9.38 kB
../src/easy_personal/dashboard/static/assets/index-DyyWCd3E.js    347.36 kB │ gzip: 104.04 kB
✓ built in 150ms
```
(the `tail -5` of `npm test` is the vitest timing footer; the counts above are its `Test Files` / `Tests` lines from the same run.)

AFTER screenshots (fixture server on `web/src/test/fixtures.js`, CDP, page height = content height): `tasks/evidence-T070/after-{usage,warnings,kit,workspaces,agents}-{1440,1024,375}.png`.

### Difference table (AC1)

Screenshots of the five views at 1440 px vs. the artboards in `docs/design/graphite-mint/`. Sample numbers and names in artboards are not targets.

| # | View | Difference | Decision | Why | Closed by / Test |
|---|------|------------|----------|-----|------------------|
| 1 | All | Problem red `#ef7b7b` vs artboard `#E8968F` | fix | See Red token decision | `4f79fe4`; `shell.test.js` "uses the artboard problem red" |
| 2 | All | Delete buttons plain text colour; artboard red | fix | markup class only | `1ff0980`; `WorkspacesView.test.jsx` "marks every Delete as a problem action" |
| 3 | First run (shared CSS) | Raw `12.5px` / `13.5px` font sizes | fix | Design-system rule: type tokens only (nearest tokens `--fs-note`, `--fs-control`) | `4f79fe4`; `shell.test.js` "sets First run font sizes from the type tokens" |
| 4 | Usage | Artboard has no "You and agents", Totals, By tool, Session timeline blocks; cache card has extra "Written to cache" line | keep | These are working features beyond the artboard (HUN-126 restyle landed); removing them is a behaviour change | n/a |
| 5 | Usage | Table headings capitalised (`Model`, `Total`) vs lowercase in artboard | keep | Text of headings is asserted in existing tests and shared by all tables; no value in a rename | n/a |
| 6 | Usage | Artboard By day has 7 bars, outliers wording "90 or more" / "212K context"; app shows what the API returns | keep | Sample data, not layout | n/a |
| 7 | Usage | KPI strip, notice, period toggle, By model, By project, right rail match the artboard | keep (no diff) | Nothing to fix | n/a |
| 8 | Warnings | No "Not measured is not the same as passed." note | fix | text only | `5c40010`; `WarningsPlanView.test.jsx` "says in the Not checked block that not measured is not the same as passed" |
| 9 | Warnings | Artboard: separate "Warnings" and "Improvement plan" sections, "Could not be measured" table always open; app: one numbered card list (Fix now / Improve / Watch) and a collapsed "Not checked" block | keep | Different data model (steps carry severity) and collapse behaviour; changing it is a behaviour change. The note (row 8) sits inside the collapsed block | n/a |
| 10 | Kit | "no data is not zero" note | keep (already present) | Exists since HUN-51 under "Show details"; this task adds the missing test only | `KitView.test.jsx` "no data is not zero note" (new) |
| 11 | Kit | easy-ui checks were a bullet list; artboard: cells with a pass/fail bar, passed mint, failed amber | fix | markup + CSS on data the page already has | `3ec20b4`, `37af7ea` (one row); `KitView.test.jsx` "lays the counts out as a strip of cells..." and "shows no bar when ... all zero" |
| 12 | Kit | Artboard is one project table (kernel / verifier / context / git); app is cards plus a details table, plus the "Kit by workspace" block | keep | Cards carry the spark, health chip and next action; the table is behind "Show details". A re-layout is a behaviour change | n/a |
| 13 | Workspaces | Legacy banner: text with Adopt stacked below, no icon, ids in plain text | fix | CSS row layout + icon + mono ids | `1ff0980`; `WorkspacesView.test.jsx` banner test (mono id, svg) |
| 14 | Workspaces | Adopt with "This will change:" preview | keep (already present) | Adopt calls the plan/confirm flow (`act.review`) and the `Changes` block says "This will change:". Needs no server work | existing legacy-banner tests (AC6) |
| 15 | Workspaces | "Changes" block was a normal card; artboard: the one lit block with a solid Confirm | fix | CSS only | `1ff0980` (visual; no new element) |
| 16 | Workspaces | Card meta: email and repo count on two lines; aliases as a disc list; no letter when there is no icon; orphan card solid; connectors not in boxes | fix | markup + CSS | `1ff0980`; `WorkspacesView.test.jsx` "puts email and repo count on one line, aliases as chips, and a letter in the tile..." (also `data-orphan`) |
| 17 | Workspaces | Orphan card is listed first; artboard lists it last | needs server | Order comes from the API list; sorting it in the page changes behaviour and is not asked for | for the owner |
| 18 | Workspaces | Field label `Alias` vs artboard `alias`; "Used by" wording matches | keep | Label text is asserted by tests; sample styling | n/a |
| 19 | Agents | `missing` card showed "last active never / Tokens: 0 / Plan limits" | fix | text only | `31f09d3`; `AgentsView.test.jsx` "a missing folder says the folder is gone..." |
| 20 | Agents | Status block plain; artboard raised box, mono lines, person icon instead of the emoji, 5 px bars | fix | CSS + inline svg | `31f09d3`; `AgentsView.test.jsx` (`.who` with svg) |
| 21 | Agents | Workspaces group plain list; artboard fieldset with legend; detected card not lit | fix | CSS + `data-status` attribute | `31f09d3`; `AgentsView.test.jsx` "each card carries its status so a detected account can be lit" |
| 22 | Agents | Folder shown as `/home/o/.claude`; artboard `~/.claude` | needs server | The page does not know the home folder; the API returns the absolute path | for the owner |
| 23 | Agents | Limit line has no "near limit" text at 78 %; Ignore button on every confirmed card (artboard: Rename only on the first); chip icon; "← Back to First run" link | keep | "near limit" needs a threshold rule the artboard does not define (amber bar from 70 % already shows it); Ignore is working behaviour; the link needs routing, not markup | n/a |

Needs server: rows 17 and 22. Keep rows the owner should know: 9 and 12 (Warnings and Kit layouts differ from the artboard on purpose) and 23.

### Red token decision (AC3)

`--color-red` changed from `#ef7b7b` to `#e8968f` in `web/src/tokens.css`. Check run in this session:
- Every red in the five view artboards (Main, Warnings, Kit, Workspaces, Agents): `grep -ohiE "#[0-9a-f]{6}"` over the five files lists 1 use of `#E8968F` (Workspaces, the Delete button) and no other red or pink hex (the rest are greys, mint, amber `#E3B66B`, violet `#B89CFF`, cyan `#4FE3F0`). So every artboard red is `#E8968F`.
- WCAG relative luminance contrast, computed with `python3` (sRGB to linear, `(L1+0.05)/(L2+0.05)`):
  `#e8968f` on `--color-bg #121413` = **8.10:1** (commit message 8.1:1 confirmed); on `--color-raised #181b1a` 7.59:1; on `--color-input #0e100f` 8.36:1. Old `#ef7b7b` on bg was 6.86:1. Required: at least 4.5:1. Pass.
- `shell.test.js` re-checks `>= 4.5` on `--color-bg` at every test run.


**DELTA**: The five views now read like the Graphite Mint artboards: a user sees the connector banner with Adopt on one row, a lit "This will change:" block, a plain "folder is gone" card for missing agent accounts, the easy-ui checks as a pass/fail bar, the "not measured is not passed" note, and one problem red; no API or behaviour changed.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T070.jsonl`, never the
implementing agent alone]
