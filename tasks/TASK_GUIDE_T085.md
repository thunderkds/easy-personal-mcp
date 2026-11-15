# TASK_GUIDE — T085: Leak gate hardening: run `serve --alerts` over the planted transcripts; store-check 'no file beside the store' stable under load
**Date**: 2026-11-15
**Complexity Level**: C2
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
5. Read `rust/parity/leak_rust.py` (how `serve` is started at ~l.571/805/1155, how routes and CLI commands are registered, the store-check checks at ~l.1182), `rust/parity/README.md`, T074 security-review P3 and T076 fix-round note in `tasks/TASK_REVIEW_T074.md` / `tasks/TASK_REVIEW_T076.md`
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-15): "plan them" — the follow-ups from the Watch Stage 4 reviews (T074, T076, T077) and from the T062 cutover run. Breakdown of 4 tasks approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. The gate starts one `serve` with `--alerts` (with a session bus address that leads nowhere, or a stand-in, so no real popup reaches the owner's desktop), lets at least one alert tick run over the planted transcripts (a planted session waiting with a canary in its project path / tool name / session id), then scans `GET /api/alerts`. A canary in the alert list is a hit.
> 2. The check `store-check made no file beside the store (...)` must not fail just because the machine is loaded: under load the `-wal` / `-shm` files of a previous `serve` were still there. Find the real cause (stop order, a missing wait for the server process to exit, or the check listing too early) and fix the gate, not the app, unless the app really leaves files after a clean stop (then say so and stop: that is an app bug for the Supervisor).
> 3. The gate still ends `leak check: 0 hits ...` with no failed check and exit 0 on main.

**Out of scope**: new canaries beyond the alert path, changes to app code (unless point 2 proves an app bug — then report, do not fix).

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: NFR-002, FR-097 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T074, T076 merged
**Entry point**: `rust/parity/leak_rust.py`

## Dependency rule
No new Python package (the parity venv is locked).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | The gate run lists `/api/alerts` from a `serve --alerts` surface with at least 1 alert in it (paste the surface line) | intent |
| 2 | Mutant: make `alerts::clean` return its input unchanged → the gate reports a hit (paste), then revert | intent |
| 3 | Gate passes 3 runs in a row with an artificial load (e.g. 16 busy loops) — paste the 3 last lines and the load average | intent |
| 4 | No real notification reached the desktop during the runs (say how you made sure) | intent |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo build -q && env -u CLAUDE_CONFIG_DIR /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/rust/parity/.venv/bin/python parity/leak_rust.py; echo exit=$?
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☑ | new gate checks: alerts surface, bus leads nowhere, health-route sidecar rule; mutant gives 4 hits |
| Verification command | ☑ | Supervisor gate runs exit 0, 0 FAILED |
| code-review | ☑ | inline: 0 P0-P2; ⚠ -shm/-wal accepted as normal |
| security-review | ☐ N/A | Risk Low |
| UI: visual regression | ☐ N/A | backend only |
| UI: design-system compliance | ☐ N/A | backend only |
| UI: responsiveness | ☐ N/A | backend only |

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T085.md`.


---

## Approach
Test first (`tdd`). Smallest change that meets the ACs; record cuts.

## Edge Case Checklist
- No D-Bus session bus in the gate's environment → engine logs once, list still filled
- Popup gap (10 s) longer than the gate wants to wait → read the list, not the popups

## Files to Change (Predicted)
`rust/parity/leak_rust.py`, maybe `rust/parity/README.md`

## Files Must NOT Touch
`rust/crates/**` (report an app bug instead), `web/**`, `src/easy_personal/**`, the parity venv lock

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T085.md`
- [ ] Commits on branch `T085` only; final report with `Status:`
