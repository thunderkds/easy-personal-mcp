# TASK_GUIDE — T083: Watch follow-up: Watch view refreshes every 60 s; graphs share one 'now'
**Date**: 2026-11-15
**Complexity Level**: C1
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
5. Read `web/src/WatchView.jsx`, `web/src/WatchView.test.jsx`, `web/src/useLoad.js`, and the T077 review file `tasks/TASK_REVIEW_T077.md` (code-review P2/P3)
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-15): "plan them" — the follow-ups from the Watch Stage 4 reviews (T074, T076, T077) and from the T062 cutover run. Breakdown of 4 tasks approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. While the Watch view is open, re-read `/api/machine`, `/api/limits/history` and `/api/effectiveness` every 60 s (the sampler interval). Stop the timer when the view closes (unmount). A failed refresh keeps the last good data on screen and shows the part's error line; it does not blank the page.
> 2. All machine and limit graphs end at one shared "now": the newest sample time across all series (or the time of the last read), so a series that stopped (stale limits file) ends before "now" instead of being stretched to it.
> 3. Drop the unused `export` of `linePath` (T077 code-review P2).

**Out of scope**: a period picker, tooltips, live push (SSE/WebSocket), any API change.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-100 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T077 merged
**Entry point**: `web/src/WatchView.jsx`

## Dependency rule
No new npm package.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Vitest with fake timers: after 60 s each of the 3 routes is fetched again; after unmount no further fetch | intent |
| 2 | Vitest: a refresh that fails keeps the earlier rows and shows the error line | intent |
| 3 | Vitest: two series where one ends 1 h earlier: its path ends before the right edge (shared end time) | intent |
| 4 | Existing tests and `npm run build` pass | intent |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build 2>&1 | tail -3
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Verification command | ☐ | |
| code-review | ☐ | |
| security-review | ☐ N/A | Risk Low |
| UI: visual regression | ☐ | |
| UI: design-system compliance | ☐ | |
| UI: responsiveness | ☐ | |

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T083.md`.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| Watch view | screenshot 1440 px before/after, LLM vision | same layout as T077 (owner-approved) |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Colours | grep for raw hex in changed files | tokens only |

### 3. Layout / Responsiveness
| Viewport | Verification method | Expected result |
|---|---|---|
| 375 / 1024 / 1440 px | screenshot | no horizontal page scroll |

---

## Approach
Test first (`tdd`). Smallest change that meets the ACs; record cuts.

## Edge Case Checklist
- Tab hidden for a long time: no pile-up of overlapping requests (skip a tick while one is in flight)
- Server restarted: buffers empty again → empty states, no crash

## Files to Change (Predicted)
`web/src/WatchView.jsx`, `web/src/WatchView.test.jsx`

## Files Must NOT Touch
`rust/**`, `src/easy_personal/**`, other views, `app.css`

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T083.md`
- [ ] Commits on branch `T083` only; final report with `Status:`
