# TASK_GUIDE — T078: Keychain leftovers: `#[path]` double compile of `keychain.rs`, `claude` call inside a write transaction, D-Bus timeout skip recorded
**Date**: 2026-11-15
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P3
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `tasks/TASK_REVIEW_T050.md` row "Review scope bounded…" (the deferred list), `tasks/TASK_GUIDE_T066.md` (f) and its cut list, `memory/decisions.md` lines 250-310
6. Read `rust/crates/app/src/launch.rs`, `keychain.rs`, `main.rs` (module list), `connector_writes.rs`, `sync_apply.rs`, `writes.rs`, `legacy.rs`, `rust/crates/app/Cargo.toml`, `rust/crates/core/src/workspaces.rs` (`write_session`, `session`)
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)
- Never touch the real Secret Service: do NOT run `real_keychain_items_cross_check` or any `#[ignore]` keychain test (the owner runs it)

---

## Requirement

**Original request** (owner, 2026-11-15, `[admin]`): "you will be another supervisor will support another session that running the tasks, pick the task parallel and make sure you don't conflict with another session". Task picked with the other supervisor (session `rust-57`): the T050 Stage 4 deferred items not closed by T066.

**Already closed, do not redo**: keychain delete before row delete (T066 (f), `connector_writes.rs` `remove_connector`); `kill()` of the whole process group (T066, `claude_cli.rs:86` `process_group(0)`).

**Restated intent**:
> 1. **`#[path]` double compile.** `launch.rs:12` pulls `keychain.rs` in with `#[path = "keychain.rs"] mod keychain;`, and `main.rs:15` declares `mod keychain;` too, so the file compiles twice, once per binary, and the launcher needs `#[allow(dead_code)]`. Give `keychain.rs` one owner so it compiles once, with no `#[allow(dead_code)]` over the whole module. Preferred: a `[lib]` target in `rust/crates/app` (or the smallest equivalent) that both bins use. Hard limit: the launcher binary must still link no server and no store (check `cargo tree -p easy-personal-rs -e normal` and the launcher's own imports); if the only way to keep that is a new crate, a new workspace member is allowed (no new third-party crate). If the change costs more than it saves (state it with numbers: build time, binary size, lines), stop, report `Status: blocked` with the numbers, and change nothing.
> 2. **`claude` call inside a write transaction.** Find every place where a `claude` subprocess (`Claude` trait, `claude_cli.rs`) runs while a SQLite transaction is open (`unchecked_transaction`, `BEGIN`, the transaction of `write_session` if it holds one). Measure first: a test that proves whether a reader on a second connection is blocked while a slow fake `claude` runs. If no `claude` call runs inside an open transaction, record that with the test and change no code. If one does, move the `claude` call out of the transaction with the smallest change that keeps T066's guarantee "never a keychain item without its row, never a row change committed after a failed `claude` step". The process-wide `WRITE_LOCK` (`writes.rs`) stays held across the call: one write at a time is FR-038 and is not in scope.
> 3. **D-Bus call timeout under the write lock**: skip. Record the decision (below) in `tasks/TASK_REVIEW_T078.md`; no code.
> 4. **Unused `dbus` entries in `Cargo.lock`**: `dbus` and `libdbus-sys` are used (`tauri -> tauri-runtime-wry -> tao`, window feature; `cargo tree -i dbus --all-features`). Check `dbus-secret-service` the same way (`cargo tree -i dbus-secret-service --workspace --all-features --target all`; network may be needed, else read `Cargo.lock` `dependencies` lists). Remove a lock entry only when no target and no feature reaches it AND `cargo` itself drops it (`cargo update -p <name> --precise …` / `cargo check` rewrites the lock); never hand-edit `Cargo.lock`. If `cargo` keeps it, record why and change nothing.

**Decision ⚠ made for the owner (`[admin]` mode), D-Bus timeout**: skip. Reason: a timeout changes behaviour (a slow but working keychain unlock prompt would fail the write); the Secret Service call already fails fast when no session bus exists (T050 negative test "no session bus refuses cleanly"); the recommendation of record since T050 (`memory/decisions.md`, `tasks/MINIMAL_LOG.md`) is skip. One revert re-opens it.

**Out of scope**: AC9 real-keyring interop test, the launcher 200 ms measurement on the real keychain, the debug fake keychain file mode (closed by T065), any change to `WRITE_LOCK`, any Python file, `web/**`, the Watch files (`alerts.rs`, sampler, tray: other session's T072-T077).

### Requirement Fidelity Gate
- [x] Restated intent agreed with the other supervisor and in the owner's `[admin]` scope
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: T050 Stage 4 deferred list (`tasks/TASK_REVIEW_T050.md`), FR-018, NFR-006, FR-038 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: none (T066 merged)
**Entry point**: `rust/crates/app/src/launch.rs`, `main.rs`, `rust/crates/app/Cargo.toml`, `connector_writes.rs`, `sync_apply.rs`

## Dependency rule
No new third-party crate. A new workspace member made of existing code is allowed only under intent 1's hard limit.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `keychain.rs` compiles once: no `#[path = "keychain.rs"]`, no module-wide `#[allow(dead_code)]`; `cargo build --workspace` and `cargo build --features window` pass; the launcher's dependency set gains no server/store crate (before/after `cargo tree` pasted) — or `Status: blocked` with numbers and no change | intent 1 |
| 2 | Launcher tests (`rust/crates/app/tests/launcher.rs`) and connector tests pass unchanged | intent 1 |
| 3 | A test with a slow fake `claude` proves whether a second connection can read the store while `claude` runs during connector add/remove/sync apply; result pasted | intent 2 |
| 4 | If a `claude` call ran inside a transaction: it no longer does (AC3's test now passes), and T066's rollback tests still pass | intent 2 |
| 5 | `TASK_REVIEW_T078.md` records the D-Bus skip and the `Cargo.lock` finding with the `cargo tree` output | intents 3, 4 |
| 6 | `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` pass; leak script 0 hits | all |
| 7 | `security-review` at Stage 4 (Risk Medium) | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test --workspace --no-fail-fast 2>&1 | grep -E '^test result|FAILED|panicked' | tail -20 \
  && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check \
  && cargo build --features window \
  && env -u CLAUDE_CONFIG_DIR ../../../easy-personal-mcp/rust/parity/.venv/bin/python parity/leak_rust.py
```

### Evidence (filled by reviewer at Stage 4/5)
> Moved to `tasks/TASK_REVIEW_T078.md`.

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T078.md`. BEFORE: the `#[path]` line and `cargo tree` of the launcher, captured before the first code commit.

## UI / Design Acceptance Criteria
N/A — backend only.

---

## Approach
Test first, one item at a time (`tdd`): item 2's measuring test first (it decides whether code changes), then item 1, then the records for 3 and 4. Smallest change that meets the ACs; record cuts.

## Edge Case Checklist
- `cargo test` builds the bins' unit tests: tests now inside `keychain.rs` must run once, not twice, and still run
- `#[cfg(debug_assertions)]` fake keychain (`EASY_PERSONAL_TEST_KEYCHAIN`) must still reach the launcher in debug builds (launcher tests use it)
- the window build (`--features window`) and the default build both compile
- a `claude` step that fails after a row change: the row change must not be committed (T066)

## Files to Change (Predicted)
`rust/crates/app/Cargo.toml`, `rust/crates/app/src/launch.rs`, `main.rs`, maybe a new `rust/crates/app/src/lib.rs`, `connector_writes.rs` / `sync_apply.rs` (only if AC3 shows a call inside a transaction), tests, `tasks/TASK_REVIEW_T078.md`

## Files Must NOT Touch
store schema, `ingest/**`, `src/easy_personal/**`, `web/**`, `PRD.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_KANBAN.md`, `memory/**`, `rust/crates/app/src/alerts.rs` and any Watch file, `.claude/**` settings

## Test Plan
The ACs above, each as an automated test or a pasted command output, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T078.md`
- [ ] Commits on branch `t078-keychain-leftovers` only; final report with `Status:`
- [ ] If `keychain.rs` moves, its new path is in the final report (the other supervisor's T074 guide reads it)
