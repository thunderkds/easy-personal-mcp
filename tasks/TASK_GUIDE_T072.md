# TASK_GUIDE — T072: Watch 1: machine status reader + 60 s in-memory sampler, `GET /api/machine`, CLI `machine`
**Date**: 2026-11-15
**Complexity Level**: C2
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
5. Read `BRAINSTORMING_LOG_WATCH.md` (decisions D1-D7, Option A, Edge Case Checklist) and the PRD rows FR-095
6. Read `rust/crates/core/src/lib.rs`, `rust/crates/app/src/api.rs` (route table, guard), `rust/crates/app/src/main.rs` (subcommands), `rust/crates/app/src/live_feed.rs` (how a background task runs beside `serve`/`app`)
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-14): a background service that watches and collects state, with a tray icon and desktop alerts. Path A (compute-on-read, no store migration) chosen 2026-11-14; task breakdown approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. A core module `machine` that reads one *machine status* sample on Linux: CPU busy % (delta of two `/proc/stat` reads), RAM used/total (`/proc/meminfo` MemTotal - MemAvailable), disk used/total for the file system of the store folder (`statvfs`), 1-minute load (`/proc/loadavg`), battery % + charging (`/sys/class/power_supply/*` type Battery; none = "no data"), count of processes whose `comm` is `claude`. Every reader takes a root path so tests use a fake `/proc` and `/sys` tree. A measure that cannot be read is `null` ("no data"), never 0.
> 2. A *sampler*: one sample every 60 s into an in-memory ring buffer of 1,440 samples per measure; nothing is written to the store or to disk. It runs inside `serve` and `app` (the later `tray` reuses it). Memory stays bounded.
> 3. `GET /api/machine` (guarded like the other routes): `{latest, samples:[{t, cpu_pct, ram_used, ram_total, disk_used, disk_total, load1, battery_pct, charging, claude_procs}], interval_s: 60}`.
> 4. CLI `machine` (and `--json`): prints the latest sample, one line per measure, "no data" where null.
> 5. Non-Linux build: every measure "no data", compiles.

**Out of scope**: alerts (T074), plan limits (T073), any UI, any store write, any new route that writes.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-095 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: none
**Entry point**: `rust/crates/core/src/machine.rs` (new), `rust/crates/app/src/api.rs`, `rust/crates/app/src/main.rs`

## Dependency rule
No new crate (use `std` + `libc`/`nix` only if already in `Cargo.lock`; record which).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Unit tests on a fake `/proc` + `/sys` tree: each measure parsed; missing battery = null; unreadable file = null; CPU % from two reads | intent |
| 2 | Ring buffer test: 1,441 pushes keep 1,440, oldest dropped, order kept | intent |
| 3 | API test: `GET /api/machine` shape, guard refuses without token, no write | intent |
| 4 | CLI test: `machine` text and `--json` | intent |
| 5 | `cargo test --workspace` and the leak test pass; real run of `machine` on this host pasted | intent |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test --workspace 2>&1 | tail -5 && cargo run -q -p easy-personal-rs --bin easy-personal-rs -- machine
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
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T072.md`.

## UI / Design Acceptance Criteria
N/A — backend only.

---

## Approach
Test first, one vertical slice at a time (`tdd`). Smallest code that meets the ACs; record cuts.

## Edge Case Checklist
- Container/WSL: `/sys/class/power_supply` empty → battery null
- CPU first sample has no previous read → null, not 0
- Disk path = store folder; folder missing → null
- Process count reads `/proc/<pid>/comm` races (process gone) → skip

## Files to Change (Predicted)
`rust/crates/core/src/machine.rs`, `lib.rs`, `rust/crates/app/src/api.rs`, `main.rs`, `cli_cmds.rs`, tests

## Files Must NOT Touch
`live_feed.rs` read-position logic, store schema, `web/**`, `src/easy_personal/**`

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T072.md`
- [ ] Commits on branch `T072` only; final report with `Status:`
