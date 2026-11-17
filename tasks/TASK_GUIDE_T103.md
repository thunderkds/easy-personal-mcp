# TASK_GUIDE — T103: Write sessions 1 — app session runner and routes (start, talk, interrupt, stop, prompts)
**Date**: 2026-11-17
**Complexity Level**: C3
**Risk Level**: High (starts the owner's Claude Code with full tools; new write routes)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `docs/ddr/0006-write-sessions-cli-parity.md`
2. Read `PRD.md` FR-101, FR-102, FR-103, FR-109, FR-110, FR-111, FR-115, FR-116, NFR-002, NFR-008
3. Read `tasks/evidence-T102/SPIKE.md`: the stream-json shapes and the permission-tool wiring it chose are binding
4. Read `rust/crates/app/src/agent_run.rs` (account flag, env scrub `DROPPED`, lock, process group stop), `agent_routes.rs`, `server.rs` (guard), `mcp.rs` / `mcp_http.rs`
- Tests use a fake `claude` script (as the agent run tests do); never start the real `claude` in a test.

## Requirement
Owner 2026-11-17: "what we can do with the claude CLI, we can do with our application" (PRD question 40).

## Dependencies & Reachability
**Depends on**: T102 (go on routes 1, 3, 4, 5)
**Entry point**: new `rust/crates/app/src/app_session.rs` + routes under `/api/app-sessions`

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `POST /api/app-sessions` `{repo, workspace, account?, model, mode, worktree, prompt, confirmed:true}` → 202 `{id}`; the argument list is built only from fixed lists and stored repos (a test feeds `;`, `--`, `$(…)`, an unknown model/mode, a path not in the store → 400 each, nothing started) | FR-109, FR-116 |
| 2 | No `--allowedTools` / `--disallowedTools` narrowing; `--permission-prompt-tool` always set; `--dangerously-skip-permissions` only when the owner picked that mode; the FR-102 variables removed from the child's environment (test reads the fake child's env) | FR-109, FR-111 |
| 3 | `worktree: true` → `git worktree add` under `~/.easy-personal/worktrees/` on branch `easy/<yyyymmdd-hhmm>-<4 hex>`; not a git repo / branch taken / folder exists → 400 fixed message | FR-109 |
| 4 | `POST /api/app-sessions/{id}/message` writes one stdin line; `/interrupt` sends the stdin `control_request` interrupt (T102 AC 3; never SIGINT); `/stop` SIGTERM then SIGKILL after 5 s to the process group; a 5th concurrent session → 409 | FR-110, question 41 |
| 5 | `GET /api/app-sessions` and `/{id}` give state, account, model, mode, folder name, branch, changed file names, and the conversation items (owner text, assistant text, tool labels + outcome, pending prompt card) — no tool input or output | FR-110, FR-115 |
| 6 | A permission prompt from the child becomes a pending card; `POST /api/app-sessions/{id}/answer` `{prompt_id, behavior, updatedInput?|message?}` returns it to the child; no answer → the child waits (no auto-allow, test with a 2 s fake wait) | FR-111 |
| 7 | Every route: NFR-008 guard + `confirmed:true` on writes; token never in a URL | FR-116 |
| 8 | Leak gate: planted owner prompt, assistant text and a planted secret in a tool input never reach the store, the log, MCP tools, CLI output, `/api/alerts`; extend `rust/parity/leak_rust.py` | FR-115 |
| 9 | Text dropped on session end + panel close and at exit (memory test) | FR-115 |
| 10 | `cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` pass | all |

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/app_session.rs` (new) | runner, stream parse, prompt bridge |
| `rust/crates/app/src/app_session_routes.rs` (new) | routes |
| `rust/crates/app/src/server.rs`, `api.rs` | mount routes |
| `rust/crates/app/src/server.rs` (new loopback route in the dashboard process) | the permission prompt tool as a minimal HTTP MCP endpoint (per T102 AC 5 option (a)), per-session bearer secret, `MCP_TOOL_TIMEOUT` in the child env, one answer per prompt even when Claude retries the call — **not** part of the shared MCP surface |
| `rust/parity/leak_rust.py` | new planted texts |

## Files must NOT touch
| File | Reason |
|------|--------|
| `agent_run.rs` behaviour | the agent run stays as FR-102 (share helpers only) |
| the MCP tool surface visible to Claude sessions | frozen; the prompt tool is reachable only by app sessions |
| `src/easy_personal/**`, parity goldens | Python app untouched |

## Edge cases
Child exits at once (no `claude`, bad login) → state `ended` with a fixed reason; account turned off mid-session → stop; a stdin write to a dead child; two answers to one prompt → 409; the owner's prompt with control/bidi characters (FR-092 cleaning on display only, sent as typed).

## Test plan
Unit + route tests with the fake `claude`; a real run on the owner's machine is T104's AC.

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -5 && python3 parity/leak_rust.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `docs/` updated (README "App sessions" part)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
