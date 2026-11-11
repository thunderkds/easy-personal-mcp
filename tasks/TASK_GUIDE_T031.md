# TASK_GUIDE — T031: Coexistence tests — two writers, one store, no corruption, no surprises
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P0
**Assigned agent**: qa-automation
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

Before writing anything:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the 2026-11-11 entry "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/qa.md`
5. Read `BRAINSTORMING_LOG_RUSTP2.md` (the Edge Case Checklist and the Adversarial Review of Option A) and `rust/parity/README.md` (the oracle and its venv)
6. Read the Python you test against, read-only: `src/easy_personal/core/{store,lock,config}.py`, `src/easy_personal/surfaces/cli.py` (`_ingest`, the dashboard's start-up ingest), `tests/ops/test_lock.py` (how Python tests its own concurrency)
7. Read the Rust you test: `rust/crates/app/src/{main.rs,ingest_cmd.rs}`, `rust/crates/core/src/{writer,ingest_lock}.rs`, `rust/crates/core/src/ingest/folders.rs`, and the existing tests `rust/crates/core/tests/ingest_folders_*.rs` (sequential coexistence is already proven there: do not repeat it)
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Every test sets `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT` to temp folders on the Linux filesystem (not `/mnt/*`: `flock` may not work there)

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): the Rust ingest (P2a) is a second writer of the schema-v11 store. Two implementations on one file is the risk of Option A, so it needs its own evidence.

**Restated intent**:
> Tests start the real Rust binary and the real Python app against the same store and show that they never write at once, never corrupt the file, never lose or duplicate a row, resume after a kill, and let readers (`serve`, `mcp`) keep answering while an ingest runs.

**Out of scope**: any change to product code (report a defect, do not fix it); sequential Rust-then-Python and Python-then-Rust on one folder (done in T035); performance (T032); the leak check (T030); `kit_trace`/`ui`/`verifier`/`context`/`live` (the Python dashboard's live tail is a continuous writer: it is listed under "Not covered" in your report, not tested).

**Requirement Refs**: RS-003, RS-006, NFR-002, NFR-005 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T029 (the `ingest` command), T035, T027
**Entry point**: `cargo test -p easy-personal-rs --test ingest_coexist` (and its `--ignored` Python half)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New test file `rust/crates/app/tests/ingest_coexist.rs` (and helpers) spawning `target/debug/easy-personal-rs` with a built env and, in the `#[ignore]`d half, the real Python ingest from `rust/parity/.venv` (`PYTHONPATH=src`). No test touches a real home; a guard refuses to run if `EASY_PERSONAL_HOME` is unset | real processes |
| 2 | **Simultaneous writers** (Rust ‖ Python, Rust ‖ Rust, Python ‖ Python as the control): start two ingests of the same folder at the same instant (a barrier file or a short sleep ladder 0/5/20/50 ms) against one fresh store; the final tables are **identical to one clean single run** (use the T026 goldens' tables: S01 for the committed fixtures and S06 for the synthetic set); exit codes are 0 for both, or 0 and 75 when `EASY_PERSONAL_LOCK_TIMEOUT` is short; `PRAGMA integrity_check` is `ok`; no duplicate `(machine_id, source, source_path, inode, byte_offset)`; repeat each case 20 times and report 20 of 20 | single writer |
| 3 | **Kill and resume**: ingest a generated large set (several MB, many files); `SIGKILL` the Rust ingest after 30 ms, 100 ms and at a random point (fixed seed list, pasted); the store opens, `integrity_check` is `ok`, no file is half-saved (each `source_files` offset sits at a line start and its events are complete: check that the count of events of a file equals what a clean run stores up to that offset); rerun Rust: the final tables equal a clean run's. Then the same with a Python rerun after the Rust kill, and a Rust rerun after a Python kill | crash safety |
| 4 | **Lock hygiene**: after a kill the lock is free at once (a new ingest starts without waiting); `ingest.lock` still exists; no `-wal`/`-shm` junk blocks the next open; a second ingest started while the first is **stopped** (`SIGSTOP`) waits and exits 75 after its timeout, then, after `SIGCONT` of the first, a third succeeds | OS releases the lock |
| 5 | **Readers while writing**: while a large Rust ingest runs, `serve` answers `/api/usage/summary` and `/api/warnings` 200 with a consistent, valid JSON each time (poll 50 times; never a 5xx, never "database is locked"), `mcp` `tools/call usage_summary` answers, and `store-info` prints; after the ingest, the numbers equal a read of the final store. Then the same with a **Python** ingest running | WAL readers |
| 6 | **Copied and restored files**: copy a transcript to a new name and to a new inode (`cp`), ingest with Rust then Python and Python then Rust: both implementations produce the **same** rows (inode is part of the key, so the copy adds rows in both: the golden S13 shows the Python behaviour); record it as a documented weak spot, not a defect | parity incl. weak spots |
| 7 | **Version guards while the other side acts**: a v10 store (made like the oracle's S12) is refused by Rust (exit 2) and left byte-identical; the Python app then migrates it to v11 and Rust can ingest into it; a store set to v12 is refused by both (Rust exit 2, Python exit 1) and left byte-identical | no Rust migration |
| 8 | **Weird environment**: `EASY_PERSONAL_HOME` on a path with a space and a non-ASCII character; a read-only data dir (clean exit 1, message without a path); a store file with mode 0444 (clean error); `ingest.lock` owned by the same user with mode 0444 (the append-open fails: clean exit 1, never a hang) | robustness |
| 9 | Every test is deterministic or reports its seeds; the whole file runs in under 3 minutes (paste the time); nothing is left running (check by PID); `cargo test --workspace` under `TZ` unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago` (paste all three); clippy and fmt clean (also `--features window`); no new dependency | quality |
| 10 | **Defects found** are listed with a minimal reproduction and are **not fixed** here; a clean "none found" is a valid result only if the repeat counts above are pasted | honest result |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T031.md`.
> **Locked oracle**: the Rust and Python products (read-only) and the T026 goldens.

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | two ingests started together, 20 times | tables equal one clean run, integrity ok | test |
| 2 | SIGKILL mid-ingest | resume equals a clean run | test |
| 3 | `serve` polled during an ingest | no 5xx, valid JSON | test |
| 4 | v10 and v12 stores | refused, bytes unchanged | test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain, git, and (ignored half) rust/parity/.venv
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test -p easy-personal-rs --test ingest_coexist && cargo test -p easy-personal-rs --test ingest_coexist -- --ignored && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/app/tests/{cli.rs,server.rs}` (spawn helpers), `tests/ops/test_lock.py`, the T035 coexistence test.
**Vital slice**: simultaneous writers (2) → kill and resume (3, 4) → readers while writing (5) → copies (6) → version guards (7) → environment (8).
**Cut list**: no new product code, no perf numbers, no live tail, no new crate.

---

## Edge Case Checklist

- [ ] Children are always reaped (a guard kills a leftover child on test failure, by PID)
- [ ] Timing tests use barriers or polling with deadlines, not fixed sleeps alone, so they are not flaky under load
- [ ] A test that needs Python is `#[ignore]` and says how to run it; it never runs on `cargo test` without the flag
- [ ] The large generated set is written under the temp folder and is deterministic (seeded), valid Claude Code transcript lines only
- [ ] `integrity_check`, the duplicate-key query and the table dump are one helper, reused
- [ ] No path, SQL or event content in any assertion message that could end up in a log
- [ ] Stale references: `BRAINSTORMING_LOG_RUSTP2.md` task table (do not edit)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/tests/ingest_coexist.rs`, `rust/crates/app/tests/ingest_coexist_common/mod.rs` (new) | the tests and helpers |
| `rust/README.md` | a short "Coexistence tests" note |
| `tasks/TASK_REVIEW_T031.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/crates/*/src/**`, `rust/Cargo.*` | product code (report defects) |
| `rust/parity/**` | the oracle |
| `skills/verify/SKILL.md`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Helpers and the simultaneous-writer test first (red: run it against a deliberately lock-less copy if you can do so without editing the repo, else show the lock log), then kill/resume, readers, copies, guards, environment. Paste every output with counts.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T031.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] All repeat counts, seeds and timings pasted
- [ ] Defects found (or "none found" with counts) listed
- [ ] Supervisor notified: task ready for Stage 4 review
