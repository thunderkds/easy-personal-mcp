# TASK_GUIDE — T050: Rust keychain, launcher, connector writes, sync apply, legacy (`claude` subprocesses)
**Date**: 2026-11-13
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P3
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the T050 decisions in `memory/decisions.md` ("Rust P3 planned", "Rust P2 ingest") and the 2026-11-13 entry on the merged tasks
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/docs/T050_CRATE_OPTIONS.md` (the owner gate: option A chosen) and `rust/docs/PYTHON_RETIREMENT_PLAN.md`
6. Read the Python you copy, **read-only**: `src/easy_personal/launch.py`, `src/easy_personal/connectors/{keychain,writer,sync,legacy,catalog,suggestions}.py`, the connector, account and workspace parts of `src/easy_personal/surfaces/{api,cli}.py`, and every test under `tests/connectors/` plus the connector parts of `tests/surfaces/` and `tests/leak/` (and any fake `claude` they use)
7. Read the Rust you extend: `rust/crates/app/src/{main,api,writes,server}.rs`, `rust/crates/core/src/` (the connector catalog and status code from T046, the workspace and account writes from T048, `ingest_lock`), `rust/crates/app/tests/{connectors,writes}.rs`, `rust/parity/README.md`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. **No test touches a real keychain item, a real `claude` binary or a real account folder.** Tests use temp folders, a fake `claude` script placed first on a temp `PATH`, and a fake keychain behind a trait

---

## Requirement

**Original request** (owner, 2026-11-13): "approve all" for `rust/docs/PYTHON_RETIREMENT_PLAN.md`; on the crate gate: "as you choose" (the Supervisor chose option A in `rust/docs/T050_CRATE_OPTIONS.md`).

**Restated intent**:
> The Rust app does what Python's connector code does: it stores connector secrets in the OS keychain (and refuses when none exists), ships a launcher program `easy-personal-launch` that Claude Code can run, writes and removes connector entries through `claude mcp`, applies the sync plan per account, and adopts legacy connectors. It serves the Python write routes. Secrets are never in a file, a log, argv, an HTTP body that is echoed, or any output.

**Out of scope**: the `connector …` and `doctor` CLI commands (T060, next); any change to Python files; macOS and Windows keychain backends (Linux Secret Service only; the code says so when run elsewhere); any non-loopback network access.

**Requirement Refs**: FR-017, FR-018, FR-019, FR-025, NFR-002, NFR-006 (`PRD.md`); RS-001..RS-003 in `rust/docs/SPEC.md`. Check the exact numbers in `PRD.md`; fix this line if one is wrong.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the owner ("approve all", "as you choose", 2026-11-13)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T046, T048 (merged), T051 (merged)
**Entry points**: `POST /api/connectors`, `POST /api/connectors/{id}/alias`, `POST /api/connectors/{id}/move`, `DELETE /api/connectors/{name}`, `POST /api/connectors/legacy`, `POST /api/sync`, and the `easy-personal-launch` binary. (Read Python's `api.py` for the exact methods and bodies.)

---

## Dependency rule (read before any code)

The owner gate is passed for **exactly one change**: in `rust/crates/app/Cargo.toml` add `keyring = { version = "=3.6.3", default-features = false, features = ["sync-secret-service", "crypto-rust"] }` (check the feature names against the crate; if they differ, stop and report). Nothing else.

Step 0 (stop rule): before any code, record in `tasks/TASK_REVIEW_T050.md` (Demonstration BEFORE) `cargo tree` and the package list before the change; make the change; record the new package list, the licence of each new package (`cargo metadata`), and `git diff --stat rust/Cargo.lock`. **Stop and report to the Supervisor if**: the new package count is not about 75 (60 to 90 is fine), any new package has a licence that is not MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib or Unicode (or an OR of these), the build needs a system library that is not installed, or any other crate is needed. Do not continue until the Supervisor answers. Any other crate, feature or lockfile change is the same stop. The `claude` calls use `std::process::Command`; no crate.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Step 0 recorded** (dependency rule above) before the first code file | — |
| 2 | **Keychain module.** One module `keychain.rs` behind a trait (`Keychain`) with the real `keyring` implementation and a fake for tests. Items: service `easy-personal/<name>`, user `connector`, secret = the JSON `{"command":[...],"env":{...},"headers":{...}}` exactly as Python writes it. Name rule `[a-z0-9][a-z0-9_-]{0,63}`. No usable backend: refuse with Python's fixed text ("no usable OS keychain (Secret Service); unlock or install one, e.g. gnome-keyring. Secrets are never stored in plaintext"), write nothing. Removing an absent item is not an error | FR-017, NFR-002 |
| 3 | **Launcher.** A binary `easy-personal-launch` (second `[[bin]]` in the app crate, no new crate) runs `easy-personal-launch exec <name>`: reads the item, starts the provider command with `env` set from the item (never argv), forwards stdin/stdout/stderr and the exit code, passes `headers` as Python does. Messages never contain a secret value. A missing item, a malformed item or an invalid name gives Python's fixed text and a non-zero exit. Test with a fake keychain and a fake provider script | FR-018, NFR-006 |
| 4 | **`claude` subprocess code.** One module `claude_cli.rs`: finds `claude` on `PATH` (not found: Python's text, a fixed non-secret message), runs `claude mcp get/add/remove` with the same arguments and scope as Python's `writer.py`, a timeout, a captured and size-capped output that is never printed raw, an environment built as Python builds it (read `_claude`). Tests use a fake `claude` script on a temp `PATH` that records its argv and environment to a file | FR-019 |
| 5 | **Connector writes.** The routes in "Entry points" behave as Python's: same bodies, same validation, same status codes, same fixed texts, same ownership rule (a foreign entry or a non-catalog name is refused; `remove` follows HUN-72). A write that needs `claude` or the keychain and cannot get it is refused with the fixed text and writes nothing. The store side (T048) and the keychain and `claude` side stay consistent: a failure part-way rolls back or reports as Python does. The T048 refusal "a change needing `claude` is refused until T050" is replaced by the real action | FR-017..019 |
| 6 | **Sync apply.** `POST /api/sync` computes the per-account plan (T048 code already shows the plan) and applies it through `claude_cli`, step by step, as Python's `sync.py`: the same `record`, `forget`, `refused` steps, the same summary shape, the same ordering, one account failing does not stop the others. `HOME`-like paths for an account come from the account row, never from the request | FR-019, FR-025 |
| 7 | **Legacy.** `POST /api/connectors/legacy` and the status of pre-workspace connectors match Python's `legacy.py` (adopt into `orphan`, doctor-style skip of adopted ids) | FR-019 |
| 8 | **Oracle.** A generator `rust/parity/gen_connectors.py` drives Python's connector code with a fake keychain and the same fake `claude` script over scripted cases, and writes `rust/parity/golden-connectors/`: add, add refused (no backend), add with env and headers, alias and `--default`, move, remove, remove foreign, remove absent, sync plan with each step kind, sync with a failing `claude`, legacy adopt. The Rust test replays the same cases and compares: the HTTP answers, the keychain fake's contents, the recorded `claude` argv list, and the store rows | FR-017..019 |
| 9 | **Cross-check with the real keychain (manual, `#[ignore]`).** An ignored test writes an item with Rust and reads it with Python's `keyring`, and the reverse, in a throwaway service name `easy-personal/t050-crosscheck-<random>`, then deletes it. The Supervisor or owner runs it once by hand; the agent does not (it needs a session bus). State in the report that it was not run | FR-017 |
| 10 | **Leak.** `parity/leak_rust.py` covers every new route, the launcher and the `claude_cli` path over planted canaries in env values, header values, keychain secrets, `claude` stderr and account paths: 0 hits over stdout, stderr, logs, the store, `.dump`, every response body, the recorded fake `claude` argv (secrets must never be in argv) and the process list of the provider child (secrets only in its environment) | NFR-002, NFR-006 |
| 11 | **Security shape.** Same guard on every new route (token header, Host, Origin); a body-size cap; the connector name check before any keychain or subprocess call; no shell (`Command` with an argument list, never `sh -c`); the `claude` and provider processes are killed on timeout; the launcher refuses a name that is not in the keychain (it never executes an arbitrary command from the request) | NFR-002, NFR-006, NFR-008 |
| 12 | **Regression.** Three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, existing tests and goldens pass (the connector write routes that were 405 and the "needs `claude`" refusals in existing tests are updated by a few lines only, listed in the report) | RS-003 |
| 13 | **Docs.** `rust/README.md`: the keychain rules, the launcher, the new routes, the dependency step, the "decided for you" list, the corrected "Not served yet" lists. `rust/PROJECT_*.md` and `memory/**` are not edited (Supervisor) | — |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T050.md`.
> **Locked oracle**: `golden-connectors/` made once by Python, plus the Python code and tests (read-only).

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

**Reviewer note**: High risk (secrets, subprocesses, a new dependency tree, writes to the user's Claude config through `claude`). Stage 4 has a code review and an independent security review that tries to read a secret from argv, logs, errors and the process list, and to make `claude` or the provider run something the request chose.

---

## Approach

**Vital slice**: Step 0; keychain trait and fake; launcher; `claude_cli`; `POST /api/connectors` and `DELETE`; then alias, move, sync, legacy; then the oracle and leak gate. Write the oracle generator early: it decides every message.
**Cut list**: no macOS or Windows backend, no retry, no connector CLI (T060), no new catalog entry, no change to the T046 read side.
**Order**: red test first for each criterion.

---

## Edge Case Checklist

- [ ] A connector name with `/`, `..`, upper case, 65 characters, empty, a NUL: refused before any keychain or subprocess call
- [ ] A provider command that is empty, a path, a list with a non-string: Python's refusal
- [ ] The keychain is locked or absent: the fixed text, nothing written to the store or to `claude`
- [ ] `claude` missing, slow (timeout), exiting non-zero, printing a secret on stderr: a fixed non-secret message; the raw text is never shown
- [ ] A write that succeeds in the keychain and fails in `claude`: the same rollback or report as Python
- [ ] Two writes at the same instant: the same lock rule as the T048 writes
- [ ] The account row points outside its root, or to a symlink: refused as Python does
- [ ] The launcher gets a closed stdin, a huge env value, a provider that never exits: bounded and killed on a signal
- [ ] Time uses the injected clock; three `TZ` values

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/Cargo.toml`, `rust/Cargo.lock` | the one allowed dependency (Step 0) and the `easy-personal-launch` `[[bin]]` |
| `rust/crates/app/src/{keychain,claude_cli,connector_writes,sync_apply,legacy,launch}.rs` (new), `main.rs`, `api.rs`, `writes.rs`, `server.rs` (small edits) | the code and routes |
| `rust/crates/app/tests/{connector_writes,launcher,claude_cli,sync_apply,connector_parity}.rs` (new), existing connector tests (a few lines) | tests |
| `rust/parity/gen_connectors.py`, `rust/parity/golden-connectors/` (new); `rust/parity/leak_rust.py` | oracle, leak registry |
| `rust/README.md` | docs |
| `tasks/TASK_REVIEW_T050.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| existing `rust/parity/golden*/**`, `store-rich/**`, `fixtures-rich/**` | the oracle |
| `rust/Cargo.toml` and any other `Cargo.toml`, any other lockfile line | only the one allowed change |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*`, any real keychain item | the owner's real data |

T058 and T059 run in parallel and touch `main.rs`, `api.rs`, `server.rs`, `README.md` and `leak_rust.py`. Keep your edits to those files small and put new code in new modules.

---

## Test Plan

Red test per criterion, then green; the oracle; the leak gate; three `TZ` runs (`unset`, `Pacific/Kiritimati`, `Pacific/Pago_Pago`: `cargo test --workspace --no-fail-fast`).

---

## Completion Checklist

- [ ] Step 0 and BEFORE filled in `tasks/TASK_REVIEW_T050.md` before the first code file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts, three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written
- [ ] Supervisor notified: task ready for Stage 4 review
