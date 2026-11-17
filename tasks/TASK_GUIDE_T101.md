# TASK_GUIDE — T101: opening the app starts the tray, and the tray owns the only window
**Date**: 2026-11-17 (rewritten the same day)
**Complexity Level**: C2
**Risk Level**: Medium (3 source files, a signal between processes, the main launcher changes behaviour; no store or schema change)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/app/src/window.rs` (`run`, `start`, `open`, `tray`, `run_tray`, `tray_loop`, `show_dashboard`, `watch_clicks`, `watch_signals`), `rust/crates/app/src/tray.rs` (`lock_path`, `try_lock`, `desktop_entry`), `rust/crates/app/src/main.rs` (`Command::App`, `Command::Tray`), `rust/crates/app/tests/app.rs`, `rust/crates/app/tests/window.rs`, `rust/packaging/README.md`
- Read relevant files from `docs/legacy/` before making any code changes (this repo has none; `rust/PROJECT_SPEC.md` and `rust/docs/` stand in).
- Never write under `~/.easy-personal`, `~/.claude*`, `~/.local`, `~/.config`. Never run an `#[ignore]` test. Never start a real tray on the owner's desktop from a test, and never signal a process you did not start: tests use a temp `HOME`/`XDG_RUNTIME_DIR` and `--no-tray` or `--self-test`.

---

## Requirement

**Original request** (owner, 2026-11-17): "when I install the new version, i don't see the tray icon appear when start the application". Supervisor found: the `easy-personal` launcher runs `easy-personal-rs app`, which opens only the window; the tray is a separate launcher and no autostart entry exists. Owner picked: "Make opening the app also start the tray, if one isn't running already" (`2`, `[admin]`).

**Rewritten 2026-11-17** after the owner asked: "when click on the notification, do we open the current window, and direct to the notif in current screen, not open the new window, right?" Supervisor found: a popup click is handled by the tray (`show_dashboard`, `window.rs`), which reuses only *its own* window; the launcher's `app` window is another process, so a click opened a second window. Owner picked "Yes, tray owns the window": the tray process owns the only dashboard window.

**Restated intent**:
> 1. A plain `easy-personal-rs app --static-dir <DIR>` (no `--self-test`, no `--no-tray`) opens no window of its own: it starts `<this exe> tray --static-dir <same canonical DIR> --show` as a detached process (own process group, null stdio) and exits 0. If that spawn fails: one stderr line `easy-personal-rs: tray not started: <reason>` and `app` opens its own window as today.
> 2. `tray --show`: when it takes the tray lock it runs the tray as today and opens the dashboard window at start. When another tray holds the lock it asks that tray to show its window (hand-over, below) and exits 0; a failed hand-over prints one line and exits 1.
> 3. Hand-over: the running tray writes its pid into its lock file after taking the lock (through the held file: truncate, then the pid as decimal text). The asker reads the pid (waits up to 2 s for it to appear), checks `/proc/<pid>/cmdline` is an `easy-personal-rs` running `tray`, and sends `SIGUSR1`. No pid (a tray from an older version) means no signal: an older tray has no handler and `SIGUSR1` would end it.
> 4. The tray on `SIGUSR1` runs `show_dashboard(Target::Dashboard)` on the main thread: an open window is unminimized and focused as it is (no navigation); with none, it opens one. So the launcher, the tray menu and a popup click all land in the one window; a popup click still moves it to the Live view of its session (unchanged).
> 5. `app --self-test` and `app --no-tray` keep today's behaviour exactly (own window, no tray). Closing the window leaves the tray running (unchanged).

**Out of scope**: autostart at login (stays `tray --autostart on`); the `.desktop` files (the launcher stays `app`); macOS/Windows; the Python app; any change to popup click routing other than intent 4.

### Options considered (stands in for the brainstorming log; owner chose 2026-11-17)
| Option | Invasiveness | Regression risk | Chosen |
|---|---|---|---|
| A. `app` keeps its window and also spawns a tray (first T101 design) | Low | a popup click opens a second window | no (owner question) |
| B. Tray owns the only window; `app` hands over and exits | Medium | old tray without pid; signal to a wrong pid (guarded) | **yes** |
| C. Two processes; the tray asks the `app` window to navigate | High | a new tray-to-app channel, two windows still possible | no |

### Requirement Fidelity Gate
- [x] Restated intent confirmed: owner picked option B via question, 2026-11-17, `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: owner requests 2026-11-17; T076 (tray, lock); T095/T096 (popup → Live view)

---

## Dependencies & Reachability
**Depends on**: T076, T087, T096 (merged)
**Entry points**: the `easy-personal` launcher → `easy-personal-rs app`; `easy-personal-rs tray --show`; a popup click

## Dependency rule
No new crate (`libc` is already a dependency, for `kill`; tokio's `SignalKind::user_defined1` for the handler).

## Partial work already in the worktree (from the first design; keep what fits)
Uncommitted on branch `T101`: `--no-tray` on `App` (main.rs), `wants_tray`, `tray_command`, `tray_not_started`, `spawn_detached` (process group 0, null stdio, reaping thread) and 4 unit tests in `window.rs`; `--no-tray` in `tests/app.rs` and `tests/window.rs`; one README line each; the Demonstration BEFORE in `tasks/TASK_REVIEW_T101.md` (keep it: it is valid for this design too). Change `tray_command` to add `--show`, make `app` exit after the spawn instead of opening a window, and fix the README lines to the new behaviour.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Unit: `tray_command(exe, root)` gives `exe` and `["tray", "--static-dir", <root>, "--show"]`; `wants_tray` is false for `--self-test` or `--no-tray` (RED pasted) | 1, 5 |
| 2 | Unit: the detached child has its own process group and null stdio; a spawn error gives the `tray not started:` line and the caller falls back to its own window (the fallback is a decision function the test calls) | 1 |
| 3 | Unit (tray.rs): after `try_lock` succeeds, `write_pid` leaves exactly the pid text in the file and the lock is still held; `read_pid` gives `None` for an empty or non-numeric file, and retries within its limit (test with a short limit) | 3 |
| 4 | Unit: the cmdline check accepts `easy-personal-rs\0tray\0...` (any folder) and refuses another program, `app`, an empty cmdline and a gone pid; no signal is sent when it refuses or when there is no pid | 3 |
| 5 | Integration (temp `XDG_RUNTIME_DIR`, no display needed): a helper holds the lock with the pid of a process that is not a tray → `tray --show` sends no signal and exits 1 with one line; a held lock with no pid → exits 1 with one line after the wait | 2, 3 |
| 6 | Window build: the tray handles `SIGUSR1` by calling `show_dashboard(.., Target::Dashboard)` on the main thread (code, plus the Stage 5 Demonstration) | 4 |
| 7 | `tray --help` lists `--show`; `app --help` lists `--no-tray`; existing `tests/app.rs` and `tests/window.rs` cases pass (they use `--self-test` or `--no-tray`, or fail before any window) | 5 |
| 8 | Real machine Demonstration (Supervisor, Stage 5, owner's display): no tray → `app` → one tray and one window, the `app` process gone; `app` again → still one tray, one window, focused; close the window, `app` → the tray opens one window; a popup click → the same window | 1-4 |
| 9 | `cd rust && cargo test --workspace` (with and without `--features window`), clippy with and without `--features window` `-D warnings`, `cargo fmt --check`, the leak gate (0 hits) | all |
| 10 | `rust/README.md` and `rust/packaging/README.md`: opening the app starts the tray, which owns the window; quit an older tray once after upgrading (one line each) | 1, 3 |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test --workspace --features window && cargo test --workspace && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

### Evidence (filled by reviewer at Stage 4/5)
> In `tasks/TASK_REVIEW_T101.md`.

## Demonstration
BEFORE (already captured): the `app` code path starts no tray; `app --help` has no `--no-tray`. AFTER: AC 8 on the owner's display.

## Approach
`app` plain → `spawn_detached(tray_command(..))`, exit 0; on a spawn error → today's window. `tray --show` → `run_tray(dir, show)`: lock taken → `write_pid`, run, open the window at start when `show`; lock held → `hand_over(lock_path)` = `read_pid` (2 s) → `is_tray(pid)` via `/proc/<pid>/cmdline` → `libc::kill(pid, SIGUSR1)`. In `tray_loop`, a `SIGUSR1` watcher (tokio `signal(SignalKind::user_defined1())` on the server runtime, like `watch_signals`) → `handle.run_on_main_thread(|| show_dashboard(.., Target::Dashboard))`. Pure helpers in `tray.rs` (no Tauri), the Tauri wiring in `window.rs`.

## Edge Case Checklist
- Two launches in quick succession: the second `tray --show` finds the lock held, waits for the pid, hands over
- A tray from 0.3.0 still running (no pid in its lock): no signal, one line, exit 1; the README line says to quit the old tray once after upgrading
- A pid reused by another program: the cmdline check refuses
- The pid write must keep the lock: write through the held `File`, never reopen or replace the path
- `SIGUSR1` before the handler exists: install the handler before the event loop starts; an earlier signal is a known gap (the window opens at start anyway with `--show`)

## Files to Change (Predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/window.rs` | `app` hand-over to `tray --show`; `run_tray(dir, show)`; `SIGUSR1` watcher; unit tests |
| `rust/crates/app/src/tray.rs` | `write_pid`, `read_pid`, `is_tray` (cmdline check), `hand_over`; unit tests |
| `rust/crates/app/src/main.rs` | `--no-tray` on `App`, `--show` on `Tray` |
| `rust/crates/app/tests/app.rs`, `tests/window.rs`, a tray integration test | AC 5, 7 |
| `rust/README.md`, `rust/packaging/README.md` | one line each |

## Files Must NOT Touch
| File | Reason |
|------|--------|
| `rust/crates/app/src/alerts.rs` | popup click routing unchanged |
| `rust/packaging/*.desktop`, `build-deb.sh` | the launcher stays `app` |
| `src/easy_personal/**`, `site/**` | out of scope |

## Test Plan
Unit and integration tests per AC 1-5 and 7; the Verification Command; the Stage 5 real-machine Demonstration (AC 6, 8) by the Supervisor on the owner's display.

## Completion Checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] TASK_GUIDE marked complete

## Questions Log
