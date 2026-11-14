# TASK_GUIDE — T066: Retire Python follow-ups — connector input, legacy/doctor "claude cannot run", keychain order (T050/T060 Stage 4 leftovers)
**Date**: 2026-11-14
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P2
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `tasks/TASK_REVIEW_T050.md` and `tasks/TASK_REVIEW_T060.md` (the P2/P3 findings this task closes are listed there with file:line)
6. Read the Rust: `rust/crates/app/src/{connector_cmd,doctor_cmd}.rs`, `rust/crates/core/src/{keychain,launch,legacy,connector_writes}.rs` (or the files the reviews name if the names differ)
7. Read the Python oracle, **read-only**: `src/easy_personal/surfaces/cli.py` (connector add/secret input), `src/easy_personal/core/legacy.py`, the doctor command
8. Never touch the real keychain, a real `claude`, `~/.easy-personal`, `~/.claude*`: the debug fake keychain and the fake `claude` only (as T050, T060)

---

## Requirement

**Original request** (owner, 2026-11-14): "note all the work for the next wake, I will work on each" (handoff items 4 and 5); then "continue your work as parallel [admin]".

**Restated intent**:
> Close the small correctness gaps the T050 and T060 Stage 4 reviews left open, before Python is retired, so the Rust `connector`, `doctor` and legacy paths behave like Python (the oracle) or fail honestly:
> (a) a secret read from piped stdin keeps a trailing `\r` exactly as Python does (strip only the final `\n`);
> (b) when `/dev/tty` cannot be opened but stdin is a terminal, echo is still turned off on stdin; the `tcsetattr` result is checked (on failure: refuse to read the secret, fixed error text, no echo of input); a test proves echo is restored after the read;
> (c) `legacy.rs` no longer reads "could not run `claude`" as "no entry": adopt-legacy and the doctor note report that `claude` could not run;
> (d) `doctor` prints one FAIL line "`claude` could not run" instead of one FAIL per unconfigured catalog id when `claude` cannot run;
> (e) the test that hard-codes `/usr/bin/script` finds `script` on `PATH` and is skipped with a printed reason if absent;
> (f) T050 review: the order of keychain delete versus store-row delete leaves no orphan secret and no row pointing at a missing secret on failure (follow Python's order unless Python's order loses data; then state why); `kill()` on timeout stops the whole `claude` process group, not only the direct child.

**Out of scope (cut list, decided for the owner in `[admin]` mode)**: D-Bus call timeout under the write lock (recommendation: skip); AC9 Python/Rust keyring interop test (touches the real Secret Service; the owner runs it); the launcher 200 ms measurement on the real keychain; the `#[path]` double compile of `keychain.rs`; unused `dbus` entries in `Cargo.lock`; the `claude` call inside a write transaction (report in Evidence if (f) shows it matters; do not redesign).

**Requirement Refs**: NFR-002, NFR-008 (`PRD.md`); `rust/docs/PYTHON_RETIREMENT_PLAN.md` T060 row. Check the numbers; fix this line if one is wrong.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the owner (handoff list 2026-11-14; "[admin]" go-ahead 2026-11-14)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T050, T060 (merged on `main`)
**Entry point**: `easy-personal-rs connector add|remove ...`, `easy-personal-rs doctor`, `easy-personal-rs connector adopt-legacy` (check the real subcommand names in `connector_cmd.rs`)

## Dependency rule

No new crate, feature or lockfile change. `libc`/`nix` only if already a dependency.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Piped secret `"abc\r\n"` is stored as `"abc\r"` (fake keychain), as Python stores it; `"abc\n"` → `"abc"`; `"abc"` → `"abc"` | (a) |
| 2 | With `/dev/tty` unopenable and stdin a pty: echo is off during the read and restored after (test drives a pty); a failing `tcsetattr` refuses with a fixed text, exit non-zero, input never echoed | (b) |
| 3 | With a fake `claude` that cannot run (missing / not executable): adopt-legacy and the doctor note say `claude` could not run; with a `claude` that answers "no entry" the old text stays | (c) |
| 4 | `doctor` with `claude` unrunnable prints exactly one FAIL line for it, not one per catalog id | (d) |
| 5 | No test hard-codes `/usr/bin/script` | (e) |
| 6 | A failure injected between keychain delete and row delete (fake keychain) leaves a state Python would leave, or a strictly safer one (no secret without row, no row without secret); a timed-out fake `claude` that spawns a grandchild leaves no live process | (f) |
| 7 | Whole Rust suite, clippy (default and `--features window` if it builds), fmt, leak script: pass; numbers pasted | all |

---

## Evaluation & Acceptance

### Success Criteria
- AC 1-7 each shown by a named new test, output pasted.

### Verification Command
```bash
cd rust && cargo test --workspace 2>&1 | tail -20 && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Full suite passes | ☐ | |
| Leak script 0 hits | ☐ | |
| code-review | ☐ | |
| security-review (Risk High) | ☐ | |
| UI: visual regression | ☐ N/A | no UI |
| UI: design-system compliance | ☐ N/A | no UI |
| UI: responsiveness | ☐ N/A | no UI |

---

## Demonstration
BEFORE: run each AC test on `main` (they fail). AFTER: same tests pass on the branch. DELTA: list per AC. WITNESS: test names + pasted output.

## UI / Design Acceptance Criteria
N/A — pure backend.

---

## Approach
Test first per item (a)→(f), one commit per item. Python is the oracle; where you diverge from Python on purpose, add a line to the T060 (or T050) "Decided for you" list in `rust/README.md`.

## Edge Case Checklist
- Empty secret; secret of only `\r\n`; non-UTF-8 bytes on stdin
- stdin not a tty and `/dev/tty` absent (no prompt, plain read)
- signal during the read: echo restored
- fake `claude` exits 127 vs exits 1 with "no entry" text

## Files to Change (Predicted)
`rust/crates/app/src/connector_cmd.rs`, `rust/crates/app/src/doctor_cmd.rs`, `rust/crates/core/src/legacy.rs`, `rust/crates/core/src/keychain.rs`, `rust/crates/core/src/launch.rs` (or where `kill()` lives), their tests, `rust/README.md` lists.

## Files Must NOT Touch
`src/easy_personal/**` (Python app), `rust/parity/run_parity.py`, `rust/parity/gen_*.py` oracles, `Cargo.lock` (except as cargo rewrites it unchanged), the dashboard bundle.

## Test Plan
One new test per AC; reuse T050/T060 fakes (debug fake keychain, fake `claude` on a scratch `PATH`). Keep tests TZ-independent and TMPDIR-independent.

## Completion Checklist
- [ ] All AC tests added and passing
- [ ] README "Decided for you" lines added where Rust differs from Python on purpose
- [ ] Commits on branch `T066` only; report with `Status:` and the Evidence numbers
