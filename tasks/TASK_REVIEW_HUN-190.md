# TASK_REVIEW — HUN-190: [Short Title]

> Sibling of `tasks/TASK_GUIDE_HUN-190.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/Widgets.test.jsx` (7 `useTopN` tests incl. 0/1/10/11/25 rows, Infinity limit, focus, count format), `web/src/UsageView.test.jsx` (`top 10 lists (HUN-190, FR-058)`, 12 tests: AC1 project 25 rows 10 + "Show 15 more" -> 25 + "Show fewer" -> 10, AC2 table own button / model / tool / outliers, AC3 10, 11, 0 rows, AC4 bar width 4% folded and open, AC5 By day 25 bars/rows no button, AC6 period and workspace fold again, AC7 native button, independent lists). Red first (`useTopN is not a function`), green after. Output: `Test Files 2 passed (2) / Tests 91 passed (91)` for `vitest run Usage Breakdown Widgets`. |
| Verification command run | ☑ pass | `cd web && npx vitest run Usage Breakdown Widgets` -> `Test Files 2 passed (2), Tests 91 passed (91)` ("Breakdown" matches no file; Breakdown is covered through UsageView.test.jsx); `npm --prefix web run build` -> `✓ built in 525ms`; `uv run pytest tests/test_web_toolchain.py -q` -> `16 passed in 3.33s` |
| Negative cases hold | ☑ pass | 0 rows: empty text, no button; <=10 rows: no button; By day never folded; By model has 5 rows on owner data: no button (real run) |
| verify | ☐ N/A | Frontend implementer ran the real run (below); `verify` skill left to the Supervisor at Stage 5. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ N/A | Left to Stage 4 reviewer. Blast radius: `Breakdown.jsx` (By day/project/model/tool, all in Usage), `UsageView.jsx` Outliers, new export `useTopN`/`TOP_N` in `Widgets.jsx` (no other consumer). |
| Full smoke suite still green (no regression) | ☑ pass | `cd web && npx vitest run` -> `Tests 467 passed (467)` |
| **Companion score (easy-verifier)** | ☑ N/A | Not run by the implementer; no companion tool run for this task. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Real dashboard, owner data, Usage 30 days, headless Chrome. After: By project and Outlier sessions end with the button under row 10 (`Show 113 more`, `Show 104 more`), By tool `Show 91 more`; button looks like the Kit button (same `.details-toggle`, no new CSS). Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun190/after-1280.png`, crop `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun190/after-1280-crop.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun190/after-375.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun190/after-expanded-1280.png`; before: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun190/before-1280.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun190/before-375.png`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Reuses `.details-toggle`; `git diff main -- web/src/app.css web/src/tokens.css` is empty (no new CSS, no new colour or token). |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280 px: `scrollWidth 1280 / clientWidth 1280`; 375 px (after `Menu` click): `scrollWidth 375 / clientWidth 375`. No horizontal scroll. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-02T11:24:54Z, branch HUN-190 at 45b8e06 (no implementation commit), real `easy-personal dashboard` (page built from main) on the owner's data, Usage > 30 days, headless Chrome; driver `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/093552ec-c663-4553-a758-f85c71fffea0/scratchpad/hun190/shot.cjs`. Rows rendered per list:

```
1280 {"project_bars":123,"project_table":123,"model":5,"tool":101,"outliers":114,"day_bars":27,"buttons":{"project":[],"model":[],"tool":[],"outliers":[],"day":[]},"scrollWidth":1280,"clientWidth":1280}
375  {"project_bars":123,"project_table":123,"model":5,"tool":101,"outliers":114,"day_bars":27,"buttons":{...all empty},"scrollWidth":375,"clientWidth":375}
```

No list has a fold button. Screenshots: `.../scratchpad/hun190/before-1280.png`, `.../scratchpad/hun190/before-375.png`.

**AFTER**: captured 2026-11-02T11:27:54Z, branch HUN-190 after the implementation commit, same driver and data (Usage, 30 days):

```
1280 {"project_bars":10,"project_table":10,"model":5,"tool":10,"outliers":10,"day_bars":27,"buttons":{"project":["Show 113 more|false","Show 113 more|false"],"model":[],"tool":["Show 91 more|false"],"outliers":["Show 104 more|false"],"day":[]},"scrollWidth":1280,"clientWidth":1280}
after clicking the first button of each list (By project bars, By tool, Outliers):
expanded {"project_bars":123,"model":5,"tool":101,"outliers":114,"buttons":["Show fewer|true","Show 113 more|false","Show fewer|true","Show fewer|true"]}
375  {"project_bars":10,"project_table":10,"model":5,"tool":10,"outliers":10,"day_bars":27,"buttons":{same as 1280},"scrollWidth":375,"clientWidth":375}
```

Rows before -> after: By project bars 123 -> 10, By project table 123 -> 10, By tool 101 -> 10, Outlier sessions 114 -> 10, By model 5 -> 5 (no button), By day 27 -> 27. Expanded: 123 / 101 / 114 rows; the project table button stayed independent (`Show 113 more`).

**DELTA**: The owner opens Usage and sees the top 10 of each ranked list, and one click on "Show N more" shows every row.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
