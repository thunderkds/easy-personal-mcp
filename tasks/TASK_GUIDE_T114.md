# TASK_GUIDE — T114: Find `claude` when the app's PATH lacks `~/.local/bin`
**Date**: 2026-11-18
**Risk Level**: Medium
**Assigned agents**: backend-developer

> Read relevant files from `docs/legacy/` before making any code changes.

## Bug Fix Task Guide — T114

### Mental Model (confirmed by user, 2026-11-18, "yes, fix it")
- Observed: the desktop/tray app (`/usr/bin/easy-personal-rs tray`, started by `systemd --user`) shows
  "Claude Code was not found on this machine, so no session can start." on App sessions, though `claude` is
  installed at `~/.local/bin/claude`. The tray's PATH is
  `~/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/games:/usr/local/games:/snap/bin`
  — no `~/.local/bin` (that folder is added by the login shell only).
- Expected: when `claude` is not on the process PATH, the app also finds it at the places Claude Code's
  installers put it — `$HOME/.local/bin/claude`, then `$HOME/.claude/local/claude` — as long as it is an
  executable regular file. Then `GET /api/agent-run` answers `claude_found: true`, App sessions can start a
  session, and Agent run / connectors / doctor find it the same way. A `claude` on PATH still wins.
- Likely divergence point: every lookup goes through `claude_cli::which(PATH, "claude")`
  (`rust/crates/app/src/claude_cli.rs:55`), which searches PATH only. Three callers:
  `SystemClaude::which` (`claude_cli.rs:49`, used by `on_path`/`run` → connectors, doctor, legacy, sync),
  `AgentRun::claude()` (`rust/crates/app/src/agent_run.rs:342` → `claude_found`, Agent run spawn),
  `app_session.rs:1103` (App session start).
- Recent context: no regression — the lookup has been PATH-only since T050; it surfaced now because App
  sessions (T103/T104) are used from the .deb tray, which systemd starts with a minimal PATH.

### Intake
- Trigger: install the .deb, let the tray start from login/autostart (parent `systemd --user`), open App
  sessions with an agent run account chosen.
- Severity: P1 (App sessions and Agent run blocked in the desktop app; workaround = add `~/.local/bin` to the
  systemd user PATH via `~/.config/environment.d/`).
- Affected area: `rust/crates/app/src/claude_cli.rs`, `agent_run.rs`, `app_session.rs`.

### Complexity & Risk
- Complexity: C1
- Risk: Medium (shared lookup used by 3 callers)

## Acceptance criteria
- [x] AC1: one shared function (e.g. `find_claude(path, home)` in `claude_cli.rs`) returns PATH's `claude` if any,
      else `$HOME/.local/bin/claude`, else `$HOME/.claude/local/claude` — each only if an executable regular file.
- [x] AC2: all three callers use it; `HOME` is read from the same env source each caller already uses for PATH
      (`self.env` in `AgentRun` / app sessions; the process env in `SystemClaude::new`), so tests stay hermetic.
- [x] AC3: with a PATH that lacks the folder and `HOME` holding `.local/bin/claude`, `GET /api/agent-run`
      answers `claude_found: true` (route/unit test).
- [x] AC4: no PATH hit, no fallback file, or a fallback that is a directory / not executable → not found
      (existing `claude_found: false` tests stay green; add the negative cases).
- [x] AC5: a `claude` on PATH is still chosen over the fallback.
- [x] AC6: the launcher lookup (`LAUNCHER`) and `core/src/jobs.rs` are unchanged.

## Approach
Add the fallback in one place in `claude_cli.rs`, next to `which`; switch the three `which(..., "claude")`
calls to it. No PATH mutation, no shell, no new config. The fallback path is absolute already.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/claude_cli.rs` | `find_claude` + `SystemClaude` keeps `HOME`; tests |
| `rust/crates/app/src/agent_run.rs` | `claude()` uses `find_claude(PATH, HOME)` |
| `rust/crates/app/src/app_session.rs` | session start uses `find_claude` |
| `rust/crates/app/src/agent_routes.rs` | test for AC3 (if the route tests are the natural place) |

## Files must NOT touch
| File | Reason |
|------|--------|
| Python `src/easy_personal/**` | out of scope: the desktop app is the Rust binary; Python parity divergence noted for the owner |
| `rust/crates/core/src/jobs.rs` | read-only allow-list tool lookup, unrelated |
| packaging / systemd units | fix lives in the lookup, not the environment |
| `web/**` | the page already shows the right text for the server's flag |

## Test plan
- Unit tests in `claude_cli.rs` for AC1/AC4/AC5 with temp dirs (fake `HOME`, fake PATH).
- Route test for AC3.
- `cd rust && cargo test -p easy-personal-app` (or the workspace), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- Repro loop: run `env -i HOME=$HOME PATH=/usr/bin:/bin <built binary> serve` (or the tray) and
  `curl` `/api/agent-run` → `claude_found` false before, true after.

### Diagnosis Gates (Pillar 1 — must pass before any fix)
- [x] Phase 1 feedback loop built and running (`env -i HOME=<fake home> PATH=/usr/bin:/bin <binary> serve --no-ingest` + `curl /api/agent-run`; script in the session scratchpad)
- [x] Bug reproduces deterministically on the loop (`claude_found: false` on every run; the loop starts a fresh server each time)
- [x] 3–5 ranked falsifiable hypotheses listed (consistent with confirmed mental model; see Attempts Log)
- [x] Correct hypothesis identified via Phase 4 instrumentation (H1; the same binary and fake HOME, only `PATH` changed: no code instrumentation needed)

### Attempts Log (filled live during diagnosis — required if >1 hypothesis tested)
| # | Hypothesis | Predicted signal | Actual result | Verdict |
|---|---|---|---|---|
| 1 | H1: the lookup searches `PATH` only; the fake `claude` in `$HOME/.local/bin` is never looked at | same binary, same fake `HOME`: `PATH=/usr/bin:/bin` → `claude_found: false`; `PATH=/usr/bin:/bin:<home>/.local/bin` → `true` | `false`, then `true` | confirmed |
| 2 | H2: the file is the problem (not executable / not a regular file, so `is_executable` refuses it) | `claude_found` false even with the folder on `PATH` | `true` with the folder on `PATH` (file is `-rwxr-xr-x`, regular) | rejected |
| 3 | H3: the server caches `claude_found` at start, so a late `claude` is never seen | the flag stays false after a `claude` appears on a running server | not tested live; read instead: `claude_found` is `runner.claude().is_some()` per request (`agent_routes.rs:123`), no cache | rejected by reading, not by a run |
| 4 | H4: `HOME` is dropped from the tray's env, so a fallback would have nothing to read | the tray's `/proc/<pid>/environ` lacks `HOME` | not run on the real tray (never read the real env); `systemd --user` always sets `HOME`, and `AgentRun` / app sessions hold the whole `std::env::vars_os()` (`agent_run.rs:192`) | not an issue |

### Fix Gates (Pillar 2)
- [x] Regression test written before the fix (or no-seam documented) — commit `c8159bb`, 3 tests red on their assertion
- [x] Fix applied; regression test passes
- [x] Phase 1 loop no longer reproduces the bug (`claude_found: true`)
- [x] Fix matches the "correct behaviour" definition from the mental model (PATH wins; then `~/.local/bin/claude`, `~/.claude/local/claude`; executable regular files only)

### Cleanup Checklist (Pillar 3)
- [x] All [DEBUG-...] instrumentation removed (grep verified; none was added)
- [x] Throwaway prototypes deleted (the repro script and fake home live in the session scratchpad, outside the repo)
- [x] Correct hypothesis stated in commit message
- [x] Post-mortem: what would have prevented this? A smoke run of the .deb tray under `systemd --user`'s PATH (or `env -i`) when App sessions landed (T103/T104); every `claude` lookup was a PATH-only copy of Python's `shutil.which`, and a desktop launch never has the login shell's PATH.

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** Filled by the reviewer at Stage 4/5 in `tasks/TASK_REVIEW_T114.md`.

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_T114.md`.

## Completion checklist
- [x] Implementation done
- [x] Lint passes
- [x] Tests pass (workspace: 1329 passed; 2 load-flaky `store_check` signal tests pass alone, see the review file)
- [x] Smoke test confirmed (repro loop, fake HOME)
- [ ] `docs/legacy/` updated (if new insights)
- [ ] `memory/` updated (if new patterns learned)
- [ ] TASK_GUIDE marked complete
