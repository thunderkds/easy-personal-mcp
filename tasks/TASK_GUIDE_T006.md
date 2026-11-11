# TASK_GUIDE — T006: P1 slice — rule pack and `warnings` in the core, parity with goldens
**Date**: 2026-11-10
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 "Rust port" entries in `memory/decisions.md` and all 2026-11-10 entries in `memory/learnings.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/docs/spike-T001.md`, `BRAINSTORMING_LOG_RUSTPORT.md` and `rust/docs/SPEC.md`
6. Read the Python code you are porting: `src/easy_personal/core/rules.py`, `src/easy_personal/data/rules.yaml`, `PRD.md` FR-026 and FR-073, and the migration files for the tables the rules read
7. Read the oracle: `rust/parity/golden/manifest.json` (parameters), `api__warnings__all.json`, `mcp__warnings__all.json`; the store is `rust/parity/store-v11.sqlite`
8. Read what T005 built: `rust/crates/core/src/metrics.rs`, `prices.rs`, `tests/usage_parity.rs`

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>` listing the files you actually read, and say which listed files you did not read.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> The Rust core evaluates the shipped rule pack over the store and returns the same `warnings` answer as the Python app's goldens: the same rules, the same measured values and thresholds, the same causes and diagnoses. The rule thresholds come from a hand-ported reader of `data/rules.yaml`.

**Out of scope**: HTTP, MCP and CLI wiring; the `plan` / `improvement_plan` answer (its own later task); `workspace` / `account` scope filters and any parameter no golden exercises (list them in the report); the owner `config.yaml` thresholds override unless a golden needs it; ingest or any write.

**Requirement Refs**: RS-003 in `rust/docs/SPEC.md`; Python `PRD.md` FR-026 (the v1 rule pack), FR-073 (causes and diagnosis), NFR-013.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md` and `PRD.md`

---

## Dependencies & Reachability

**Depends on**: T005 — `metrics`, `prices` and the price-file line parser in `rust/crates/core`
**Entry point**: `easy_personal_core::rules::evaluate` (library function; wiring comes later)

**Parallel tasks**: you own `rust/crates/core/src/rules.rs`, `rust/crates/core/tests/warnings_parity.rs` and the new `pub mod rules;` line in `rust/crates/core/src/lib.rs` (re-read it before editing). The T005 agent is finishing two small fixes in `rust/crates/core/src/metrics.rs` and `store.rs`: do not edit those two files; if you need a helper from `metrics.rs`, call it, and if it is private or missing, say so in your report instead of editing it. T008 works in `rust/crates/app/**`: do not touch it. `rust/Cargo.toml` is shared: re-read it before any edit.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `rules::evaluate(conn, ...)` returns JSON equal (parsed; key order ignored) to `api__warnings__all.json` over `rust/parity/store-v11.sqlite` with the golden's frozen clock `2026-01-20T00:00:00Z` (pass the clock in as a parameter; do not read the system clock inside the rule code) | RS-003 |
| 2 | The same value equals `mcp__warnings__all.json` | RS-003 |
| 3 | `data/rules.yaml` is read by a hand-written line parser in the Rust core (no YAML crate). The shipped file is read by `include_str!` of `src/easy_personal/data/rules.yaml`, like T005 did for prices. A malformed line returns an error naming the line number, never the text | Brainstorming YAML finding |
| 4 | Every rule in FR-026 that the fixture can fire, and every rule that abstains on it, is reported exactly as Python reports it, including the abstain reason text | RS-003 |
| 5 | A rule whose input has no data reports "abstained" with Python's reason, never a guessed value | FR-003 |
| 6 | Synthetic cases beyond the goldens: for each rule, a temp store (built in the test) that makes the rule fire and one that does not, with expected values taken by running the Python function over the same temp store (record the Python command in the test comment). At least `cache_hit_low`, `session_huge_no_compact`, `tokens_jump`, `project_dominates`, `tool_errors_high`, `context_index_stale`, `records_unreadable` | RS-003, FR-026 |
| 7 | The store is opened only with `store::open_read_only`; the fixture bytes are identical after the tests | RS-002 |
| 8 | Negative: flipping one fixture value in a temp copy changes the output of at least one comparison (the tests are not constant) | RS-003 |
| 9 | `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output in `rust/parity/golden/*.json` over `rust/parity/store-v11.sqlite`; for the synthetic cases, the Python function run by you over the same temp store. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden/api__warnings__all.json, rust/parity/golden/mcp__warnings__all.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | store-v11, frozen clock | parsed JSON equals both warning goldens | test |
| 2 | a store built to trip `cache_hit_low` | hit with Python's measured value, threshold, causes, diagnosis | test |
| 3 | a store with no data for a rule | abstained with Python's reason | test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/src/prices.rs` (hand-written line parser, errors with line numbers only) and `rust/crates/core/tests/usage_parity.rs` (golden comparison, mutation check); `src/easy_personal/core/rules.py` for the logic.
**Vital slice**: the golden path first (`api__warnings__all`), then one rule at a time with its synthetic Python-checked case.
**Cut list**: no `plan`; no scope filters; no `config.yaml` threshold override unless a golden needs it; no caching; no clock read inside rule code.

Read `rules.py`, `rules.yaml` and `PRD.md` FR-026 / FR-073 before porting. Reuse `metrics` functions from T005 where Python reuses them. Rounding: use `metrics::round_py` (T005) for every figure Python rounds. If a rule needs a table or column T005 does not touch, read its migration file for the schema.

---

## Edge Case Checklist

- [ ] Clock: all "now" windows (7 days, week-over-week) come from the parameter, so the test is deterministic; note the local-time effect (Python uses OS local time here — see the report's open point on T009)
- [ ] Rounding and ratio edge cases equal Python's (`round_py`)
- [ ] Division by zero and empty windows abstain, as Python does
- [ ] Threshold boundary: exactly at the threshold fires or not, as Python does (`<` vs `<=`)
- [ ] Rule order and the order of `causes` equal Python's
- [ ] Unknown key or malformed line in `rules.yaml` → error with line number
- [ ] No absolute path, row text or secret in any output or error (NFR-002)
- [ ] The test never opens `~/.easy-personal`
- [ ] Stale references: `rust/README.md` "Ported library functions" gets one line

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/rules.rs` | new — line parser, rule evaluation |
| `rust/crates/core/src/lib.rs` | add `pub mod rules;` only (re-read first) |
| `rust/crates/core/tests/warnings_parity.rs` | new — golden and synthetic comparisons |
| `rust/README.md` | one line |
| Pre-existing pin tests | None — `usage_parity.rs` and `store_info.rs` must still pass |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/core/src/metrics.rs`, `rust/crates/core/src/store.rs` | the T005 agent is editing them |
| `rust/crates/app/**` | T008 owns it |
| `rust/parity/**` | the oracle; locked |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: golden-comparison tests fail on stubs; implement slice by slice; then the synthetic cases (each with its recorded Python command and result); then the mutation check.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T006.md`
- [ ] Ran the comparison tests and the mutation check
- [ ] Supervisor notified: task ready for Stage 4 review
