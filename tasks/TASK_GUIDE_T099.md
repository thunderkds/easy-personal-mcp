# TASK_GUIDE — T099: terminal `account ignore` of the agent run account also turns the agent run off
**Date**: 2026-11-17
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P2
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `tasks/TASK_REVIEW_T091.md` decision 5 (the debate that made this task), `rust/crates/app/src/cli_cmds.rs` (`run_account`, `AccountAction::Ignore`), `rust/crates/app/src/agent_run.rs` (`Runner::forget`, `flag`), `rust/crates/app/src/agent_routes.rs` (`account_gone`, the route-side twin of this fix), `rust/crates/app/src/writes.rs` (where the ignore route calls `account_gone`, about line 185)
- Read relevant files from `docs/legacy/` before making any code changes (this repo has none; `rust/PROJECT_SPEC.md` and `rust/docs/` stand in).
- **Never run the real `claude`** and never run against the owner's real setup: no write under `~/.easy-personal`, `~/.claude*`, `~/.local`, `~/.config`. Every test uses a throw-away `EASY_PERSONAL_HOME`. Never run an `#[ignore]` test. Do not kill any process you did not start.

---

## Requirement

**Original request**: follow-up from the T091 Stage 4 debate, 2026-11-17 (2 of 3 Advocates wanted it inside T091); board row T099.

**Restated intent**:
> When `easy-personal-rs account ignore <account>` succeeds and that account is the agent run account (the flag file names its id), the flag is removed: the setting turns off, and a run already going sees the flag gone at its next poll and stops. This is what the dashboard's ignore route already does through `agent_routes::account_gone`. Another account's flag, no flag, and a failed ignore leave the flag as it is.

**Out of scope**: a status or store check inside the run's poll (all three Advocates ruled it out: a store open every 50 ms, a busy store would stop a healthy run); the P2 400-for-503 on the setting route; `account confirm`, `add`, `use`; the Python app; any UI.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-17, session tag `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `PRD.md` FR-101, FR-103; `tasks/TASK_REVIEW_T091.md` decision 5

---

## Dependencies & Reachability
**Depends on**: T091 (Ready for Review, auto verify pass; this branch is stacked on branch `T091`)
**Entry points**: `easy-personal-rs account ignore <account>`

## Dependency rule
No new crate.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | After a successful `account ignore` of the account the flag names, the flag file is gone and `agent-run status` says off: a test against a throw-away home (RED pasted before the change) | intent |
| 2 | The id passed to `forget` comes from the row the ignore returned (the numeric id, as the route uses it), not the raw CLI argument, so `account ignore <id>` and `account ignore <folder path>` both work (the command takes no label): a test for each | intent |
| 3 | Another account's flag stays; no flag stays no flag; a failed ignore (unknown account) leaves the flag: one test each | intent |
| 4 | A run already going stops at its next poll after the terminal ignore: a test with the fake `claude` already used by the T091 tests (`agent_run::tests::Fake`), no real `claude` | intent |
| 5 | `cd rust && cargo test --workspace`, `cargo clippy --workspace --all-targets --features window -- -D warnings`, `cargo fmt --check` pass; leak gate unchanged at 0 hits | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs agent && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

### Evidence (filled by reviewer at Stage 4/5)
> In `tasks/TASK_REVIEW_T099.md`.

## Demonstration
BEFORE: a throw-away home, flag names account A, `account ignore A`, flag still there. AFTER: flag gone, `agent-run status` off.

## Approach
In `run_account`, `AccountAction::Ignore`: after `write(...)` returns the body, read the ignored row's id from it and call the same helper the route uses (`agent_routes::account_gone(&dir, id)`, or `Runner::forget` with the agent run dir), ignoring its I/O error exactly as the route does. One call, no new type. If the body does not carry the id, stop and report rather than re-resolving by name.

## Edge Case Checklist
- Ignore of an account that is already ignored (no-op ignore): flag rule as for any success
- Flag file empty (default account): `forget` compares ids, an empty flag never equals an id — keep that
- Flag dir missing: no error

## Files to Change (Predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/cli_cmds.rs` | one `forget` call after a successful ignore |
| a test file next to the existing agent run / CLI tests | AC 1–4 tests |

## Files Must NOT Touch
| File | Reason |
|------|--------|
| `rust/crates/app/src/agent_run.rs` poll loop | no status check in the poll (debate) |
| `web/**`, `src/easy_personal/**` | out of scope |

## Test Plan
Unit/integration tests per AC; the Verification Command; leak gate run.

## Completion Checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] TASK_GUIDE marked complete

## Questions Log
