# TASK_GUIDE — T091: Agent run 2: `GET /api/agent-run`, `POST /api/agent-run`, `POST /api/agent-run/stop`
**Date**: 2026-11-16
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P2
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_AGENTRUN.md` and the PRD rows FR-103, FR-101, FR-102, NFR-002, NFR-008
6. Read T090's `rust/crates/app/src/agent_run.rs`, then `rust/crates/app/src/server.rs` (the guard), `api.rs` (`/api/alerts` and `/api/alerts/pause` as the model for a small state route and a guarded POST), `writes.rs` (body checks: 415, 400, 413), `api_tests.rs`, `rust/parity/leak_rust.py`
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures, a fake `claude` and temp folders)
- Never run the real `claude -p` in a test

---

## Requirement

**Original request** (owner, 2026-11-16): the app runs the agent job itself on the Claude subscription; "opt-in and off by default".

**Restated intent** (PRD FR-103):
> 1. `GET /api/agent-run` → `{enabled, claude_found, running, last}`. `last` is null or `{job, started, ended, outcome, report_id}`; `outcome` is one of T090's words. No path, no argv, no output text, no environment value.
> 2. `POST /api/agent-run` with a JSON object `{job: "investigate", workspace?, account?}` starts a run in the background and answers 202 with the same shape as the GET. 403 with a fixed text when the setting is off — the flag is read at each request, not cached. 409 when a run is going. 400 for another `job`, an unknown key, a wrong type, or a workspace or account the store does not hold.
> 3. `POST /api/agent-run/stop` stops the going run and answers 200 with the state; with no run going it answers 200 and changes nothing.
> 4. All three are behind the existing guard (token, Host, Origin) and the existing body checks. No route reads or writes the setting's flag except to read it. A run started by the server ends when the server stops (no child lives on).
> 5. The routes work in `serve`, `app` and `tray` the same way. `serve` is no longer read-only for this one feature: say so in its `--help` text.

**Out of scope**: the setting route (never: PRD question 36); UI (T092); MCP tools for agent runs; a run history; progress text; the Python app.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16 (`docs/claude-md/tracks.md` row 13); the owner has not read it
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: US-039, FR-103, NFR-002, NFR-008, question 36 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T090 merged
**Entry point**: `rust/crates/app/src/api.rs` routes; state shared through `Api`

## Dependency rule
No new crate.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Guard tests: each of the three routes without the token, with a wrong Host, or with a foreign Origin is refused exactly as the other write routes are, and starts nothing | intent 4, NFR-008 |
| 2 | Off test: with no flag, `POST /api/agent-run` → 403, the fake `claude` is never started; the flag written while the server runs → the next POST is 202 (read per request); the flag removed → 403 again and a going run is stopped | intent 2 |
| 3 | Body tests: wrong content type 415; not JSON, not an object, unknown key, `job` other than `investigate`, wrong types → 400; unknown workspace or account → 400; an oversized body → 413 | intent 2 |
| 4 | Run test with a fake `claude`: 202, then `running: true`; a second POST → 409; after the end `running: false` and `last.outcome`; with a report saved by the fake run (through the real `save_report` code path in a temp store), `last.report_id` is that id | intent 1, 2 |
| 5 | Stop test: stop during a run → `last.outcome` is `stopped`, no process of the group lives on; stop with no run → 200, no change | intent 3 |
| 6 | Shutdown test: the server stops during a run → the child's group is gone | intent 4 |
| 7 | Leak gate: the three routes are registered in `rust/parity/leak_rust.py` and the run (from `rust/`) `env -u CLAUDE_CONFIG_DIR <main checkout>/rust/parity/.venv/bin/python parity/leak_rust.py` prints `0 hits` with no `failed checks`, exit 0; a unit test proves a planted secret in the fake `claude`'s output is in no response body | intent 1, NFR-002 |
| 8 | `cd rust && cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` pass; the parity check `--check` (run from `main`) reports no new difference except the three new routes, named as Rust-only; `security-review` at Stage 4 (Risk High) | intent |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test --workspace 2>&1 | tail -5
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Verification command | ☐ | |
| code-review | ☐ | |
| security-review | ☐ | |
| UI: visual regression | ☐ N/A | backend only |
| UI: design-system compliance | ☐ N/A | backend only |
| UI: responsiveness | ☐ N/A | backend only |

## Demonstration
> BEFORE/AFTER/DELTA/WITNESS go in `tasks/TASK_REVIEW_T091.md`.

## UI / Design Acceptance Criteria
N/A — backend only.

---

## Approach
Test first, one vertical slice at a time (`tdd`): GET → POST off/403 → POST run → 409 → stop → shutdown → leak gate. The handler never waits for the run: it starts it on a blocking thread and answers. Smallest code that meets the ACs; record cuts.

## Edge Case Checklist
- Two dashboards (and the tray) on one machine: T090's lock decides, each server answers 409 for the other's run and shows `running: true`
- A run started by `agent-run now` in a terminal: the GET shows `running: true`; stop from the dashboard — decide and record: it may only stop its own server's run (the smaller change) and answers 409 for a foreign one
- The server's `last` is in memory only: after a restart it is null
- A token in the query string is accepted only by `/api/live`: never by these routes
- The 403 text names no path and no command line beyond `easy-personal-rs agent-run on`

## Files to Change (Predicted)
`rust/crates/app/src/api.rs`, `server.rs`, `agent_run.rs`, `main.rs` (help text), `api_tests.rs`, `rust/parity/leak_rust.py`

## Files Must NOT Touch
the guard's rules in `server.rs` (add routes only), `writes.rs` body rules, the store schema, `web/**`, `src/easy_personal/**`, any Claude config or credential file

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Questions Log

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T091.md`
- [ ] Commits on branch `T091` only; final report with `Status:`
