# TASK_GUIDE — T027: Rust store writer — lock, one transaction per file, the exact upsert, v11 only
**Date**: 2026-11-11
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P0
**Assigned agent**: backend-implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover; read the 2026-11-11 entry "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP2.md` completely and `rust/parity/README.md` (the "Ingest oracle" section written by T026)
6. Read the Python you are matching, read-only: `src/easy_personal/core/store.py` (all), `core/lock.py`, `core/config.py` (data dir), `core/migrations/m0001_baseline.py` .. `m0011_*.py` (to understand the schema, you will not port them)
7. Read the Rust you extend: `rust/crates/core/src/{lib.rs,store.rs,clock.rs}`, `rust/crates/core/Cargo.toml`, `rust/Cargo.toml`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. **Tests use temp folders and copies of `rust/parity/store-v11.sqlite` only**

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): build the Rust ingest as a second writer of the schema-v11 store (P2a). This task is the **write side only**: the store writer and the lock. No file parsing, no CLI.

**Restated intent**:
> The Rust core can open the shared store **for writing** without ever changing its schema, take the same single-writer lock the Python app takes, and save one transcript file's rows and its read offset in **one transaction** with exactly the Python upsert, so that a crash resumes cleanly and the two implementations never write at once or disagree on a row.

**Out of scope**: parsing transcripts (T028); the `ingest` CLI (T029); `kit_trace`/`ui`/`verifier` tables (T033); any migration (none in P2: refuse instead); the live tail; changes to `rust/crates/app/**`.

**Requirement Refs**: RS-003 (parity), NFR-002 (privacy), RS-006 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T026 — `rust/parity/ingest/golden/schema-v11.sql` (the fresh-store snapshot) and the Python behaviour it records
**Entry point**: `easy_personal_core::writer` (public API used by T028/T029; reached by the tests of this task)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New module `rust/crates/core/src/writer.rs` (and `ingest_lock.rs` if you prefer a separate file) exports: `open_for_write(path) -> Result<Store, WriteError>`, `ingest_lock(path, timeout) -> Result<LockGuard, LockBusy>`, `Store::save_file(...)`, `Store::retire_files(...)`, `Store::prune(days, now)`, and the row types it needs (event, signal, offset). The existing read-only `store.rs` API and every existing test are unchanged | second writer, read path untouched |
| 2 | **Open rules**: a path that does not exist, or an existing file that is empty or has no tables → create a **new** store from the embedded schema snapshot (`rust/crates/core/src/schema_v11.sql`, a copy of T026's golden; a test proves the two files are byte-equal) and set `PRAGMA user_version = 11`; `user_version == 11` → open; `user_version > 11` → error `Newer` (same text and exit code 2 as the reader's `StoreError::Newer`); `0 < user_version < 11` → error `Older` with the line `store is schema v<N>: run the Python app once to migrate it` and exit code 2; a path that is a directory, a symlink loop or not a SQLite file → one fixed error text, **no path in it**, never a panic. **No migration code exists in this task** | no Rust migration in P2 |
| 3 | **Pragmas on every connection**: `busy_timeout=5000`, `foreign_keys=ON`, `journal_mode=WAL` (as `store.py:101-115`). A test reads them back | same as Python |
| 4 | **machine_id**: read `meta.machine_id` with a `SELECT` first; only if absent `INSERT OR IGNORE` a new random UUID v4 in Python's text format (lower-case, hyphenated). An up-to-date store must **not** take a write lock just to open (a test opens it while another connection holds a read transaction) | `store.py:108-114` |
| 5 | **Created-store equality**: `sqlite_master` of a Rust-created store equals the snapshot (type, name, tbl_name, sql), the seed rows (orphan `workspaces` row) are present, `user_version` is 11, `PRAGMA integrity_check` is `ok`. **The Python app opens it without migrating** (`store.opened` leaves `user_version` 11 and creates no `.bak-v*` file): prove it once with the parity venv (`rust/parity/.venv/bin/python`) in a test marked `#[ignore]` and paste the real run | Python can use a Rust-created store |
| 6 | **`ingest_lock`**: `flock(2)` (not POSIX `fcntl` locks) exclusive and non-blocking on `<dir>/ingest.lock`, opened for append and created if missing, never deleted; polls every 0.05 s until `timeout`, then `LockBusy`; released on guard drop and on process death; the data dir is created with mode 0700 and `chmod`ed after creation like `config.data_dir()`. Prove mutual exclusion **with a different process holding a real `flock`**: spawn `flock -x <file> -c 'sleep N'` (util-linux) and show `LockBusy` until it ends, then success. Say which std/crate call you used and show it is `flock` (for example `strace -e trace=flock` or the library source) | same primitive as Python |
| 7 | **`save_file`** is **one transaction** containing, in this order, exactly what `store.py:188-218` does: the event inserts with the conditional upsert `ON CONFLICT (machine_id, source, source_path, inode, byte_offset) DO UPDATE SET entrypoint = COALESCE(...), tool_error_count = COALESCE(...) WHERE (... IS NOT ...) OR (...)`; the **added** count computed as rows with `id > MAX(id)` taken before the insert; the session-entrypoint propagation (first entrypoint seen in the batch fills the NULL ones of the same session in this file; if none in the batch, any non-NULL entrypoint already stored for that session and file); `tool_signals` `INSERT OR IGNORE`; the `source_files` upsert `ON CONFLICT (machine_id, source, path) DO UPDATE SET inode, byte_offset`. Quote the Python lines you matched in the code comments. A failure in any step rolls back all of it (a test injects a failure on the last step and shows no event row and no offset change) | crash-safe resume |
| 8 | **Idempotency tests** (table-driven, on a temp copy of `store-v11.sqlite` and on a fresh store): the same batch twice adds 0 rows the second time; a re-save with `entrypoint` / `tool_error_count` newly known **fills** them and adds no row; a re-save with `NULL` never **erases** a known value; two different inodes for the same path and offset make two rows (the Python key); `added` equals the Python count in each case. Where T026's goldens already hold the Python result for an equivalent scenario, compare to it | upsert parity |
| 9 | **`retire_files`** deletes only the `source_files` rows named, never events; **`prune(days, now)`** deletes from `events`, `kit_trace_events` and `tool_signals` where `ts_utc IS NOT NULL AND ts_utc < cutoff` using a **text** comparison on `%Y-%m-%dT%H:%M:%S.%fZ` (6 digits), keeps NULL-`ts_utc` rows, never touches `source_files`, and returns the same counts shape as Python's `store.prune`. `now` is a parameter (no system clock inside) | `store.py:232-253` |
| 10 | **Foreign keys**: `events.agent_account_id` must reference an existing `agent_accounts` row; a test shows an unknown account id is rejected (the row is not written and the error is a clean `WriteError`, not a panic) | `foreign_keys=ON` |
| 11 | **No secrets, no paths** in any error text or log line: errors name the problem, never a store path, a row value or SQL | NFR-002 |
| 12 | Tests never touch `~/.easy-personal`: every store is a temp copy or a fresh temp store. `cargo test --workspace` passes under `TZ` unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago` (paste all three); `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` are clean, also with `--features window`; no new dependency except what the lock and UUID need (list each with its licence; prefer `std` and the existing `getrandom`); `Cargo.lock` additive | quality |
| 13 | **Mutation checks** (show the red run, then restore): remove the lock call → the exclusion test fails; drop the `WHERE` of the upsert → the idempotency test fails; make the transaction commit events before the offset → the rollback test fails | the tests bite |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T027.md`.
> **Locked oracle**: `rust/parity/ingest/**` (T026) and `src/easy_personal/core/store.py` (Python is the truth). The implementing agent must not edit them.

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | an existing v11 store copy | opens, pragmas set, no schema change | test |
| 2 | no store file | created, `sqlite_master` equals the snapshot, Python opens it without migrating | test + the ignored real run |
| 3 | a store at v12 / v8 | refused with exit-code-2 errors, file untouched (hash equal) | test |
| 4 | another process holds `flock` | `LockBusy` after the timeout; success after release | test with `flock(1)` |
| 5 | the same batch twice | 0 added; offsets and rows equal | test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain; the ignored test also needs rust/parity/.venv
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && cargo test -p easy-personal-core --test writer_python -- --ignored && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/src/store.rs` (open + errors), `rust/crates/core/src/clock.rs`, and Python `core/store.py`, `core/lock.py`.
**Vital slice**: open rules and pragmas (criteria 2, 3, 4) → created-store equality (5) → lock (6) → `save_file` and its tests (7, 8, 10) → retire/prune (9) → mutation checks (13).
**Cut list**: no parser, no CLI, no migration, no `kit_trace`/`ui`/`verifier` writers, no async, no connection pool.

Keep `rusqlite` usage in `writer.rs` like `rmcp` stays in `mcp.rs`. Use `rusqlite`'s `Transaction` for the one-transaction rule. Match Python's SQL text where it matters (the upsert, the `MAX(id)` count) and write why in a comment.

---

## Edge Case Checklist

- [ ] A second writer waits `busy_timeout` 5 s on a locked database and then returns a clean error, not a hang
- [ ] The lock file is opened for append and **never deleted**, even on error
- [ ] `user_version` is read before any write; a refused store is byte-identical after the attempt (hash)
- [ ] The WAL and SHM files of a refused store are not left behind by the attempt
- [ ] A store whose `meta.machine_id` exists is never given a second one
- [ ] `inode` and `byte_offset` are `i64`; no cast panics on values above `i32`
- [ ] The offset saved is the offset of the **end of the last complete line** the caller passes: this task stores what it is given and never computes it
- [ ] Text comparisons for prune use exactly 6 fractional digits; a row with `+00:00` or 3 digits is compared as text like Python (note it in the report if it can occur)
- [ ] Error text has no path, no SQL, no row value
- [ ] Stale references: `rust/README.md` says "Nothing here ever writes" for the store module: update the wording (the read-only `store.rs` still never writes; the writer is a separate module)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/writer.rs` (new), `ingest_lock.rs` (new, optional) | the writer and the lock |
| `rust/crates/core/src/schema_v11.sql` (new) | embedded copy of `rust/parity/ingest/golden/schema-v11.sql` |
| `rust/crates/core/src/lib.rs` | `pub mod` lines only |
| `rust/crates/core/Cargo.toml`, `rust/Cargo.toml`, `rust/Cargo.lock` | only if the lock or UUID needs a crate |
| `rust/crates/core/tests/writer_*.rs` (new) | the tests |
| `rust/README.md` | a short "Store writer (P2)" paragraph and the stale wording |
| `tasks/TASK_REVIEW_T027.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app (read-only oracle) |
| `rust/parity/**` | the oracle (T026 owns it) |
| `rust/crates/app/**`, `rust/crates/core/src/{metrics,rules,plan,scope,sessions,workspaces,prices,store}.rs` | not part of this task (`store.rs`: read-only API stays as is) |
| `rust/spikes/**` | spikes |
| `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Red first: the open-rules table, the pragma read-back, the lock exclusion with `flock(1)`, the upsert idempotency table, the rollback test, the prune/retire tests. Then green. Then the ignored real Python-opens-it run. Then the three TZ runs, clippy, fmt, mutation checks. Paste every output.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T027.md` before the first code change
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Tests written AND pass, red runs and mutation checks pasted
- [ ] The ignored real run (Python opens the Rust-created store) pasted
- [ ] Supervisor notified: task ready for Stage 4 review
