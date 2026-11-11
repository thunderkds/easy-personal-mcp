# TASK_GUIDE — T018: `session_timeline` and `session_outliers` in the core (with scope)
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
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T005, T007, T009, T012, T017)
3. Read this file completely
4. Read `agents/backend.md`
5. Read the Python you are porting: `src/easy_personal/core/metrics.py` lines 425-560 (`session_outliers`, `session_timeline`) and the constants they use (`OUTLIER_TOOL_CALLS`, `OUTLIER_CONTEXT_TOKENS`, `TIMELINE_GAP_MINUTES`, `_utc_bound`, `_inner_window`, `_period`, `_folder`); `src/easy_personal/surfaces/api.py` lines 56 and 680-690 (the dashboard sends `detail_sessions=50`)
6. Read the Rust you extend: `rust/crates/core/src/{metrics,scope}.rs` (`usage_summary_scoped` is the pattern: `bounds`, `period`, `scope::resolve(..).clause(..)`, the inner window), `rust/crates/core/tests/scope_parity.rs` and `rich_parity.rs`
7. Read the oracle: `rust/parity/golden/manifest.json` and `rust/parity/golden-rich/manifest.json` (the entries named below), and the goldens themselves
8. Do not open `~/.easy-personal` or any `~/.claude*` folder, not even to check that it exists

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> The Rust core has `session_outliers` and `session_timeline`, each with the optional `workspace` / `account` scope from T017 (and `detail_sessions` for the timeline), returning the same JSON as Python for every `usage_outliers`, `usage_sessions` and `session_outliers` golden in both oracles, including the 400 for a bad `since`.

**Out of scope**: HTTP routes and the MCP `session_outliers` tool (the next task, T020); `session_contexts`, `record_readability` and every other function of `metrics.py`; any change to `usage_summary` / `usage_breakdown` / rules / plan; `rust/crates/app/**`.

**Requirement Refs**: RS-003 (parity) in `rust/docs/SPEC.md`; Python `PRD.md` FR-035, FR-036.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T017 (scope), T009 (local zone), T005 (the shared helpers), T007 (rich oracle)
**Entry point**: `easy_personal_core::metrics::{session_outliers, session_outliers_scoped, session_timeline, session_timeline_scoped}` (library functions; wiring is T020). Prefer a new module `sessions.rs` if `metrics.rs` would pass 1,300 lines (it is 1,092 now); re-export the four names from `metrics` or document the path in the README
**Parallel task**: T019 works in `rust/crates/app/**` and edits comment lines in `rust/crates/core/tests/rich_parity.rs` and `plan_parity.rs`. You own `rust/crates/core/src/**` (new `sessions.rs` or `metrics.rs`), a new `rust/crates/core/tests/sessions_parity.rs`, and `rust/README.md`'s core paragraph. Do not edit the app crate or those two test files. `cargo` may print "waiting for file lock".

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `session_outliers(conn, since, until)` and `session_outliers_scoped(.., Scope)`; `session_timeline(conn, since, until, detail_sessions: Option<usize>)` and `session_timeline_scoped(.., Scope)`. Errors use `MetricsError::Invalid` with Python's text for a bad `since` / `until` and an unknown scope (the same as `usage_summary_scoped`). No existing function changes | RS-003 |
| 2 | Over `rust/parity/store-v11.sqlite`: `session_outliers` equals `api__usage_outliers__all` and `mcp__session_outliers__all`; `session_timeline(.., Some(50))` equals `api__usage_sessions__all` and `api__usage_sessions__window` (parameters from the manifest); `since=bad` is an `Invalid` error (status 400 in the golden) | RS-003 |
| 3 | Over `rust/parity/store-rich/store-v11.sqlite` (frozen clock is irrelevant: these take no clock): the 7 `api__usage_outliers__*` goldens (all, window, account_default, account_work, workspace_acme, workspace_acme_account_work, workspace_personal), the 2 `mcp__session_outliers__*` goldens, and the 9 `api__usage_sessions__*` goldens (all, window, window_mid, account_default, account_work, workspace_acme, workspace_acme_account_work, workspace_personal) plus `bad_since` (compare status and the `error` key only, as its manifest entry says). The test counts the entries from the manifests so a missing one fails | RS-003 |
| 4 | The Python rules are kept: outlier sessions need `tool_calls >= OUTLIER_TOOL_CALLS` or a context `>= OUTLIER_CONTEXT_TOKENS`, sorted by `(-tool_calls, session_id)`; the timeline's newest-first order with ties by session id, gaps `>= TIMELINE_GAP_MINUTES` with minutes rounded to one decimal (Python `round`, half to even on the float: check a value near .x5 against Python), only the newest `detail_sessions` carry `points` and `gaps`, and the top-level `detail_sessions` key is present only when given. Integers stay integers; floats stay floats | RS-003 |
| 5 | The local zone (T009) still applies to offset-less `since` / `until`: run the comparison through a child process under `TZ=UTC` like the other goldens, and add one non-UTC case with an expected value taken from Python on a scratch copy (say how you ran Python) | RS-003 |
| 6 | The queries bind parameters only; the store is opened read-only; the rich store bytes are unchanged after the tests and no `-wal` / `-shm` appears. No absolute path, row text, SQL or secret in any error (NFR-002) | RS-002, NFR-002 |
| 7 | A mutation test: change one `ts_utc` or one `tool_names` in a temp copy and a comparison fails | RS-003 |
| 8 | `rust/README.md` lists the four functions; `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass; the Python tree check (`git status --short -- src web tests pyproject.toml uv.lock`) is empty before and after; the oracle hash over `golden golden-tz golden-rich fixtures-rich store-rich store-v11.sqlite` (310 files, computed as in `tasks/TASK_REVIEW_T011.md`) is the same before and after; it starts `aa6fcaed` and ends `bff92`; print it both times | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output in the goldens named above. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden/api__usage_outliers__all.json, rust/parity/golden/api__usage_sessions__all.json, rust/parity/golden/api__usage_sessions__window.json, rust/parity/golden/mcp__session_outliers__all.json, rust/parity/golden-rich/api__usage_outliers__workspace_acme.json, rust/parity/golden-rich/api__usage_sessions__all.json, rust/parity/golden-rich/api__usage_sessions__window_mid.json, rust/parity/golden-rich/api__usage_sessions__workspace_acme_account_work.json, rust/parity/golden-rich/mcp__session_outliers__window.json, rust/parity/golden-rich/manifest.json, rust/parity/golden/manifest.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the default store and each manifest entry | parsed JSON equal | test |
| 2 | the rich store and each manifest entry (18 usage_sessions/outliers/session_outliers, with scope) | parsed JSON equal; counted from the manifest | test |
| 3 | a changed event in a temp copy | a comparison fails and names the entry | mutation test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `usage_summary_scoped` in `metrics.rs` (bounds, inner window, scope clause) and `tests/scope_parity.rs` (manifest-driven, child process under `TZ=UTC`, mutation tests on a temp copy).
**Vital slice**: outliers first (smaller, no zone-dependent grouping), then the timeline points and totals, then gaps and `detail_sessions`, then the scoped goldens.
**Cut list**: no caching; no new dependency; no HTTP, MCP or CLI; no change to the unscoped paths.

Python derives `project` with `_folder(path)`: reuse the Rust equivalent that `usage_breakdown` already uses; do not write a second one. `session_outliers` sees `tool_names` as a comma list: count with the same split as Python (`len(tools.split(","))`, so an empty string counts as 1).

If a golden cannot be matched because the golden looks wrong, re-run Python over the same store with a scratch `EASY_PERSONAL_HOME` and `rust/parity/.venv/bin/python -B -I`; if the port is wrong keep that comparison as its own `#[ignore = "..."]` test and report it; if you cannot tell, stop and report.

---

## Edge Case Checklist

- [ ] A session with tool calls but no priced message (in `calls` but not in `contexts`), and the reverse
- [ ] Session ids sorted as bytes (Python sorts `str` by code point: equal for ASCII; say how you handle non-ASCII)
- [ ] A session whose events straddle the window start: start/end from all non-abstained events in the window, points from the inner window
- [ ] A gap of exactly `TIMELINE_GAP_MINUTES`; a gap whose rounding crosses `.x5`
- [ ] `detail_sessions` of 0, 1 and larger than the number of sessions
- [ ] An event with `ts_utc` NULL (the timeline skips it)
- [ ] Workspace and account together; the unknown ones (the error text equals the T017 one)
- [ ] Stale references: `rust/README.md`; the comment in `rich_parity.rs` is T019's, not yours

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/sessions.rs` (new) or `metrics.rs` | the four functions |
| `rust/crates/core/src/lib.rs` | `pub mod sessions;` if a new module |
| `rust/crates/core/src/metrics.rs` | only `pub(crate)` visibility of helpers you need |
| `rust/crates/core/tests/sessions_parity.rs` | new — the goldens, errors, mutation |
| `rust/README.md` | the four functions |
| Pre-existing pin tests | None — all current suites stay green and unchanged |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/app/**` | T019 and T020 |
| `rust/crates/core/tests/{rich_parity,plan_parity}.rs` | T019 edits their comments |
| `rust/parity/**` | the oracle |
| `rust/spikes/**` | the spikes |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: `sessions_parity.rs` compares each golden (fails while the functions are stubs). Implement outliers, then the timeline, then the scoped entries, re-running after each. Then the error cases and the mutation test. Run the whole suite under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`; the two `crates/app/tests/mcp.rs` failures under the other zones are known (T019 fixes them): report them as known, do not edit that file.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into the report (the Supervisor writes `tasks/TASK_REVIEW_T018.md`)
- [ ] Supervisor notified: task ready for Stage 4 review
