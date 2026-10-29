# TASK_REVIEW — HUN-128: Dashboard shell in a centered max-width container

> Sibling of `tasks/TASK_GUIDE_HUN-128.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/shell.test.js` (3 tests: `--shell-max: 90rem`; `.shell` has `max-width: var(--shell-max)` + `margin-inline: auto`; `border-inline: 1px solid var(--color-divider)` only inside `@media (min-width: 90.001rem)`). Red before the CSS change: `Tests 3 failed` (`expected '/* Design tokens...' to match /--shell-max:\s*90rem;/`). Green after: `Test Files 1 passed (1) / Tests 3 passed (3)`. |
| Verification command run | ☑ pass | `npm --prefix web test && npm --prefix web run build && uv run pytest -q`, rc=0, 2026-10-29: `Test Files 11 passed (11)`, `Tests 248 passed (248)`, `✓ built in 328ms`, `815 passed, 5 deselected, 1 xfailed, 1 warning in 140.97s`. |
| Negative cases hold | ☑ pass | At exactly 1440 px the shell has no border (`border 0px/0px`) and `scrollWidth 1440 <= clientWidth 1440`; at 1024 and 375 px no horizontal scroll (`1024/1024`, `375/375`); 375 px menu overlay opens at x 0 w 232 (not clipped by the container); no color literal added outside `tokens.css` (the one hex hit is the pre-existing test fixture `web/src/test/fixtures.js:317`, not CSS). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q`: `815 passed, 5 deselected, 1 xfailed, 1 warning in 140.97s (0:02:20)`; Vitest `248 passed (248)`. |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Host Playwright (easy-ui has a fixed 1280x720 viewport) on a fixture dashboard. Verdict: at 1920 px the shell is a centered 1440 px container with 1 px divider borders left and right, page background on both sides; at 1440 px identical to before. `reports/HUN-128/before-usage-{1920,1440,1024,375}.png` vs `after-usage-{1920,1440,1024,375}.png` (+ `-375-menu-open.png`), rects in `results-before.json` / `results-after.json`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Method: grep for `#hex`/`rgb(`/`hsl(` in `web/src` outside `tokens.css` and tests, empty for CSS/JSX. New values: `--shell-max: 90rem` in `tokens.css`; border uses `var(--color-divider)`. |
| **UI: Responsiveness at target viewports** | ☑ pass | Rects (x/w), after change: 1920: shell x240 w1440, border 1px/1px, sidebar x241, rail x1391 w260, scrollW=clientW=1920. 1440: shell x0 w1440, border 0, sidebar x0 w232, rail x1152 w260, scrollW=clientW=1440. 1024: shell x0 w1024, sidebar x0 w232, rail hidden, scrollW=clientW=1024. 375: shell x0 w375, sidebar hidden, menu opens sidebar x0 w232, scrollW=clientW=375. Sticky at 1920 after `scrollTo(0,1500)` (scrollY 1500): sidebar x241 y0 (same x), topbar y0, rail x1391 y56 (same x, stuck under the header). No overflow findings. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-29T11:30:56Z on the unchanged build (worktree `HUN-128` at main, no implementation commit), host Playwright 1920x1080, fixture dashboard (temp `EASY_PERSONAL_HOME`/`EASY_PERSONAL_AGENT_ROOT`), command `check.mjs before` (script kept in the session scratchpad, rects in `reports/HUN-128/results-before.json`):

```
1920 shell x0 w1920 border 0px/0px | scrollW/clientW 1920 1920 | sidebar x0 w232 | rail x1632 w260
1440 shell x0 w1440 border 0px/0px | scrollW/clientW 1440 1440 | sidebar x0 w232 | rail x1152 w260
1024 shell x0 w1024 border 0px/0px | scrollW/clientW 1024 1024 | sidebar x0 w232 | rail x0 w0
375  shell x0 w375  border 0px/0px | scrollW/clientW 375 375 | sidebar x0 w0 (hidden) | menu open: sidebar x0 w232
```

At 1920 px the `.shell` is 1920 wide at x 0 (full screen width); screenshot `reports/HUN-128/before-usage-1920.png`.

**AFTER**: captured 2026-10-29T11:31:42Z, same script and fixture, after the CSS change (`reports/HUN-128/results-after.json`):

```
1920 shell x240 w1440 border 1px/1px | scrollW/clientW 1920 1920 | sidebar x241 w232 | rail x1391 w260 | scrolled(y=1500) sidebar x241 y0, topbar y0, rail x1391 y56
1440 shell x0 w1440 border 0px/0px | scrollW/clientW 1440 1440 | sidebar x0 w232 | rail x1152 w260
1024 shell x0 w1024 border 0px/0px | scrollW/clientW 1024 1024 | sidebar x0 w232 | rail x0 w0
375  shell x0 w375 border 0px/0px | scrollW/clientW 375 375 | sidebar hidden | menu open: sidebar x0 w232
```

Screenshot `reports/HUN-128/after-usage-1920.png`.

**DELTA**: On a screen wider than 1440 px the dashboard now sits in a centered 1440 px container with the page background on both sides, instead of stretching to the full screen width.

**WITNESS**: Frontend-Implementer agent (Claude) ran the Playwright script and the verification command in worktree `HUN-128` on 2026-10-29; the trace hook attributes the Bash calls to HUN-128 (`memory/event-trace/HUN-128.jsonl`). Reviewer re-check pending.
