# TASK_GUIDE — T038: Two P3 hardening items: the stale doc comment in `folders.rs` and the 15-round cap in the readers coexistence test
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
5. Read `rust/crates/core/src/ingest/folders.rs` (the top doc comment and `read_from`, `save_file_streamed` use), `rust/crates/core/src/writer.rs` (`save_file`, `save_file_streamed`), and `rust/crates/app/tests/ingest_coexist.rs` around the "readers while writing" test (the `rounds < 15` check and its message)
6. Read `tasks/TASK_REVIEW_T032.md` and `tasks/TASK_REVIEW_T031.md` (where both items were recorded)
7. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "continue the next tasks as parallel"; open item 2 of the Rust close-out (P3 hardening).

**Restated intent**:
> (a) The module comment at the top of `folders.rs` says files are saved through `Store::save_file`, but since T032 the product saves through `Store::save_file_streamed`; make the comment true. (b) The "readers while writing" coexistence test gives up after 15 rounds without 50 polls during an ingest; on a slow or loaded machine that is a false failure of the test, not of the product. Replace the round cap with a time deadline that fits the documented limits, keeping the same requirement (50 polls during an ingest, never a 5xx, valid JSON).

**Out of scope**: any change to product behaviour, `writer.rs` logic, other tests, the display flake (T036).

**Requirement Refs**: the P3 hardening list in `rust/docs/closeout-2026-11-11.md` (no FR or NFR applies).

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist (the close-out file)

---

## Dependencies & Reachability

**Depends on**: T032, T031
**Entry point**: Standalone — N/A

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | The doc comment of `folders.rs` names `Store::save_file_streamed` where it is now the call, and still says one transaction per file; read every line of the comment and fix any other claim that is no longer true (list each); the code is not changed | true comment |
| 2 | In `ingest_coexist.rs` the `rounds < 15` assertion becomes a wall-clock deadline (a named constant, default 120 s, with a comment saying why), the 50-poll requirement and every other assertion stay as they are, and the failure message still carries no path | no false failure |
| 3 | The affected test file is run in full with the real-Python half too: `cargo test -p easy-personal-rs --test ingest_coexist` and `cargo test -p easy-personal-rs --test ingest_coexist -- --ignored` (about 400 s); paste both summaries; run the readers test alone 5 times and paste the pass count | still green |
| 4 | `cargo test --workspace` under `TZ` unset (paste the summary); clippy and fmt clean (also `--features window`); `git status` shows only `folders.rs`, `ingest_coexist.rs` and `tasks/TASK_REVIEW_T038.md`; `git status -- src tests web` empty | quality |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T038.md`.

### Success Criteria

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the readers test 5 times | 5 of 5 | pasted |
| 2 | `git diff --stat` | the two files only, no product line changed in `folders.rs` | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain; the ignored half needs rust/parity/.venv
. "$HOME/.cargo/env" && cd rust && cargo test -p easy-personal-rs --test ingest_coexist && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: the other deadline loops in `ingest_coexist.rs` and its `common` helpers.
**Vital slice**: the comment, then the deadline, then the runs.
**Cut list**: no refactor of the test, no new helper unless the deadline needs one.

---

## Edge Case Checklist

- [ ] The deadline does not make the test slower when the machine is fast (it ends at 50 polls, as now)
- [ ] The comment fix does not change a line of code (`git diff -w` shows comments only in `folders.rs`)
- [ ] Stale references: `rust/README.md` "Coexistence tests" note if it mentions 15 rounds

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/ingest/folders.rs` | the top doc comment only |
| `rust/crates/app/tests/ingest_coexist.rs` | the cap becomes a deadline |
| `rust/README.md` | only if it states the 15 rounds |
| `tasks/TASK_REVIEW_T038.md` | Demonstration BEFORE only (verbatim prior comment and prior assertion) |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**` | the Python app |
| `rust/crates/core/src/writer.rs`, any other product source | not part of this task |
| `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

BEFORE (quote the comment and the assertion), the two edits, the runs listed in criterion 3, workspace tests, lint. Paste every output.

---

## Completion Checklist

- [ ] BEFORE (verbatim prior text) in `tasks/TASK_REVIEW_T038.md`
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] The two coexistence summaries and the 5-run count pasted
- [ ] Supervisor notified: task ready for Stage 4 review
