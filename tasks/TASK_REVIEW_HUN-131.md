# TASK_REVIEW — HUN-131: [Short Title]

> Sibling of `tasks/TASK_GUIDE_HUN-131.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/neon.test.jsx` (8 tests: accent + tints, link tokens, 4 glow values, `prefers-contrast: more` override, glow on each listed class with the matching token, active period button style, no glow on shared widgets, active nav/period at default load + `kpi-cache` class). Written first: `7 failed | 1 passed` before the change. After: `Test Files 12 passed (12) / Tests 256 passed (256)`. |
| Verification command run | ☑ pass | `npm --prefix web test` -> `Test Files 12 passed (12), Tests 256 passed (256)` (BEFORE: 248). `npm --prefix web run build` -> `✓ built in 205ms` (css `index-DQ6CI46t.css` 21.19 kB). `uv run pytest -q` -> `815 passed, 5 deselected, 1 xfailed, 1 warning in 105.45s`. |
| Negative cases hold | ☑ pass | grep `#[0-9a-fA-F]{3,8}\b\|rgba?\(` in `web/src/app.css` + non-test `web/src/*.jsx` -> no match (exit 1). Computed styles at 1440 px: `.rail-card .progress-fill` not rendered (no limit data), `.bar:not(.bar-latest)` `box-shadow: none`; test asserts `.progress-fill`, `.bar`, `.bar-fill` carry no shadow/filter. `prefers-contrast: more` (Playwright `contrast:'more'`): nav `text-shadow: none`, `.bar-latest` `box-shadow: none`, `.kpi .spark-line` `filter: none`, `.ring-value` `filter: none`. Empty states: ring/progress/value elements are not rendered without data, so no glow. |
| verify | ☐ N/A | Supervisor/hook runs `/verify`. Implementer's real run on fixtures (synthetic 7-day sessions, temp `EASY_PERSONAL_HOME`/`CLAUDE_PROJECTS`/`AGENT_ROOT`): easy-ui session `5b49d141-e64a-4ed3-b8cd-4d5782e4d0f5` ended (passed), `ui_check` "Check passed" (active nav colour `rgb(60, 245, 168)`, `.bar-latest` 18px glow, ring + KPI sparkline drop-shadow) — pass. Reports: `~/workspace/pets/hungnguyen111/easy-ui-mcp/reports/session-5b49d141-e64a-4ed3-b8cd-4d5782e4d0f5.{html,json}`. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review 2026-10-29 on `git diff main...HEAD`: tokens.css (accent, tints, link, 4 glow tokens, prefers-contrast override), app.css (glow on nav, dot, period button, KPI spark, cache KPI, bar-latest, top project bar, ring, rail numbers; link tokens), UsageView.jsx (`kpi-cache` class), neon.test.jsx; callers: App.jsx shell, Widgets/Breakdown/Rail via shared classes. lane_check `lane: C1, 5 files, 309 lines, EXCEEDS (lines 309 > 250); unpredicted: tasks/TASK_GUIDE_HUN-131.md` — overage is guide + review docs, code ~55 lines; no relabel. Entry point `App.jsx` reachable. Findings: P0 none; P1 1 (process: implementer read `agents/frontend.md` first 40 lines only, guide requires a full read; no code defect traced to it; recorded, conf 100); P2 none; P3 1 out of scope (fixture models abstain from pricing → cost KPI shows a long wrapped `abstained:` value and By model cost cell clips; pre-existing, not this diff; follow-up candidate, conf 75). Supervisor viewed `shot-top-usage-1440.png`: glow matches the artboard. security-review: not required (Risk Low). 0 fixes applied. |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `815 passed, 5 deselected, 1 xfailed`; Vitest 256 passed. |
| **Companion score (easy-verifier)** | ☑ N/A | not run by the implementer; Stage 4/5 owns it. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Host Playwright (real viewports) screenshots on the fixture dashboard, read with vision: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/5a654c3b-f981-4de5-b6fd-f21813c8d2f1/scratchpad/shot-top-usage-1440.png` shows neon `#3CF5A8` active nav with inset ring and dot glow, active "Last 7 days" soft-bg + accent text + ring, glowing sparklines, Cache hit ratio value + bar, glowing latest By-day bar, glowing Cache reuse ring with accent centre text; layout as in HUN-129. Also `shot-top-warnings-1440.png`, `shot-top-kit-1440.png`. easy-ui shots (1280x720, fixed viewport): `~/workspace/pets/hungnguyen111/easy-ui-mcp/reports/screenshot-1793287765842.png`. Computed at 1440 px: nav `text-shadow rgba(60,245,168,.35) 0 0 12px` + inset ring; dot `0 0 8px rgb(60,245,168)`; `.bar-latest` `0 0 0 1px …/.25, 0 0 18px …/.25`; spark/ring `drop-shadow(rgba(60,245,168,.6) 0 0 3px)` — equal to `renderVals()`. Not shown in the fixture (no data, so not exercised in a browser): Warnings rail step numbers (CSS asserted by Vitest) and the Kit "checks passed" element, which the app does not have (not added; layout/copy out of scope). Top-project bar glow is computed (`0 0 8px`) but sits below the fold in the top shots. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | grep for hex/`rgb(` outside `tokens.css` -> no match. Contrast computed (WCAG): accent `#3CF5A8` on page `#121413` 13.05:1; on soft bg 11.15:1; link `#7EDDB0` on page 11.33:1 (all >= 4.5:1). No text sits on an accent fill. Link rendered colour at 1440 px: `.family a` `rgb(171,179,174)` (secondary, as before); `a` uses `--color-link`. |
| **UI: Responsiveness at target viewports** | ☑ pass | Host Playwright at 375 / 1024 / 1440 px, `scrollWidth` vs `clientWidth`: 375/375, 1024/1024, 1440/1440 (no horizontal scroll). Shots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/5a654c3b-f981-4de5-b6fd-f21813c8d2f1/scratchpad/shot-top-usage-375.png`, `shot-top-usage-1024.png`, `shot-top-usage-1440.png` (+ `shot-after-*` full-page). Sparkline glow not clipped at 375 px (`.kpi .spark { overflow: visible }`). Note: easy-ui's viewport is fixed at 1280x720, so the 3 widths come from host Playwright (learnings HUN-128/129). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-29T15:23:48Z in the HUN-131 worktree, before any implementation commit; the one `box-shadow` match is the existing KPI-strip divider):

```
2026-10-29T15:23:48Z
$ grep -nE "accent|glow|link" src/tokens.css
70:  --color-accent: #7eddb0;
71:  --color-accent-hover: #b2edcf;
72:  --color-accent-soft: #7eddb017;
73:  --color-accent-bar: #7eddb066;
$ grep -cE "text-shadow|box-shadow|drop-shadow" src/app.css
1
$ npm --prefix web test (tail)
      Tests  248 passed (248)
   Start at  22:23:48
   Duration  4.34s (tests 57%, environment 31%, transform 5%, setup 5%, import 1%)

Environment  jsdom was created 11 times · 7.91s total, 31% of tracked time
             create it once per worker with pool: 'vmThreads' (keeps per-file isolation) or isolate: false (shares it across files)
             learn more: https://vitest.dev/guide/improving-performance#test-environments
```

**AFTER** (same commands, 2026-10-29, after the change):

```
$ grep -nE "accent|glow|link" web/src/tokens.css
--color-accent: #3cf5a8;  --color-accent-soft: #3cf5a814;  --color-accent-line: #3cf5a859;  --color-accent-bar: #3cf5a84d;
--color-link: #7eddb0;  --color-link-hover: #b2edcf;
--glow-text: 0 0 12px #3cf5a859;  --glow-box: 0 0 0 1px #3cf5a840, 0 0 18px #3cf5a840;
--glow-dot: 0 0 8px #3cf5a8;  --glow-line: drop-shadow(0 0 3px #3cf5a899);
$ npm --prefix web test   ->  Test Files 12 passed (12), Tests 256 passed (256)
```

**DELTA**: The dashboard shows the neon mint look with glow on the active nav and period button, KPI sparklines, Cache hit ratio, latest day bar, top project bar, Cache reuse ring and Next actions numbers, and the glow turns off under `prefers-contrast: more`.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-131.jsonl`, never the
implementing agent alone]
