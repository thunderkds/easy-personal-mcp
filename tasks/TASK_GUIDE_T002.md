# TASK_GUIDE — T002: P0 foundation (workspace, `--version`, read-only store guard)
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md` and `memory/decisions.md` (the 2026-11-10 Rust port entry)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/docs/spike-T001.md` (the verified crate choices — use those, not guesses) and `BRAINSTORMING_LOG_RUSTPORT.md`

> Read relevant files from `rust/docs/` before making any code changes.

---

## Requirement

**Original request** (owner, 2026-11-10): a Rust application version; the version must be visible in the application.

**Restated intent**:
> A Rust binary exists that builds on Ubuntu, reports its version, and can open the existing Python store without ever changing it — and refuses a store newer than it understands.

**Out of scope**: HTTP, MCP, the window (the `/api` version field, MCP server info and window title land in P1 and P1b); any write to the store; ingest; macOS and Windows; the React bundle.

**Requirement Refs**: RS-001 (CLI part), RS-002, RS-004 (read side) in `rust/docs/SPEC.md`; Python `PRD.md` FR-091 (the store health check's read shape) as the model.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T001 — the toolchain and the verified crate choices must exist
**Entry point**: `easy-personal-rs --version` and `easy-personal-rs store-info` (the CLI binary)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `cargo build` passes on Ubuntu; one Cargo workspace; the binary is `easy-personal-rs` | Intent |
| 2 | `easy-personal-rs --version` prints `easy-personal-rs <version>` where the version is the Cargo package version (single source) | RS-001 |
| 3 | `easy-personal-rs store-info [--json]` opens the store **read-only** and prints: file exists, `PRAGMA user_version`, the newest migration number this build knows (11), pending yes/no, and each table's row count — no row content, no absolute path (file name only) | RS-002 |
| 4 | A store with `user_version` greater than 11 → exit code 2, a clear message, nothing written (file hash unchanged) | RS-002 |
| 5 | A missing store → "no data" and exit 0, never a crash, no file created | RS-002 |
| 6 | The store path honours the same override environment variable the Python app uses (read `src/easy_personal/core/config.py` to find it); the tests use a temp path and never the owner's real `~/.easy-personal` | RS-002 |
| 7 | Negative: opening is read-only — a test proves a write attempt on the connection fails and the file hash is unchanged | RS-004 |
| 8 | `cargo test` and `cargo clippy -- -D warnings` pass | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | a fixture store built from the Python repo's fixtures at `user_version = 11` | `store-info --json` shows 11, pending false, the table counts | test |
| 2 | a fixture store with `user_version = 12` | exit 2, message names both numbers, hash unchanged | test |
| 3 | no store file | "no data", exit 0, no file created | test |
| 4 | `--version` | the Cargo version string | test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy -- -D warnings \
  && ./target/debug/easy-personal-rs --version
```

---

## Approach

**Pattern reference**: `src/easy_personal/core/store.py` (WAL, `busy_timeout`, `foreign_keys`, `user_version`) and `core/migrations/__init__.py` (the migration list) — imitate the settings, never write.
**Vital slice**: `--version` and `store-info` with the newer-store guard.
**Cut list**: no HTTP, no MCP, no window, no migrations, no CI, no packaging. No `--json` fields beyond those in criterion 3.

Build a small workspace: one binary crate with `clap`, one library module for the store read. Use the crate choices T001 verified (`rust/docs/spike-T001.md`): `rusqlite` 0.41.1 with `bundled`, `clap` 4.6.7, pinned with `=` like the scratch crate (`rust/spikes/t001-crates/Cargo.toml` is the pattern). Set `rust-version = "1.90"`. Do **not** add `tauri`, `rmcp`, `axum`, `keyring` or any YAML crate in this task. Run `. "$HOME/.cargo/env"` first: rustup was installed without touching the shell profile. Open with SQLite read-only flags and `PRAGMA query_only=ON`. Set the known migration number as a single constant with a comment pointing to the Python migrations folder. Build the fixture store inside the test from SQL; do not copy the owner's data.

---

## Edge Case Checklist

- [ ] Store `user_version` newer than known → refuse, write nothing
- [ ] WAL file present next to the store → reading must not checkpoint or delete it
- [ ] Python writing at the same moment → `busy_timeout`, then a clear "busy" message, not a panic
- [ ] Store path unreadable or a directory → clear error, exit non-zero
- [ ] No absolute path or row content in any output (NFR-002)
- [ ] Tests never touch the real `~/.easy-personal`
- [ ] Stale references: after the change, `rust/PROJECT_SPEC.md` Tech stack should name the verified crates — tell the Supervisor

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/Cargo.toml`, `rust/rust-toolchain.toml` | new — workspace, pinned stable |
| `rust/crates/app/src/main.rs`, `rust/crates/app/Cargo.toml` | new — CLI (`--version`, `store-info`) |
| `rust/crates/core/src/store.rs`, `rust/crates/core/src/lib.rs`, `rust/crates/core/Cargo.toml` | new — read-only store open, guard, counts |
| `rust/crates/core/tests/store_info.rs` | new — fixture-store tests |
| `rust/README.md` | new — build and run, one screen |
| Pre-existing pin tests | None — the Rust code has no tests yet |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/easy_personal/**`, `web/**`, `tests/**`, `pyproject.toml` (the Python app) | Python parity reference; read-only |
| `~/.easy-personal/**` | the owner's real store |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_SPEC.md`, `rust/PROJECT_KANBAN.md`, `memory/**` | Supervisor-owned (root board is a Linear cache) |

---

## Test Plan

Unit tests over a temp SQLite file created in the test (versions 11, 12, missing, directory, WAL present); a hash-before/after assertion; the Verification Command; then a manual run against a *copy* of the real store, if the owner provides one.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes (`cargo clippy -- -D warnings`)
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T002.md`
- [ ] Run the binary and confirm `--version` and `store-info` work
- [ ] `memory/MEMORY.md` updated (Supervisor)
- [ ] Supervisor notified: task ready for Stage 4 review
