# TASK_GUIDE — T086: Cutover follow-up: `store-check` on a big store and its hint text; CUTOVER.md steps 0.2 / 0.4 / the week
**Date**: 2026-11-15
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
5. Read `rust/docs/CUTOVER.md` (steps 0.2, 0.4, "The week"), `rust/crates/core/src/store_health.rs` (~l.224 and ~l.315-320: the time budget and the `run \`easy-personal store-check --full\`` hint), the store-check tests, and any parity test that compares this text with the Python app
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-15): "plan them" — the follow-ups from the Watch Stage 4 reviews (T074, T076, T077) and from the T062 cutover run. Breakdown of 4 tasks approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. On the owner's 514 MB store, plain `easy-personal-rs store-check` prints `not fully checked (... not finished in time)` (debug build exit 3) and never `verdict: healthy`; only `--full` (~11 s) does. CUTOVER.md steps 0.4 and "The week" tell the owner to expect `verdict: healthy` from plain `store-check`. Make the doc and the program agree: the doc tells the owner to run `easy-personal-rs store-check --full` where a verdict is needed, and says what the quick check's "not fully checked" means.
> 2. The hint text names the Rust binary: `easy-personal-rs store-check --full`, not the Python `easy-personal`. If a parity test pins the Python text, change only the Rust side and keep the parity test honest (record the intended difference the way other Rust-only texts are recorded).
> 3. CUTOVER.md step 0.2 no longer asks the owner to close every Claude Code session: the backup uses the SQLite backup API and is safe with live readers/writers. Keep "stop the timer" if the steps still need it, and say why.

**Out of scope**: changing the time budget or the integrity check itself, Python app changes, re-running the cutover.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-091 (store health; < 2 s on the owner store per NFR-013), T062 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T062 merged and run
**Entry point**: `rust/docs/CUTOVER.md`, `rust/crates/core/src/store_health.rs`

## Dependency rule
No new crate.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Test: the 'not fully checked' text names `easy-personal-rs store-check --full` | intent |
| 2 | `cutover_doc` test updated/extended: CUTOVER.md names `store-check --full` where it expects `verdict: healthy`, and step 0.2 no longer requires closing every session | intent |
| 3 | `cargo test --workspace` and the leak gate pass | intent |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs --test cutover_doc 2>&1 | tail -3 && cargo test --workspace 2>&1 | grep -E 'test result: FAILED'; echo done
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Verification command | ☐ | |
| code-review | ☐ | |
| security-review | ☐ N/A | Risk Low |
| UI: visual regression | ☐ N/A | backend only |
| UI: design-system compliance | ☐ N/A | backend only |
| UI: responsiveness | ☐ N/A | backend only |

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T086.md`.


---

## Approach
Test first (`tdd`). Smallest change that meets the ACs; record cuts.

## Edge Case Checklist
- Small store: quick check still finishes and says `healthy`
- Release vs debug build exit codes for 'not fully checked' — document what the owner sees with the installed (release) binary

## Files to Change (Predicted)
`rust/docs/CUTOVER.md`, `rust/crates/core/src/store_health.rs`, `rust/crates/app/tests/cutover_doc.rs`, store-check tests

## Files Must NOT Touch
`src/easy_personal/**`, `web/**`, the owner's real store or systemd units

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T086.md`
- [ ] Commits on branch `T086` only; final report with `Status:`
