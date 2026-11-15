# TASK_GUIDE — T077: Watch 6: dashboard Watch view — 24 h machine and plan-limit graphs, effectiveness table
**Date**: 2026-11-15
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
5. Read `BRAINSTORMING_LOG_WATCH.md` (decisions D1-D7, Option A, Edge Case Checklist) and the PRD rows FR-100
6. Read `docs/design/graphite-mint/README.md` and the artboards; T072/T073/T075 API shapes; `web/src/App.jsx` nav, an existing view (`UsageView.jsx`) and `web/src/tokens.css`
7. HITL: if no artboard covers the Watch view, propose a layout in the Graphite Mint style and wait for the owner's OK (the Supervisor relays)
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-14): a background service that watches and collects state, with a tray icon and desktop alerts. Path A (compute-on-read, no store migration) chosen 2026-11-14; task breakdown approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. A Watch view in the dashboard nav: machine status (CPU, RAM, disk, load, battery, claude processes) as 24 h sparklines with the latest value; plan limits per account (5 h / 7 d) as 24 h lines; "no data" gaps drawn as gaps, never 0.
> 2. Effectiveness table per task (task, tokens, cost, sessions, good results, tokens per good result, done, tokens per task done) with the unattributed line last and the "oldest stored day" note when given.
> 3. Read-only; data from `GET /api/machine`, `/api/limits/history`, `/api/effectiveness`. Graphite Mint tokens only.

**Out of scope**: any API change, alerts list UI (beyond showing nothing), Live/Board views.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-100 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T072, T073, T075 merged
**Entry point**: `web/src/WatchView.jsx` (new), `App.jsx`

## Dependency rule
No new npm package (draw sparklines in SVG as existing views do).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Vitest: renders fixture data, gaps for null, unattributed last, oldest-day note | intent |
| 2 | Existing tests and `npm run build` pass | intent |
| 3 | UI evidence rows: 375 / 1024 / 1440 px screenshots, tokens only (grep raw hex) | intent |

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
| Verification command | ☑ | Supervisor `/verify` 2026-11-15 pass (build, real serve, 3 routes 200, screenshots) |
| code-review | ☑ | 0 P0/P1, 2 P2 (dead export; no auto refresh -> follow-up), 2 P3 |
| UI: visual regression | ☐ | |
| UI: design-system compliance | ☐ | |
| UI: responsiveness | ☐ | |

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T077.md`.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| Watch view | screenshot, LLM vision vs `docs/design/graphite-mint/` | matches the design |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Colours | grep for raw hex / pixel check of icon colours | tokens / T071 icon set only |

### 3. Layout / Responsiveness
| Viewport | Verification method | Expected result |
|---|---|---|
| 375 / 1024 / 1440 px | screenshot | no horizontal page scroll |

---

## Approach
Test first, one vertical slice at a time (`tdd`). Smallest code that meets the ACs; record cuts.

## Edge Case Checklist
- Empty buffers right after start
- One account only / no account
- Long task list → scroll inside the table, no page-wide horizontal scroll at 375 px

## Files to Change (Predicted)
`web/src/WatchView.jsx`, `App.jsx`, CSS, tests, fixtures

## Files Must NOT Touch
`rust/**` (except none), `src/easy_personal/**`

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T077.md`
- [ ] Commits on branch `T077` only; final report with `Status:`
