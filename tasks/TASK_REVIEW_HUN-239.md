# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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

## Auto decisions

Mode: auto
1. Put the block above the project list (the guide let the implementer choose and note why) — FACT: tasks/TASK_GUIDE_HUN-239.md:27 "Under the project list (or above it, the implementer chooses what reads best at 1280×720 and notes why)"
2. Used the app-wide period (default 7 days) for the block, not a new control on the Kit view — FACT: web/src/App.jsx:114 "const [period, setPeriod] = useState('7') // Usage period id, app-wide (FR-064)"
3. Did not send the workspace filter in the request; it only marks the selected row — FACT: tasks/TASK_GUIDE_HUN-239.md:27 "the block still lists all workspaces and marks the selected one"
4. Edited `web/src/App.test.jsx` (one regex) and `web/src/test/fixtures.js` (one fixture, one route line), which the guide did not list — SAFER OPTION: the smaller change that keeps the existing and new tests green; test files only, no source behaviour ⚠ decided for you — please review
5. Placed the block on the Kit view, not the Workspaces page — DEBATED: Kit view (Advocate 1: Kit view, `web/src/KitView.jsx:339`; Advocate 2: Workspaces page, conceded low confidence) ⚠ decided for you — please review (see the guide's Questions Log)
6. Did not run `/verify` (only the user can) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule`
7. Did not merge, push or move the task to Done — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule`
Checks: tests `cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest -q` — 724 passed, build ok, 1660 passed (agent run 2026-11-07); reviews code-review — 0 P0, 0 P1, 0 P2, 2 P3; security-review — no finding at confidence 8 or above; easy-verifier changes score — overall 75, change_risk 100 (advisory); real run — dashboard from the branch on a COPY of the owner store, headless Chrome at 1280×720 and 375×812, printed `block:true, rows:3`, scrollWidth = clientWidth in both (auto-checked, not /verify)
Not done: merge (yours); /verify (yours)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/WorkspaceKit.test.jsx` (21 tests: AC1, 2, 3, 4 inline states, 6, 7, edge cases incl. planted-markup name, ellipsis title, mean 0 vs no data, request path) and `web/src/KitView.test.jsx` describe `by workspace block (HUN-239)` (8 tests: AC1 placement, AC4 pending + failed read, AC5 since/account, period refetch, late answer dropped, AC6, AC7). Result: `Test Files 22 passed (22) / Tests 724 passed (724)`. |
| Verification command run | ☑ pass | `cd web && npx vitest run` → `Test Files  22 passed (22)  Tests  724 passed (724)`; `npm --prefix web run build` → `✓ built in 109ms`; `uv run pytest -q` → `1660 passed, 6 deselected, 1 xfailed, 1 warning in 149.65s (0:02:29)` (AC8) |
| Negative cases hold | ☑ pass | no-data workspace: no bar, no number, text `no data`; `not used` text; failed read: `role=alert` inside the block, project cards still render; late answer for an older period dropped; markup in a workspace name rendered as text (no `<img>`/`<b>` element, no handler run); non-#rrggbb colour ignored |
| verify | ☑ N/A | Stage 5 `/verify` is the Supervisor's; the author's real run is under Demonstration (headless Chrome on a COPY of the owner store) — observed rows, numbers and no horizontal scroll: pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | affected set = KitView.jsx (one consumer: App.jsx) + new WorkspaceKit.jsx + app.css additions + tests; two existing tests that count `/api/kit*` calls (`App.test.jsx` line 245, `KitView.test.jsx` scope tests) were adjusted; Stage 4 code-review (Supervisor): lane check `lane: C2, 7 files, 457 lines, FITS; unpredicted: web/src/App.test.jsx, web/src/test/fixtures.js`. Findings: P3 `App.test.jsx` and `fixtures.js` are outside the guide's file list (test-only, needed; decision 4); P3 component and tests were written close together, not strictly red first (the author said so). Read App.jsx: it already passes `period` and `onOpenWorkspaces` to every view, so the Kit view needs no App change. Checked and suppressed: block is shown only after the page read is ready, as the guide's one-pill rule needs. Startup reads line present in the agent report. Process note: the step-limit hook stopped the agent once at 90 calls; it ran about 8 more calls to finish. Security-review: no finding at confidence 8 or above (API text is rendered as React text; colour must match `#rrggbb`; query values go through `encodeURIComponent`; no `dangerouslySetInnerHTML`). |
| Full smoke suite still green (no regression) | ☑ pass | vitest 724/724 and pytest 1660 passed (see above) |
| **Companion score (easy-verifier)** | ☑ pass | overall 75 (advisory, no gate), change_risk 100, scope changes, ref `bc9581c...HEAD`; security 30 comes from 127 redaction hits in lockfiles and fixtures (integrity hashes), not a planted secret. Report: /home/hungnguyenhuu/.easy-verifier-reports/agent-abdd3dc1f4a26a565/evidence-report-changes-20261107T005909-138698Z.html |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | BEFORE `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/c3664c23-c7cd-41b3-891f-7ee9777a70f9/scratchpad/before-1280x720.png`, `before-375x812.png`; AFTER `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/c3664c23-c7cd-41b3-891f-7ee9777a70f9/scratchpad/after-1280x720.png`, `after-375x812.png` (and `-full.png`). Verdict: the new "Kit by workspace (3)" block sits above the project list; the project list ("Kit by project (20)", 11 cards shown, 124 gone folders note) is unchanged below it. Placement above, because the project list is 11 tall cards at 1280×720 and a block below would sit thousands of px down; the block is 3 compact rows. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | new CSS in `web/src/app.css` (`.ws-*`) uses only `var(--…)` tokens, no colour literal; reuses `.progress`, `.chip`, `.muted`, `.link-button`, `.count`, `Loading`. The one non-token colour is the owner's own workspace colour on the 8 px dot, set inline only if it is `#rrggbb` (tested). Accessibility: labelled list (`aria-label`), `<h2>` with count, `aria-current` on the selected row, bar has `role=progressbar` + `aria-label` ("Verifier mean 70 of 100") and the number as text, error `role=alert`. vitest `shell.test.js` (token/colour scan) still passes. |
| **UI: Responsiveness at target viewports** | ☑ pass | CDP on the COPY store: 1280×720 `{"sw":1265,"cw":1265,"block":true,"rows":3,"nameClipped":[false,true,false],"cards":11}`; 375×812 `{"sw":375,"cw":375,"block":true,"rows":3,"nameClipped":[false,false,false],"cards":11}` → scrollWidth = clientWidth (no horizontal scroll); the 40-char name is ellipsised at 1280 (`title` holds it) and fits at 375 where the Verifier and easy-ui cells stack. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-07T00:48:50Z by frontend-developer, before any HUN-239 implementation commit; branch head `bc9581c`, built page = unmodified web/):
Dashboard from the branch on a COPY of the owner store (temp `EASY_PERSONAL_HOME`; fixture verifier reports in a temp `EASY_PERSONAL_VERIFIER_REPORTS`, fixture easy-ui session files in a temp `EASY_PERSONAL_UI_REPORTS`; copy gets one extra workspace with a 40-char name). Headless Chrome (CDP), Kit view opened from the sidebar. Script: `rundash.sh before 9301` (scratchpad).
```
$ rundash.sh before 9301
dashboard url found: http://127.0.0.1:42377/#<token>
1280x720 {"sw":1265,"cw":1265,"block":false}
375x812 {"sw":375,"cw":375,"block":false}
--- GET /api/kit/workspaces (HUN-238 route exists; the page does not call it)
{"workspaces":[{"name":"bep-work",...,"verifier":{"status":"no data",...},"ui":{"status":"ok","sessions":40,"calls":4494,"last_utc":"2026-11-06T11:15:17.333000Z"}},
 {"name":"client-with-a-very-long-workspace-name-x",...,"verifier":{"status":"ok","repos_scored":1,"repos_total":1,"mean":61.0,"lowest":{"project":"easy-verifier-mcp","value":61.0},...},"ui":{"status":"not used",...}},
 {"name":"personal",...,"verifier":{"status":"ok","repos_scored":2,"repos_total":5,"mean":71.5,"lowest":{"project":"easy-verifier-mcp","value":61.0},"newest_utc":"2026-11-06T09:15:00.000000Z"},"ui":{"status":"not used",...}}],
 "ui_checks":{"status":"ok","sessions":3,"passed":2,"failed":1,...}}
```
Screenshots: `.../scratchpad/before-1280x720.png`, `before-375x812.png` (and `-full.png`): the Kit view shows "Kit by project (20)" cards and the "easy-ui checks (overall)" section; no by-workspace block (`block:false`).

**AFTER** (2026-11-07T00:53:24Z, same script and fixtures, built page from this branch): `rundash.sh after 9303`
```
1280x720 {"sw":1265,"cw":1265,"block":true,"rows":3,"nameClipped":[false,true,false],"cards":11}
375x812  {"sw":375,"cw":375,"block":true,"rows":3,"nameClipped":[false,false,false],"cards":11}
```
Block text on screen (1280×720): `bep-work` — Verifier `no data` · easy-ui `15 sessions · 1,459 calls · last 2026-11-06` (7-day period); `personal` — `2 of 5 repos scored`, `mean 71.5` + bar, `lowest easy-verifier-mcp 61`, `newest report 2026-11-06` · easy-ui `not used`; the long-name workspace — `1 of 1 repo scored`, `mean 61`; under the rows: `easy-ui checks (all workspaces): 3 sessions, 2 passed, 1 failed — not split by workspace`. Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/c3664c23-c7cd-41b3-891f-7ee9777a70f9/scratchpad/after-1280x720.png`, `after-375x812.png`.

**DELTA**: The owner can open the Kit view and see, for every workspace side by side, how its repos score in easy-verifier (scored count, mean, lowest repo, newest report) and how much easy-ui was used there, without opening each repo.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-239.jsonl`, never the
implementing agent alone]
