# TASK_GUIDE — T005: P1 slice — price table parser and `usage_summary` / `usage_breakdown` in the core
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
2. Read `memory/MEMORY.md`, the 2026-11-10 "Rust port" entries in `memory/decisions.md` and `memory/learnings.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/docs/spike-T001.md` (no YAML crate for the shipped data), `BRAINSTORMING_LOG_RUSTPORT.md` (parity edge cases), `rust/docs/SPEC.md` (RS-003)
6. Read the Python code you are porting: `src/easy_personal/core/metrics.py` (`usage_summary`, `usage_breakdown` and the helpers they call), `src/easy_personal/core/prices.py`, `src/easy_personal/data/prices.yaml`, and the migration files for the tables they read
7. Read the oracle: `rust/parity/golden/manifest.json` (request parameters per entry) and the `api__usage_summary__*`, `api__usage_breakdown__*` goldens; the store is `rust/parity/store-v11.sqlite`

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> Given the shipped fixture store, the Rust core answers `usage_summary` and `usage_breakdown` with the same JSON as the Python app's goldens — including the cost figures from the price table, read by a hand-ported line parser.

**Out of scope**: HTTP, MCP and CLI wiring (T004, later tasks); `workspace` / `account` scope filters and any parameter no golden exercises (list them in the report as not ported); `session_outliers`, sessions, rules, kit; ingest or any write.

**Requirement Refs**: RS-003 (parity), RS-002 (read-only open) in `rust/docs/SPEC.md`; Python `PRD.md` NFR-013, and the FRs for usage summary and breakdown (find them by grepping `PRD.md` for `usage_summary` and `usage_breakdown`).

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T002 — `easy_personal_core::store::open_read_only`; T003 — the goldens and `store-v11.sqlite`
**Entry point**: `easy_personal_core::metrics::usage_summary` and `easy_personal_core::metrics::usage_breakdown` (library functions; CLI and API wiring come in later tasks)

**Parallel task**: T004 runs at the same time in the `app` crate. You own `rust/crates/core/**`; T004 owns `rust/crates/app/**` — do not edit `rust/crates/app/**` at all. Both of you edit `rust/Cargo.toml` and `rust/Cargo.lock`: re-read `rust/Cargo.toml` immediately before every edit, change only your own lines, and expect `cargo` to print "waiting for file lock" while the other task builds.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `metrics::usage_summary(conn, since, until)` returns a JSON value equal (parsed, so key order is ignored) to `api__usage_summary__all.json` and `api__usage_summary__window.json` for the parameters the manifest records | RS-003 |
| 2 | `metrics::usage_breakdown(conn, by, since, until)` equals `api__usage_breakdown__day`, `__model`, `__model_window`, `__project`, `__tool` for the manifest's parameters | RS-003 |
| 3 | An invalid `by` or `since` returns an error whose text equals the Python 400 message in `api__usage_breakdown__bad_by.json` / the sessions `bad_since` golden where the same check applies | RS-003 |
| 4 | `prices` parses `rust/`-independent copies of the shipped `prices.yaml` format by hand (`as_of:`, `models:` and `name: key=value ...` lines; comments and blanks ignored) — no YAML crate. A model missing from the table is abstained: no cost is shown, never guessed | Brainstorming YAML finding |
| 5 | Rounding matches Python: every figure Python rounds (e.g. `cache_hit_ratio` 0.7905 to 4 places) is equal after parsing; the test names the rounding function used and why it matches `round()` on ties | RS-003 |
| 6 | The store is opened only with `store::open_read_only`; no table is written; the test asserts the fixture file bytes are unchanged | RS-002 |
| 7 | Negative: a store with an unknown model in the events yields `abstained` counts exactly as Python's golden does, and no cost for that model | RS-003 |
| 8 | `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output captured in `rust/parity/golden/*.json` over `rust/parity/store-v11.sqlite`. The implementing agent must not edit those files.
> **Locked oracle**: rust/parity/golden/api__usage_summary__all.json, rust/parity/golden/api__usage_summary__window.json, rust/parity/golden/api__usage_breakdown__day.json, rust/parity/golden/api__usage_breakdown__model.json, rust/parity/golden/api__usage_breakdown__model_window.json, rust/parity/golden/api__usage_breakdown__project.json, rust/parity/golden/api__usage_breakdown__tool.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | store-v11 and the manifest's parameters | parsed JSON equals each golden | test |
| 2 | a `prices.yaml` line with an unknown key or a missing model | clear error or abstain as Python does | test |
| 3 | the fixture file | byte-identical before and after | test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/src/store.rs` (read-only open, no path in errors) and `rust/crates/core/tests/store_info.rs` (fixture-driven tests); the Python modules named in Startup reads for the logic.
**Vital slice**: `usage_summary` over all time, then the window; then `usage_breakdown` by `model` (it needs prices), then `day`, `project`, `tool`.
**Cut list**: no scope filters; no CLI, API or MCP wiring; no `serde` derive (build values with `serde_json::json!` / `Value`); no caching or memo; no abstraction over "metrics" beyond what the two functions need.

Add `serde_json` (pin `=` the latest 1.x after checking with `cargo info`) to `[workspace.dependencies]` and use it from the core crate only. Read the SQL the Python code runs and keep the same statements and the same grouping/ordering. Dates and local-day boundaries: the goldens were made with `TZ=UTC`; port the UTC logic and note what a non-UTC machine would change.

---

## Edge Case Checklist

- [ ] Rounding on ties: Python's `round()` rounds half to even on the binary value; a naive `(x * 1e4).round() / 1e4` can differ — prove the chosen method on the golden values and say how
- [ ] Integers stay integers and floats stay floats in the JSON (`1000` not `1000.0`)
- [ ] `null` vs missing keys exactly as Python (`period.since_utc: null`)
- [ ] Sort order of rows in the breakdown (ties) equals Python's
- [ ] A model absent from `prices.yaml` → abstained, never a guessed price
- [ ] `since` / `until` accepted formats and their error text equal Python's
- [ ] The price file parser rejects or reports a malformed line instead of silently skipping it (state which Python does and match it)
- [ ] No absolute path, row text or secret in any output or error (NFR-002)
- [ ] The test never opens `~/.easy-personal`; it copies or opens the fixture read-only
- [ ] Stale references: nothing else states the list of ported functions yet; add one line to `rust/README.md`

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/Cargo.toml` | add `serde_json` to `[workspace.dependencies]` (re-read first; T004 edits it too) |
| `rust/crates/core/Cargo.toml` | use it |
| `rust/crates/core/src/lib.rs` | add `pub mod metrics; pub mod prices;` |
| `rust/crates/core/src/prices.rs` | new — line parser and price table |
| `rust/crates/core/src/metrics.rs` | new — `usage_summary`, `usage_breakdown` and their helpers |
| `rust/crates/core/tests/usage_parity.rs` | new — golden comparisons |
| `rust/README.md` | one line: the ported functions |
| Pre-existing pin tests | None — `rust/crates/core/tests/store_info.rs` must still pass |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/app/**` | T004 owns it |
| `rust/parity/**` | the oracle; locked |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: write the golden-comparison tests (they fail on stubs), then implement slice by slice, re-running the tests after each. Finally flip one fixture value in a temp copy and confirm the matching comparison fails (the tests are not constant).

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T005.md`
- [ ] Ran the comparison tests and the mutation check
- [ ] Supervisor notified: task ready for Stage 4 review
