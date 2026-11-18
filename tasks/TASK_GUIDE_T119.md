# TASK_GUIDE — T119: Live like Claude 5 — filters, names, past sessions
**Date**: 2026-11-18
**Complexity Level**: C2
**Risk Level**: Medium (page only)
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `PRD.md` question 45, US-043, FR-128 (filters, Past), FR-126, FR-127, FR-112, NFR-002 (fifth exception), NFR-008
2. `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVENATIVE.md` (Edge Case Checklist: all of it applies)
3. `web/src/LiveView.jsx` after T118, `api.js`, the T117 route shapes
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-18: "should check the live, cause i need the work like claude native, where user can input, choose the option, trace the logs, and manage the sections, [admin], supervisor". Owner answers the same day: full transcript (shown, never stored); queue until the turn end for a busy terminal session; one place for all sessions; stop a terminal session; past sessions + resume; filters and names. This task is the page half of *filters*, *names* and *past sessions*.

## Dependencies & Reachability
**Depends on**: T117, T118
**Entry point**: Live view filters and *Past* switch

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Filters above the list: *needs me*, *working*, *idle*, project, kind (terminal / app / headless); kept in `localStorage` (filters only, wrapped in try/catch); counts per filter | FR-128 |
| 2 | Rename in the panel header: inline field, Enter saves (FR-127 route), Esc cancels, empty removes; the shown name falls back to Claude's name, then project + short id | FR-127 |
| 3 | *Past* switch lists FR-126 sessions with the same filters + date range, *Load more*; opening one shows the read-only conversation (FR-123) with Resume / Fork (FR-112 flow) | FR-126, FR-128 |
| 4 | `ux-evaluate` at 375 and 1280; tokens only; no horizontal page scroll | FR-128 |
| 5 | `cd web && npx vitest run && npm run build`; `python3 scripts/check_static.py` ok | all |

## Approach
Small components on top of T118's panel.

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/LiveView.jsx` | filters, Past switch |
| `web/src/PastSessions.jsx` | new |
| `web/src/api.js` | past, rename |
| `web/src/*.test.jsx` | tests |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/**` | T115-T117 own the server |
| `src/easy_personal/**` | Python app |

## Test plan
vitest with mocked routes; ux-evaluate.

## Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: find yesterday's session and resume it)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
