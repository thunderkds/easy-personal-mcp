# TASK_GUIDE — T009: Local-timezone parity for `usage_summary` / `usage_breakdown`
**Date**: 2026-11-10
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 "Rust port" entries in `memory/decisions.md` and `memory/learnings.md` (especially "usage metrics: rounding, floats, git, time zone")
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/docs/spike-T001.md`, `rust/parity/README.md`, `tasks/TASK_REVIEW_T005.md` (the P2 finding)
6. Read the Python code: `src/easy_personal/core/metrics.py` lines 14-60 (`_utc_bound`, `_period`) and 200-230 (`_keys`), and every other place in that file or in `src/easy_personal/core/` that calls `astimezone()` with no argument or builds a day from a UTC stamp (grep `astimezone`, `localtime`, `strftime("%Y-%m-%d")`)
7. Read the Rust code you will change: `rust/crates/core/src/metrics.rs` (the `utc_bound`, `period`, day-key paths), `rust/crates/core/tests/usage_parity.rs`, and the generator `rust/parity/gen_golden.py` with its `README.md`

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> On a machine whose zone is not UTC, the Rust core answers `usage_summary` and `usage_breakdown` with the same JSON as the Python app: `by=day` groups by the **local** day and an offset-less `since` / `until` is read as **local** time, both DST-aware, in the zone the OS (or the `TZ` variable) names.

**Why it blocks**: today the core reads both as UTC. On the owner's machine (Asia/Bangkok, +07) a day boundary is 7 hours off, so a Rust surface would show different daily totals from the Python dashboard. T005's review (P2) set this as the gate before any usage surface is wired.

**Out of scope**: any other function; new goldens for kit, board, accounts (T007); generator hardening (T011); Windows zone names (P5: note what `jiff` or the chosen crate does there, do not build it); HTTP/MCP wiring.

**Requirement Refs**: RS-003 (parity) in `rust/docs/SPEC.md`; Python `PRD.md` NFR-013.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T005 (done, Ready for Review) — `metrics::usage_summary`, `usage_breakdown`; T003 — the generator and goldens
**Entry point**: `easy_personal_core::metrics::usage_summary` and `usage_breakdown` (library functions); the new zone code is used only by them
**Parallel task**: T006 runs in the `core` crate (`rules.rs`, `lib.rs`, `tests/warnings_parity.rs`). Do not edit `rules.rs` or `warnings_parity.rs`. Re-read `lib.rs`, `rust/Cargo.toml` and `rust/Cargo.lock` immediately before every edit and change only your own lines; `cargo` may print "waiting for file lock". T008 owns `rust/crates/app/**`: do not edit it. If `cargo test` fails only in `warnings_parity` because T006 is unfinished, say so and run `cargo test -p easy-personal-core --test usage_parity --test store_info --test git_timeout --lib` instead.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | **Crate choice first.** Pick the zone source with a short spike recorded in `rust/docs/spike-T009.md`: candidate `jiff` (reads the OS zone and `TZ`, bundles or reads tzdata) versus `chrono` + `iana-time-zone` + `chrono-tz`. Record: exact pinned version, MSRV vs 1.90, that it honours `TZ=Asia/Bangkok` on this machine, that it reads the system tz database or bundles one (and which), build-time cost, and how it behaves on macOS and Windows (documented, not built). Pin with `=` like the other crates | RS-003, NFR-001 |
| 2 | With `TZ=Asia/Bangkok`, `usage_breakdown(by=day)` over the fixture store returns the day keys Python returns under the same `TZ` (a record at `…T20:00Z` lands on the **next** day) | RS-003 |
| 3 | With `TZ=Asia/Bangkok`, an offset-less `since` / `until` (date and datetime forms) produces the same `period.since_utc` / `until_utc` as Python, so `usage_summary` and `usage_breakdown` windows pick the same records | RS-003 |
| 4 | A zone with DST (`America/New_York`) matches Python on: a day that has 23 hours (2026-03-08) and one with 25 (2026-11-01), a record just before and after the change, an offset-less `since` that falls in the skipped hour (02:30 on 2026-03-08) and one in the repeated hour (01:30 on 2026-11-01). Python's rule is `datetime.astimezone()` on a naive value: fold=0 (the first occurrence) and the skipped hour is shifted by the offset in force **before** the change. Match it and say so in a code comment | RS-003 |
| 5 | A bound that already has an offset (`Z`, `+07:00`) ignores the zone; `TZ=UTC` still equals every existing golden (the T005 suite stays green unchanged) | RS-003 |
| 6 | **Oracle for non-UTC zones.** Extend `rust/parity/gen_golden.py` (you own it for this task; the Supervisor lifts the lock) with an opt-in mode that writes goldens for `usage_summary` (window with offset-less bounds), `usage_breakdown by=day`, and `usage_breakdown by=model` with an offset-less window, under each of `TZ=Asia/Bangkok` and `TZ=America/New_York`, into `rust/parity/golden-tz/<zone>/` with their own manifest section. The default run (no flag) must stay byte-identical to today's `rust/parity/golden/` (prove with `diff -r` before and after). The fixture store may need a few extra events so the zone changes a result: add them **only** to the generator's fixture source if that keeps `store-v11.sqlite` and the existing goldens unchanged; if it cannot, stop and report instead of regenerating the existing goldens | RS-003 |
| 7 | New tests compare the Rust core to those goldens: the test sets the process zone for the call (one place, one helper; document how — `TZ` plus whatever the chosen crate needs to re-read it, and that tests touching it run in one test binary serialised) and compares parsed JSON | RS-003 |
| 8 | An unknown or malformed `TZ` value behaves as Python does (libc falls back to UTC for an unknown name). State what you found and match it | RS-003 |
| 9 | Fix the stale text: `rust/README.md` ("Day keys and an offset-less `since`/`until` are read as UTC"), the module comment in `metrics.rs` lines 9-10, the `usage_breakdown` doc comment at line 535, and the tool descriptions are already "local time" (do not edit the app crate: list it if a description needs a change) | stale-reference sweep |
| 10 | The store is opened read-only only; the fixture bytes are unchanged; `git status --short -- src web tests pyproject.toml` is empty before and after | RS-002 |
| 11 | `cargo test` (see the note on T006), `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is Python's own output under each zone. The implementing agent must not edit `rust/parity/golden/**` or `rust/parity/store-v11.sqlite`; it may add `rust/parity/golden-tz/**` and edit `rust/parity/gen_golden.py` and `rust/parity/README.md` as in criterion 6.
> **Locked oracle**: rust/parity/golden/manifest.json, rust/parity/golden/api__usage_summary__all.json, rust/parity/golden/api__usage_summary__window.json, rust/parity/golden/api__usage_breakdown__day.json, rust/parity/golden/api__usage_breakdown__model.json, rust/parity/golden/api__usage_breakdown__model_window.json, rust/parity/golden/api__usage_breakdown__project.json, rust/parity/golden/api__usage_breakdown__tool.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | `TZ=Asia/Bangkok` and an event at 20:00Z | its day key is the next calendar day | test against the golden |
| 2 | `TZ=America/New_York` and the two DST days | 23-hour and 25-hour days match Python | test against the golden |
| 3 | `TZ=UTC` | all existing T005 goldens still equal | existing suite |
| 4 | a default generator run | `diff -r` against the committed `rust/parity/golden` is empty | command |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/src/metrics.rs` (where UTC is assumed today) and `rust/crates/core/tests/usage_parity.rs` (golden comparison); the T001 spike notes for how crates were verified.
**Vital slice**: first the crate spike (criterion 1); then the generator mode and its goldens (red tests first); then `utc_bound` for offset-less bounds; then the day key.
**Cut list**: no caching of the zone; no per-call zone argument on the public API beyond what the tests need (the core reads the process zone, like Python); no Windows code; no new dependency beyond the one zone crate.

The generator runs Python with `TZ=…` and `time.tzset()` (see `rust/parity/gen_golden.py`, which fixes `TZ=UTC` today). Keep the frozen clock. Do not change how existing goldens are produced.

---

## Edge Case Checklist

- [ ] A day key near midnight local in both directions (+07 and -05)
- [ ] The two DST transition days and the skipped / repeated local hour (criterion 4)
- [ ] `since` given as a bare date (`2026-01-15`) and as a datetime without offset; with `Z`; with `+07:00`
- [ ] `until` exclusive and `since` inclusive stay as in Python after the shift
- [ ] Output `period.since_utc` / `until_utc` text is byte-equal to Python's (`%Y-%m-%dT%H:%M:%S.%fZ` form)
- [ ] A leap day and a year boundary in a non-UTC zone
- [ ] `TZ` unset: the machine's zone (`/etc/localtime`) is used, as Python does
- [ ] No absolute path, row text or secret in any output or error (NFR-002)
- [ ] Tests never open `~/.easy-personal`; the process zone is restored after each test
- [ ] Stale references: `rust/README.md`, `metrics.rs` comments, `rust/parity/README.md` mention of `TZ=UTC` only

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/docs/spike-T009.md` | new — the crate choice with evidence |
| `rust/Cargo.toml`, `rust/crates/core/Cargo.toml` | one zone crate, pinned `=` (re-read first; T006 may edit them) |
| `rust/crates/core/src/metrics.rs` | local-zone `utc_bound` and day key; comments |
| `rust/crates/core/src/lib.rs` | only if a new module is needed (re-read first) |
| `rust/crates/core/tests/usage_tz_parity.rs` | new — one test binary, zone-setting helper, serialised |
| `rust/parity/gen_golden.py`, `rust/parity/README.md` | opt-in zone mode (criterion 6) |
| `rust/parity/golden-tz/**` | new goldens per zone |
| `rust/README.md` | correct the UTC sentence |
| Pre-existing pin tests | None — `usage_parity`, `store_info`, `git_timeout` must still pass |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/parity/golden/**`, `rust/parity/store-v11.sqlite` | the committed oracle; a default generator run must leave them byte-identical |
| `rust/crates/app/**` | T008 owns it |
| `rust/crates/core/src/rules.rs`, `rust/crates/core/tests/warnings_parity.rs` | T006 owns them |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: generate the zone goldens, write the comparison tests (they fail while the core reads UTC), then implement. Finally mutate one zone golden value in a temp copy and confirm the comparison fails. Run the default generator again and `diff -r` the default goldens.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T009.md`
- [ ] Ran the comparison tests and the mutation check
- [ ] Default generator run is byte-identical to the committed goldens
- [ ] Supervisor notified: task ready for Stage 4 review
