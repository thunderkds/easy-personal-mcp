# TASK_GUIDE — T107: Write sessions 5 — Settings bridge block and the Live view answer card for terminal sessions
**Date**: 2026-11-17
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

## Mandatory Startup
1. `PRD.md` FR-113, FR-114, FR-115, FR-108; `docs/ddr/0006-write-sessions-cli-parity.md`
2. `tasks/TASK_GUIDE_T106.md` (route JSON), `tasks/TASK_GUIDE_T104.md` (reuse its prompt cards, do not fork them)
3. `web/src/LiveView.jsx`, the Settings view (T092)

## Requirement
Owner 2026-11-17: "you can input the text to the ask question in the app and send to claude session are running".

## Dependencies & Reachability
**Depends on**: T106; T104 for the shared card components (if T104 is not merged, build the cards here and T104 reuses them)

## Acceptance Criteria
| # | Criterion | Traces to |
|---|---|---|
| 1 | Settings: *Answer terminal sessions from the app* block; per account on / off / changed by hand / binary moved; Turn on shows the exact entry text and needs confirm; note that sessions started earlier are not covered | FR-113 |
| 2 | Live view: a session with `pending_answer` shows the card (question options or Allow/Deny) plus **Answer in terminal**; 409 → *already answered*; expiry → *the terminal asks now* | FR-114 |
| 3 | Footnote of the Live view names the texts it now shows (FR-115) | FR-115 |
| 4 | Vitest per state; `ux-evaluate` 375 / 1280 | — |
| 5 | Real run (owner gate): a terminal session's `AskUserQuestion` answered from the app; a `Bash` permission allowed from the app | FR-114 |

## Verification Command
```bash
cd web && npm run lint && npx vitest run 2>&1 | tail -5
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `docs/` updated
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
