# TASK_GUIDE — T091: Agent run 2: the agent run account (setting chosen in the app) and the routes `GET /api/agent-run`, `PUT /api/agent-run/setting`, `POST /api/agent-run`, `POST /api/agent-run/stop`
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

**Original request** (owner, 2026-11-16): the app runs the agent job itself on the Claude subscription; "opt-in and off by default"; then, the same day: "all the actions should be in the application, not from the command, or the manual, like when I run the app, I will have the option to choose the account that is used for the application, when I choose, it will show on the settings, from this time, the application will use this account for the automate all progress in the application".

**Restated intent** (PRD FR-101 and FR-103, both rewritten 2026-11-16 after T090 was built):
> 1. The setting becomes the **agent run account**. T090's flag file `agent-run.on` now holds the chosen account's id as its content (one line). An account chosen = on; no file = off. A flag file with no content (written by T090's `agent-run on`) means on with the default account, as T090 runs it. `Runner::run` with no account given uses the stored one. `agent-run status` prints the account. `agent-run on --account <id or label>` writes it; plain `agent-run on` keeps T090's behaviour.
> 2. `GET /api/agent-run` → `{enabled, account, claude_found, running, last}`. `account` is null or `{id, label}`. `last` is null or `{job, started, ended, outcome, report_id}`; `outcome` is one of T090's words. No path, no argv, no output text, no environment value.
> 3. `PUT /api/agent-run/setting` with `{account: "<id>", confirmed: true}` stores that account. 400 when the account is unknown, not a confirmed Claude account (a Codex or an ignored one), or `confirmed` is not `true`. `{account: null}` removes the flag and stops a going run. It answers the GET shape.
> 4. `POST /api/agent-run` with `{job: "investigate", workspace?}` starts a run in the background with the stored account and answers 202 with the GET shape. 403 with a fixed text when no account is chosen — the flag is read at each request, not cached. 409 when a run is going. 400 for another `job`, an unknown key (an `account` key too: the run's account is the setting's), a wrong type, or a workspace the store does not hold.
> 5. `POST /api/agent-run/stop` stops the going run and answers 200 with the state; with no run going it answers 200 and changes nothing.
> 6. All four are behind the existing guard (token, Host, Origin) and the existing body checks. A run started by the server ends when the server stops (no child lives on). Start the run from a thread that lives until it ends (`PR_SET_PDEATHSIG` follows the thread: `memory/learnings.md`, 2026-11-16).
> 7. Deleting or ignoring the chosen account through the existing account routes turns the setting off (the GET then shows `enabled: false`); a stored id that names no confirmed Claude account counts as off and starts nothing.
> 8. The routes work in `serve`, `app` and `tray` the same way. `serve` is no longer read-only for this one feature: say so in its `--help` text.

**Out of scope**: UI (T092); MCP tools for agent runs; a run history; progress text; more than one agent run account; signing an account in; runs the app starts by itself (PRD question 38); the Python app.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16 (`docs/claude-md/tracks.md` row 13); the owner has not read it
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: US-039, FR-101, FR-103, NFR-002, NFR-008, questions 36 and 38 in `PRD.md`

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
| 1 | Guard tests: each of the four routes without the token, with a wrong Host, or with a foreign Origin is refused exactly as the other write routes are, and stores or starts nothing | intent 6, NFR-008 |
| 2 | Setting tests: default off; PUT with a confirmed Claude account and `confirmed: true` → the flag holds that id (mode 0600) and the GET shows `{id, label}`; PUT without `confirmed`, with an unknown id, a Codex account or an ignored account → 400 and no flag; PUT `{account: null}` → off, and a going run is stopped; an empty flag file (T090) reads as on with the default account | intent 1, 3 |
| 3 | Off test: with no flag, `POST /api/agent-run` → 403, the fake `claude` is never started; the flag written while the server runs → the next POST is 202 (read per request) | intent 4 |
| 4 | Body tests: wrong content type 415; not JSON, not an object, unknown key (`account` included), `job` other than `investigate`, wrong types → 400; unknown workspace → 400; an oversized body → 413 | intent 4 |
| 5 | Run test with a fake `claude`: 202, then `running: true`; the child's `CLAUDE_CONFIG_DIR` is the chosen account's (removed for the default account); a second POST → 409; after the end `running: false` and `last.outcome`; with a report saved by the fake run (through the real `save_report` code path in a temp store), `last.report_id` is that id | intent 2, 4 |
| 6 | Stop and shutdown tests: stop during a run → `last.outcome` is `stopped`, no process of the group lives on; stop with no run → 200, no change; the server stops during a run → the child's group is gone | intent 5, 6 |
| 7 | Account tests: the chosen account is deleted or ignored through the existing routes → the GET shows `enabled: false` and a POST is 403; a flag naming an id that is gone starts nothing | intent 7 |
| 8 | CLI tests: `agent-run on --account <id>` writes the id; `agent-run status` prints the account; `agent-run now` with no `--account` uses the stored one; T090's 12 tests still pass | intent 1 |
| 9 | Leak gate: the four routes are registered in `rust/parity/leak_rust.py` and the run (from `rust/`) `env -u CLAUDE_CONFIG_DIR <main checkout>/rust/parity/.venv/bin/python parity/leak_rust.py` prints `0 hits` with no `failed checks`, exit 0; a unit test proves a planted secret in the fake `claude`'s output is in no response body | intent 2, NFR-002 |
| 10 | `cd rust && cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` pass; the parity check `--check` (run from `main`) reports no new difference except the four new routes, named as Rust-only; `security-review` at Stage 4 (Risk High) | intent |

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
Test first, one vertical slice at a time (`tdd`): the account in the flag → GET → PUT setting → POST off/403 → POST run → 409 → stop → shutdown → account deleted → leak gate. The handler never waits for the run: it starts it on a blocking thread and answers. Smallest code that meets the ACs; record cuts.

## Edge Case Checklist
- Two dashboards (and the tray) on one machine: T090's lock decides, each server answers 409 for the other's run and shows `running: true`
- A run started by `agent-run now` in a terminal: the GET shows `running: true`; stop from the dashboard — decide and record: it may only stop its own server's run (the smaller change) and answers 409 for a foreign one
- The server's `last` is in memory only: after a restart it is null
- A token in the query string is accepted only by `/api/live`: never by these routes
- The 403 text names no path; it says to choose an agent run account in Settings
- A page fault could call the PUT (the guard does not prove a click): `confirmed: true` is required, and the owner accepted this risk (PRD question 36)
- The chosen account's folder is gone (`status: missing`): the GET says so; a run ends `failed`, not a hang

## Files to Change (Predicted)
`rust/crates/app/src/api.rs`, `server.rs`, `agent_run.rs`, `writes.rs` (the account delete/ignore hook only), `main.rs` (help text), `api_tests.rs`, `rust/parity/leak_rust.py`

## Files Must NOT Touch
the guard's rules in `server.rs` (add routes only), `writes.rs` body rules (reuse them), the store schema, `web/**`, `src/easy_personal/**`, any Claude config or credential file

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Questions Log

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T091.md`
- [ ] Commits on branch `T091` only; final report with `Status:`
- 2026-11-17 DEBATE should the terminal `account ignore` also turn the setting off and stop a going run in this task → follow-up T099, same fix (`forget` in `cli_cmds.rs`), no poll check (Advocate 1: fix now in `cli_cmds.rs`; Advocate 2: fix now in `cli_cmds.rs`; Advocate 3: follow-up task) ⚠ decided for you — please review (`tasks/TASK_REVIEW_T091.md` decision 5)
