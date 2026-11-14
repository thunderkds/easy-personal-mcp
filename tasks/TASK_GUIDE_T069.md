# TASK_GUIDE — T069: First run screen rework (progress, step cards, Copy, Check again, completion, nav badge)
**Date**: 2026-11-14
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/frontend.md`
5. Read `rust/docs/UI_REDESIGN_PLAN.md` (row G2) and `docs/design/graphite-mint/README.md`
6. Read the design sources: `docs/design/graphite-mint/FirstRun.dc.html` (static look) and the `firstrun` view of `Prototype.dc.html` (markup and the state logic in its script: `copied`, `timerDone`, "Check again", completion)
7. Read `web/src/FirstRun.jsx` + test, the first-run nav entry in `web/src/App.jsx`, `GET /api/first-run` in `rust/crates/app/src/api.rs` (`first_run_steps`) for the step ids and fields
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder

---

## Requirement

**Original request** (owner, 2026-11-14): UI update from the Graphite Mint prototype; plan accepted ("yes, just following the guide").

**Restated intent**:
> The First run view matches the prototype:
> - Intro line "Four steps to fill the dashboard with your own sessions. Everything stays on this machine." (keep the real step count if the server returns another number).
> - `N of M steps done` (N in accent mono) + `K left`, and a 6 px progress bar (`role="progressbar"`, `aria-valuenow`).
> - One card per step, numbered `01`…: done steps muted with a `done` tick; the **first not-done step** has the accent border/glow and `next step`; later steps `not done`.
> - A step done by a command shows the command in a mono box with `$ ` prefix and a **Copy** button (clipboard; label turns to `Copied` for ~2 s), then "Ran it in your terminal?" + **Check again** (re-fetches `/api/first-run`).
> - Account step: "Only confirmed accounts are read." + **Open Agents**; workspace step: **Open Workspaces** (existing callbacks).
> - All done: a banner "Setup is complete. Your usage appears after the next ingest." + **Go to Usage**.
> - Sidebar First run entry shows a mono badge with steps left; no badge at 0.

**Out of scope**: new routes; the Agents and Workspaces flows inside the prototype (they exist in their own views); restart-prototype button.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (plan accepted 2026-11-14)
- [x] Every Acceptance Criterion traces to the Requirement
- [ ] Requirement Refs: the first-run FR in `PRD.md` (find and cite)

---

## Dependencies & Reachability
**Depends on**: none (T068 touches the top bar, not this view; rebase if both edit `App.jsx` nav)
**Entry point**: `web/src/FirstRun.jsx`

## Dependency rule
No new npm package.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Vitest: 2 of 4 done → `2 of 4 steps done`, `2 left`, progressbar `aria-valuenow=2`, step 02 marked `next step`, 03 done, 04 `not done` | progress, cards |
| 2 | Vitest: Copy writes `easy-personal schedule install` (the server's command text) to a mocked clipboard and shows `Copied`; clipboard failure shows a short error, not a crash | Copy |
| 3 | Vitest: Check again calls `/api/first-run` again and re-renders with the new state | Check again |
| 4 | Vitest: all done → completion banner; Go to Usage switches view; nav badge absent; with 2 left the badge reads `2` | completion, badge |
| 5 | Existing tests and `npm run build` pass | all |
| 6 | UI evidence rows below filled | all |

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
| Web suite and build pass | ☐ | |
| code-review | ☐ | |
| UI: visual regression | ☐ | |
| UI: design-system compliance | ☐ | |
| UI: responsiveness | ☐ | |

## Demonstration
BEFORE: current First run screenshot. AFTER: same fixture, 2 of 4 done, and the all-done state.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| First run at 1440 px, 2 of 4 done | easy-ui MCP or headless `google-chrome --screenshot`, LLM vision vs `FirstRun.dc.html` | same layout, max-width 820 px, card borders, highlight on the next step |
| First run all done | same | completion banner as in `Prototype.dc.html` |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Colours | grep new CSS for raw hex | tokens only |
| Typography | computed style | h1 28 px/600; step titles 15 px/600; numbers mono 12 px |
| Spacing | computed style | cards 16px 18px padding, 10 px gap |

### 3. Layout / Responsiveness
| Viewport | Verification method | Expected result |
|---|---|---|
| 375 px | screenshot | cards full width; command box scrolls inside itself |
| 1024 / 1440 px | screenshot | as the artboard |

---

## Approach
Test-first. Keep the server as the only source of step state (no client-side "done"). The clipboard: `navigator.clipboard.writeText` (127.0.0.1 is a secure context); on failure select the command text and say "Copy failed — select and copy". 

## Edge Case Checklist
- Server returns a step id the page does not know → generic card with its title, no crash
- `/api/first-run` fails on Check again → keep old state, show error line
- Command text from the server is shown as text, never as HTML
- Copy pressed twice fast → one timer

## Files to Change (Predicted)
`web/src/FirstRun.jsx` + test, `web/src/App.jsx` (nav badge only), `web/src/app.css`.

## Files Must NOT Touch
`rust/**`, `src/easy_personal/**`, `web/src/tokens.css` values, the top bar (T068).

## Test Plan
Vitest for AC 1-4; build; screenshots.

## Completion Checklist
- [ ] AC 1-6 with pasted output
- [ ] Commits on branch `T069` only; final report with `Status:`
