# TASK_GUIDE — T126: Live like Claude 2c — steer an app session (model, mode, effort, context)
**Date**: 2026-11-19
**Complexity Level**: C3
**Risk Level**: High (new stdin control requests to a running CLI; new start flags)
**Priority**: P1
**Assigned agent**: backend-developer → frontend-developer
**Agent guide**: `agents/backend.md`, `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` question 46, US-044, FR-131, FR-109, FR-110, FR-116, NFR-008
2. `docs/ddr/0008-live-subagent-logs-and-steering.md`, `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVE2.md` (Edge Case Checklist: all of it applies)
3. `rust/crates/app/src/app_session.rs` (`argv` ~202, `MODELS` ~61, `interrupt` control_request ~667, `handle_line` ~700), `app_session_routes.rs` (`form` ~161, write guard), `web/src/SessionPanel.jsx` (`AppControls`), `web/src/NewSession.jsx`, `api.js`; the real CLI: `~/.local/share/claude/versions/2.1.328` handles `set_model`, `set_permission_mode`, `set_max_thinking_tokens`, `get_context_usage`, `apply_flag_settings`; `claude --help` lists `--effort low|medium|high|xhigh|max`
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-19: "yes, I want from the live, I can do all the things, prompt, choose the option, start new session, trace the agents logs include sub-agents, ..., let investigate and update all" [admin]. This task is *steer a running app session like the CLI*.

## Dependencies & Reachability
**Depends on**: none
**Entry point**: app session panel header; New session form

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Spike first (evidence in `tasks/evidence-T126/SPIKE.md`): with a real `claude -p --input-format stream-json` in a temp folder, send `set_model`, `set_permission_mode`, `get_context_usage` and `apply_flag_settings {effortLevel}`; record each control_response shape. Effort mid-session is built only if the spike shows it applies | FR-131 |
| 2 | `POST /api/app-sessions/{id}/model {model, confirmed}` and `/mode {mode, confirmed}` send the control request, wait ≤ 10 s for its `control_response`, and answer the CLI's success or error text; values only from fixed lists (models: the aliases + `opusplan` + the `[1m]` variants the spike shows accepted; modes: the FR-109 list); `bypassPermissions` needs `confirmed_bypass: true` | FR-131, FR-116 |
| 3 | `GET /api/app-sessions/{id}/context` returns the CLI's `get_context_usage` answer (numbers only: used, limit, percent by category if given) | FR-131 |
| 4 | Start form gains Effort (`--effort`, fixed list, default none) and the wider model list; argv test proves no free value reaches the argument list | FR-131, FR-116 |
| 5 | Panel: Model / Mode / Effort pickers in a running app session's header; on error the picker reverts and the CLI's line is shown; bypass asks twice; Context button shows the numbers | FR-131 |
| 6 | Guard tests (403 without token / bad Host / Origin), `confirmed` required; leak gate unchanged (0 hits) | NFR-008, NFR-002 |
| 7 | Rust lint/test/leak and web vitest/build/check_static pass; `rust/README.md` app-session section updated | all |

## UX Acceptance Criteria

| # | Criterion | Verdict |
|---|---|---|
| U1 | The current model, mode and effort are visible in the header without opening anything | |
| U2 | A failed switch says why in one line | |

## Approach
Backend first (spike, routes, argv), then the page. Reuse the interrupt control_request writer; add a pending-control map keyed by `request_id` resolved in `handle_line`.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/app_session.rs` | control requests, argv effort/models |
| `rust/crates/app/src/app_session_routes.rs` | routes, form lists |
| `web/src/SessionPanel.jsx` | pickers, context |
| `web/src/NewSession.jsx` | effort, models |
| `web/src/api.js` | calls |
| `tasks/evidence-T126/SPIKE.md` | spike record |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |
| store schema (`rust/crates/core/src/schema_v11.sql`) | no schema change in this plan |
| `src/easy_personal/**` | Python app |
| `rust/crates/app/src/answer_bridge.rs`, `PromptCard.jsx` | T127 |
| `rust/crates/core/src/transcript.rs` | T124 |

## Test plan
Rust unit tests with a fake CLI child (existing app-session test harness), the spike with the real CLI, vitest for the pickers.

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -5 && python3 parity/leak_rust.py && cd .. && cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: switch a running app session from sonnet to opus and to plan mode)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
