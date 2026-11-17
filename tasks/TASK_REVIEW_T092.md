# TASK_REVIEW — T092: Agent run 3: Settings view, first-open prompt, Run block, README

> Sibling of `tasks/TASK_GUIDE_T092.md`. Stage 3 evidence by the frontend agent (second pass). Stage 4 (code-review, security-review) and the auto verify are the Supervisor's and are not filled here.

---

## Auto decisions

Mode: auto
1. Scope of the change: `web/src/api.js` touched (errors carry `.status`, four agent-run helpers) though the guide's file list omits it; all 56 evidence screenshots kept under `tasks/evidence-T092/` — DEBATED: keep the api.js change and all screenshots (Advocate 1: keep api.js, additive and the only way to tell 404/403/409/400 apart; trim six 1280/BEFORE shots; Advocate 2: keep api.js, no other caller reads `.status`; keep all 56, they are already in commits d433619 and 17d51fe and the merge keeps branch history, so a delete saves no space). Advocate 1's trim is settled by that fact; the BEFORE shots are the required Demonstration ⚠ decided for you — please review
2. State and error handling in the Run block and Settings: a missing account folder gets a no-button line (Run) and a note with Change / Turn off (Settings); after a 403 or 409 the block reads GET /api/agent-run once more; the block renders nothing while loading; the elapsed time counts from this page's click (the words at least when a poll first finds a run), since the server exposes no start time until follow-up T100 — DEBATED: keep all four (Advocate 1: keep all four, the server reports `missing` and a run there cannot work, polling runs only while running, any loading text would flash a wrong state, the start time lives only in the run thread; Advocate 2: keep all four, with T100 as a must since a run started elsewhere shows a lower elapsed time) ⚠ decided for you — please review
3. Interaction: a Choose button opens the confirm step (not the radio); no first-open prompt and no mark when there is no confirmed Claude account, Esc = Not now, Esc in confirm returns to the picker; the picker keeps the last pick and focus returns to Choose after Cancel (changed at Stage 4, commit 8b05ef9); 44 px tap area for inline links at 375 px; the Run block stays above Saved reports although at 375 px its button is below the fold (ux-evaluate B-a, accepted) — DEBATED: keep 2, 3, 16 and the placement, change 14 (Advocate 1: keep, change 14 — arrow keys would open the dialog on each press, the prompt shows once per profile, the block sits with the copy hint after the warnings; Advocate 2: keep, change 14 — same, plus the done line points to Saved reports just below; a link near the headline could come later) ⚠ decided for you — please review
4. Setting on with no account id reads the default Claude account — FACT: rust/crates/app/src/agent_routes.rs:105 "Setting::Default => (true, Value::Null),"
5. Outcome lines follow the guide's fixed words, never the server body — RULE: tasks/TASK_GUIDE_T092.md:36 "A run ended: `ok` with a report"
6. A 404 on GET /api/agent-run: the Settings and Run blocks show nothing — RULE: tasks/TASK_GUIDE_T092.md:39 "**Out of scope**: other settings in the new view"
7. Settings and Agents links are buttons that switch tabs, because the token is the URL fragment — FACT: web/src/api.js:4 "const token = window.location.hash.replace(/^#/, '')"
8. The run keeps its own scope (workspace only) — RULE: tasks/TASK_GUIDE_T092.md:129 "- The page's scope changes during a run: the run keeps its own scope; the button stays off"
Checks: tests `cd web && npm test` — 29 files, 1006 passed (fix round 8b05ef9); build `npm run build` — exit 0; real run `easy-personal-rs serve --static-dir` (debug binary, temp home, fake `claude`) — states in `tasks/evidence-T092/` (auto-checked, not /verify); reviews code-review (independent reviewer, 2026-11-17) — 0 P0, 0 P1, 1 P2 (Stop 409 line, fixed 8b05ef9), 2 P3 (prompt over Settings, fixture id type; fixed 8b05ef9); security-review (scoped to the T092 web diff) — no finding; ux-evaluate 11 pass, B-a fail accepted (decision 3), B-e fail (older)
Memory: nothing new
Not done: auto verify (the Supervisor starts it), merge; follow-up T100 (server start time); `html-report` pages and the companion score not made

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/SettingsView.test.jsx` (40 tests: AC 1, 2, 6), `web/src/AgentRun.test.jsx` (36 tests: AC 3, 4, 5, 6), additions in `web/src/App.test.jsx` (nav tab, prompt mount, First run and Settings suppression) and `web/src/WarningsPlanView.test.jsx` (block above Saved reports, reload hook) |
| Verification command run | ☑ pass | `cd web && npm test` → `Test Files  29 passed (29)`, `Tests  1006 passed (1006)` (Stage 4 fix round; was 1001); `npm run build` → `index-DPeOsCNz.css 55.24 kB`, `index-tyGOC3WT.js 368.51 kB`, `✓ built in 434ms` (output goes to `src/easy_personal/dashboard/static/`, the folder `serve --static-dir` reads; AC 7). The project has no lint script. |
| Negative cases hold | ☑ pass | tests: a Codex or ignored account is not listed; Cancel sends nothing; 403, 409, 400 each show their fixed line; a `<script>` body is never rendered; a double click sends one POST; `localStorage` that throws does not break the page; a 404 draws nothing; polling stops on unmount; a Stop that answers 409 shows its own fixed line; Cancel in the prompt's confirm step keeps the choice and refocuses Choose |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ N/A | Stage 4 is the Supervisor's. Blast radius for the reviewer: `App.jsx` (nav, prompt mount), `WarningsPlanView.jsx` and `SavedReports.jsx` (reload key), `api.js` (every caller of `getJson`/`sendJson` now gets an error with `.status`; messages unchanged), `app.css` |
| Full smoke suite still green (no regression) | ☑ pass | the same 1001 tests include every existing file; `App.test.jsx` passed in the full run |
| **Companion score (easy-verifier)** | ☑ N/A | not run in Stage 3 |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | 56 screenshots in `tasks/evidence-T092/`, all cited ones opened and read. Verdict: Settings (off, confirm, chosen, missing folder), the first-open prompt and confirm step, and the Run block (off, idle, running, running-first-seen, failed, ok without report, ok with report, timed out, stopped, claude not found, missing account) match the Graphite Mint card, primary-button and dialog style of the FirstRun dialog; Saved reports is unchanged below the block (`after-run-ok-report-375.png`: the list reloaded, same rows). The full-page capture sizes the window to the page, so a modal in a tall page sits mid-page in the image (`after-prompt-375.png`); in a real viewport it is centred |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `git diff e5ba2dc -- web/src/app.css \| grep '^+' \| grep -ciE '#[0-9a-f]{3,8}\b'` → `0` (no raw hex in the new CSS); buttons are the existing `primary` and default button styles and `.link-button`; no new button style |
| **UI: Responsiveness at target viewports** | ☑ pass | 375, 1024, 1440 px for every state: `scrollWidth == innerWidth` in each capture (e.g. `after-run-idle-longws 375 {"scrollW":375,"innerW":375}`); buttons measured 44 px high at 375 and 32 px at 1024/1440; the workspace `very-long-workspace-name-without-a-break` wraps in the Run block's Scope line (`after-run-idle-longws-375.png`) and is cut with an ellipsis by the existing header chip |

### ux-evaluate (read from `.claude/skills/ux-evaluate/SKILL.md` of the main checkout; its own captures are in the scratch folder `…/scratchpad/t092/ux/`, not committed)

Rows are the guide's UX Acceptance Criteria (US-039); states from the real `serve` (evidence folder) and the fixture server (ux folder), 375 and 1280.

| # | User story | Flow / state | Width | Verdict | Screenshot | Reason |
|---|------------|--------------|-------|---------|------------|--------|
| 1 | US-039 | which account pays, before the choice and each run | 375 / 1280 | pass | `after-prompt-confirm-375.png`, `after-run-idle-longws-375.png`, `after-run-idle-1440.png` | the confirm step shows "Agent runs use this account's Claude subscription … They count against its plan limits." and the Run block shows "Runs with Claude personal. It counts against that account's plan limits." as text above the button |
| 4 | US-039 | finding the choice again | 375 / 1280 | pass | `after-settings-on-375.png`, `after-settings-on-1440.png` | label "Claude personal" and folder ".claude-personal", "The app uses this account for every agent run.", Change and Turn off, in one card |
| 5 | US-039 | the first-open prompt does not nag | 375 / 1280 | pass | `ux/prompt-375.png`, `ux/prompt-1280.png` | Choose and Not now sit side by side with the same size; tests show the mark is set and the prompt does not return. The prompt comes back only if `localStorage` is unavailable (by design) |
| 2 | US-039 | stop where the run started | 375 / 1280 | pass | `after-run-running-375.png`, `ux/running-1280.png` | Stop is next to a disabled Run investigate inside the same block, "Claude is investigating…" above them |
| 3 | US-039 | an off feature does not look broken | 375 / 1280 | pass | `after-run-off-375.png`, `ux/settings-off-1280.png` | "Agent runs are off. Choose an account in Settings." in normal text colour with a link, no error colour, no button; Settings reads "Off. The app starts no agent run." with the picker |
| B-a | baseline | default | 375 | fail | `after-run-idle-375.png`, `after-run-idle-longws-375.png` | on Warnings & plan the Run investigate button is below the fold at 375 (it sits under the warning cards and the copy line; the guide fixes the place: "above Saved reports", `tasks/TASK_GUIDE_T092.md:34`). At 1280 it is in view with no warnings (`after-run-idle-1440.png`). Settings and the prompt pass (Choose / Use this account is the one primary button, in view) |
| B-b | baseline | default | 375 / 1280 | pass | `after-settings-on-375.png`, `ux/settings-off-1280.png` | title, then card heading, then line, then buttons: one reading order |
| B-c | baseline | default | 375 / 1280 | pass | `after-run-running-375.png`, `after-settings-missing-375.png` | card padding and line gaps match the neighbouring cards; related lines grouped |
| B-d | baseline | default | 375 | pass | probe in `shot.mjs` (`getBoundingClientRect` of every button in the new blocks) | all buttons 44 px high at 375 (`Run investigate`, `Stop`, `Change`, `Turn off`, `Choose`, `Not now`, `Use this account`, `Cancel`); the inline Settings link was 18 px, fixed to 44 px (decision 16) and re-measured: `after-run-off 375 … {"t":"Settings","h":44}`. Radio rows are about 44 px tall (`ux/settings-off-375.png`) |
| B-e | baseline | default | 375 | fail (older) | `after-run-idle-375.png` | body text is 14 px, the project's existing `--fs-body` token, used by every view; not caused by this change; contrast is clearly readable (light text on near-black; the muted lines are the project's muted token) |
| B-f | baseline | every named state | 375 / 1280 | pass | `after-run-failed-375.png`, `after-run-notfound-375.png`, `after-settings-missing-375.png` | each state says in words what happened and what to do next; failed shows the Run button again; not-found and off show no button by the guide. Not checked: the loading state (draws nothing by decision 11) and the 404 line (see below) |
| B-g | baseline | after an action | 375 / 1280 | pass | `after-run-running-375.png`, `after-run-stopped-1440.png` | after Run the block shows "Claude is investigating…" and "Running for 0:00"; after Stop "Stopped." |
| B-h | baseline | default | 375 | pass | `after-run-idle-longws-375.png` | no horizontal scroll in any capture (`scrollW` equals `innerW`); the Settings card and the Run block are one column; buttons are not clipped |
| B-i | baseline | default | 375 / 1280 | pass | `after-settings-confirm-1024.png`, `after-run-idle-1440.png` | one mint accent for the primary action, one button style, one radius |

ux-evaluate: 11 pass, 2 fail (B-e older; B-a new, by the guide's placement), 0 not checked, 0 not checkable. Overall: fail. Reported to the Supervisor: B-a needs a decision (move the block above the warning cards, or accept the guide's placement). Open and reported to the Supervisor: the 404 line (Settings view when the server has no route) and the loading state have no screenshot of their own (covered by Vitest), the dialogs were judged at 1024/1440 and 375 but a modal in the full-page captures sits mid-page (capture artefact); the elapsed-time wording differs from the guide (decision 15).

### Stage 4 fix round

Red first, then green (`cd web && npx vitest run`): the four new tests failed (1002 passed, 4 failed of 1006) before the fixes, and all 1006 passed after. Build after the fixes: `index-tyGOC3WT.js 368.51 kB`, `✓ built in 434ms`.

| # | Finding | Test (name) | Red → green | Fix |
|---|---|---|---|---|
| 1 | P2: a Stop answering 409 (run of another process) read "Could not stop the run" | `AgentRun.test.jsx` › "a Stop that answers 409 says the run belongs to another process, with a fixed line and no server text" | red (line not found, timed out) → green | `web/src/AgentRun.jsx`: the Stop failure map has `409 → "This run was started outside the app. Stop it there."`; other statuses keep "Could not stop the run. Try again."; never the body |
| 2 | P3: the first-open prompt could open over the Settings tab | `App.test.jsx` › "does not offer it while the Settings tab is open, so that view never goes stale under it"; one more row in the `it.each` of `SettingsView.test.jsx` ("the Settings tab is open", at the component level it is the same `suppressed: true` as First run, so it was green at once; the App test is the red one) | red → green | `web/src/App.jsx`: `suppressed={active === 'first-run' \|\| active === 'settings'}`; no mark is set, so it may come back on another tab |
| 3 | P3: fixture sent the account id as a number | all `agentRunOn` users, `AgentRun.test.jsx` (two inline accounts) | green before and after (the page compares ids as strings; no red expected) | `web/src/test/fixtures.js` `agentRunOn`: `id: '2'`; the two inline accounts in `AgentRun.test.jsx` too |
| 4 | Decision 14 (debate result): Cancel from the confirm step returns to the picker with the last choice selected and the focus on Choose | `SettingsView.test.jsx` › "Cancel in the confirm step keeps the last chosen account selected and puts the focus on Choose" and "Escape in the confirm step does the same: the picker with the choice kept, the focus on Choose" | both red → green | `web/src/SettingsView.jsx` `FirstOpenPrompt`: `last` state passed as `Picker`'s `initial`; `chooseRef` on the prompt's Choose button; a `backs` counter effect focuses it after the dialog's own focus move |

Decision 14's "cut list" wording (the picker forgets the choice) is superseded by fix 4; the Supervisor rewrites the Auto decisions block. Settings view's own confirm step already returned the focus to Choose and is unchanged.

Memory: nothing new

---

## Demonstration

**BEFORE** (captured 2026-11-17 10:40:39 and 10:40:41, before any T092 implementation commit; branch tip then `e5ba2dc`, T091 merged): `tasks/evidence-T092/before-warnings-1280.png` and `before-warnings-375.png` — Warnings & plan ends with the copy line "Ask Claude to investigate: run /mcp__easy-personal__investigate in Claude Code." and Copy, then Saved reports. There is no Settings tab under `// system` (First run was the only item), no Run block and no first-open prompt; the agent run could be started only by `easy-personal-rs agent-run` from a terminal.

**AFTER** (2026-11-17, branch `T092`, the debug `easy-personal-rs serve --static-dir` of the branch on a temp home with a fake `claude`): `tasks/evidence-T092/after-prompt-*.png`, `after-settings-*.png`, `after-run-*.png` — the first open offers the account picker and the confirm step with "Not now"; Settings holds the *Agent run account* block (off, confirm step, chosen, missing folder); Warnings & plan has the *Agent run* block above Saved reports with off, idle, running (Stop), and ended lines (failed, ok, ok with report, timed out, stopped, claude not found). `GET /api/agent-run` after the setting `{"account":"3","confirmed":true}`: `{"account":{"id":"3","label":"Claude personal","status":"confirmed"},"claude_found":true,"enabled":true,"last":null,"running":false}`; after a run started by a click on Run in the page and ended with Stop through the API the same call printed `"last":{"ended":"2026-11-17T04:12:31Z","job":"investigate","outcome":"stopped","report_id":null,"started":"2026-11-17T04:12:25Z"},"running":false`.

**DELTA**: the owner can choose the agent run account, see it in Settings, and start, watch and stop an investigate run from the Warnings & plan page, with no terminal.

**WITNESS**: frontend agent, second pass, 2026-11-17, against the real Rust server; not yet witnessed by the Supervisor (auto verify is the Supervisor's).

---

## Observations for the Supervisor (not decisions)

- The real server answers `account.id` as a string (`"id":"3"`) in `GET /api/agent-run` and its `PUT` answer, while `GET /api/accounts` has a number and `web/src/test/fixtures.js` uses a number. The page compares ids as strings, so it works with both; the Rust shape could be aligned in a T091-area task.
- `PUT /api/agent-run/setting` refuses a number: `{"error":"account must be an account id or null"}`; the page sends `String(id)` (`web/src/api.js`, `setAgentRunAccount`).
