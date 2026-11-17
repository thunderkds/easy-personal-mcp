# TASK_GUIDE — T104: Write sessions 2 — app session page: New session form, session panel, prompt cards
**Date**: 2026-11-17
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. Read `PROJECT_SPEC.md`, `docs/ddr/0006-write-sessions-cli-parity.md`, `PRD.md` FR-109..FR-111, FR-115, FR-116, FR-104 (Settings view, Graphite Mint design)
2. Read `tasks/TASK_GUIDE_T103.md` and its route JSON (the contract)
3. Read `web/src/LiveView.jsx`, the Settings view and the Workspaces page from T092, `web/src/api.js`

## Requirement
Owner 2026-11-17: "what we can do with the claude CLI, we can do with our application".

## Dependencies & Reachability
**Depends on**: T103
**Entry point**: Workspaces page (**New session** per repo) and Live view (**Open in app** on an app session row)

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | New session form: repo (stored repos only), account, model, mode, *new worktree*, first prompt; a confirm step that names the account and mode; *bypass permissions* mode needs a second, explicit confirm | FR-109, FR-116 |
| 2 | Session panel streams the conversation (React text only, never HTML); tool calls as labels; message box with Enter to send and Shift+Enter for a new line; Interrupt and Stop | FR-110 |
| 3 | Prompt cards: Allow / Deny + note; a question with options (radio or check boxes) + Other; a plan with Approve / Keep planning; the card stays until answered | FR-111 |
| 4 | Panel header: session id, account, model, mode, folder, branch + changed file names for a worktree | FR-110 |
| 5 | Vitest for each card and the form's fixed lists; no token in any URL the page builds | FR-116 |
| 6 | `ux-evaluate` at 375 px and 1280 px | — |
| 7 | Real run on the owner's machine (owner gate): start a session on a scratch repo, send `/help`, allow one `Bash` prompt, answer one question, interrupt, stop | FR-109..FR-111 |

## Files must NOT touch
`src/easy_personal/**`, Rust routes (T103 owns them), any text in `localStorage`.

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
