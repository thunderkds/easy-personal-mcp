# TASK_GUIDE — T101: opening the app also starts the tray (one per user)
**Date**: 2026-11-17
**Complexity Level**: C1
**Risk Level**: Low (one source file, `window.rs`; no shared logic, no store or schema change; revert = one commit)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/app/src/window.rs` (`run`, `start`, `open`, `tray`, `run_tray`), `rust/crates/app/src/tray.rs` (`lock_path`, `try_lock`, `desktop_entry`), `rust/crates/app/src/main.rs` (`Command::App`), `rust/crates/app/tests/app.rs`, `rust/crates/app/tests/window.rs`, `rust/packaging/README.md`
- Read relevant files from `docs/legacy/` before making any code changes (this repo has none; `rust/PROJECT_SPEC.md` and `rust/docs/` stand in).
- Never write under `~/.easy-personal`, `~/.claude*`, `~/.local`, `~/.config`. Never run an `#[ignore]` test. Never start a real tray on the owner's desktop from a test: tests use a temp `HOME`/`XDG_RUNTIME_DIR` and `--no-tray` or `--self-test`.

---

## Requirement

**Original request** (owner, 2026-11-17): "when I install the new version, i don't see the tray icon appear when start the application". Supervisor found: the `easy-personal` launcher runs `easy-personal-rs app`, which opens only the window; the tray is a separate launcher (`easy-personal tray`) and no autostart entry exists. Owner picked option 2: "Make opening the app also start the tray, if one isn't running already" (`2`, `[admin]`).

**Restated intent**:
> 1. `easy-personal-rs app --static-dir <DIR>`, once its window is open, starts the tray as its own process (`<this exe> tray --static-dir <same canonical DIR>`) when no tray runs for this user. The tray's existing per-user lock decides: a second tray prints `a tray already runs for this user` and exits 0, so no new lock logic.
> 2. The tray outlives the window: closing the window, or Ctrl-C / SIGTERM to `app`, leaves the tray running (own process group, stdio to null).
> 3. `--self-test` never starts a tray. A new `app --no-tray` flag starts none either (tests, and users who do not want it).
> 4. A failed tray start never fails `app`: one stderr line `easy-personal-rs: tray not started: <reason>` and the window goes on.

**Out of scope**: running the tray inside the `app` process; autostart at login (stays `tray --autostart on`); changing the tray itself, the `.desktop` files, macOS/Windows; the Python app.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-17, session tag `[admin]`, owner picked option 2
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: owner request 2026-11-17; T076 (tray, lock)

---

## Dependencies & Reachability
**Depends on**: T076, T087 (merged)
**Entry points**: the `easy-personal` launcher → `easy-personal-rs app`

## Dependency rule
No new crate.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | A unit test of a pure helper (e.g. `tray_command(exe, root) -> (program, args)`) gives `exe` and `["tray", "--static-dir", <root>]` (RED pasted) | intent 1 |
| 2 | The spawn sets a new process group and null stdin/stdout/stderr; the child is reaped on a thread (no zombie while the window stays open) — shown in code and by a test or the Demonstration | intent 1, 2 |
| 3 | `app --self-test` and `app --no-tray` start no tray: a test proves the decision helper (`wants_tray(self_test, no_tray)`) and `app --help` lists `--no-tray` | intent 3 |
| 4 | A spawn error prints `easy-personal-rs: tray not started: …` and `app` goes on (the error path is a function the test can call) | intent 4 |
| 5 | The tray starts only after the window opened (no tray for a run that fails before: bad folder, no display) — existing `tests/app.rs` cases still pass unchanged in exit code and output | intent 1 |
| 6 | Real machine Demonstration (window build, owner's display, temp-free): with no tray running, `app` → `pgrep -af 'easy-personal-rs tray'` shows one; a second `app` adds none; closing the window leaves the tray | intent 1, 2 |
| 7 | `cd rust && cargo test --workspace`, clippy with and without `--features window` `-D warnings`, `cargo fmt --check`, the leak gate (0 hits) | all |
| 8 | `rust/packaging/README.md` and `rust/README.md` say that opening the app also starts the tray (one line each) | intent 1 |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs --features window window && cargo test --workspace && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

### Evidence (filled by reviewer at Stage 4/5)
> In `tasks/TASK_REVIEW_T101.md`.

## Demonstration
BEFORE: installed 0.3.0, kill the tray, open "easy-personal" → `pgrep -af 'easy-personal-rs tray'` is empty. AFTER: built binary `app --static-dir <web dist>` → one tray process; second `app` → still one; window closed → tray still there.

## Approach
Simplest non-invasive path (Supervisor decision, Low risk): in `shell::open` (or right after it opens the window in `run`), when `wants_tray(self_test, no_tray)`, `std::process::Command::new(current_exe)` with `tray --static-dir <canonical root>`, `CommandExt::process_group(0)`, `Stdio::null()` x3, `spawn()`; move the `Child` into a thread that `wait()`s. No lock check in `app`: the tray's own lock already handles "already runs". Thread `no_tray` from `main.rs` `Command::App`.

Rejected: tray inside the `app` process (two event-loop owners, a second server, tray dies with the window — the owner wants the tray to stay); an autostart entry written by `app` (writes under `~/.config` without asking).

## Edge Case Checklist
- A tray already runs → child prints the line and exits 0; reaped
- `current_exe` fails → stderr line, window goes on
- Ctrl-C in the terminal that ran `app` → tray not hit (own process group)
- `.deb` path `/usr/bin/easy-personal-rs`, dev path `target/...` both work (same exe as `app`)

## Files to Change (Predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/window.rs` | spawn helper, `wants_tray`, call after the window opens; unit tests |
| `rust/crates/app/src/main.rs` | `--no-tray` flag on `App`, pass to `window::run` |
| `rust/crates/app/tests/app.rs` | `--help` lists `--no-tray` (if that is where help is checked) |
| `rust/README.md`, `rust/packaging/README.md` | one line each |

## Files Must NOT Touch
| File | Reason |
|------|--------|
| `rust/crates/app/src/tray.rs` lock and autostart | reused as is |
| `rust/packaging/*.desktop`, `build-deb.sh` | the launcher stays `app` |
| `src/easy_personal/**`, `site/**` | out of scope |

## Test Plan
Unit tests per AC 1-4; existing app/window tests for AC 5; the real-machine Demonstration for AC 6 (Supervisor runs it at Stage 5 on the owner's display); the Verification Command.

## Completion Checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] TASK_GUIDE marked complete

## Questions Log
