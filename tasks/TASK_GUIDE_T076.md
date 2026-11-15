# TASK_GUIDE — T076: Watch 5: `tray` command — tray icon (4 states), native menu, alerts on, XDG autostart
**Date**: 2026-11-15
**Complexity Level**: C3
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
5. Read `BRAINSTORMING_LOG_WATCH.md` (decisions D1-D7, Option A, Edge Case Checklist) and the PRD rows FR-099, FR-097
6. Read `rust/crates/app/src/window.rs`, `rust/crates/app/icons/README.md` (T071 icons), `docs/design/graphite-mint/Tray*.dc.html` and `docs/design/graphite-mint/README.md`, T074's `alerts.rs`
7. HITL: at the start, show the owner the menu rows you will build from the canvas and wait for an OK (the Supervisor relays)
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-14): a background service that watches and collects state, with a tray icon and desktop alerts. Path A (compute-on-read, no store migration) chosen 2026-11-14; task breakdown approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. `tray` subcommand behind `--features window`: one process runs the live tail, the T072/T073 sampler and T074 alerts (on), and shows a tray icon. State: problem (any failing or pressure alert active) > attention (waiting / limit ≥ 90 %) > active (any active session) > idle. Icon from `icons/tray/<dark|light>/<state>-<size>.png`; theme from the desktop setting when readable, else dark.
> 2. Linux: native menu with the popover rows of the canvas (state line, sessions waiting, 5 h / 7 d %, machine line), "Pause alerts" toggle, "Open dashboard" (opens the existing window), "Quit".
> 3. No tray area (no StatusNotifier / AppIndicator host) → print once that the tray is not shown; alerts keep working.
> 4. One `tray` per user: a lock in the runtime folder; a second start says so and exits 0.
> 5. `tray --autostart on|off` writes / removes `$XDG_CONFIG_HOME/autostart/easy-personal-tray.desktop` (only that file).

**Out of scope**: macOS/Windows tray, light theme for the dashboard, a click popover (AppIndicator gives no click events), packaging.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-099, FR-097 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T072, T073, T074 merged; T071 icons (merged)
**Entry point**: `rust/crates/app/src/tray.rs` (new), `main.rs`, `window.rs`

## Dependency rule
No new crate beyond Tauri v2 features already pinned (`tray-icon` is a Tauri feature; record any feature flag added).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | State-pick unit test over alert/session inputs | intent |
| 2 | Autostart test on a temp `XDG_CONFIG_HOME`: on writes the exact file, off removes it, nothing else touched | intent |
| 3 | Single-instance test | intent |
| 4 | `cargo build --features window` + suite pass; real run on this desktop: screenshot of the tray icon and the open menu, compared with the canvas by LLM vision | intent |
| 5 | `security-review` at Stage 4 (Risk Medium) | intent |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo build --features window 2>&1 | tail -2 && cargo test --workspace 2>&1 | tail -5
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Verification command | ☐ | |
| code-review | ☐ | |
| security-review | ☐ | |
| UI: visual regression | ☐ | |
| UI: design-system compliance | ☐ | |
| UI: responsiveness | ☐ | |

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T076.md`.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| tray icon + open menu | screenshot, LLM vision vs `docs/design/graphite-mint/` | matches the design |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Colours | grep for raw hex / pixel check of icon colours | tokens / T071 icon set only |

### 3. Layout / Responsiveness
| Viewport | Verification method | Expected result |
|---|---|---|
| N/A (desktop tray) | screenshot | menu readable on dark and light panels |

---

## Approach
Test first, one vertical slice at a time (`tdd`). Smallest code that meets the ACs; record cuts.

## Edge Case Checklist
- GNOME without AppIndicator
- Display gone at start (reuse the window's display preflight with a timeout)
- Window opened from the menu twice → focus the existing one
- Feature build swaps the binary: run window checks last, then plain `cargo build`

## Files to Change (Predicted)
`rust/crates/app/src/tray.rs`, `main.rs`, `window.rs`, `Cargo.toml` (feature only), tests

## Files Must NOT Touch
`web/**`, store schema, `src/easy_personal/**`, any desktop config other than the one autostart file

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T076.md`
- [ ] Commits on branch `T076` only; final report with `Status:`
