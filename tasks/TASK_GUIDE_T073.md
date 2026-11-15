# TASK_GUIDE — T073: Watch 2: plan-limit watch per account (5 h / 7 d %) in the sampler, `GET /api/limits/history`
**Date**: 2026-11-15
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_WATCH.md` (decisions D1-D7, Option A, Edge Case Checklist) and the PRD rows FR-096
6. Read `rust/crates/core/src/accounts.rs` (how `easy-personal-limits.json` is read today and when it is stale) and the T068 top-bar 5 h chip route it feeds
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-14): a background service that watches and collects state, with a tray icon and desktop alerts. Path A (compute-on-read, no store migration) chosen 2026-11-14; task breakdown approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. Each 60 s, for each agent account, take the 5 h and 7 d used % from that account's status-line snapshot `<account>/easy-personal-limits.json`, using the existing reader in `accounts.rs` (no second parser). Missing, broken or *stale* (reset time passed) = null ("no data").
> 2. Keep them in an in-memory 24 h ring buffer per account (1,440 samples), nothing on disk.
> 3. `GET /api/limits/history`: `{accounts:[{id, label, samples:[{t, five_h_pct, seven_d_pct}]}], interval_s: 60}`.
> 4. Coordination: T072 builds the machine sampler in parallel. Make the limit sampler its own small struct with its own 60 s tick (or a shared tick only if T072 is already on `main`); do not edit T072's files. The Supervisor merges T072 first and resolves any `api.rs` route-table conflict.

**Out of scope**: alerts (T074), machine status (T072), UI, any write to the account folder.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-096 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: none (parallel with T072)
**Entry point**: `rust/crates/core/src/accounts.rs` reader, new `limits_watch` module, `rust/crates/app/src/api.rs`

## Dependency rule
No new crate.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Test: two accounts, one fresh, one stale, one missing file → samples with values / null / null | intent |
| 2 | Ring buffer bound test (1,440) | intent |
| 3 | API test: shape, guard, no write | intent |
| 4 | `cargo test --workspace` and the leak test pass | intent |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test --workspace 2>&1 | tail -5
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Verification command | ☐ | |
| code-review | ☐ | |
| UI: visual regression | ☐ N/A | backend only |
| UI: design-system compliance | ☐ N/A | backend only |
| UI: responsiveness | ☐ N/A | backend only |

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T073.md`.

## UI / Design Acceptance Criteria
N/A — backend only.

---

## Approach
Test first, one vertical slice at a time (`tdd`). Smallest code that meets the ACs; record cuts.

## Edge Case Checklist
- Account added while running → appears at the next tick
- Account removed → its buffer dropped
- Snapshot with a future reset time but percent > 100 → keep the value as read, flag nothing

## Files to Change (Predicted)
new `rust/crates/core/src/limits_watch.rs`, `lib.rs`, `rust/crates/app/src/api.rs`, tests

## Files Must NOT Touch
T072's `machine.rs`, `accounts.rs` parsing logic (call it, do not change it unless a bug; then say so), `web/**`, `src/easy_personal/**`

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T073.md`
- [ ] Commits on branch `T073` only; final report with `Status:`
