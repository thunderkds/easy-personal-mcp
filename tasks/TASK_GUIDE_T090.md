# TASK_GUIDE — T090: Agent run 1: the setting, the runner and the `agent-run on|off|status|now` command
**Date**: 2026-11-16
**Complexity Level**: C3
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
5. Read `BRAINSTORMING_LOG_AGENTRUN.md` (Option A, Edge Case Checklist, Auto decisions) and the PRD rows US-039, FR-101, FR-102, FR-075, FR-076, FR-077, NFR-002, NFR-004, questions 35-37
6. Read `rust/crates/app/src/claude_cli.rs` (how `claude` is found and run, the process group, the kill), `rust/crates/app/src/mcp.rs` (`investigate_text`, `INVESTIGATE_PROMPT`, `SAVE_REPORT_STEP`, the tool names), `rust/crates/app/src/main.rs` (commands), `rust/crates/core/src/store.rs` (`store_path`)
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures, a fake `claude` and temp folders)
- Do not run the real `claude -p` in a test. The one real run is AC 9, done once by the Supervisor or the owner, never in CI or a loop

---

## Requirement

**Original request** (owner, 2026-11-16): "I want each time the app want using agent to analyze or do a job, let use claude subscribetion, not need for the users type the message in claude and call through MCP, application will do that all for user"; then "yes, plan the run model, opt-in and off by default [admin]".

**Restated intent** (PRD FR-101, FR-102):
> 1. A setting, off by default, held as one flag file beside the store (owner-only mode 0600, in a 0700 folder; no schema change). `easy-personal-rs agent-run on` prints the notice "Agent runs use the Claude subscription login of Claude Code on this machine and count against your plan limits." and writes the flag; `off` removes it and stops a going run; `status` prints `on` or `off`, whether `claude` is found, and whether a run is going.
> 2. A runner for the one fixed job `investigate`: it builds the prompt from the same fixed text as the MCP prompt (one source, not a copy) with optional `workspace` and `account`, and starts `claude -p` with an argument list (never a shell).
> 3. The child gets: `--mcp-config` with one server entry that starts this same binary (`std::env::current_exe`) with `mcp`, `--strict-mcp-config`, an allow-list of exactly `mcp__<server>__warnings`, `improvement_plan`, `usage_breakdown`, `session_outliers`, `save_report`, no built-in tool, a permission mode that refuses (never asks for) any other tool, a working folder the app owns and keeps empty, stdin closed, its own process group.
> 4. The child's environment: this process's, minus every variable that changes billing or login (at least `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_PROFILE`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_FEDERATION_RULE_ID`, `ANTHROPIC_ORGANIZATION_ID`, `ANTHROPIC_FOUNDRY_API_KEY`, `ANTHROPIC_FOUNDRY_AUTH_TOKEN`, `ANTHROPIC_AWS_API_KEY`, `AWS_BEARER_TOKEN_BEDROCK`, `CLAUDE_CODE_OAUTH_TOKEN`, `CLAUDE_CODE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX`, `CLAUDE_CODE_USE_FOUNDRY`), with `CLAUDE_CONFIG_DIR` set or removed for the account exactly as `claude_cli.rs` does. Never `--bare`, never `--dangerously-skip-permissions`.
> 5. One run at a time on the machine: a lock file beside the flag, taken without blocking; a stale lock (its process is gone) is taken over. Limit 10 minutes, then the whole process group is killed.
> 6. stdout and stderr are read and dropped. Nothing of them is stored, logged, printed or returned. The outcome is: start, end, one of `ok` / `failed` / `timed out` / `stopped` / `claude not found` / `off`, and the id of the newest report saved after the start for that scope (read from the store), or none.
> 7. `easy-personal-rs agent-run now [--workspace W] [--account A]`: refuses with exit 1 and one line when the setting is off; else runs the job in the foreground and prints the outcome line. Exit 0 only for `ok`.

**Out of scope**: any HTTP route (T091); any UI (T092); a schedule; other jobs; a free-text prompt; a run history in the store; reading any Claude credential; the Python app; macOS and Windows.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16 (`docs/claude-md/tracks.md` row 13); the owner has not read it
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: US-039, FR-101, FR-102, NFR-002, NFR-004, questions 35-37 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: none (T050 `claude_cli.rs` and T047 `mcp.rs` are on `main`)
**Entry point**: new `rust/crates/app/src/agent_run.rs`; `main.rs` command `AgentRun`

## Dependency rule
No new crate. If the lock needs one, stop and report (`Status: blocked`).

---

## Step 0 — Spike (before any code; paste the result in `tasks/TASK_REVIEW_T090.md`)

With the installed `claude` (`claude --version`, `claude --help`), and **without** starting a model run, write down the exact flags for: the MCP config (file or inline JSON), strict MCP, the allow-list, "no built-in tool" (`--tools`), the permission mode that refuses instead of asking in `-p`, and `--setting-sources` (does a value shut out user hooks and user MCP servers and still keep the `/login` subscription?). Record the Claude Code version. A flag that the help does not list is not used. If no listed flag gives "refuse, never ask", stop and report (`Status: blocked` + `Question:`).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Setting tests in a temp folder: default `off`; `on` writes the flag (mode 0600) and prints the notice; `off` removes it; `status` prints the three facts; `on` twice and `off` twice are no errors | intent 1 |
| 2 | Argument test with a fake `claude` that logs its argv, working folder and environment: one argument per value, no shell; the prompt is one argument and equals the MCP prompt text for the same scope (one shared function); the MCP config names only this binary + `mcp`; the allow-list holds exactly the five tools; `--bare` and `--dangerously-skip-permissions` are absent; the working folder is the app's own empty folder | intent 2, 3 |
| 3 | Environment test: every variable of intent 4 is set in the parent and absent in the fake child; `CLAUDE_CONFIG_DIR` is set for a non-default account and removed for the default one | intent 4 |
| 4 | Scope test: a workspace or account value with `'`, `"`, a newline, a leading `-` and `$(x)` reaches the child unchanged inside the prompt argument; nothing else in argv changes | intent 2 |
| 5 | One-at-a-time test: a second start while a fake `claude` sleeps answers "a run is going" and starts no process; after the first ends, a start works; a lock whose process is gone is taken over | intent 5 |
| 6 | Limit and stop tests (short limit injected): a fake `claude` that starts a child and sleeps is killed with its child (`timed out`); `off` during a run ends it (`stopped`) and no process of the group lives on | intent 5, 1 |
| 7 | Leak test: a fake `claude` prints a planted secret and message text on stdout and stderr; the secret is in no printed line, no log line, no file under the temp data folder and no store row | intent 6, NFR-002 |
| 8 | Outcome tests: exit 0 + a report saved after the start → `ok` with that id; exit 0 + no report → `ok`, no id; non-zero exit → `failed`; no `claude` on `PATH` → `claude not found`; setting off → `agent-run now` exits 1, starts nothing | intent 6, 7 |
| 9 | Real run, once (HITL: it spends the owner's plan limits; the Supervisor or the owner runs it, the output pasted): `agent-run on`, `agent-run now` on this machine ends `ok` with a report id, and that report shows in Saved reports; `agent-run off` after it | intent 7 |
| 10 | `cd rust && cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` pass; the existing 60 s `SystemClaude` tests are unchanged and pass; `security-review` at Stage 4 (Risk High) | intent |

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
> BEFORE/AFTER/DELTA/WITNESS go in `tasks/TASK_REVIEW_T090.md`.

## UI / Design Acceptance Criteria
N/A — backend only.

---

## Approach
Step 0 first. Then test first, one vertical slice at a time (`tdd`): setting → argv → environment → lock → limit/stop → outcome → CLI. Smallest code that meets the ACs; record cuts. Share `which` and the group kill with `claude_cli.rs` instead of copying them; do not change the 60 s path's behaviour.

## Edge Case Checklist
- `ANTHROPIC_API_KEY` and the others set in the app's environment: the child never sees them
- `claude` not on `PATH`, or not logged in (non-zero exit): outcome says so, no hang
- The setting is turned off while a run is going (another process): the run is stopped
- Two starts at once (two processes): one runs
- The app is killed during a run: no stale lock blocks for ever; note in the review file what happens to the child
- A tool outside the allow-list: refused, never a wait for an answer
- The model never calls `save_report`, or calls it twice
- Plan limit reached: `failed`, no retry
- A full stdout pipe never blocks the child (read and drop to the end)

## Files to Change (Predicted)
`rust/crates/app/src/agent_run.rs` (new), `claude_cli.rs` (share helpers), `mcp.rs` (expose the prompt function), `main.rs`, tests

## Files Must NOT Touch
`rust/crates/core/src/schema_v11.sql`, the store schema, `rust/crates/core/src/reports.rs`, `api.rs`, `server.rs`, `web/**`, `src/easy_personal/**`, any Claude config or credential file

## Test Plan
The ACs above, each as an automated test (AC 9 is the one manual run), plus the verification command's real output.

## Questions Log
- 2026-11-16 DEBATE where does the owner turn agent runs on: a dashboard switch, the CLI only, or both? → the CLI only (Advocate 1: CLI only, dashboard read-only plus Run; Advocate 2: CLI only, an off-only browser switch may come later; Advocate 3: CLI only, `agent-run on|off|status`) ⚠ decided for you — please review
- 2026-11-16 DEBATE should an agent run keep its Claude Code session on disk? → keep, decided by the owner 2026-11-16 ("keep the run's claude session") after the Advocates disagreed (Advocate 1: keep; Advocate 2: `--no-session-persistence`; Advocate 3: keep)

## Completion Checklist
- [ ] Step 0 result and every AC with pasted output in `tasks/TASK_REVIEW_T090.md`
- [ ] Commits on branch `T090` only; final report with `Status:`
