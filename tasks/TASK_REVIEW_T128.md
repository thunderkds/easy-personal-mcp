# TASK_REVIEW — T128: Live like Claude 2e — New session in Live, todo checklist

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

## Auto decisions

Mode: auto
1. App sessions tab removed from navigation, its URL opens Live — FACT: PRD.md:232 "the App sessions tab is removed from the navigation (its URL opens Live)"
2. Direction Path A of `BRAINSTORMING_LOG_LIVE2.md` — DEBATED: Path A (Advocate 1: Path A, the only path whose Start/Read bullets cover T128, BRAINSTORMING_LOG_LIVE2.md:32-33; Advocate 2: Path A over keeping the tab, with three losses to patch: privacy line, stale PRD FR-128 wording, stale LiveView comment) ⚠ decided for you — please review
3. Advocate 2's three losses fixed in Stage 4 (`51b2a1e`): memory-only line restored in NewSessionSheet, PRD FR-128 wording aligned with FR-133, LiveView comment — RULE: `.claude/skills/code-review/SKILL.md` § Phase 4 — Apply Safe Fixes
4. Resume/Fork while the sheet is open replaces the typed form; no New session button on the retired Python server — SAFER OPTION: both keep FR-133 as written and add no state; recorded as P3, not built ⚠ decided for you — please review
Checks: tests `npx vitest run` — 43 files, 1470/1470 pass (Supervisor re-run after fixes); mutation (todo reader + ?app-sessions disabled) — 5 red, restored green; reviews code-review — 0 P0/P1/P2, 2 P3; security-review — no finding >= 8; real run `npm run build` — printed `check_static: ok` (implementer run)
Memory: memory/decisions.md — 2026-11-20 — T124-T128 Live like Claude 2 resumed and Stage 4 under [admin] (T128: New session sheet in Live, App sessions tab gone)
Not done: merge (admin: after the auto verify pass); auto verify: running

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/AppSessionEntry.test.jsx` (AC 1,2,3,5), `web/src/LivePanel.test.jsx` (AC 4), `web/src/NewSessionSheet.test.jsx`, `web/src/App.test.jsx` (nav) |
| Verification command run | ☑ pass | `npx vitest run`: Test Files 43 passed (43), Tests 1470 passed (1470); `npm run build`: built in 394ms, `check_static: ok` (rc=0) |
| Negative cases hold | ☑ pass | mutation `git diff --stat`: web/src/App.jsx 2 +-, web/src/Transcript.jsx 2 +- (todo reader returns null; ?app-sessions mapping removed) → `Tests 5 failed | 83 passed (88)` in LivePanel/AppSessionEntry tests; restored with `git checkout`, suite green 1470/1470 |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff main...T128` web/src (App, LiveView, NewSessionSheet, Transcript, appsession.css) + their callers (App.jsx View props) and tests; Rust and Python untouched, not reviewed |
| Full smoke suite still green (no regression) | ☑ pass | `npx vitest run` 43 files, 1470/1470 after the Stage 4 fixes (`51b2a1e`) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier refuses repo paths since 2026-11-01 (memory/learnings.md, easy-verifier registered); advisory only |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | ux-evaluate on the built bundle with a stubbed API (scratch, not committed): U1 pass (New session in the Live header at 1280 and 375, no scroll), U2 pass (form drawn in place, no dialog; the Which colour? card stays fully visible below it), U3 pass (TodoWrite: done struck through, in progress bold with half mark, pending open circle). Screenshots in the session scratchpad ux/: a-1280, a-375, b2-1280, c-1280, c-375 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | New CSS in `web/src/appsession.css` uses var(--color-*), var(--space-*), var(--fs-cell), var(--font-*) only; no raw colours |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280 px: no horizontal overflow seen; the New session button gets min-height 44px at 375 (CSS rule added, not measured with tap_probe) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any implementation commit; `main` the branch point):
```
$ date -u; git log -1 --oneline; grep -n 'New session' web/src/LiveView.jsx | wc -l; grep -n 'TodoWrite' web/src/Transcript.jsx | wc -l
2026-11-19T11:18:19Z
af4aa48 Board + memory: T122, T123 Done (merged); font decision
0
0
```

**AFTER** (worktree branch T128, implementer; the two counts are `grep -c` of 'New session' in LiveView.jsx and 'TodoWrite' in Transcript.jsx):
```
2026-11-19T22:42:08Z
4fe6226 Plan: Live like Claude 2 (question 46, FR-129..FR-133, DDR-0008, T124-T128)
3
3
```

**DELTA**: From Live the owner can open the New session form in place (also for Resume / Fork / Continue), start a session and see its panel open, and a TodoWrite call reads as a checklist; the App sessions tab is gone and its old address opens Live.

**WITNESS**: `memory/event-trace/T128.jsonl` (126 records): implementer `npx vitest run` 2026-11-19T22:32:56Z; Supervisor session code-review Skill 2026-11-19T22:42:34Z, `npx vitest run` 22:43:46Z, security-review Skill 22:45:03Z, re-run after fixes and mutation check after 22:45Z.
