# TASK_REVIEW — T131: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T131.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/BoardView.test.jsx` (describe "Run button (T131)": AC1-AC8, 18 cases incl. 4 server-error codes) |
| Verification command run | ☑ pass | `cd web && npm test -- --run`: Test Files 46 passed (46), Tests 1531 passed (1531); `npm run build`: built in 197ms. `npm run lint`: **no `lint` script exists in `web/package.json`** (npm: Missing script: "lint"), so it could not be run |
| Negative cases hold | ☑ pass | Red run before the implementation (commit b3945f6, BoardView unchanged): `Tests 17 failed \| 34 passed (51)`; after: `51 passed`. Covered: off, no repo, orphan-only repo, two repos, route 404 (no button), workspaces 404, double click, 403/409/400/500 and network error, same id in two projects, huge title |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `BoardView.jsx`, `BoardView.test.jsx`, `app.css` (board block). `App.jsx` needs no change: it already passes `onOpenAppSession` and `onOpenSettings` to every view. Not touched: `rust/**`, `NewSession.jsx` |
| Full smoke suite still green (no regression) | ☑ pass | 46 files, 1531 tests passed |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **AC 10: Run button highlighted (class `primary`)** | ☑ pass | Test `AC10: the Run button uses the existing accent style (class primary)` was red before (`1 failed \| 51 passed (52)`), green after; whole web suite `46 files, 1532 tests passed`; `npm run build` built in 223ms. No new CSS rule or colour (`main button.primary` already in `app.css`). 1280 screenshot of the real BoardView + app.css with mocked API (scratch probe, deleted, not committed): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/8fb95ba1-34f5-47f8-bde4-9f115f127e09/scratchpad/ux131/board-1280.png` shows accent border, soft accent fill, accent text on every enabled Run |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | ux-evaluate on a scratch probe page (real BoardView + app.css, mocked API; scratch files deleted, never committed): screenshots at 375 and 1280 in `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/8fb95ba1-34f5-47f8-bde4-9f115f127e09/scratchpad/ux/` (`ok-`, `off-`, `norepo-`, `fail-` + `375.png`/`1280.png`). Cards keep id, title, project meta; Run sits last in the card |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Run uses the existing `main button` style; new CSS (`.board-run`, `.board-reason`, `.board-off`) uses tokens only; the existing "board CSS has no raw colour" test still passes |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280 viewed: no horizontal overflow, Run 44 px tall at 375 (existing mobile `main button` rule), "Open Settings" link has a 44 px tap area at 375 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-20T05:50:38Z in `web/` of the T131 worktree, before any implementation commit (HEAD `c820103`):

```
$ grep -nic "run" src/BoardView.jsx
0
$ grep -n "Run\|startAppSession\|app-sessions" src/BoardView.jsx
(no match, exit 1)
$ npx vitest run src/BoardView.test.jsx
 Test Files  1 passed (1)
      Tests  33 passed (33)
```

BoardView has no Run button and never calls `/api/app-sessions`.

**AFTER**: captured 2026-11-20 in `web/` after commit 141246c:

```
$ grep -n "startAppSession\|export const NO_REPO\|export const AMBIGUOUS" src/BoardView.jsx
2:import { getAgentRun, getJson, startAppSession } from './api.js'
26:export const NO_REPO = 'Add this repo to a workspace to run it'
27:export const AMBIGUOUS = 'Two workspace repos share this name'
109:      const answer = await startAppSession({
$ npx vitest run src/BoardView.test.jsx
      Tests  51 passed (51)
$ npm test -- --run
 Test Files  46 passed (46)
      Tests  1531 passed (1531)
```

**BEFORE (AC 10)**: captured 2026-11-20T07:37:28Z in the T131 worktree, before the AC 10 implementation commit (the Run `<button>` has no `className`):

```
$ grep -n 'aria-label={`Run' web/src/BoardView.jsx
129:      <button type="button" aria-label={`Run ${task.id}`} aria-describedby={off ? 'board-run-off' : undefined} disabled={busy || off || Boolean(target.reason)} onClick={start}>
```

**AFTER (AC 10)**: captured 2026-11-20 in the T131 worktree, after the AC 10 change:

```
$ grep -n 'aria-label={`Run' web/src/BoardView.jsx
129:      <button type="button" className="primary" aria-label={`Run ${task.id}`} aria-describedby=...
$ npx vitest run           # whole web suite
 Test Files  46 passed (46)
      Tests  1532 passed (1532)
```

**DELTA**: From a Todo, In Progress, Ready for Review or Blocked card the owner can start a Claude app session on that task with one click, and lands in Live on it.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T131.jsonl`, never the
implementing agent alone]
