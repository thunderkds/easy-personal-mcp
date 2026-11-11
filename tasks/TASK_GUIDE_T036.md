# TASK_GUIDE — T036: Remove the `display::tests` flake (a just-dropped socket checked without the `refused_soon` wait)
**Date**: 2026-11-11
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/app/src/display.rs` (the `tests` module, `refused_soon`, and the two sibling tests that already use it)
6. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "continue the next tasks as parallel"; the P3 hardening item recorded in the Rust close-out.

**Restated intent**:
> The test `when_both_fail_the_message_names_each_variable_and_no_path_or_value` in `rust/crates/app/src/display.rs` failed once in a full run (0 of 30 in isolation): it drops a `UnixListener` and calls `check` at once, while another test that forks a child may still hold a copy of the socket descriptor, so the socket can still answer for a few milliseconds. The sibling tests wait with `refused_soon`; this one does not. Make it wait the same way, keep what it asserts, and prove the flake is gone.

**Out of scope**: any change outside the `tests` module of `display.rs`; changing `check` or any product code; other tests.

**Requirement Refs**: the P3 hardening list in `rust/docs/closeout-2026-11-11.md` (no FR or NFR applies).

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist (the close-out file)

---

## Dependencies & Reachability

**Depends on**: Standalone — N/A
**Entry point**: Standalone — N/A

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Reproduce first**: show the flake or, if it will not reproduce, say so and show the cause by reading (the dropped socket, the fork in another test); a BEFORE run of the test 30 times in isolation and once in a loaded full run is pasted | measured |
| 2 | The test waits until the dropped socket stops answering before it calls `check`, using the existing `refused_soon` (and a small helper only if two sockets need it); every assertion and message string it checks stays exactly as it is | no weaker test |
| 3 | Stress proof: the whole `display` test module run 200 times in a loop with other test binaries running at the same time (for example `cargo test -p easy-personal-rs --lib display` repeated while `cargo test -p easy-personal-rs --test cli` runs in a second shell) shows 0 failures; paste the counts | flake gone |
| 4 | `cargo test --workspace` under `TZ` unset (paste the summary); clippy and fmt clean (also `--features window`); `git status` shows only `rust/crates/app/src/display.rs` and `tasks/TASK_REVIEW_T036.md`; `git status -- src tests web` empty | quality |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T036.md`.

### Success Criteria

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the module run 200 times under load | 0 failures | pasted loop output |
| 2 | `git diff --stat` | only `display.rs` | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo test -p easy-personal-rs --lib display && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: the two sibling tests `a_unix_socket_that_listens_answers_and_a_closed_or_missing_one_does_not` and `a_dead_tcp_display_fails_fast_and_never_takes_longer_than_the_limits_added` (both call `refused_soon` after the drop).
**Vital slice**: add the wait after `drop(UnixListener::bind(&secret))`, then the loop proof.
**Cut list**: no new abstraction, no change to `check`, no sleeps added to the product code.

---

## Edge Case Checklist

- [ ] `refused_soon` returns a bool: assert it, never ignore it, so a socket that never closes fails with a clear message and no path
- [ ] Assertion messages carry no path or value (the test checks exactly that for `check`'s error)
- [ ] Stale references: the comment on `refused_soon` stays true

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/display.rs` | the `tests` module only |
| `tasks/TASK_REVIEW_T036.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**` | the Python app |
| every other file under `rust/crates/**`, `rust/Cargo.*` | not part of this task |
| `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

BEFORE (30 isolated runs, one loaded), the one-line fix, AFTER (200 looped runs under load, workspace tests, clippy, fmt). Paste every output.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T036.md` before the first change
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] The 200-run loop output pasted
- [ ] Supervisor notified: task ready for Stage 4 review
