# TASK_GUIDE — T127: Live like Claude 2d — allow always and plan cards
**Date**: 2026-11-19
**Complexity Level**: C2
**Risk Level**: High (an answer can make a permission rule persist; Claude Code writes it)
**Priority**: P1
**Assigned agent**: backend-developer → frontend-developer
**Agent guide**: `agents/backend.md`, `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` question 46, US-044, FR-132, FR-111, FR-113, FR-114, FR-115, NFR-008
2. `docs/ddr/0008-live-subagent-logs-and-steering.md`, `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVE2.md` (Edge Case Checklist: all of it applies)
3. `rust/crates/app/src/app_session.rs` (prompt card build ~930-955), `app_session_routes.rs` (`answer` ~605, `mcp_post` ~698), `answer_bridge.rs` (`hook ask` PermissionRequest output, `/api/answer` ~1467), `web/src/PromptCard.jsx` (`readPrompt` ~50, plan card ~184), `PendingAnswer.jsx`, `SessionPanel.jsx` (`answerBody` ~36); Claude Code hooks docs for `PermissionRequest` output (`decision.updatedPermissions`) and the permission-prompt tool answer (`updatedPermissions`)
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-19: "yes, I want from the live, I can do all the things, prompt, choose the option, start new session, trace the agents logs include sub-agents, ..., let investigate and update all" [admin]. This task is *choose the option* at full CLI parity.

## Dependencies & Reachability
**Depends on**: none (merge after T126 if both touch `app_session.rs`; rebase, never overwrite)
**Entry point**: prompt cards in the Live panel

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | App sessions: when the prompt tool input has `permission_suggestions`, the card gets them (each with its destination and a short rule summary); answering **Allow and don't ask again** returns `{behavior:"allow", updatedInput, updatedPermissions:[<that suggestion, unchanged>]}`. The server only accepts a suggestion index the CLI offered for that prompt — never a rule from the page | FR-132 |
| 2 | Terminal sessions: same for the `PermissionRequest` hook (`permission_suggestions` in the hook input → `hookSpecificOutput.decision.updatedPermissions`); if the hook input has none, no button | FR-132 |
| 3 | Terminal `ExitPlanMode` (PermissionRequest; PreToolUse cut 2026-11-20 by a 3-Advocate Debate, see TASK_REVIEW_T127.md) shows the plan card (plan text as FR-115, Approve / Keep planning + note), not a generic tool card | FR-132 |
| 4 | Plan approve offers **Approve and auto-accept edits**: app sessions answer allow + `updatedPermissions` `{type:"setMode", mode:"acceptEdits", destination:"session"}` only if the CLI offered it, else the server sends `set_permission_mode acceptEdits` after the allow; terminal: only if offered in suggestions, else hidden | FR-132 |
| 5 | Each button names where the rule lands: session → "this session", localSettings → "this project (just you)", projectSettings → "this project (shared)" (split 2026-11-20 by Debate, FR-132 "naming where it lands"), userSettings → "you, everywhere" | FR-132 |
| 6 | Tests: suggestion passed through byte-identical; forged index → 400; no suggestions → no button; guard tests; leak gate 0 hits | FR-132, NFR-008 |
| 7 | Rust lint/test/leak and web vitest/build/check_static pass | all |

## UX Acceptance Criteria

| # | Criterion | Verdict |
|---|---|---|
| U1 | Where an always-allow lands is said on the button itself | |
| U2 | A terminal plan reads as a plan, not JSON | |

## Approach
Keep the suggestions in the in-memory prompt record (as the input is kept today) and answer by index. No settings file is opened by the app.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/app_session.rs` | keep suggestions on the prompt |
| `rust/crates/app/src/app_session_routes.rs` | answer with index |
| `rust/crates/app/src/answer_bridge.rs` | hook input suggestions, output, plan kind |
| `web/src/PromptCard.jsx` | buttons, plan card for terminal |
| `web/src/PendingAnswer.jsx`, `SessionPanel.jsx` | answer bodies |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |
| store schema (`rust/crates/core/src/schema_v11.sql`) | no schema change in this plan |
| `src/easy_personal/**` | Python app |
| any Claude settings file | Claude Code writes the rule itself (DDR-0008) |
| `rust/crates/core/src/transcript.rs` | T124 |

## Test plan
Rust tests with recorded prompt inputs that carry suggestions; vitest for the cards.

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -5 && python3 parity/leak_rust.py && cd .. && cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: allow a Bash command with "don't ask again" and see the next same command not ask)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
