# TASK_GUIDE — T105: Write sessions 3 — Resume in app, fork, Continue
**Date**: 2026-11-17
**Complexity Level**: C2
**Risk Level**: Medium (wrong resume of an open session loses messages)
**Priority**: P2
**Assigned agent**: backend-developer (routes) then frontend-developer (buttons)
**Agent guide**: `agents/backend.md`, `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

## Mandatory Startup
1. `PRD.md` FR-112, FR-109, question 41; `docs/ddr/0006-write-sessions-cli-parity.md`; `tasks/evidence-T102/SPIKE.md` AC 7
2. `rust/crates/app/src/app_session.rs` (T103), `rust/crates/core/src/live.rs` (last record time per session)

## Requirement
Owner 2026-11-17: CLI parity; `claude --resume` / `--continue` are CLI features.

## Dependencies & Reachability
**Depends on**: T103, T104

## Acceptance Criteria
| # | Criterion | Traces to |
|---|---|---|
| 1 | `POST /api/app-sessions` accepts `resume: <session id>` (hex and `-` only) or `continue: true`; the folder is the session's stored project folder, never a page-given path | FR-112, FR-116 |
| 2 | A session with a record in the last 2 minutes: resume without `fork: true` → 409 fixed message; with fork → `--fork-session` | FR-112 |
| 3 | Live view: **Resume in app** on a quiet non-app session; **Fork in app** with the fixed note on an active one; Workspaces: **Continue** | FR-112 |
| 4 | Tests with the fake `claude` (argument list asserted) and Vitest for the buttons | — |

## Files must NOT touch
`~/.claude/projects/**` (never written by the app), `src/easy_personal/**`.

## Verification Command
```bash
cd rust && cargo test --workspace 2>&1 | tail -5 && cd ../web && npx vitest run 2>&1 | tail -5
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `docs/` updated
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
