# TASK_GUIDE — T128: Live like Claude 2e — New session in Live, todo checklist
**Date**: 2026-11-19
**Complexity Level**: C1
**Risk Level**: Medium (page only; navigation change)
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` question 46, US-044, FR-133, FR-109, FR-112, FR-128
2. `docs/ddr/0008-live-subagent-logs-and-steering.md`, `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVE2.md` (Edge Case Checklist: all of it applies)
3. `web/src/App.jsx` (`openAppSession` ~226, `resumeInApp` ~232, nav), `LiveView.jsx`, `NewSession.jsx`, `AppSessions.jsx`, `SessionPanel.jsx` (Resume/Fork ~342), `PastSessions.jsx` (~150), `Transcript.jsx` (`Tool`), `web/src/test/fixtures.js`
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-19: "yes, I want from the live, I can do all the things, prompt, choose the option, start new session, trace the agents logs include sub-agents, ..., let investigate and update all" [admin]. This task is *start new session* from Live, and a readable todo list.

## Dependencies & Reachability
**Depends on**: none (T126 adds fields to `NewSession.jsx`; rebase onto it if it lands first)
**Entry point**: Live view header

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Live has **New session**; it opens the FR-109 form inside Live (a panel or sheet, not another tab); on start the new session's panel is selected | FR-133 |
| 2 | Resume / Fork / Continue (Live panel, Past, Workspaces) open the form inside Live, prefilled as today | FR-133 |
| 3 | The App sessions tab is gone from the navigation; its old hash/URL opens Live | FR-133 |
| 4 | A `TodoWrite` tool entry renders as a checklist (pending ○, in progress ◐, done ●, text as React text); invalid input falls back to JSON | FR-133 |
| 5 | No dialog covers a pending prompt card (FR-128 rule holds with the form open) | FR-128 |
| 6 | vitest: open form from Live, start selects panel, resume prefill, old URL, todo render + fallback; build and check_static pass | all |

## UX Acceptance Criteria

| # | Criterion | Verdict |
|---|---|---|
| U1 | New session is visible in Live without scrolling at 1280 px | |
| U2 | The form never hides a waiting question card | |
| U3 | A todo list reads as a checklist | |

## Approach
Move `NewSession` into Live as an in-place sheet; keep its API calls unchanged.

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/App.jsx` | nav, routing |
| `web/src/LiveView.jsx` | New session button + sheet |
| `web/src/AppSessions.jsx` | remove tab body / reuse |
| `web/src/Transcript.jsx` | TodoWrite checklist |
| tests | specs |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |
| store schema (`rust/crates/core/src/schema_v11.sql`) | no schema change in this plan |
| `src/easy_personal/**` | Python app |
| `rust/**` | page only |

## Test plan
vitest; `ux-evaluate` at 375/1280.

## Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: start a session from Live and see its panel open)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
