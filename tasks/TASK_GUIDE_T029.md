# TASK_GUIDE — T029: `easy-personal-rs ingest` — the command, the lock, the JSON, the exit codes
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend-implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the 2026-11-11 entry "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP2.md`, the "Ingest oracle" section of `rust/parity/README.md`, the findings section at the end of `tasks/TASK_REVIEW_T026.md`
6. Read the Python you are matching, read-only: `src/easy_personal/surfaces/cli.py` (the `ingest` parser lines 49-68, `positive_int` 30-37, `_ingest` 159-178, `_print_json`), `src/easy_personal/core/config.py`, `core/lock.py`
7. Read the Rust you plug into: `rust/crates/app/src/main.rs` (the subcommands, the exit-code style, how `serve`/`store-info` resolve the store path), `rust/crates/core/src/{writer.rs,ingest_lock.rs,clock.rs}`, `rust/crates/core/src/ingest/` (T028/T035)
8. Read the oracle goldens `rust/parity/ingest/golden/S01*`, `S08a/b/c`, `S09`, `S10a/b`, `S11`, `S12` (stdout, stderr, exit codes)
9. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use temp folders and set `EASY_PERSONAL_HOME` / `EASY_PERSONAL_AGENT_ROOT` explicitly

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): build the Rust ingest (P2a). This task is the **command**: `easy-personal-rs ingest`, wired to the pieces from T027 (writer, lock), T028 (parse layer) and T035 (folders).

**Restated intent**:
> `easy-personal-rs ingest [--source PATH] [--retention-days N]` does what `easy-personal ingest` does for the events source: resolves the data dir and store from the same environment variables, takes the single-writer lock (waiting up to the same timeout, exit 75 with the same message when busy), detects accounts, ingests the transcripts, optionally prunes, and prints the result as JSON with the same formatting and the same numbers; it refuses a store it must not touch with a clean message; and it is honest about what it did **not** ingest.

**Out of scope**: `kit_trace`, `ui`, `verifier`, `context`, `live`, the scheduler, `schedule install`, the dashboard start-up ingest, any migration; changes to T027/T028/T035 behaviour (report a needed change instead).

**Requirement Refs**: RS-003 (parity), RS-006, NFR-002 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T035 (folders), T028 (parse layer), T027 (writer, lock). **Built API (T035, 2026-11-11):** `easy_personal_core::ingest::folders::ingest_accounts(&mut Store, &Options { agent_root, projects_override: Option<PathBuf>, cwd_base: PathBuf }) -> Result<Summary { files, new_events, skipped }, WriteError>`; it calls `ingest::accounts::detect` itself first (a second call is harmless). Writer: `writer::{open_for_write, Store::prune}`, lock: `ingest_lock::{ingest_lock, LockError}` (takes the **data dir**).
**Entry point**: `easy-personal-rs ingest`

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New subcommand `Ingest { --source PATH, --retention-days N }` in `rust/crates/app/src/main.rs` (one new module `ingest_cmd.rs` is fine); `--help` lists it; `--retention-days` is a positive integer (**`0` and negatives are a clap usage error, exit 2**, like the oracle's S08c) | S08c |
| 2 | **Environment, same names and meaning as Python** (`core/config.py`): `EASY_PERSONAL_HOME` (else `~/.easy-personal`) is the data dir, created with mode 0700 and `chmod`ed after creation; the store is `<data dir>/store.sqlite`; `EASY_PERSONAL_AGENT_ROOT` (else `~`) is the agent root; `EASY_PERSONAL_CLAUDE_PROJECTS` and `--source` (the flag wins) replace the default account's projects folder; `EASY_PERSONAL_LOCK_TIMEOUT` (float seconds, default 300) is the lock wait. A malformed `EASY_PERSONAL_LOCK_TIMEOUT` is a clean error (exit 1), not a panic | same configuration |
| 3 | **Order** = Python `_ingest`: take the lock (`ingest_lock(data dir, timeout)`) **before** opening the store → `open_for_write` → `accounts::detect` → `ingest_accounts` → optional prune → print. The lock is held until the result is printed | same order |
| 4 | **Busy lock** (S09): stderr is exactly `another ingest is running; try again later` plus a newline, stdout empty, exit code **75**, `ingest.lock` exists afterwards (never deleted) | S09 |
| 5 | **Refused stores** (decided in T027 and T026, ⚠ decided for you): a store newer than v11 → the clean `Newer` message on stderr, exit **2** (Python: traceback, exit 1; S11); an older store (`1..=10` with tables) → `store is schema v<N>: run the Python app once to migrate it`, exit **2** (S12); neither leaves a `-wal`/`-shm`, creates a backup or changes a byte of the store. A missing store is **created** (T027) | S11, S12 |
| 6 | **JSON output**: `indent=2`, keys **sorted**, the same number formatting as Python's `json.dumps(..., indent=2, sort_keys=True)`, a trailing newline, ASCII only. For the events source it holds `files` and `new_events` (equal to the S01..S06, S10 goldens' numbers) and **`skipped`** (the count of entries Rust skipped: Python has no such key; it prints it so a skipped file is never silent; **⚠ decided for you**) and, with `--retention-days`, `pruned` (equal to S08b). **It must not contain `kit_trace`, `context`, `ui` or `verifier` keys with invented values**: instead it contains `"not_ingested": ["context", "kit_trace", "ui", "verifier"]` (sorted) saying what P2a skipped. **⚠ decided for you** (the Python output shape has those four keys; Rust prints only what it did) | honest output |
| 7 | **Exit codes**: 0 success; 75 busy; 2 a store that must not be touched or a usage error; 1 any other failure with a one-line message that holds no path, SQL or row value; never a panic or a Rust backtrace. An unreadable agent root or projects folder is a counted skip inside the result's `files`/zero numbers (as Python), not an error | exit codes |
| 8 | **Oracle replay through the real binary** (spawn `target/debug/easy-personal-rs ingest` with a built env, like the T004/T023 tests): S01, S02, S03, S04, S05, S06, S07, S08a, S08b, S10a, S10b: compare stdout numbers and the four tables to the goldens (use the T035 test helpers; do not duplicate the layout code); S09, S11, S12: compare exit code and stderr | RS-003 |
| 9 | **Retention**: `--retention-days 36500` prunes nothing; `--retention-days 1` deletes every row with a timestamp and keeps NULL-timestamp rows, `source_files` untouched (S08a, S08b); the result's `pruned` equals the golden's | S08 |
| 10 | **Two processes**: run the Rust binary while another process holds `flock` on `ingest.lock` for 1 s with `EASY_PERSONAL_LOCK_TIMEOUT=3`: it waits and then succeeds (exit 0); with `EASY_PERSONAL_LOCK_TIMEOUT=0.2` it exits 75. Real Python run (ignored test, parity venv): Rust ingest then Python ingest on the same data dir, and Python then Rust: the second adds 0 events and exits 0; paste both | coexistence |
| 11 | **Privacy**: stdout, stderr and the store contain no transcript text; a canary test over the binary: a transcript with `CANARY-…` in every free-text field, ingest, then scan stdout, stderr, the store, WAL and SHM bytes: zero hits (T030 widens this into the leak suite) | NFR-002 |
| 12 | `cargo test --workspace` passes under `TZ` unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago` (paste all three); `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` clean, also with `--features window`; no new dependency; the **default build still links no Tauri/GTK/WebKit** (`cargo tree` line pasted) | quality |
| 13 | `rust/README.md`: an "Ingest (P2a)" section: the command, the environment variables, what is and is not ingested, the exit codes, the coexistence rule (one writer at a time; the Rust ingest never migrates) and the differences from Python, each flagged. The `/verify` recipe (`skills/verify/SKILL.md`) is **not** edited here: list the ingest step you would add in your report (the Supervisor will make it a task) | docs |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T029.md`.
> **Locked oracle**: `rust/parity/ingest/**` (T026) and Python's `cli.py` `_ingest`. The implementing agent must not edit them.

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the S01 inputs | stdout numbers and tables equal the golden | test over the binary |
| 2 | a held lock, timeout 0.5 | exit 75, the exact stderr line | test |
| 3 | a v12 store and a v10 store | exit 2, the stated messages, file untouched | test |
| 4 | `--retention-days 0` | exit 2 usage error | test |
| 5 | Rust then Python on one data dir | Python adds 0 | ignored real run |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain, git, and (ignored test) rust/parity/.venv
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && cargo test -p easy-personal-rs --test ingest_cli -- --ignored && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/app/src/main.rs` and `tests/cli.rs` (spawn helpers), Python `cli.py` `_ingest`.
**Vital slice**: env and data dir → lock and order → the JSON and `not_ingested` → S01 replay → refused stores and exit codes → retention → two-process tests → privacy canary → README.
**Cut list**: no other sources, no schedule, no progress output, no `--json` flag variants, no new crate.

---

## Edge Case Checklist

- [ ] The data dir is created 0700 only when it did not exist; an existing dir keeps its mode
- [ ] A relative `--source` or `EASY_PERSONAL_CLAUDE_PROJECTS` is normalised like Python's `Path(...)`: a leading `./`, repeated separators and a trailing separator are dropped before it is handed to `ingest_accounts` (T035's `components()` keeps a leading `./`, where Python drops it)
- [ ] `EASY_PERSONAL_HOME` pointing at a file, a dangling symlink or an unwritable dir gives a one-line message, exit 1
- [ ] The lock is taken before the store is opened, so a busy lock never touches the store
- [ ] stdout carries only the JSON (nothing else, no warnings); diagnostics go to stderr
- [ ] The result JSON is printed once, after prune, and only on success
- [ ] A second run with nothing new prints `new_events: 0` and the same `files`
- [ ] `--source` pointing at a missing folder behaves as Python does for S10 (check the goldens), not as an error
- [ ] No panic on a closed stdout (broken pipe)
- [ ] Stale references: `rust/README.md` "four subcommands" / "five commands" wording; the `--help` text in the README

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/main.rs`, `rust/crates/app/src/ingest_cmd.rs` (new) | the subcommand and its runner |
| `rust/crates/app/tests/ingest_cli.rs` (new) | the tests over the binary |
| `rust/README.md` | the "Ingest (P2a)" section and the stale command-count wording |
| `tasks/TASK_REVIEW_T029.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app (read-only oracle) |
| `rust/parity/**` | the oracle |
| `rust/crates/core/**` | T027/T028/T035 (report a needed change instead; a one-line visibility change only if unavoidable, and say so) |
| `rust/crates/app/src/{api,mcp,server,static_files,window,display}.rs`, `skills/verify/SKILL.md` | not part of this task |
| `rust/spikes/**` | spikes |
| `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Red first: the busy lock, the refused stores, the usage error, then S01 over the binary. Then green. Then the rest of the replay, the two-process tests, the ignored Python run, the privacy canary, the three TZ runs, clippy, fmt, `cargo tree`. Paste every output.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T029.md` before the first code change
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Tests written AND pass, output pasted
- [ ] Both coexistence runs pasted
- [ ] Supervisor notified: task ready for Stage 4 review
