# TASK_REVIEW — HUN-129: Usage By model card: no clipped cost column

> Sibling of `tasks/TASK_GUIDE_HUN-129.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/UsageView.test.jsx` test `By model: short visible headers, full names for assistive tech, in a compact table`. Red first: `AssertionError ... - "Total" + "Total tokens"` (1 failed, 42 passed). Green after the change: `Test Files 11 passed (11)  Tests 248 passed (248)`. Two older assertions adapted (headers by aria-label; bare `Cost` text allowed only on a TH). Layout checks (Success 2-3): host Playwright, see the UI rows |
| Verification command run | ☑ pass | `npm --prefix web test && npm --prefix web run build && uv run pytest -q` exit 0: `Tests 248 passed (248)`; `built in 177ms`; `815 passed, 5 deselected, 1 xfailed, 1 warning in 126.31s` |
| Negative cases hold | ☑ pass | A long model id ellipsizes with the full id in `title` (1024 px: `claude-sonn...`); the table never widens the card. Abstained cost still renders `abstained: mystery-model` and never `$0` (existing test `shows abstained cost as abstained, not $0` green). Other tables keep `Total tokens` / `API-equivalent cost` headers (existing test green) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q`: 815 passed, 5 deselected, 1 xfailed; Vitest 248 passed |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Screenshots in `reports/HUN-129/`: `before-bymodel-{1440,1024,375}.png` vs `after-bymodel-{1440,1024,375}.png` (full pages `*-page-*.png`). After: all four columns and the `Cost` header sit inside the card at 1440 and 1024; cost header no longer wraps over 3 lines |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `git diff -U0 -- web/src/app.css web/src/Breakdown.jsx | grep -E '#[0-9a-fA-F]{3,6}|rgb|hsl'` printed nothing (exit 1). Added CSS uses only `4px` padding and `6rem` min-width, no color; no new dependency |
| **UI: Responsiveness at target viewports** | ☑ pass | By model `.table-wrap` scrollWidth/clientWidth, fixture data (`reports/HUN-129/results-after.json`): 1440 = 382/382 (before 390/382); 1024 = 318/318 (before 390/318); 375 = 304/301 (before 390/301), so 375 keeps the inside scroll. Page scrollWidth/clientWidth 1440/1440, 1024/1024, 375/375. Cost cell right edge inside the card at all three |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-29T11:43Z on the unchanged build, fixture dashboard, `reports/HUN-129/results-before.json`; headers read `Model,Total tokens,Output,API-equivalent cost`):

```
1440 wrap scrollW/clientW 390/382 | page 1440/1440 | cost-col right 1111 vs card right 1124 inside: true
1024 wrap scrollW/clientW 390/318 | page 1024/1024 | cost-col right 1047 vs card right 996 inside: false
375  wrap scrollW/clientW 390/301 | page 375/375   | cost-col right 427 vs card right 359 inside: false
```

Screenshot of the card at 1440 px: `reports/HUN-129/before-bymodel-1440.png` (cost header wraps over 3 lines and is cut at the right edge; the table scrolls 8 px sideways). Other widths: `before-bymodel-1024.png`, `before-bymodel-375.png`.

**AFTER** (same script, 2026-10-29T11:45:58Z, `reports/HUN-129/results-after.json`; headers read `Model,Total,Output,Cost`):

```
1440 wrap scrollW/clientW 382/382 | page 1440/1440 | cost-col right 1103 vs card right 1124 inside: true
1024 wrap scrollW/clientW 318/318 | page 1024/1024 | cost-col right 975 vs card right 996 inside: true
375  wrap scrollW/clientW 304/301 | page 375/375   | cost-col right 341 vs card right 359 inside: true
```

Screenshots: `reports/HUN-129/after-bymodel-1440.png`, `after-bymodel-1024.png`, `after-bymodel-375.png`.

**DELTA**: The owner reads the whole By model table, cost column included, at 1440 and 1024 px without scrolling sideways.

**WITNESS**: frontend-developer agent ran the Playwright script and the verification command in worktree `HUN-129` on 2026-10-29 (11:43Z BEFORE, 11:45Z AFTER); the trace hook records the Bash calls under HUN-129 in `memory/event-trace/HUN-129.jsonl`.
