# TASK_GUIDE — T058: Rust CLI commands Python has and Rust has not
**Date**: 2026-11-13
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/docs/PYTHON_RETIREMENT_PLAN.md`
6. Read the Python you copy, **read-only**: `src/easy_personal/surfaces/cli.py` (the commands `ping`, `usage`, `outliers`, `kit-findings`, `workspace`, `account`, `dashboard`: arguments, output text, exit codes, the confirmation question and `--yes`), and the tests that cover them under `tests/surfaces/`
7. Read the Rust you extend: `rust/crates/app/src/{main,api,mcp,server,writes,ingest_cmd}.rs` and `rust/crates/core/src/` (the code the API routes and MCP tools already call: usage, outliers, kit findings, workspaces, accounts), `rust/crates/app/tests/` (how a test runs the binary), `rust/parity/README.md`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use temp folders and fixtures. Account detection must run only over a temp `EASY_PERSONAL_AGENT_ROOT`

---

## Requirement

**Original request** (owner, 2026-11-13): "approve all" for `rust/docs/PYTHON_RETIREMENT_PLAN.md`.

**Restated intent**:
> `easy-personal-rs` gets the CLI commands Python's `easy-personal` has and Rust lacks, and prints what Python prints. They reuse the code that the Rust API routes and MCP tools already run. No new store logic.
> Commands: `ping`, `usage [--since] [--until]`, `outliers [--since] [--until]`, `kit-findings [--json] [--limit]`, `workspace list|create|edit|delete [--yes]`, `account list [--by]|confirm --label|ignore|add <path>|use <account> [workspaces…]`, and `dashboard` (ingest, then serve until Ctrl-C).

**Not in this task**: `connector …`, `doctor` (T060, after T050); `store-check` (T059); `schedule install/remove` (exists); any change that needs `claude` or the keychain (T048's rule stays: refused with the same fixed text until T050); any change to Python files.

**Requirement Refs**: FR-025, FR-040..FR-042 (usage, outliers, workspaces, accounts), FR-080, NFR-002; RS-001..RS-003 in `rust/docs/SPEC.md`. (Check the exact FR numbers in `PRD.md`; fix this line if one is wrong.)

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the owner ("approve all", 2026-11-13)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T041, T048 (accounts and workspaces store logic), T046..T049 (merged)
**Entry point**: the `Command` enum in `rust/crates/app/src/main.rs`

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Output equals Python.** For each command and for a fixture store, stdout and the exit code equal Python's (same lines, same order, same words). Generator `rust/parity/gen_cli.py` runs Python's CLI over fixture stores and home folders and writes `rust/parity/golden-cli/`; the Rust test compares. Cases: every command with data, empty store, bad dates, an unknown workspace or account, `--json`, `--limit` out of range | FR-025, FR-040..042 |
| 2 | **Writes follow the API rules.** `workspace create/edit/delete` and `account confirm/ignore/add/use` write the store through the same code as the T048 routes (same validation, same fixed error texts, same lock). A change that needs `claude` is refused with the T048 text, exit as Python's. `delete` asks for confirmation unless `--yes`; stdin that is not a terminal does not hang (same answer as Python) | FR-040..042 |
| 3 | **Account detection stays in the temp root.** `account list` and `account add` look only in `EASY_PERSONAL_AGENT_ROOT` (tests set it to a temp folder). No test reads a real home folder | NFR-002 |
| 4 | **`dashboard`.** `dashboard` ingests (as `serve` does since T051, with the same one-line progress and the same fixed failure line) and serves until Ctrl-C; it prints the `dashboard: http://127.0.0.1:<port>/#<token>` line as `serve` does. It may be an alias of `serve`; `--no-ingest` is kept | FR-012 |
| 5 | **No leak.** `parity/leak_rust.py` covers every new command over planted canaries in rows, paths and labels: 0 hits. Errors never echo a value the user typed beyond what Python echoes (say which in "decided for you") | NFR-002 |
| 6 | **Regression.** Three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml` and `Cargo.lock` unchanged, existing tests and goldens pass | RS-003 |
| 7 | **Docs.** `rust/README.md`: the commands, the differences, the "decided for you" list, the corrected command list and "not served" text. `rust/PROJECT_*.md` and `memory/**` are not edited (Supervisor) | — |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T058.md`.
> **Locked oracle**: `golden-cli/` made once by Python, plus the Python CLI and tests (read-only).

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

**Reviewer note**: Medium risk (store writes from the CLI, account folders). Stage 4 has a code review and a security review.

---

## Approach

**Vital slice**: the read commands first (`ping`, `usage`, `outliers`, `kit-findings`, `workspace list`, `account list`), then the writes, then `dashboard`. Reuse the API and MCP code; do not copy logic.
**Cut list**: no `connector`, no `doctor`, no new flag Python lacks except `--no-ingest` on `dashboard`.
**Order**: red test first for each command.

---

## Edge Case Checklist

- [ ] A date with no zone, a date after `--until`, a bad date: Python's message and exit code
- [ ] `workspace delete` of a workspace with connectors (they move to `orphan`), of an unknown name, with `--yes` and without a terminal
- [ ] `account use` with no names clears the picks; an unknown workspace name
- [ ] `account add` of a path that does not exist, is a file, or holds a folder outside the temp root
- [ ] The store is missing or newer than the code: the same fixed text as the API (never migrates)
- [ ] Time uses the injected clock; three `TZ` values

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/main.rs` (small), `rust/crates/app/src/cli_cmds.rs` (new) | the commands |
| `rust/crates/app/tests/cli_cmds.rs`, `rust/crates/app/tests/cli_parity.rs` (new) | process and oracle tests |
| `rust/parity/gen_cli.py`, `rust/parity/golden-cli/` (new); `rust/parity/leak_rust.py` | oracle, leak registry |
| `rust/README.md` | docs |
| `tasks/TASK_REVIEW_T058.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| existing `rust/parity/golden*/**`, `store-rich/**`, `fixtures-rich/**` | the oracle |
| `rust/Cargo.toml`, `rust/Cargo.lock`, `crates/*/Cargo.toml` | no new dependency without a stop-and-report |
| `rust/crates/app/src/{store_health,live_feed,mcp_http}.rs`, `rust/crates/core/src/ingest/context.rs` | other tasks own them |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

T059 works in parallel on `main.rs`, `api.rs`, `server.rs`, `README.md` and `leak_rust.py`. Keep your edits to those files small, so the merge stays easy.

---

## Test Plan

Red test per criterion, then green; oracle; leak gate; three `TZ` runs (`unset`, `Pacific/Kiritimati`, `Pacific/Pago_Pago`: `cargo test --workspace --no-fail-fast`).

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T058.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts, three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written
- [ ] Supervisor notified: task ready for Stage 4 review
