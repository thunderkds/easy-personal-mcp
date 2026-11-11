# TASK_GUIDE — T012: Remove the code `rules.rs` copied from `metrics.rs`
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T005, T006, T009)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/core/src/rules.rs` from the line `// ---- COPY of private helpers of metrics.rs` to the end, and the `metrics.rs` functions it copies (`parse_iso`, `project_map`, `run_git`, `folder`, and what they call)
6. Read `tasks/TASK_REVIEW_T006.md` and `tasks/TASK_REVIEW_T009.md`

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> `rules.rs` carries about 330 lines copied from the private helpers of `metrics.rs` (T006 could not edit `metrics.rs`). One copy of each helper must remain, so a later fix cannot drift. No behaviour changes.

**Out of scope**: any behaviour change; splitting `rules.rs` or `metrics.rs` into smaller modules (note it as a P3 if you think it is worth doing); `plan`; the app crate; the parity folder.

**Requirement Refs**: RS-003 (parity) in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T006 (rules.rs) and T009 (metrics.rs, local zone) — both landed
**Entry point**: `easy_personal_core::rules::evaluate` and `easy_personal_core::metrics::{usage_summary, usage_breakdown}` (unchanged public API)
**Parallel task**: T007 works in `rust/parity/**` and adds `rust/crates/core/tests/rich_parity.rs`; do not edit those. You own `rust/crates/core/src/**` and the existing `rust/crates/core/tests/*.rs` named below.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | The helpers `rules.rs` copied (`parse_iso`, `project_map`, `run_git`, `folder`, and any helper only they use) exist once: in `metrics.rs`, as `pub(crate)`, or in a new private module `rust/crates/core/src/common.rs` if you move them there (prefer the smaller diff; say which and why). The marked copy section of `rules.rs` is deleted | RS-003 |
| 2 | The two git limits stay as they are: the owner-rule calls wait 5 s and `latest_commit` 10 s. A single function takes the limit as a parameter | RS-003 |
| 3 | The public API (`metrics::usage_summary`, `usage_breakdown`, `rules::evaluate`, `rules::shipped_rules`, `parse_rules`) is unchanged: no signature or JSON change | RS-003 |
| 4 | Every existing test passes unchanged: `usage_parity` (22), `usage_tz_parity` (9), `warnings_parity` (15, incl. the 36 synthetic cases), `store_info`, `git_timeout`, the unit tests, and the app tests | RS-003 |
| 5 | T005's `breakdown_by_day_matches_golden` (in `usage_parity.rs`) is pinned to `TZ=UTC` the way `usage_tz_parity.rs` pins its tests (a child process with `TZ` set), so it passes under `Pacific/Kiritimati` (+14) and `Pacific/Pago_Pago` (-11) as well as UTC. Prove it by running that one test under both zones | T009 finding |
| 6 | Net effect on size: `git diff --stat` shows `rules.rs` smaller by about the copy; paste it | Intent |
| 7 | `git status --short -- src web tests pyproject.toml` is empty before and after; `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the existing test suite: it must stay green and unchanged except for criterion 5.
> **Locked oracle**: rust/parity/golden/api__warnings__all.json, rust/parity/golden/mcp__warnings__all.json, rust/parity/golden/api__usage_breakdown__day.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the refactor | all suites pass unchanged (except criterion 5) | `cargo test` |
| 2 | a grep for `COPY of private helpers` and for a second `fn run_git` / `fn parse_iso` in `rust/crates/core/src` | one definition each | command |
| 3 | `TZ=Pacific/Kiritimati cargo test -p easy-personal-core --test usage_parity` | passes | command |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/src/metrics.rs`.
**Vital slice**: make the four helpers shared, delete the copy, run the suites; then criterion 5.
**Cut list**: no new abstraction; no rename of public items; no split of files.

Format only your own files with `rustfmt --edition 2024 <file>` if another agent is running (`cargo fmt` rewrites everything).

---

## Edge Case Checklist

- [ ] The `run_git` copy in `rules.rs` took a time limit; the original did not — confirm the merged version is behaviourally identical for both callers (arguments, environment, kill-on-timeout)
- [ ] No secret, path or row value in an error (NFR-002)
- [ ] The T009 local-zone code is not touched
- [ ] Stale references: `rules.rs` module comments that mention the copy; `rust/README.md` if it mentions it

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/metrics.rs` | make the helpers shared (`pub(crate)`), add the limit parameter |
| `rust/crates/core/src/rules.rs` | delete the copy section, import |
| `rust/crates/core/src/lib.rs` | only if `common.rs` is chosen |
| `rust/crates/core/tests/usage_parity.rs` | criterion 5 only |
| Pre-existing pin tests | None other than criterion 5 |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/parity/**`, `rust/crates/core/tests/rich_parity.rs` | T007 owns them; the oracle is locked |
| `rust/crates/app/**` | no change needed |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

The suites are the safety net: run them before the change (record the counts), after each step, and at the end. Then the two-zone run of criterion 5.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests pass — output pasted into `tasks/TASK_REVIEW_T012.md`
- [ ] Supervisor notified: task ready for Stage 4 review
