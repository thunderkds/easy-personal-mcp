# TASK_GUIDE — T092: Agent run 3: the agent run block on Warnings & plan, and the README text
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

**Original request** (owner, 2026-11-16): "not need for the users type the message in claude and call through MCP, application will do that all for user"; "opt-in and off by default".

**Restated intent** (PRD FR-104):
> 1. A new component, the agent run block, on the Warnings & plan page above the *Saved reports* section. It reads `GET /api/agent-run`.
> 2. Setting off (`enabled: false`): one line, "Agent runs are off. To let this app start Claude Code for you, run `easy-personal-rs agent-run on` in a terminal." No button.
> 3. `claude_found: false` (setting on): "Claude Code was not found on this machine." No button.
> 4. Setting on, no run going: a **Run investigate** button for the page's current scope (workspace, account), with the fixed note "Uses your Claude subscription through Claude Code on this machine. It counts against your plan limits." A click sends `POST /api/agent-run` with `{job: "investigate"}` plus the scope.
> 5. A run going (`running: true`, from this page or elsewhere): the status "Claude is investigating…" in a polite live region, the elapsed time from `last.started`, a **Stop** button (`POST /api/agent-run/stop`); the Run button is off. The block asks the GET again every 5 s only while a run is going.
> 6. A run ended: the outcome in plain words — `ok` with a report: "Done. The report is first in Saved reports." and the Saved reports list reloads; `ok` without a report: "Done, but no report was saved."; `failed`: "The run failed. Check that Claude Code is logged in and your plan limit is not reached."; `timed out`: "The run was stopped after 10 minutes."; `stopped`: "Stopped." A 403, 409 or 400 answer shows a fixed line for that status, never the body text as HTML.
> 7. `rust/README.md` and the root `README.md` get a short "Agent runs (optional, off by default)" part: what it does, the three commands, that it uses the owner's own Claude Code login and plan limits, that the app never reads the login, and that it is Linux and Rust app only. The landing page (`site/`) is not changed (PRD question 37 is open).

**Out of scope**: a switch for the setting; a job picker; progress text or output of the run; a run history; the Python dashboard server (the block shows nothing when `GET /api/agent-run` answers 404); the nav; other views; `site/**`.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16 (`docs/claude-md/tracks.md` row 13); the owner has not read it
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: US-039, FR-104, FR-103, FR-078, question 37 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T091 merged
**Entry point**: `web/src/AgentRun.jsx` (new), rendered by `web/src/WarningsPlanView.jsx`

## Dependency rule
No new npm package.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Vitest, one test per state from fixtures: off (text names the command, no button); `claude` not found (no button); on and idle (button + note); running (status in a live region, elapsed time, Stop, Run off); each of the five ended outcomes with its exact line | intent 2-6 |
| 2 | Vitest: a click on Run sends one POST with `job: "investigate"` and the page's scope; a click on Stop sends one POST to the stop route; a 403, 409 and 400 answer each show their fixed line; a body with `<script>` in it is never rendered | intent 4-6 |
| 3 | Vitest with fake timers: the GET repeats every 5 s only while `running`; it stops after the end and on unmount; after `ok` with a report id the Saved reports list is asked again once | intent 5, 6 |
| 4 | Vitest: `GET /api/agent-run` 404 → the block renders nothing and the rest of the page is unchanged | out of scope |
| 5 | Keyboard and screen reader: Run and Stop are real buttons with names, reachable by Tab, focus visible; the status changes are announced once (polite) | intent 5 |
| 6 | Existing tests and `npm --prefix web run build` pass; the built files are copied where the Rust app serves them, as the project's build step does | intent |
| 7 | UI evidence rows: screenshots of the off, idle, running and ended states at 375 / 1024 / 1440 px from the real `easy-personal-rs serve` with fixture answers; tokens only (grep raw hex in the new CSS) | intent |
| 8 | README part present in both files with the three commands and the four statements of intent 7 | intent 7 |

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
| Agent run block, 4 states, on Warnings & plan | screenshot, LLM vision vs `docs/design/graphite-mint/` cards and buttons | matches the design; the Saved reports section below is unchanged |

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
| 1 | The owner can tell before the click what a run spends | the fixed note is visible next to the Run button, not in a tooltip |
| 2 | The owner can stop a run from the place where they started it | Stop is in the same block, visible while running |
| 3 | An off feature does not look broken | the off state reads as a choice with its command, no error colour |

---

## Approach
Test first, one state at a time (`tdd`). Smallest code that meets the ACs; record cuts. Run `ux-evaluate` before Ready for Review.

## Edge Case Checklist
- A run started in another window or by `agent-run now`: the block shows running on the next load
- The page's scope changes during a run: the run keeps its own scope; the button stays off
- The GET fails (500, network): one quiet error line, no button, no loop of requests
- A double click on Run: one POST
- A very long workspace name in the scope: wraps, no overflow at 375 px
- Reduced motion: no spinner animation needed to read the state

## Files to Change (Predicted)
`web/src/AgentRun.jsx` (new), `AgentRun.test.jsx` (new), `WarningsPlanView.jsx`, `app.css`, `test/fixtures.js`, the built dashboard files, `rust/README.md`, `README.md`

## Files Must NOT Touch
`web/src/CopyCommand.jsx`, `web/src/SavedReports.jsx` beyond a reload hook, `web/src/App.jsx` and the nav, `rust/crates/**`, `src/easy_personal/**`, `site/**`

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output and the screenshots.

## Questions Log

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T092.md`
- [ ] Commits on branch `T092` only; final report with `Status:`
