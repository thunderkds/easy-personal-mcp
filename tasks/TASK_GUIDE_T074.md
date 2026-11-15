# TASK_GUIDE — T074: Watch 3: alerts with hysteresis + desktop notifications over D-Bus, `GET /api/alerts`, pause
**Date**: 2026-11-15
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_WATCH.md` (decisions D1-D7, Option A, Edge Case Checklist) and the PRD rows FR-097, FR-050, NFR-001, NFR-002
6. Read `rust/crates/app/src/live_feed.rs` and `rust/crates/core/src/live.rs` (attention states), T072's `machine.rs` and T073's limits watch, and `rust/crates/app/src/keychain.rs` for how `zbus` is already used
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-14): a background service that watches and collects state, with a tray icon and desktop alerts. Path A (compute-on-read, no store migration) chosen 2026-11-14; task breakdown approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. An alert engine that takes the current states and raises an *alert* once per state change: session *waiting for you* or *failing* (live tail states), plan limit ≥ 90 % per account and window (re-armed below 85 %), disk ≥ 90 % used, RAM ≥ 90 % used (each re-armed below 85 %), battery ≤ 15 % and not charging (re-armed at ≥ 20 % or charging). A session alert re-arms when that session leaves the state.
> 2. Text names only the state, tool name, project folder name, short session id or the measure and its number. Never message text, tool input or output; pass every text through the existing secret check.
> 3. A `Notifier` trait; the real one calls `org.freedesktop.Notifications.Notify` over the session D-Bus with the existing `zbus` dependency, and fails soft (no D-Bus → log once, keep going). Tests use a fake notifier.
> 4. `GET /api/alerts`: the last 50 alerts (time, kind, text, account/session) and `paused`. `POST /api/alerts/pause` with `{paused: bool}` (guarded like other writes, in memory only). Pause stops popups, never the state or the list.
> 5. Runs in `serve`/`app` only when started with `--alerts` (default off there; `tray` turns it on in T076).

**Out of scope**: tray icon (T076), UI (T077), sounds, email, any store write, MCP/CLI surfaces for attention states.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-097, FR-050, NFR-001, NFR-002 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T072, T073 merged
**Entry point**: new `rust/crates/app/src/alerts.rs`, `api.rs`, `main.rs` flag

## Dependency rule
No new crate (`zbus` is already a dependency; record the version).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Hysteresis tests per measure: value 89→91→90→86→91 raises 2 alerts, 84→91 re-arms | intent |
| 2 | Session tests: waiting → alert; still waiting → none; leaves and comes back → second alert | intent |
| 3 | Leak test: planted secret / message text in a fake live state never reaches the notifier or `/api/alerts` | intent |
| 4 | Pause test: paused → notifier not called, `/api/alerts` still lists | intent |
| 5 | D-Bus missing → no panic, one log line; real run on this desktop: one notification seen (screenshot or `dbus-monitor` line pasted) | intent |
| 6 | `cargo test --workspace` and the leak test pass; `security-review` at Stage 4 (Risk Medium) | intent |

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
| security-review | ☐ | |
| UI: visual regression | ☐ N/A | backend only |
| UI: design-system compliance | ☐ N/A | backend only |
| UI: responsiveness | ☐ N/A | backend only |

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T074.md`.

## UI / Design Acceptance Criteria
N/A — backend only.

---

## Approach
Test first, one vertical slice at a time (`tdd`). Smallest code that meets the ACs; record cuts.

## Edge Case Checklist
- Flapping value at a threshold (hysteresis)
- Two dashboards + tray: only the process started with `--alerts` notifies
- Many sessions waiting at once → at most 1 popup per 10 s, rest summarised ("3 sessions waiting")
- Clock jump → no burst

## Files to Change (Predicted)
`rust/crates/app/src/alerts.rs`, `api.rs`, `main.rs`, `server.rs`, tests

## Files Must NOT Touch
`live_feed.rs` read-position logic, store schema, `web/**`, `src/easy_personal/**`, any Claude config

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T074.md`
- [ ] Commits on branch `T074` only; final report with `Status:`
