# TASK_GUIDE — T084: Watch follow-up: alert engine's first look never blocks server start on a hung D-Bus
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
5. Read `rust/crates/app/src/alerts.rs`, `rust/crates/app/src/sampler.rs` (`Sampler::start` runs the first look on the calling thread), `rust/crates/app/src/server.rs` (`start_with_alerts`), and the T074 code-review P2 in `tasks/TASK_REVIEW_T074.md`
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-15): "plan them" — the follow-ups from the Watch Stage 4 reviews (T074, T076, T077) and from the T062 cutover run. Breakdown of 4 tasks approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. `serve --alerts`, `app --alerts` and `tray` print their URL / open the window without waiting for any D-Bus `Notify` call. Today the first look runs inside `start_with_alerts` and may wait up to `NOTIFY_WAIT` (3 s) when the notification service hangs.
> 2. A session already waiting at start still alerts, on the first tick (within one tick of start), not never.
> 3. Keep the rest of T074's behaviour (hysteresis, popup gap, pause, log once).

**Out of scope**: a persistent D-Bus connection, changes to the popup texts or thresholds.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-097 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T074, T076 merged
**Entry point**: `rust/crates/app/src/alerts.rs`

## Dependency rule
No new crate.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Test with a fake notifier that blocks 3 s: `start_with_alerts` (or the engine start) returns in well under 1 s | intent |
| 2 | Test: a session waiting at start produces exactly one alert within the first tick | intent |
| 3 | Existing alerts/server tests pass; `cargo test --workspace`; leak gate `0 hits`, exit 0 | intent |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs alert 2>&1 | tail -3 && cargo test --workspace 2>&1 | grep -E 'test result: FAILED' ; echo done
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☑ | `a_hung_notifier_never_blocks_the_start`, `stop_before_the_first_tick_shows_nothing`, `paused_before_the_first_tick…`, real-tail one-alert test; 23 alert tests pass |
| Verification command | ☑ | Supervisor `/verify` pass |
| code-review | ☑ | inline: 0 findings |
| security-review | ☐ N/A | Risk Low |
| UI: visual regression | ☐ N/A | backend only |
| UI: design-system compliance | ☐ N/A | backend only |
| UI: responsiveness | ☐ N/A | backend only |

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T084.md`.


---

## Approach
Test first (`tdd`). Smallest change that meets the ACs; record cuts.

## Edge Case Checklist
- `stop()` called right after start, before the first tick: no hang, no popup
- Pause set before the first tick: no popup, alert still listed

## Files to Change (Predicted)
`rust/crates/app/src/alerts.rs` (maybe `sampler.rs` if a 'skip first look' option is the smallest fix), tests

## Files Must NOT Touch
`tray.rs` menu logic, `api.rs` routes, `web/**`, `src/easy_personal/**`

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T084.md`
- [ ] Commits on branch `T084` only; final report with `Status:`
