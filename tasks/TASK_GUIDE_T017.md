# TASK_GUIDE — T017: `workspace` and `account` scope in the core (usage, warnings, plan)
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
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T005, T006, T007, T009, T010, T012)
3. Read this file completely
4. Read `agents/backend.md`
5. Read the Python code you are porting: `src/easy_personal/core/metrics.py` (`_scope`, `_padded`, and every function that takes `workspace` / `account`), `core/workspaces.py` (`project_paths`, how a workspace name resolves), `core/accounts.py` (how an id or label resolves; the account column on events), `core/rules.py` (how `evaluate` passes the scope into each rule), `core/plan.py`, and `core/owners.py` where the workspace paths use it
6. Read the Rust you extend: `rust/crates/core/src/{metrics,rules,plan}.rs`, `tests/{usage_parity,warnings_parity,plan_parity,rich_parity}.rs`
7. Read the oracle: `rust/parity/golden-rich/manifest.json` and the scoped entries named below, and `rust/parity/store-rich/store-v11.sqlite` (the only store with workspaces and accounts)
8. Do not open `~/.easy-personal` or any `~/.claude*` folder, not even to check that it exists

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> `usage_summary`, `usage_breakdown`, `rules::evaluate` and `plan::improvement_plan` accept an optional workspace name and an optional account (id or label) and return the same JSON as Python for each of the 41 scoped goldens in `golden-rich`, including the unknown workspace and unknown account answers.

**Out of scope**: HTTP and MCP wiring (the next task); `session_outliers`, `session_timeline`, kit, board, reports, accounts and workspaces routes; any write; changing the existing public function signatures' behaviour without a scope (they keep working unchanged).

**Requirement Refs**: RS-003 (parity) in `rust/docs/SPEC.md`; Python `PRD.md` FR-035 (workspace), FR-036 (account).

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T005, T006, T009, T010, T012 (the functions and the shared helpers), T007 (the scoped goldens and the rich store)
**Entry point**: `easy_personal_core::metrics::{usage_summary_scoped, usage_breakdown_scoped}`, `rules::evaluate_scoped`, `plan::improvement_plan_scoped` (library functions; wiring comes next)
**Parallel task**: T016 is a window spike in `rust/spikes/t016-window/`; it does not touch the core. You own `rust/crates/core/**` and `rust/README.md`. Do not edit `rust/crates/app/**` or `rust/parity/**`.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | A `Scope { workspace: Option<String>, account: Option<String> }` type (or borrowed equivalent; empty string means no filter, like Python's `or None`) and `*_scoped` variants of the four functions. The existing functions keep their exact signatures and results and call the scoped ones with no scope, so every current test passes unchanged | RS-003 |
| 2 | Over the rich store, `usage_summary_scoped` equals the 8 goldens `api__usage_summary__{account_default,account_unknown,account_work,window_workspace_acme,workspace_acme,workspace_acme_account_work,workspace_personal,workspace_unknown}` (parameters from the manifest: account `1`, `nope`, `Claude work`; workspace `acme`, `personal`, `nope`; the window entry) | RS-003 |
| 3 | `usage_breakdown_scoped` equals the 11 scoped `api__usage_breakdown__*` goldens (every `by` with its scope, as the manifest lists them) | RS-003 |
| 4 | `rules::evaluate_scoped` equals the 6 `api__warnings__*` scoped goldens and the 5 `mcp__warnings__*` scoped ones; `improvement_plan_scoped` equals the 6 `api__plan__*` and 5 `mcp__improvement_plan__*` scoped ones. 41 goldens in all (the test counts them from the manifest so a missing one fails) | RS-003 |
| 5 | Error and edge behaviour equals Python: an unknown workspace raises the same error text as Python (`ValueError`, which the HTTP layer will map to 400: see the golden for the status and body); an unknown account selects nothing or errors exactly as the golden shows; a workspace with no matching projects selects nothing (`0` clause) | RS-003 |
| 6 | How a workspace's repo paths resolve to stored project paths (`workspaces.project_paths`, padded window) and how an account id or label resolves are ported with the same rules; if a rule needs `git` it uses the shared `run_git_within` with its limit. State in a code comment each Python function the logic comes from | RS-003 |
| 7 | The scoped queries bind parameters only (no SQL built from the workspace or account text); the store is opened read-only; the rich store bytes are unchanged after the tests and no `-wal`/`-shm` appears | RS-002, NFR-002 |
| 8 | No absolute path, row text, SQL or secret in any error (NFR-002); tests never open `~/.easy-personal` | NFR-002 |
| 9 | A mutation test: change one scoped value (for example move one event to the other account) in a temp copy and a scoped comparison fails | RS-003 |
| 10 | `rust/README.md` lists the scoped functions and drops "not ported" for these filters; `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass; `git status --short -- src web tests pyproject.toml uv.lock` is empty before and after | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output in the scoped goldens of `rust/parity/golden-rich/`. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden-rich/manifest.json, rust/parity/golden-rich/api__usage_summary__account_default.json, rust/parity/golden-rich/api__usage_summary__workspace_acme.json, rust/parity/golden-rich/api__usage_summary__workspace_unknown.json, rust/parity/golden-rich/api__warnings__workspace_acme_account_work.json, rust/parity/golden-rich/api__plan__workspace_personal.json, rust/parity/store-rich/store-v11.sqlite

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the rich store and each scoped manifest entry | parsed JSON equal (the 41 goldens) | test |
| 2 | the unknown workspace entry | the same error text/status as the golden | test |
| 3 | every existing suite | unchanged and green | `cargo test` |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/tests/rich_parity.rs` (manifest-driven comparison with a frozen clock) and `warnings_parity.rs` (how evaluate is called).
**Vital slice**: the scope clause and the account/workspace resolution first, then `usage_summary_scoped` (8 goldens), then breakdown, then rules and plan.
**Cut list**: no caching; no new dependency; no change to the unscoped paths; no HTTP, MCP or CLI.

If a scoped golden cannot be matched because the golden itself looks wrong, re-run Python over the same store to check; if the port is wrong keep that comparison as its own `#[ignore = "..."]` test and report it; if you cannot tell, stop and report.

---

## Edge Case Checklist

- [ ] Account given as an id (`1`) and as a label (`Claude work`); an unknown one; an empty string
- [ ] Workspace and account together (`acme` + `Claude work`)
- [ ] A scoped window (`since`/`until` with a workspace): the padded window used to resolve paths
- [ ] A session whose records straddle the window start and the account tag
- [ ] Integers stay integers and floats stay floats; key order is not compared
- [ ] The local zone (T009) still applies to scoped `by=day`: run the scoped suites under `TZ=UTC` through a child process like the other goldens
- [ ] Stale references: `rust/README.md` and the module comments that say the filters are not ported (`metrics.rs`, `rules.rs`, `plan.rs` headers)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/metrics.rs` | scope type, clause, resolution, scoped variants |
| `rust/crates/core/src/rules.rs`, `plan.rs` | scoped variants, header comments |
| `rust/crates/core/src/lib.rs` | only if a new module (for example `scope.rs`) is needed — prefer one if `metrics.rs` would pass 1,300 lines |
| `rust/crates/core/tests/scope_parity.rs` | new — the 41 goldens, errors, mutation |
| `rust/README.md` | the scoped functions |
| Pre-existing pin tests | None — all current suites stay green and unchanged |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/app/**` | wiring is the next task |
| `rust/parity/**` | the oracle |
| `rust/spikes/**` | T016 owns it |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: `scope_parity.rs` compares each scoped golden (fails while the scoped functions are stubs). Implement slice by slice, re-running after each. Then the error cases and the mutation test. Run the whole suite under three zones as T012 did.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T017.md`
- [ ] Supervisor notified: task ready for Stage 4 review
