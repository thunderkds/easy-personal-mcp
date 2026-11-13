# TASK_GUIDE — T059: Rust `store-check` (with `--full`) and `GET /api/store/health`
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
5. Read `rust/docs/PYTHON_RETIREMENT_PLAN.md` and `tasks/TASK_GUIDE_HUN-244.md`, `tasks/TASK_REVIEW_HUN-244.md` (the Python task you port)
6. Read the Python you copy, **read-only**: `src/easy_personal/core/store_health.py` (whole file), `tests/core/test_store_health.py`, `tests/surfaces/test_api_store_health.py`, `tests/leak/test_store_check_leak.py`, and the `store-check` parts of `src/easy_personal/surfaces/cli.py` and `api.py`
7. Read the Rust you extend: `rust/crates/app/src/{main,api,server}.rs`, `rust/crates/core/src/` (how `store-info` opens the store read-only), `rust/crates/app/tests/` (how a test starts `serve`), `rust/parity/README.md`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder. Tests use temp folders and fixtures

---

## Requirement

**Original request** (owner, 2026-11-13): "approve all" for `rust/docs/PYTHON_RETIREMENT_PLAN.md`.

**Restated intent**:
> `easy-personal-rs store-check [--json] [--full]` and `GET /api/store/health` give the same read-only store health answer as Python's `store-check` (HUN-244, FR-091): same fields, same verdicts, same exit codes (0 healthy, 1 a problem found, 3 not fully checked, 130 Ctrl-C with `--full`). Nothing is written. No row content, no absolute path.

**Out of scope**: any change to Python files; a page change; repair, VACUUM, REINDEX, backup; the live feed; connector or keychain code.

**Requirement Refs**: FR-091, NFR-002, NFR-008, NFR-013 (`PRD.md`); RS-001..RS-003 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the owner ("approve all", 2026-11-13)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: HUN-244 (merged on `main`, `b0c0ecb`)
**Entry point**: the `store-check` subcommand in `rust/crates/app/src/main.rs`; `GET /api/store/health` in `api.rs`

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Fields.** `store-check --json` returns every field Python's does (file, size, wal size, `user_version`, newest migration, `migration_pending`, `integrity_check`, `foreign_key_check`, table counts read from the schema, per-source last ingest and abstain counts, `bak-v<N>` names, `healthy` true/false/null, `incomplete`, `unreadable`, `locked`); the human form has the same lines and verdict words as Python | FR-091 |
| 2 | **Oracle.** A generator `rust/parity/gen_store_health.py` runs Python's `store_health.check` over fixture stores (clean, empty, one migration behind, newer than the code, corrupt page, damaged index, WAL without SHM, locked, hostile table and source names, a BLOB time, a missing store, a zero-byte file). The golden is `rust/parity/golden-store-health/`, made once by Python. The Rust JSON equals it for every case (time budget forced the same way) | FR-091 |
| 3 | **Exit codes.** 0 healthy, 1 a problem or unreadable part, 3 not fully checked (the 1.5 s budget on a store over 32 MB, or a locked store), 130 for Ctrl-C under `--full` with no panic text | FR-091 |
| 4 | **`--full`.** Runs the full `integrity_check` with no time budget; prints one line `full check: this can take minutes; Ctrl-C stops it` to stderr first, so `--json` stdout stays pure JSON; the route never does this | FR-091 |
| 5 | **No write, no side file.** The store file hash, mtime and `-wal` are unchanged; a store with `-wal` and no `-shm` creates no new file beside it (read from a private temp copy as Python does); the open mode is decided once | NFR-002 |
| 6 | **Text is cleaned.** Table names, source names and times are capped at 64 characters and control characters are replaced with `?` in every output | NFR-002 |
| 7 | **Route.** `GET /api/store/health`: 200 with the token; 403 without it and with a wrong Host or Origin; POST gives 405; `HEAD` answers headers only and runs no check; one check runs at a time (others wait for it) and the answer is cached 5 s per store; the answer equals the CLI JSON | FR-091, NFR-008 |
| 8 | **Leak.** `parity/leak_rust.py` covers the CLI (default and `--full`) and the route over planted canaries in a row, a table name, a source path and a file name: 0 hits | NFR-002 |
| 9 | **Speed.** A fixture store of about 1M events answers the default run in under the 1.5 s budget or reports `healthy: null` (never "healthy" with unfinished parts) | NFR-013 |
| 10 | **Regression.** Three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml` and `Cargo.lock` unchanged, existing tests and goldens pass | RS-003 |
| 11 | **Docs.** `rust/README.md`: the command, `--full`, the route, exit codes, the "decided for you" list, the "Not served yet" list corrected. `rust/PROJECT_*.md` and `memory/**` are not edited (Supervisor) | — |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T059.md`.
> **Locked oracle**: `golden-store-health/` made once by Python, plus the Python module and tests (read-only).

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

**Reviewer note**: Medium risk. Stage 4 has a code review and a security review (the route reads a user file; nothing may be written; no content or path in any output).

---

## Approach

**Vital slice**: a core function returning one struct, used by both the CLI and the route; the oracle; then `--full`, the route cache and the leak gate.
**Cut list**: no repair, no page, no MCP tool, no history of checks.
**Order**: red test first for each criterion.

---

## Edge Case Checklist

- [ ] Store missing, zero bytes, not a database: a clear answer, no panic, exit 1
- [ ] `-wal` without `-shm`, `-shm` without `-wal`, a symlinked store (the `-wal` is beside the real target)
- [ ] A locked store: `locked: true`, `healthy: null`, exit 3, never "1 problem"
- [ ] A BLOB in a time column, a table name with invalid UTF-8: that part is "unreadable", never a panic
- [ ] Time uses the injected clock; three `TZ` values
- [ ] Ctrl-C under `--full` stops all reads and exits 130

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/store_health.rs` (new), `lib.rs` | the check |
| `rust/crates/app/src/{main,api,server}.rs` | the command, the route |
| `rust/crates/core/tests/store_health_parity.rs`, `rust/crates/app/tests/{store_check,store_health_route}.rs` (new) | oracle, process and route tests |
| `rust/parity/gen_store_health.py`, `rust/parity/golden-store-health/` (new); `rust/parity/leak_rust.py` | oracle, leak registry |
| `rust/README.md` | docs |
| `tasks/TASK_REVIEW_T059.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| existing `rust/parity/golden*/**`, `store-rich/**`, `fixtures-rich/**` | the oracle |
| `rust/Cargo.toml`, `rust/Cargo.lock`, `crates/*/Cargo.toml` | no new dependency without a stop-and-report |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Red test per criterion, then green; oracle; leak gate; three `TZ` runs (`unset`, `Pacific/Kiritimati`, `Pacific/Pago_Pago`: `cargo test --workspace --no-fail-fast`).

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T059.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts, three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written
- [ ] Supervisor notified: task ready for Stage 4 review
