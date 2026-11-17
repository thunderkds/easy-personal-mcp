# TASK_GUIDE — T100: `GET /api/agent-run` gives the going run's start time; the Run block counts from it
**Date**: 2026-11-17
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
5. Read `rust/crates/app/src/agent_routes.rs` (`json`, `start`, `last`), `rust/crates/app/src/agent_run.rs` (`Runner::running`, the lock), `web/src/AgentRun.jsx` (`since`, the `Running for` line), `web/src/test/fixtures.js` (`agentRunOn`), `tasks/TASK_REVIEW_T092.md` decision 2 (why the page counts from its own click today)
- Read relevant files from `docs/legacy/` before making any code changes (this repo has none; `rust/PROJECT_SPEC.md` and `rust/docs/` stand in).
- Never run the real `claude`; use the fake `claude` of the existing tests. Never write under `~/.easy-personal`, `~/.claude*`, `~/.local`, `~/.config`. Never run an `#[ignore]` test.

---

## Requirement

**Original request**: follow-up from the T092 build and Stage 4 debate, 2026-11-17 (board row T100; Advocate 2 rated it a must).

**Restated intent**:
> 1. While this server's run is going, `GET /api/agent-run` answers `"started": "<UTC time>"` (the same format as `last.started`); when no run is going, `"started": null`. A run of another process (the CLI `agent-run now`) gives `"started": null` unless the runner already exposes its start time without a new file or format (then use it and say how).
> 2. The Run block counts the elapsed time from `started` when the answer carries it ("Running for m:ss"), on every load and poll; with `started: null` and a run going it keeps today's behaviour ("Running for at least m:ss" from when the page first saw it, or from its own click).
> 3. Clock skew: a `started` later than the page's clock shows `0:00`, never a negative time.

**Out of scope**: a start time for another process's run that needs a new lock-file format; progress text; the run history; any other route; the Python app.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-17, session tag `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `PRD.md` FR-103, FR-104; `tasks/TASK_REVIEW_T092.md` decision 2

---

## Dependencies & Reachability
**Depends on**: T091, T092 (both merged)
**Entry points**: `GET /api/agent-run`; the Run block on Warnings & plan

## Dependency rule
No new crate, no new npm package.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | A route test: after `POST /api/agent-run` (fake `claude` that sleeps), `GET` answers `running: true` and a `started` UTC string not later than the test's clock after the POST; after the run ends, `started: null` and `last.started` equals the value seen while running (RED pasted) | intent 1 |
| 2 | A route test: no run → `started: null`; the key is always present | intent 1 |
| 3 | Vitest: an answer with `running: true, started: <2 min ago>` shows `Running for 2:00` (no "at least"), also when the page did not start the run | intent 2 |
| 4 | Vitest: `running: true, started: null` keeps today's lines (`at least` when first seen by a poll; exact from this page's click) | intent 2 |
| 5 | Vitest: a `started` in the future shows `0:00` | intent 3 |
| 6 | `cd rust && cargo test --workspace`, clippy (with and without `--features window`) `-D warnings`, `cargo fmt --check`, the leak gate (0 hits); `cd web && npm test`, `npm run build`, `python3 scripts/check_static.py` ok | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs agent && cd ../web && npm test && npm run build && cd .. && python3 scripts/check_static.py
```

### Evidence (filled by reviewer at Stage 4/5)
> In `tasks/TASK_REVIEW_T100.md`.

## Demonstration
BEFORE: `GET /api/agent-run` during a run has no `started` key (route test or real `serve` with a fake `claude`). AFTER: it carries the start time, and the Run block shows the exact elapsed time after a page reload.

## Approach
Record the start time in the shared state when `start()` spawns the run (set it in the state before the thread starts, clear it where `busy` goes false), and add `"started"` to `json()`. In `AgentRun.jsx`, prefer `data.started` over the page's own `since` when present. Keep `last` as it is.

## Edge Case Checklist
- A `GET` between `start()` and the thread's first line: `started` already set
- A refused start (409): `started` unchanged
- Stop: `started` cleared when the run ends
- Page clock behind the server: `0:00`, not negative

## Files to Change (Predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/agent_routes.rs` | `started` in the state and in `json()`; route tests |
| `web/src/AgentRun.jsx` | elapsed time from `started` |
| `web/src/AgentRun.test.jsx`, `web/src/test/fixtures.js` | AC 3-5 |

## Files Must NOT Touch
| File | Reason |
|------|--------|
| `rust/crates/app/src/agent_run.rs` lock format | out of scope (another process's start time) |
| `src/easy_personal/**`, `site/**` | out of scope |

## Test Plan
Route tests and vitest per AC; the Verification Command; leak gate.

## Completion Checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] TASK_GUIDE marked complete

## Questions Log
