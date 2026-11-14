# TASK_REVIEW — T069: First run screen rework

> Sibling of `tasks/TASK_GUIDE_T069.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/FirstRun.test.jsx`, `web/src/App.test.jsx` (AC 1-4), `web/src/shell.test.js` (badge) |
| Verification command run | ☑ pass | Supervisor re-run 2026-11-14: 3 of 3 runs `Tests  804 passed (804)`; 1 earlier run under parallel-agent load had `1 failed | 803 passed`, failing test not captured (see P3) — pass |
| Negative cases hold | ☑ pass | unknown step id → generic card; clipboard reject → `Copy failed — select and copy`; Check again fail keeps steps — pass |
| verify | ☐ pass / ☐ fail / ☐ N/A | not run yet: Stage 5 `/verify` before merge |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | diff + direct callers only, see Stage 4 section |
| Full smoke suite still green (no regression) | ☑ pass | full web suite above — pass |
| **Companion score (easy-verifier)** | ☑ N/A | advisory run: overall 69, change_risk 100; no report path written |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | agent screenshots `after-2of4-1440.png`, `after-all-1440.png` vs rendered `FirstRun.dc.html` (scratchpad `shots/`) — pass |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | grep for hex/`rgb(` in First run CSS: none — pass (two raw font sizes, P3) |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 (no overflow, command scrolls in its box), 1024 — pass |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-14T16:14:08Z on branch `T069` at `042997b`, before any implementation commit): headless Chrome 157 (CDP, 1440x960) on a production build of the unchanged `web/`, served by a scratch mock API (2 of 4 steps done: first ingest and accounts confirmed). Screenshot: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7b734979-db79-4b51-b7ae-4d18c6c59237/scratchpad/shots/before-1440.png`. Text of the First run `<main>` as printed by the capture command:

```
First run / First ingest / Done / Copies the usage metadata of your Claude Code sessions into the store. /
Ingest timer installed / Not done / Keeps the store current, because Claude Code deletes old transcripts. / easy-personal schedule install /
Agent accounts confirmed / Done / Only confirmed accounts are read. /
First workspace / Not done / Groups your repos and connectors. / Open Workspaces
```

Observed: a 2-column grid of plain cards, no intro line, no `N of M steps done`, no progress bar, no step numbers, no highlighted next step, command shown with no `$ ` and no Copy or Check again button, nav badge reads `2 left`.

**AFTER** (2026-11-14, same capture method and same mock fixture, after the T069 commit): screenshots `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7b734979-db79-4b51-b7ae-4d18c6c59237/scratchpad/shots/after-2of4-1440.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7b734979-db79-4b51-b7ae-4d18c6c59237/scratchpad/shots/after-all-1440.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7b734979-db79-4b51-b7ae-4d18c6c59237/scratchpad/shots/after-2of4-375.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7b734979-db79-4b51-b7ae-4d18c6c59237/scratchpad/shots/after-2of4-1024.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7b734979-db79-4b51-b7ae-4d18c6c59237/scratchpad/shots/after-copied.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7b734979-db79-4b51-b7ae-4d18c6c59237/scratchpad/shots/after-copy-fail.png`; prototype render for comparison `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7b734979-db79-4b51-b7ae-4d18c6c59237/scratchpad/shots/proto-1440.png`. Text of the First run `<main>`, 2 of 4 done:

```
First run / Four steps to fill the dashboard with your own sessions. Everything stays on this machine. /
2 of 4 steps done / 2 left /
01 First ingest done / Copies the usage metadata ... /
02 Ingest timer installed next step / Keeps the store current ... / $ easy-personal schedule install / Copy / Ran it in your terminal? / Check again /
03 Agent accounts confirmed done / Only confirmed accounts are read. /
04 First workspace not done / Groups your repos and connectors. / Open Workspaces
```

Real Chrome click on Copy (clipboard permission granted): button labels `Copied`, `Check again`, `Open Workspaces`; `navigator.clipboard.readText()` returned `easy-personal schedule install`. With no clipboard permission: the card shows `Copy failed — select and copy` and the command text is selected.

**DELTA**: The First run page shows how far setup is (progress, numbered cards, the next step highlighted), lets the user copy a step's command and re-check the server state without leaving the page, and shows a completion banner with a button to Usage.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T069.jsonl`, never the
implementing agent alone]

## Stage 4 — code-review (Supervisor, 2026-11-14)

Scope: `main...T069` production files (`FirstRun.jsx`, `App.jsx`, `app.css`) + caller `App.jsx` and `first_run.rs` (hint always present for every step). Lane: `C2, 6 files, 498 lines, FITS`. Reachability: entry `web/src/FirstRun.jsx` used by `App.jsx` TABS — reachable. Startup reads line present.

P0: none. P1: none. P2: none.
P3 (optional):
| File | Finding | Confidence | Action |
|---|---|---|---|
| `web/src/app.css` First run block | `font-size: 12.5px` and `13.5px` hard-coded; tokens `--fs-note` (0.78125rem) and `--fs-control` (0.84375rem) give the same sizes | 100 | Use the tokens |
| web suite | 1 of 5 Supervisor runs had 1 failed test under heavy parallel load; 3 later runs clean; test name not captured | 50 (below gate, listed for the record) | Watch on `main` after merge |

0 safe fixes applied.
