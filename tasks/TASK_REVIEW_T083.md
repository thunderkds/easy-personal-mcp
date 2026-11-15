# TASK_REVIEW — T083: Watch follow-up: Watch view refreshes every 60 s; graphs share one 'now'

> Sibling of `tasks/TASK_GUIDE_T083.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/WatchView.test.jsx`: AC1 'reads each of the three routes again after 60 s, and stops when the view closes' + 'skips a tick while ... in flight'; AC2 'keeps the last good data and shows the error line ...'; AC3 'graphs share one "now"' (2 tests). Watch file: 34 passed |
| Verification command run | ☑ pass | `npx vitest run`: `Tests  920 passed (920)`; `npm run build`: `✓ built in 923ms` |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | 1440 px shot of this build against the mock server vs T077's: layout, tiles, graphs, table identical by vision compare (series end at the same right edge because the mock data ends at one time). Shots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp-rust/cc5835e1-dc0e-4f72-a3be-4f6c5c161ce5/scratchpad/t083shots/watch-1440.png` vs `.../t077shots/watch-1440.png`. pass |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `grep -nE '#[0-9a-fA-F]{3,8}\\b|rgba?\\(|hsla?\\(' web/src/WatchView.jsx` found nothing (exit 1); watch.css and app.css unchanged; the existing token test passes. pass |
| **UI: Responsiveness at target viewports** | ☑ pass | CDP measure: 1440 px scrollW 1440; 1024 px scrollW 1024; 375 px scrollW 375 (innerW equal: no horizontal page scroll). Shots `.../t083shots/watch-{1440,1024,375}.png`. pass |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-15T09:52:43Z on branch T083 at b4c8469 (no implementation commit yet; the new T083 tests exist only in the working tree). Command: `cd web && npx vitest run src/WatchView.test.jsx` (new tests added for AC1-AC3; `grep setInterval src/WatchView.jsx` finds nothing, and `export function linePath` is still at line 22):
```
 FAIL  ... > refresh every 60 s (T083 AC1, AC2) > reads each of the three routes again after 60 s, and stops when the view closes
AssertionError: expected "vi.fn()" to be called 6 times, but got 3 times
 FAIL  ... > refresh every 60 s > skips a tick while the earlier read of that route is still in flight
AssertionError: expected "vi.fn()" to be called 6 times, but got 3 times
 FAIL  ... > refresh every 60 s > keeps the last good data and shows the error line when a refresh fails; the next good read clears it
 FAIL  ... > refresh every 60 s > shows newer rows after a refresh
 FAIL  ... > graphs share one "now" (T083 AC3) > a limits series that stopped 1 h earlier ends before the right edge
AssertionError: expected 200 to be close to 191.66666666666666, received difference is 8.333333333333343, but expected 0.05
 FAIL  ... > graphs share one "now" (T083 AC3) > a machine series ends at the newest sample of the limits, not at its own last sample
AssertionError: expected 200 to be close to 191.66666666666666, received difference is 8.333333333333343, but expected 0.05
 Test Files  1 failed (1)
      Tests  6 failed | 28 passed (34)
```

**AFTER**: 2026-11-15T09:55:56Z, commit 7b3a9dc, same command `cd web && npx vitest run src/WatchView.test.jsx`: `Test Files  1 passed (1)`, `Tests  34 passed (34)` (28 old + 6 new). `grep -n "export function linePath\|setInterval" web/src/WatchView.jsx` prints only `78:    const timer = setInterval(read, REFRESH_MS)`. Full suite `cd web && npx vitest run`: `Tests  920 passed (920)`; `npm run build`: `built in 923ms`.

**DELTA**: A Watch page left open now updates itself every minute (a failed read keeps the old graph and adds the error line), and a graph whose data stopped ends before "now" instead of being stretched to it.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
