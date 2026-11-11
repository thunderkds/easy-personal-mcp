# TASK_GUIDE — T034: `easy-personal-rs schedule install|remove` — the systemd user timer for scheduled ingest (P2c)
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the 2026-11-11 entry "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read the Python you copy, **read-only**: `src/easy_personal/core/schedule.py`, `_schedule` and the `schedule` parser in `src/easy_personal/surfaces/cli.py`, the unit templates `src/easy_personal/data/systemd/*`, and any Python test of them (`grep -rl schedule tests`)
6. Read the Rust you extend: `rust/crates/app/src/main.rs` (the subcommand table, how `ingest` is wired), `rust/crates/app/src/ingest_cmd.rs`, and `rust/crates/app/tests/ingest_cli.rs` (spawn helpers)
7. **Never touch the real systemd or the real `~/.config`.** Every run and every test sets `XDG_CONFIG_HOME` and `HOME` to temp folders and puts a **fake `systemctl`** (a shell script that records its arguments to a file) first on a `PATH` built from scratch. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "continue the next tasks as parallel"; P2c of `BRAINSTORMING_LOG_RUSTP2.md` ("scheduler waits for P2c").

**Restated intent**:
> `easy-personal-rs schedule install [--retention-days N] [--dry-run]` and `schedule remove [--dry-run]` behave like Python's `easy-personal schedule ...`: the same unit files, the same messages, the same exit codes and the same `systemctl --user` calls, so a Rust-only machine can run its ingest daily; the one difference is that the service's command line runs the Rust binary (`easy-personal-rs ingest`).

**Out of scope**: running a real systemd; cron installation (only the printed cron snippet, as Python); the dashboard's start-up ingest; any change to the Python app; macOS and Windows.

**Requirement Refs**: the Python scheduler requirements in the repo-root `PRD.md` (search "schedule"); RS-003 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T029 (the `ingest` command)
**Entry point**: `easy-personal-rs schedule` (`rust/crates/app/src/main.rs`, new `schedule_cmd.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Oracle first**: a script `rust/parity/gen_schedule.py` (new) runs the **real Python** `schedule install --dry-run`, `schedule install --retention-days 30 --dry-run`, `schedule remove --dry-run`, and a real (not dry) `install` and `remove` against a temp `XDG_CONFIG_HOME` with a fake `systemctl` on a scratch `PATH`, and stores the outputs, exit codes, written files and recorded `systemctl` calls as goldens under `rust/parity/schedule/golden/`. The `ExecStart` command is normalised (the command path differs on purpose, criterion 3). `--check` shows 0 different | locked oracle |
| 2 | **Same units**: the Rust crate embeds the two unit templates as a copy of `src/easy_personal/data/systemd/*` (a Rust-local copy, because the crate must build alone) and a test compares the copies with the Python files byte for byte, so a change on the Python side fails the Rust test; the template substitution (`exec_start`) is identical | parity |
| 3 | **The command**: the unit's `ExecStart` runs `<absolute path of the running binary, shell-quoted> ingest [--retention-days N]` (Python quotes with `shlex.quote`; copy the quoting for a path with a space or a quote); decided for you: it is the Rust binary, not `easy-personal` | the one difference |
| 4 | **Behaviour** copied from `_schedule`: `--dry-run` prints and writes nothing (exit 0); no `systemctl` or no user session (`systemctl --user show-environment` fails) prints the two Python lines (message, cron snippet `17 3 * * * <ingest command>`), writes nothing, exit 1; install writes both unit files, runs `daemon-reload` then `enable --now <timer>`; remove runs `disable --now` only when the timer file existed, deletes both files (missing is fine), then `daemon-reload` only if it had been installed; a failing `systemctl` call is a clean exit 1 with a message that has no path or output of the tool | CLI parity |
| 5 | **Same unit names** as Python (`easy-personal-ingest.service` and `.timer`): installing from Rust replaces Python's units and the reverse; this is deliberate (one scheduled ingest per machine) and is stated in the README; decided for you | one timer |
| 6 | **Safety**: a test proves that with `XDG_CONFIG_HOME` pointing at a temp folder nothing is written outside it and the fake `systemctl` is the only one called; no test ever runs without both guards; the code never reads `~/.easy-personal`; no new dependency (use `std::process::Command`, no shell) | host safety |
| 7 | **No regression**: `cargo test --workspace` under `TZ` unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago` (paste all three); clippy and fmt clean (also `--features window`); `git status -- src tests web` empty; the `--help` text lists `schedule` | quality |
| 8 | **Docs**: a "Scheduled ingest" section in `rust/README.md` (the commands, same unit names, the Rust command line, how to remove it) and in `rust/parity/README.md` (the oracle); `docs/release/**` is not edited | usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T034.md`.
> **Locked oracle**: the Python scheduler and its unit templates (read-only).

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the 3 dry-run cases and a real install/remove with a fake `systemctl` | files, messages, exit codes and `systemctl` calls equal Python's (command line normalised) | `gen_schedule.py --check` and the replay test |
| 2 | no `systemctl` on `PATH` | exit 1, the two Python lines, nothing written | test |
| 3 | `git status -- src tests web` | empty | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain and rust/parity/.venv (the oracle half)
. "$HOME/.cargo/env" && cd rust && cargo build && ./parity/.venv/bin/python parity/gen_schedule.py --check && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `ingest_cmd.rs` (a subcommand module with its own exit codes), `rust/parity/gen_ingest.py` (guards, temp folders), T031's spawn helpers.
**Vital slice**: oracle goldens → `schedule_cmd.rs` with `--dry-run` → install and remove with the fake `systemctl` → the no-systemd path → docs.
**Cut list**: no cron install, no real-systemd test, no generic "job" abstraction, no unit-name option.

---

## Edge Case Checklist

- [ ] `XDG_CONFIG_HOME` unset: use `$HOME/.config` as Python does; empty string counts as unset (Python's `or`)
- [ ] The binary path has a space, a quote or a non-ASCII character: quoted so `systemd` reads it back as one word
- [ ] A directory that cannot be created or a file that cannot be written: clean exit 1, no path in the message
- [ ] `systemctl` missing from `PATH` versus present but failing: both covered
- [ ] The Python crash cases: record any case where Python raises a traceback (for example an unwritable directory) and say what Rust does instead ("decided for you")
- [ ] Stale references: `BRAINSTORMING_LOG_RUSTP2.md` task table (do not edit), `rust/README.md` "scheduler" mentions

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/schedule_cmd.rs` (new), `rust/crates/app/src/main.rs` | the subcommand |
| `rust/crates/app/units/easy-personal-ingest.{service,timer}` (new) | the embedded template copies |
| `rust/crates/app/tests/schedule_cli.rs` (new) | golden replay, guards, byte-compare with Python's templates |
| `rust/parity/gen_schedule.py`, `rust/parity/schedule/**` (new) | the oracle |
| `rust/README.md`, `rust/parity/README.md` | docs |
| `tasks/TASK_REVIEW_T034.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/crates/core/**`, `rust/crates/app/src/ingest_cmd.rs` (another task edits it) | not part of this task |
| older parity files and goldens | the oracle |
| `skills/verify/SKILL.md`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.config/**`, `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real machine |

---

## Test Plan

Oracle first (red: no `schedule` subcommand), then dry-run, install/remove with the fake `systemctl`, the no-systemd path, the host-safety test, workspace tests under three `TZ` values. Paste every output.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T034.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] `gen_schedule.py --check` and three `TZ` runs pasted
- [ ] "Decided for you" list written (Rust command line, same unit names, Python crash cases)
- [ ] Supervisor notified: task ready for Stage 4 review
