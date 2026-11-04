# TASK_REVIEW — HUN-216: Warnings & plan page: one list with causes (FR-074)

> Sibling of `tasks/TASK_GUIDE_HUN-216.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/WarningsPlanView.test.jsx` (AC1 order+once, AC2 causes/new/steps, AC3 cache share, AC4 abstained after hits, AC5 empty state, AC6 hint+copy with mocked clipboard, AC7 scope calls kept). 26 tests pass (16 old kept, 10 new). |
| Verification command run | ☑ pass | 2026-11-04 14:45 UTC: `npx vitest run src/WarningsPlanView.test.jsx` -> `Test Files 1 passed (1) / Tests 26 passed (26)`; `npx vitest run` -> `Test Files 21 passed (21) / Tests 599 passed (599)`; `npm run build` -> `built in 272ms`. Before the change 10 of the 26 failed (red first). |
| Negative cases hold | ☑ pass | No causes -> no block, steps still shown; `cache_read_share` absent -> no line, 0 -> "0% ..."; clipboard missing and clipboard rejected -> error toast "copy it by hand", no crash; 403 -> existing alert kept. Causes render only the server's fixed `text` (and the `new` flag); no other field is read. |
| verify | ☐ N/A | Supervisor runs `/verify` at Stage 5. Implementer real run (AC8): `easy-personal dashboard` on a `.backup` copy of the owner's store, page opened in headless Chrome, "Warnings & plan" clicked: shows the owner's `tokens_jump` hit with 3 causes, "97% of these tokens are cache reads", 2 diagnosis steps, "Not checked (1)" and the hint with Copy button — pass. The real-browser click on Copy was not run (vitest covers it with a mocked clipboard). |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Changed `WarningsPlanView.jsx`, its test, and 7 CSS rules in `app.css`. Consumers: `App.jsx` only (already inside `ToastProvider`). Full web suite run, not only this file. |
| Full smoke suite still green (no regression) | ☑ pass | `npx vitest run`: 21 files, 599 tests pass; `npm run build` ok. |
| **Companion score (easy-verifier)** | ☑ N/A | Not run by the implementer; Stage 4 belongs to the Supervisor. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE `tasks/evidence/HUN-216/before-1280.png`; AFTER `tasks/evidence/HUN-216/after-1280.png` (1280 px wide) and `tasks/evidence/HUN-216/after-375.png` (375 px wide). One list, "Why this happened" block, cache line, steps, abstained, hint at the end, as in AC8. Viewed by the implementer. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Diff of `web/src/app.css` grepped for hex and rgb colour literals: 0 matches. New rules use only `var(--color-*)`, `--space-*`, `--fs-*`, `--font-mono`. Accessibility (vitest): items in `ol.actions`, causes in a nested `ul` inside `role="group"` named "Why this happened", button named "Copy the investigate command". |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280 px: item fills the main column. 375 px (mobile emulation): `scrollWidth 375 vs clientWidth 375`, no horizontal scroll; the long command wraps. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-11-04 14:43:24 UTC, before any implementation commit; `easy-personal dashboard` on a `.backup` copy of the owner's store, headless Chrome 1280x720, click on "Warnings & plan"; screenshot `tasks/evidence/HUN-216/before-1280.png`; page text read back from `<main>`):

```
scrollWidth vs clientWidth: 1280 vs 1280
Warnings and plan

1 to watch

1
Token use jumped
Watch
last 7 days

Token use rose 213.2% in the last 7 days against the 7 days before; target 50% or less

Why it matters A sudden rise is easy to miss and quickly becomes the new normal, along with its cost.

The rise comes from more sessions (81 → 396), not bigger ones: tokens per session went from 15.1M to 9.7M.
Biggest rise: easy-verifier-mcp (+442.7M tokens). Decide if that pace is intended before it becomes your baseline.
2 steps
Not checked (1)
```

The page has no "Why this happened" block (the `causes` and `cache_read_share` in `/api/warnings` are not rendered) and no investigate hint or copy button.

**AFTER** (2026-11-04 14:45 UTC, same steps, same store copy; screenshots `tasks/evidence/HUN-216/after-1280.png`, `after-375.png`):

```
scrollWidth vs clientWidth: 1265 vs 1265   (375 px run: 375 vs 375)
Warnings and plan

1 to watch

1
Token use jumped
Watch
last 7 days

Token use rose 213.2% in the last 7 days against the 7 days before; target 50% or less

Why it matters A sudden rise is easy to miss and quickly becomes the new normal, along with its cost.

Why this happened
easy-verifier-mcp: 476.4k → 443.3M tokens, 2 → 11 sessions
personal-agentic-claude: 491.6M → 929.7M tokens, 20 → 90 sessions
easy-personal-mcp: 243.2M → 634.2M tokens, 4 → 30 sessions

97% of these tokens are cache reads

The rise comes from more sessions (81 → 396), not bigger ones: tokens per session went from 15.1M to 9.7M.
Biggest rise: easy-verifier-mcp (+442.9M tokens). Decide if that pace is intended before it becomes your baseline.
2 steps
Not checked (1)

Ask Claude to investigate: run /mcp__easy-personal__investigate in Claude Code. Copy
```

**DELTA**: The owner can now read, under each warning, which projects, sessions or tools caused it and what share of the tokens were cache reads, and copy the command that asks Claude Code to investigate.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-216.jsonl`, never the
implementing agent alone]
