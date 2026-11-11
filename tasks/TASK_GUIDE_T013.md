# TASK_GUIDE — T013: Shared store opener and clock in the core
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P0
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T002, T008, T009)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/core/src/store.rs`, `rust/crates/app/src/mcp.rs` (`with_store`), `rust/crates/app/tests/mcp.rs`, `rust/crates/core/tests/store_info.rs`
6. Do not open `~/.easy-personal` or any `~/.claude*` folder, not even to check that it exists

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10: "start the P1 wiring slice"): serve the ported read functions over HTTP and MCP.

**Restated intent**:
> Two small pieces both surfaces need live in the core once: a checked read-only open (refuses a missing store and a store newer than this build, with no path in any message) and a clock that returns the current time as UTC microseconds. The MCP server uses them and behaves exactly as before.

**Out of scope**: the HTTP routes (T014) and the new MCP tools (T015); any behaviour change; creating a store (the Rust side never writes).

**Requirement Refs**: RS-002 (read-only open), NFR-002 (no path or row text in errors) in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T002 (`store.rs`), T008 (`mcp.rs`)
**Entry point**: `easy_personal_core::store::open_checked` and `easy_personal_core::clock::now_micros` (library functions), used by `mcp.rs` in this task and by T014 and T015 next
**Parallel task**: none. T014 and T015 start after this one is verified.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `store::open_checked(path: &Path) -> Result<Connection, StoreError>`: no file -> `StoreError::NoStore` (Display text exactly `no store yet: nothing has been ingested into the data folder`); a directory in its place -> `NotAFile`; a store whose `user_version` is greater than `KNOWN_MIGRATION` -> `Newer`; otherwise the connection from `open_read_only`. It never creates a file and never writes | RS-002 |
| 2 | `StoreError::NoStore` is a new variant; `exit_code()` stays 1 for it; `read_info` and `store-info` behave exactly as before (a missing store is still `State::Missing`, exit 0) | RS-002 |
| 3 | `clock::now_micros() -> i64`: microseconds since the Unix epoch in UTC, from `SystemTime`; a clock before 1970 returns a clear error or 0 (state which and why in the code comment) | Intent |
| 4 | `mcp.rs` `with_store` uses `open_checked`; the MCP tool errors keep their exact texts (missing store, newer store, not a file), so every test in `rust/crates/app/tests/mcp.rs` passes unchanged | Intent |
| 5 | New tests in `rust/crates/core/tests/store_open.rs`: missing, directory, newer (a temp copy of the fixture with `PRAGMA user_version = 99`), fixture ok, no file or sidecar created for the missing case, fixture bytes unchanged, and no message contains the temp path. `clock::now_micros` is checked against `SystemTime` within a few seconds | RS-002, NFR-002 |
| 6 | `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass; `git status --short -- src web tests pyproject.toml uv.lock` is empty before and after | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the existing suites (they must stay green and unchanged) plus the new `store_open.rs`.
> **Locked oracle**: rust/parity/store-v11.sqlite

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | an empty temp folder | `open_checked` -> `NoStore`; the folder stays empty | test |
| 2 | a store at version 99 | `Newer`, text unchanged from today | test |
| 3 | `mcp` over a missing store | the same tool error as before | existing mcp test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/src/store.rs` (`open_read_only`, `read_info`) and `mcp.rs` `with_store`, whose logic you are moving.
**Vital slice**: move the logic from `with_store` into `open_checked`, add the variant, then the clock.
**Cut list**: no async, no connection pool, no caching, no new dependency.

---

## Edge Case Checklist

- [ ] The `Newer` text equals today's (`the store is at version N, newer than the 11 this build knows; …`)
- [ ] A symlink to a regular file counts as a file (same as `read_info`)
- [ ] No absolute path, row text or secret in any error (NFR-002)
- [ ] Tests never open `~/.easy-personal`
- [ ] Stale references: `rust/README.md` and comments that describe `with_store`

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/store.rs` | `NoStore` variant, `open_checked` |
| `rust/crates/core/src/clock.rs` | new |
| `rust/crates/core/src/lib.rs` | `pub mod clock;` (re-read first) |
| `rust/crates/app/src/mcp.rs` | `with_store` uses `open_checked` |
| `rust/crates/core/tests/store_open.rs` | new |
| Pre-existing pin tests | None — `store_info.rs` and the app `mcp.rs` tests must pass unchanged |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/parity/**` | the oracle |
| `rust/crates/app/src/server.rs`, `tests/server.rs` | T014 owns them next |
| `rust/crates/core/src/{metrics,rules,plan,prices}.rs` | behaviour must not change |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Record the current test counts first. Red first: write `store_open.rs` (it fails to compile or fails on stubs), implement, then switch `mcp.rs` and run the whole suite.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T013.md`
- [ ] Supervisor notified: task ready for Stage 4 review
