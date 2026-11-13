# TASK_GUIDE — T060: Rust `connector` and `doctor` commands over T050's code
**Date**: 2026-11-13
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P3
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/docs/PYTHON_RETIREMENT_PLAN.md`, `tasks/TASK_GUIDE_T050.md` and `tasks/TASK_REVIEW_T050.md` (what T050 built: `keychain.rs`, `claude_cli.rs`, the connector routes, sync apply, legacy, the launcher)
6. Read the Python you copy, **read-only**: the `connector` (`list`, `add`, `remove`, `sync`, `move`, `alias`, `adopt-legacy`) and `doctor` parts of `src/easy_personal/surfaces/cli.py`, `src/easy_personal/connectors/writer.py` (`doctor`), and the tests `tests/connectors/test_writer.py`, `test_sync_cli.py`, `test_workspace_ops_cli.py`, `test_legacy.py`, `tests/leak/`
7. Read the Rust you extend: `rust/crates/app/src/{main,cli_cmds}.rs` (T058's command layout, if merged), the T050 modules, `rust/crates/app/tests/`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder. No test touches a real keychain item, a real `claude` binary or a real account folder: fakes only, as T050

---

## Requirement

**Original request** (owner, 2026-11-13): "approve all" and "auto approve to finish" for `rust/docs/PYTHON_RETIREMENT_PLAN.md`.

**Restated intent**:
> `easy-personal-rs connector list|add|remove|sync|move|alias|adopt-legacy` and `easy-personal-rs doctor` print what Python's commands print and exit the same way. They call the code T050 built (keychain, `claude_cli`, sync apply, legacy). No new store, keychain or subprocess logic. Secrets are read as Python reads them (a prompt, a flag, or the environment: copy exactly) and never appear in argv of any child, in output, in a log or in an error.

**Out of scope**: any change to Python files; a new connector catalog entry; macOS and Windows backends; `store-check`, `usage` and the other T058/T059 commands.

**Requirement Refs**: FR-017..FR-019, FR-025, NFR-002, NFR-006 (`PRD.md`); RS-001..RS-003 in `rust/docs/SPEC.md`. Check the numbers in `PRD.md`; fix this line if one is wrong.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the owner ("approve all", "auto approve to finish", 2026-11-13)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T050, T058 (merged on `main` before this task starts)
**Entry point**: the `Command` enum in `rust/crates/app/src/main.rs`

---

## Dependency rule

No crate, feature or lockfile change. If a command needs one, stop that command, finish the others and report.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Output equals Python.** For every `connector` action and `doctor`, stdout, stderr text that Python prints for the user, and the exit code equal Python's, over the same cases as T050's oracle plus the CLI-only cases (missing arguments, an unknown workspace, `--yes` and no terminal, `--account` filters, `--default` for alias). Generator `rust/parity/gen_connector_cli.py` runs Python's CLI with the fake keychain and the fake `claude` and writes `rust/parity/golden-connector-cli/`; the Rust test compares | FR-017..019, FR-025 |
| 2 | **Secrets.** Secret input takes only the routes Python takes. A secret is never in the argv of the `claude` or provider child, in stdout, stderr, any log or any error text; a wrong value gives Python's fixed message | NFR-002, NFR-006 |
| 3 | **`doctor`.** Checks keychain, `claude` on `PATH` and each connector entry as Python's `doctor` does, with the same line texts and the same skip rules (adopted ids), and exits as Python does. Tests use the fake keychain and the fake `claude` | FR-019 |
| 4 | **Confirmation.** Commands that ask for confirmation behave as Python's with and without `--yes` and with stdin that is not a terminal (no hang) | FR-019 |
| 5 | **Leak.** `parity/leak_rust.py` covers every new command over planted canaries in env, headers, keychain secrets, `claude` stderr and paths: 0 hits over stdout, stderr, the store, the recorded fake `claude` argv | NFR-002 |
| 6 | **Regression.** Three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml` and `Cargo.lock` unchanged, existing tests and goldens pass | RS-003 |
| 7 | **Docs.** `rust/README.md`: the commands, differences, the "decided for you" list, "Not served yet" corrected. `rust/PROJECT_*.md` and `memory/**` are not edited (Supervisor) | — |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T060.md`.
> **Locked oracle**: `golden-connector-cli/` made once by Python, plus the Python CLI and tests (read-only).

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

**Reviewer note**: High risk (secrets, subprocesses). Stage 4 has a code review and an independent security review that tries to read a secret from argv, output and logs.

---

## Approach

**Vital slice**: `connector list`, `add`, `remove`, then `sync`, `move`, `alias`, `adopt-legacy`, then `doctor`. Call T050's functions; do not copy logic.
**Cut list**: no new flag Python lacks, no new catalog entry.
**Order**: red test first for each command.

---

## Edge Case Checklist

- [ ] A connector name with `/`, `..`, upper case, 65 characters, empty: refused before any keychain or subprocess call
- [ ] The keychain is absent or locked; `claude` is missing or slow: fixed texts, nothing written
- [ ] `connector remove` of a foreign entry, a non-catalog name, an absent name (HUN-72 rule)
- [ ] `connector sync` with no confirmed account, with one failing account
- [ ] `doctor` with no connectors, with an adopted id, with a broken entry
- [ ] Time uses the injected clock; three `TZ` values

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/connector_cmd.rs`, `doctor_cmd.rs` (new), `main.rs` (small) | the commands |
| `rust/crates/app/tests/{connector_cli,doctor_cli,connector_cli_parity}.rs` (new) | tests, oracle |
| `rust/parity/gen_connector_cli.py`, `rust/parity/golden-connector-cli/` (new); `rust/parity/leak_rust.py` | oracle, leak registry |
| `rust/README.md` | docs |
| `tasks/TASK_REVIEW_T060.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| existing `rust/parity/golden*/**`, `store-rich/**`, `fixtures-rich/**` | the oracle |
| `rust/Cargo.toml`, `rust/Cargo.lock`, `crates/*/Cargo.toml` | no dependency change |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*`, any real keychain item | the owner's real data |

---

## Test Plan

Red test per criterion, then green; oracle; leak gate; three `TZ` runs (`unset`, `Pacific/Kiritimati`, `Pacific/Pago_Pago`: `cargo test --workspace --no-fail-fast`).

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T060.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts, three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written
- [ ] Supervisor notified: task ready for Stage 4 review
