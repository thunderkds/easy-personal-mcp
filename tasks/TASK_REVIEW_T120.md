# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/shell.test.js` (new describe "Plex Mono everywhere and one step larger text (T120)", 4 tests). Red against the prior CSS (stash of tokens.css/app.css/appsession.css): 4 failed / 12 passed; green after: 16 passed |
| Verification command run | ☑ pass | `cd web && npx vitest run` → Test Files 42 passed (42), Tests 1421 passed (1421) (baseline 1417 + 4 new); `npm run build` ✓ built; `python3 scripts/check_static.py` → `check_static: ok`; `pytest tests/test_web_toolchain.py` 16 passed |
| Negative cases hold | ☑ pass | the 4 new tests fail on the base CSS (see first row); no existing test needed a change |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | 3 stylesheets + the font folder; every view is affected visually, so Usage, Live (list, panel, Past list, Past panel), Settings, Warnings and Memory were shot at 375 and 1280. Not reviewed: Kit, Workflow, Workspaces, Agents, Board, Scheduled, Watch, First run (same tokens, no layout rule changed) |
| Full smoke suite still green (no regression) | ☑ pass | the whole vitest suite: 42 files / 1421 tests |
| **Companion score (easy-verifier)** | ☑ N/A | not run (frontend agent; Supervisor decides) |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | before/after screenshots in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t120/shots-before` and `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t120/shots-after` (usage, live-list, live-panel, past-list, past-panel, settings, warnings, memory; each -375 and -1280). Frontend agent read them; verdict table in the final report. B-e: body 15 px < the skill's 16 px bar, still fail (owner chose 15) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | computed in headless Chrome: body `15px`, the only text family `"IBM Plex Mono"` on every shot (before: Plex Sans + Plex Mono, body 14px); 4 new tests pin tokens (no family literal, no px/rem font size outside tokens.css, scale 15/14 in 1/32 rem, real 400/500/600 faces) |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280: `scrollWidth == clientWidth` on every shot (no horizontal page scroll). The By tool and By model tables still scroll inside their card at 375 as before (wider with mono: right edge 572 vs 539 px) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-19, before any implementation commit; base `e4e2778`; verbatim from `web/src/tokens.css`):

```css
--font-sans: 'IBM Plex Sans', system-ui, sans-serif;
--font-mono: 'IBM Plex Mono', ui-monospace, 'SF Mono', 'Cascadia Mono', Menlo, Consolas, monospace;
--fs-label: 0.71875rem;  --fs-small: 0.75rem;  --fs-note: 0.78125rem;  --fs-cell: 0.8125rem;
--fs-control: 0.84375rem;  --fs-body: 0.875rem;  --fs-card-title: 0.90625rem;  --fs-section: 0.9375rem;
--fs-brand: 1rem;  --fs-h2: 1.0625rem;  --fs-kpi: 1.625rem;  --fs-h1: 1.75rem;
/* @font-face: IBM Plex Sans 400/500/600, IBM Plex Mono 400/500 (no Mono 600) */
```

Hard-coded font sizes outside the tokens (`app.css`): `.day-labels` 11px (l.307), `.rail-card > h2` 0.875rem (l.489), `.ring-text` 17px (l.494), `.memory-node text` 13px (l.577), `.memory-mark text` 11px + weight 600 (l.586), `.toast-close` 1.125rem (l.702). `body` and 2 more rules use `var(--font-sans)` (app.css l.6, 53, 173).

Before screenshots (fixture-mock of the app routes, headless Chrome via DevTools): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f3f564e-a23f-434b-8267-534730790636/scratchpad/ux-t120/shots-before/{usage,live-list,live-panel,past-list,past-panel,settings,warnings}-{375,1280}.png`. Measured: computed body font-size `14px`, families `"IBM Plex Sans"` + `"IBM Plex Mono"`; no horizontal page scroll at 375 or 1280.

**AFTER** (verbatim from `web/src/tokens.css` and the shots):

```css
--font-mono: 'IBM Plex Mono', ui-monospace, 'SF Mono', 'Cascadia Mono', Menlo, Consolas, monospace;
--font-sans: var(--font-mono);
--fs-label: 0.78125rem;  --fs-small: 0.8125rem;  --fs-note: 0.84375rem;  --fs-cell: 0.875rem;
--fs-control: 0.90625rem;  --fs-body: 0.9375rem;  --fs-card-title: 0.96875rem;  --fs-section: 1rem;
--fs-brand: 1.0625rem;  --fs-h2: 1.125rem;  --fs-kpi: 1.75rem;  --fs-h1: 1.875rem;
/* @font-face: IBM Plex Mono 400, 500, 600 (600 is the genuine file from the npm package @fontsource/ibm-plex-mono 5.3.0); Plex Sans removed */
```

Measured on the after shots: body `15px`, only family `"IBM Plex Mono"`, no horizontal page scroll at 375 and 1280.

**DELTA**: The whole dashboard reads in IBM Plex Mono at one step larger text (body 15 px), with a real semibold face for headings.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T120.jsonl`, never the
implementing agent alone]
