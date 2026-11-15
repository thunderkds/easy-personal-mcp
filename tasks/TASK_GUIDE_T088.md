# TASK_GUIDE — T088: Tray menu: "Recent alerts" block (newest 5 alerts, listed even while paused)
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
5. Read `BRAINSTORMING_LOG_TRAYALERTS.md` (Option A, Edge Case Checklist, User Selection)
6. Read `rust/crates/app/src/tray.rs` (`view`, `Inputs::view`, `same_shape`, tests), `rust/crates/app/src/alerts.rs` (`Shared`, `Alert`, `clean`), `rust/crates/app/src/window.rs` (`tray_loop`, menu update at ~line 500-540, `OPEN` handler)
7. Read `tasks/TASK_REVIEW_T076.md` `## Menu proposal` (the approved rows this task extends)
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-15): "in the tray, any notif should be shown". Brainstormed in `BRAINSTORMING_LOG_TRAYALERTS.md`; owner chose Option A (AskUserQuestion, 2026-11-15).

**Restated intent**:
> 1. The `tray` menu gets a new block after the machine line: separator, info row `Recent alerts`, then one info row per alert, newest first, at most 5: `HH:MM · <alert text>` (local time; an alert from an earlier local day: `MM-DD HH:MM · <text>`).
> 2. When the list holds more than 5 alerts, one action row `All alerts in dashboard…` follows; it does what `Open dashboard` does (reuse the `OPEN` id or a new id with the same handler).
> 3. With 0 alerts the block (separator included) is not shown: the menu equals today's approved menu.
> 4. Alerts raised while "Pause alerts" is on are listed too (pause stops popups only).
> 5. Popups, `POPUP_GAP`, the summary merge, the alert rules and `GET /api/alerts` are unchanged.

**Out of scope**: an unread count / "Mark all read" (Option C), one popup per alert (Option B), persisting alerts across restarts, fixing `app` + `tray` running two engines.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (Option A preview chosen 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: extends FR-099 (tray menu rows) and FR-097 (alert list); no new FR

---

## Dependencies & Reachability
**Depends on**: T074, T076, T084 merged
**Entry point**: `rust/crates/app/src/tray.rs::view`

## Dependency rule
No new crate. Local time formatting: use what the crate already has (check `Cargo.toml` / `easy_personal_core::clock`); record the choice.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Unit test: 0 alerts → `view` rows equal today's rows (no block) | intent 3 |
| 2 | Unit test: 3 alerts → header + 3 rows newest first, exact `HH:MM · text` labels; no more row | intent 1 |
| 3 | Unit test: 5 alerts → 5 rows, no more row; 6 and 50 alerts → 5 rows + `All alerts in dashboard…` action | intent 1, 2 |
| 4 | Unit test: alert from an earlier local day shows `MM-DD HH:MM` | intent 1 |
| 5 | Unit test: paused + alerts → rows still listed and `Pause alerts` ticked | intent 4 |
| 6 | Unit test: `same_shape` is false when the block first appears and true between two looks with the same alert count; existing tray and alerts tests pass unchanged | intent 1, 5 |
| 7 | Real run on this desktop: `tray` with a fixture that raises alerts; screenshot of the open menu showing the block (owner visual check, Gate 6) | intent 1 |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs -- tray:: 2>&1 | tail -5 && cargo build --features window 2>&1 | tail -2 && cargo test --workspace 2>&1 | tail -5
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☑ | 9 new tests (tray.rs 8, alerts.rs 1); Supervisor rerun 2026-11-15 `cargo test -p easy-personal-rs --features window --bin easy-personal-rs -- tray:: window:: alerts::` → `test result: ok. 50 passed; 0 failed` |
| Verification command | ☑ | agent: workspace 1006 passed, 1 failed (`connector_cli_parity::every_connector_and_doctor_step_equals_the_python_cli`, fails on main too: installed `/usr/bin/easy-personal-launch` on PATH, RUNBOOK.md known failure); Supervisor: clippy window all-targets clean, fmt ok |
| code-review | ☑ | Supervisor inline 2026-11-15: 0 P0, 0 code P1, 0 P2, 3 P3 (see TASK_REVIEW_T088.md `## Stage 4`); 1 P1 process (startup reads) |
| security-review | ☐ N/A | Risk Low: alert text is already `clean`ed by the engine (NFR-002); no new input |
| UI: visual regression | ☐ | |
| UI: design-system compliance | ☐ | |
| UI: responsiveness | ☐ N/A | native text menu |

## Demonstration
> BEFORE/AFTER/DELTA/WITNESS go in `tasks/TASK_REVIEW_T088.md`.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| open tray menu with 3 and with 6 alerts | screenshot, owner check | block as in the owner-chosen preview (`BRAINSTORMING_LOG_TRAYALERTS.md` User Selection) |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Labels | unit tests on exact strings | `Recent alerts`, `HH:MM · text`, `All alerts in dashboard…`; separator `·` as the other rows |

### 3. Layout / Responsiveness
| Viewport | Verification method | Expected result |
|---|---|---|
| N/A (native menu) | screenshot | rows readable; long project names do not make the menu wider than today's longest row by much (cap text, record the cap) |

---

## Approach
Test first (`tdd`). `Shared::recent(n)` (newest first, clones under the lock) in `alerts.rs`; `view` gets a `&[Alert]` argument; `Inputs::view` passes `self.alerts.recent(6)` (6 = 5 shown + know whether more exist). Smallest code that meets the ACs; record cuts.

## Edge Case Checklist
- 0 / 1 / 5 / 6 / 50 alerts
- Paused
- First alert arrives → shape change → rebuild path in `window.rs` (no stale menu)
- Long text: cap row length, ellipsis `…`; text is already cleaned, do not re-clean differently
- `t` is UTC seconds; local day boundary for the `MM-DD` form
- Alert list lock: never hold it while building Tauri items

## Files to Change (Predicted)
`rust/crates/app/src/alerts.rs`, `rust/crates/app/src/tray.rs`, maybe `rust/crates/app/src/window.rs` (only if a new action id), `rust/README.md` tray section

## Files Must NOT Touch
Alert rules, `POPUP_GAP`, `DBusNotifier`, `/api/alerts` route, `web/**`, store schema, `src/easy_personal/**`

## Test Plan
The ACs above, each as an automated test (1-6), plus AC 7 real run and the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T088.md`
- [ ] Commits on branch `T088` only; final report with `Status:`
