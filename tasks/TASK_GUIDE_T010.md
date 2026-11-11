# TASK_GUIDE — T010: `improvement_plan` in the core
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T006, T012, T007)
3. Read this file completely
4. Read `agents/backend.md`
5. Read the Python code you are porting: `src/easy_personal/core/plan.py` (18 lines) and how `rules.evaluate` shapes a hit (`src/easy_personal/core/rules.py`)
6. Read the Rust you build on: `rust/crates/core/src/rules.rs` (`evaluate`, the hit type) and `rust/crates/core/tests/warnings_parity.rs`, `rich_parity.rs`
7. Read the oracle: `rust/parity/golden/api__plan__all.json`, `mcp__improvement_plan__all.json`, `rust/parity/golden-rich/api__plan__all.json`, `mcp__improvement_plan__all.json`, and the manifests that name their parameters

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> The Rust core answers `improvement_plan` — the fired rules grouped by priority, each with its fix line — with the same JSON as Python, over the default fixture store and the rich store.

**Out of scope**: the `workspace` and `account` parameters (not ported anywhere in the core yet; list them as not ported); HTTP or MCP wiring; any change to `rules.rs` behaviour; the parity folder.

**Requirement Refs**: RS-003 (parity) in `rust/docs/SPEC.md`; Python `PRD.md` FR-007.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T006 (`rules::evaluate`), T012 (shared helpers), T007 (rich plan golden)
**Entry point**: `easy_personal_core::plan::improvement_plan` (library function; wiring comes later)
**Parallel task**: T011 works in `rust/parity/**` only. Do not edit that folder. You own `rust/crates/core/src/plan.rs`, `lib.rs` (one line) and `rust/crates/core/tests/plan_parity.rs`.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `plan::improvement_plan(conn, now_micros, &rules)` returns JSON equal (parsed) to `api__plan__all.json` and `mcp__improvement_plan__all.json` over the default fixture store, and to the same two files in `golden-rich/` over the rich store (`rust/parity/store-rich/store-v11.sqlite`), using the frozen clock the manifests record | RS-003 |
| 2 | Groups are sorted by ascending priority; items keep the order `rules::evaluate` returns; each item has exactly the keys Python copies (`rule_id`, `label`, `measured`, `threshold`, `fix`, `title`, `why`, `steps`, `summary`, `causes`, `diagnosis`) plus `detail` and `cache_read_share` only when the hit has them. The test proves a rule with neither extra key | RS-003 |
| 3 | No fired rule gives `{"groups": []}`; a test over a store with no hits (the empty-window case from `warnings_parity.rs`, or a fixture copy) proves it and that Python gives the same | RS-003 |
| 4 | Where the plan's `cache_read_share` differs from the warning's `measured` for the same rule in the goldens (e.g. 0.1895 vs 0.1929 in the rich store), the test names why from the Python code and the port equals the golden; if the port cannot match, stop and report | RS-003 |
| 5 | No new dependency; the store is opened read-only only; the fixture bytes are unchanged; `git status --short -- src web tests pyproject.toml uv.lock` is empty before and after | RS-002 |
| 6 | `rust/README.md` lists `plan::improvement_plan` among the ported functions (one line); `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output in the four plan goldens. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden/api__plan__all.json, rust/parity/golden/mcp__improvement_plan__all.json, rust/parity/golden-rich/api__plan__all.json, rust/parity/golden-rich/mcp__improvement_plan__all.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | each of the four goldens and its store | parsed JSON equal | test |
| 2 | one changed fixture value in a temp copy | the comparison fails | mutation test |
| 3 | a store with no hits | `{"groups": []}` | test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/tests/warnings_parity.rs` and `rich_parity.rs` (golden loading, frozen clock, the TZ pin by child process if any date is read).
**Vital slice**: the function and the default goldens; then the rich goldens; then the empty and key-presence cases.
**Cut list**: no scope filters; no new type beyond what the function needs; no change to `rules.rs`.

---

## Edge Case Checklist

- [ ] Integers stay integers and floats stay floats; key order is not compared (sorted-key goldens)
- [ ] Two rules with the same priority share one group, in evaluate order
- [ ] No absolute path, row text or secret in any output or error (NFR-002)
- [ ] Tests never open `~/.easy-personal`
- [ ] Stale references: `rust/README.md` "Not ported yet" lines that name `plan`

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/plan.rs` | new |
| `rust/crates/core/src/lib.rs` | `pub mod plan;` (re-read first) |
| `rust/crates/core/tests/plan_parity.rs` | new |
| `rust/README.md` | one line; fix any "not ported" line about plan |
| Pre-existing pin tests | None — every current suite stays green and unchanged |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/parity/**` | the oracle; T011 edits the generator |
| `rust/crates/core/src/rules.rs`, `metrics.rs` | behaviour must not change; if `evaluate` lacks something the plan needs, stop and report |
| `rust/crates/app/**` | no wiring in this task |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: write the golden comparisons (they fail on a stub), implement, then the empty-store, key-presence and mutation tests.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T010.md`
- [ ] Supervisor notified: task ready for Stage 4 review
