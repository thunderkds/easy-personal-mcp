# TASK_GUIDE — T092: Agent run 3: Settings view with the agent run account, the first-open prompt, the Run block on Warnings & plan, README text
**Date**: 2026-11-16
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/frontend.md`
5. Read `BRAINSTORMING_LOG_AGENTRUN.md` and the PRD rows US-039, FR-104, FR-103, FR-078, NFR-002, question 37
6. Read `web/src/WarningsPlanView.jsx` and `web/src/SavedReports.jsx` (where the block goes, how the list loads), `web/src/CopyCommand.jsx` (a small block with a polite live region), `web/src/api.js` (`getJson`, how a POST is sent with the token), `web/src/test/fixtures.js`, the tokens at the top of `web/src/app.css`, `docs/design/graphite-mint/`
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder
- Never start a real run from a test or a screenshot session: use fixtures for every state

---

## Requirement

**Original request** (owner, 2026-11-16): "all the actions should be in the application, not from the command, or the manual, like when I run the app, I will have the option to choose the account that is used for the application, when I choose, it will show on the settings, from this time, the application will use this account for the automate all progress in the application".

**Restated intent** (PRD FR-104, rewritten 2026-11-16):
> 1. A new **Settings** view, a tab under `// system` in `web/src/App.jsx`. It holds one block, *Agent run account*. It reads `GET /api/agent-run` and `GET /api/accounts?by=account`.
> 2. No account chosen: "Off. The app starts no agent run." and a picker (radio list) of the confirmed Claude accounts: label, folder name, plan limit line when the accounts answer has one. No confirmed Claude account: a line that says so and links to Agents.
> 3. Choosing an account opens a confirm step with the fixed note "Agent runs use this account's Claude subscription through Claude Code on this machine. They count against its plan limits." and the buttons **Use this account** and **Cancel**. Use sends `PUT /api/agent-run/setting` with `{account, confirmed: true}`.
> 4. An account chosen: its label and folder name, "The app uses this account for every agent run.", **Change** (the picker again) and **Turn off** (`PUT` with `{account: null}`).
> 5. First-open prompt: when the app loads, the setting is off, and the browser holds no "asked" mark, a dialog offers the same picker and confirm step, with **Not now**. Either answer sets the mark (`localStorage`, read and written in try/catch; no mark available → the prompt may show again, the page still works). It shows once per browser profile, never over the First run view.
> 6. On Warnings & plan, above *Saved reports*: setting off → "Agent runs are off. Choose an account in Settings." with a link to Settings, no button. `claude_found: false` → "Claude Code was not found on this machine.", no button. Setting on → **Run investigate** for the page's workspace scope, with the line "Runs with <account label>. It counts against that account's plan limits." A click sends `POST /api/agent-run` with `{job: "investigate"}` plus the workspace.
> 7. A run going (`running: true`, from this page or elsewhere): "Claude is investigating…" in a polite live region, the elapsed time from `last.started`, **Stop** (`POST /api/agent-run/stop`); Run is off. The block asks the GET again every 5 s only while a run is going.
> 8. A run ended: `ok` with a report: "Done. The report is first in Saved reports." and the list reloads; `ok` without a report: "Done, but no report was saved."; `failed`: "The run failed. Check that this account is logged in to Claude Code and its plan limit is not reached."; `timed out`: "The run was stopped after 10 minutes."; `stopped`: "Stopped." A 403, 409 or 400 answer shows a fixed line for that status, never the body text as HTML.
> 9. `rust/README.md` and the root `README.md` get a short "Agent runs (optional, off by default)" part: what it does, that it is set in Settings, that it uses an account already logged in to Claude Code and its plan limits, that the app never reads the login, and that it is Linux and Rust app only. The landing page (`site/`) is not changed (PRD question 37 is open).

**Out of scope**: other settings in the new view; a job picker; more than one agent run account; a login flow; progress text or output of the run; a run history; automatic runs (PRD question 38); the Python dashboard server (the Settings block and the Run block show nothing when `GET /api/agent-run` answers 404); other views; `site/**`.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16 (`docs/claude-md/tracks.md` row 13); the owner has not read it
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: US-039, FR-101, FR-104, FR-103, FR-078, questions 36-38 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T091 merged
**Entry point**: `web/src/SettingsView.jsx` (new, tab in `web/src/App.jsx`), `web/src/AgentRun.jsx` (new, rendered by `web/src/WarningsPlanView.jsx`), the first-open prompt mounted in `App.jsx`

## Dependency rule
No new npm package.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Vitest, Settings: off with a picker of the confirmed Claude accounts only (a Codex or ignored account is not listed); no Claude account → the line with the Agents link; choosing opens the confirm step with the exact note; **Use this account** sends one PUT with `{account, confirmed: true}`; **Cancel** sends nothing; chosen state shows label, **Change**, **Turn off**; Turn off sends `{account: null}` | intent 1-4 |
| 2 | Vitest, first-open prompt: shown when off and no mark; **Not now** and a choice both set the mark and it does not show again; not shown when the setting is on, on the First run view, or when the GET is 404; `localStorage` that throws does not break the page | intent 5 |
| 3 | Vitest, Run block, one test per state from fixtures: off (text + Settings link, no button); `claude` not found; on and idle (button + the account line); running (status in a live region, elapsed time, Stop, Run off); each of the five ended outcomes with its exact line | intent 6-8 |
| 4 | Vitest: a click on Run sends one POST with `job: "investigate"` and the page's workspace and no `account` key; Stop sends one POST; a 403, 409 and 400 answer each show their fixed line; a body with `<script>` in it is never rendered; a double click sends one request | intent 6-8 |
| 5 | Vitest with fake timers: the GET repeats every 5 s only while `running`; it stops after the end and on unmount; after `ok` with a report id the Saved reports list is asked again once | intent 7, 8 |
| 6 | Keyboard and screen reader: the picker is a labelled radio group; the confirm step and the prompt are dialogs with a name, focus moves in and returns, Escape cancels; Run and Stop are real buttons; status changes are announced once (polite) | intent 3, 5, 7 |
| 7 | Existing tests and `npm --prefix web run build` pass; the nav shows Settings under `// system`; the built files are copied where the Rust app serves them, as the project's build step does | intent 1 |
| 8 | UI evidence rows: screenshots of Settings (off, confirm step, chosen), the first-open prompt, and the Run block (off, idle, running, ended) at 375 / 1024 / 1440 px from the real `easy-personal-rs serve` with fixture answers; tokens only (grep raw hex in the new CSS) | intent |
| 9 | README part present in both files with the statements of intent 9 | intent 9 |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd web && npm test 2>&1 | tail -5 && npm run build 2>&1 | tail -3
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Verification command | ☐ | |
| code-review | ☐ | |
| security-review | ☐ | |
| UI: visual regression | ☐ | |
| UI: design-system compliance | ☐ | |
| UI: responsiveness | ☐ | |

## Demonstration
> BEFORE/AFTER/DELTA/WITNESS go in `tasks/TASK_REVIEW_T092.md`.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| Settings view (3 states), first-open prompt, Run block (4 states) | screenshot, LLM vision vs `docs/design/graphite-mint/` cards, buttons and the FirstRun dialog style | matches the design; the Saved reports section is unchanged |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Colours, spacing, button style | grep for raw hex in the new CSS; compare with the existing primary and quiet buttons | tokens only; no new button style |

### 3. Layout / Responsiveness
| Viewport | Verification method | Expected result |
|---|---|---|
| 375 / 1024 / 1440 px | screenshot | no horizontal page scroll; the note wraps; buttons at least 44 px high at 375 px |

### UX Acceptance Criteria
| # | Row | Expected |
|---|---|---|
| 1 | The owner can tell before the choice, and before each run, which account pays | the note is in the confirm step, and the account label is next to the Run button, not in a tooltip |
| 4 | The owner finds the choice again | Settings shows the chosen account in one glance, with Change and Turn off |
| 5 | The first-open prompt does not nag | Not now is as easy to reach as the choice, and the prompt never returns |
| 2 | The owner can stop a run from the place where they started it | Stop is in the same block, visible while running |
| 3 | An off feature does not look broken | the off state reads as a choice with its command, no error colour |

---

## Approach
Test first, one state at a time (`tdd`). Smallest code that meets the ACs; record cuts. Run `ux-evaluate` before Ready for Review.

## Edge Case Checklist
- The chosen account is deleted in Agents while Settings is open: the next load shows Off
- One Claude account only: the picker still shows it and still needs the confirm step
- A run started in another window or by `agent-run now`: the block shows running on the next load
- The page's scope changes during a run: the run keeps its own scope; the button stays off
- The GET fails (500, network): one quiet error line, no button, no loop of requests
- A double click on Run: one POST
- A very long workspace name in the scope: wraps, no overflow at 375 px
- Reduced motion: no spinner animation needed to read the state

## Files to Change (Predicted)
`web/src/SettingsView.jsx` and test (new), `web/src/AgentRun.jsx` and test (new), `App.jsx` (one tab, the prompt mount), `App.test.jsx`, `WarningsPlanView.jsx`, `app.css`, `test/fixtures.js`, the built dashboard files, `rust/README.md`, `README.md`

## Files Must NOT Touch
`web/src/CopyCommand.jsx`, `web/src/SavedReports.jsx` beyond a reload hook, `web/src/AgentsView.jsx`, the nav beyond the one new tab, `rust/crates/**`, `src/easy_personal/**`, `site/**`

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output and the screenshots.

## Questions Log

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T092.md`
- [ ] Commits on branch `T092` only; final report with `Status:`
