# TASK_GUIDE — T019: Wire `workspace` and `account` into the HTTP routes and the MCP tools
**Date**: 2026-11-10
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md` (note the guard constraint and the "no path, SQL or row text in a body" rule)
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T004, T008, T009, T014, T015, T017)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/app/src/{api.rs,api_tests.rs,mcp.rs}`, `rust/crates/app/tests/{server,mcp}.rs`, and `rust/crates/core/src/scope.rs` (the `Scope` type, `ScopeError`) with the signatures of `usage_summary_scoped`, `usage_breakdown_scoped`, `rules::evaluate_scoped`, `plan::improvement_plan_scoped` and `ScopedRulesError`
6. Read the Python: `src/easy_personal/surfaces/api.py` lines 215-255 (`_period`, `_who`, `_scope`) and the routes; `src/easy_personal/surfaces/mcp.py` (`usage_summary`, `usage_breakdown`, `warnings`, `improvement_plan`: the `workspace` / `account` parameters and how an unknown one answers)
7. Read the oracle: both manifests (`rust/parity/golden/manifest.json`, `rust/parity/golden-rich/manifest.json`) and the 41 scoped goldens that `rust/crates/core/tests/scope_parity.rs` counts: 31 over HTTP (8 `api__usage_summary__*`, 11 `api__usage_breakdown__*`, 6 `api__warnings__*`, 6 `api__plan__*`) and 10 over MCP (5 `mcp__warnings__*`, 5 `mcp__improvement_plan__*`); `mcp__tools_list.json` for the schemas
8. Do not open `~/.easy-personal` or any `~/.claude*` folder, not even to check that it exists. Tests use `EASY_PERSONAL_HOME` pointing at a temp copy of a parity store

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> `/api/usage/summary`, `/api/usage/breakdown`, `/api/warnings`, `/api/plan` and the MCP tools `usage_summary`, `usage_breakdown`, `warnings`, `improvement_plan` honour `workspace` and `account` through the core's scoped functions, with the same JSON and status codes as Python (including the 400 for an unknown one), replacing today's 501 and tool error.

**Out of scope**: `/api/usage/sessions`, `/api/usage/outliers` and the MCP `session_outliers` tool (T020, after T018); any other route or tool; any change to `rust/crates/core/src/**` (the core is done: if it lacks something, stop and report); the static page and the window.

**Requirement Refs**: RS-003 (parity), RS-004 / NFR-008 (guard), NFR-002 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T014, T015 (the routes and tools), T017 (the scoped core)
**Entry point**: `easy-personal-rs serve` (the four routes) and `easy-personal-rs mcp` (the four tools)
**Parallel task**: T018 works in `rust/crates/core/src/**` (a new `sessions.rs` or `metrics.rs`) and a new `rust/crates/core/tests/sessions_parity.rs`. Do not edit those. You own `rust/crates/app/**`, the comment lines named in criterion 7 in `rust/crates/core/tests/{rich_parity,plan_parity}.rs`, and `rust/README.md`'s `serve` and `mcp` paragraphs. `cargo` may print "waiting for file lock"; the core crate may change under you: if a build fails in `core/src` only, wait a minute and retry before reporting.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | Over HTTP, with a valid token and the manifest parameters, the 31 scoped goldens over the rich store (`EASY_PERSONAL_HOME` pointing at a temp copy of `rust/parity/store-rich/store-v11.sqlite`) equal the bodies and status codes: `usage_summary` 8, `usage_breakdown` 11, `warnings` 6, `plan` 6, **including the four error entries** (unknown workspace / unknown account: status 400 and `{"error": "<Python's text>"}`). Usage entries run through the spawned binary under `TZ=UTC`; warnings and plan run in-process with the frozen clock (`frozen_now` of the rich manifest), as T014 did. The test counts the entries from the manifest so a missing one fails | RS-003 |
| 2 | Over MCP, the 10 scoped goldens (5 `warnings`, 5 `improvement_plan`) equal `structuredContent` and the parsed `content[0].text` (in-process with the frozen clock, as T015 did). `usage_summary` and `usage_breakdown` with a scope answer the same body as over HTTP (compare in a test against the same core call). The four tools' `inputSchema` still equals `mcp__tools_list.json` | RS-003 |
| 3 | Errors: over HTTP an unknown workspace or account is 400 with Python's text, for all four routes; an invalid `by` is still checked first (status 400 and the `bad_by` body); `workspace=` / `account=` empty is no filter (equal to the unscoped golden); the Query rules of T014 (last value wins, `+` is a space, percent-decoding) apply to the two keys. Over MCP an unknown one is a tool error `Error executing tool <name>: <Python's text>`: run Python once over a scratch copy of the rich store (`rust/parity/.venv/bin/python -B -I`, scratch `EASY_PERSONAL_HOME`, say how) to confirm the exact wording and `isError`, and paste what you saw. No path, SQL, row text or caller input appears in any body (NFR-002); the text is the core's fixed one | RS-003, NFR-002 |
| 4 | The 501 `FILTER_NOT_BUILT` path and its tests are removed (list each test you change or delete; they were written to pin the 501); the MCP "filters are not available" tool error is gone; a non-empty scope is never silently ignored | RS-003 |
| 5 | `ScopedRulesError` is mapped: `UnknownScope` -> 400 over HTTP and a tool error over MCP; `Rules(e)` as `RulesError` is today. A store problem during resolution (`ScopeError::Store`) is the same fixed 503 / tool error as any store problem. You may add a `From` or a helper in the app crate; do not edit the core | RS-002 |
| 6 | The guard covers everything as before: the 13 server tests and the existing guard probes pass unchanged; the encoded-path probes from T014 still pass; a `workspace` / `account` in the query is never logged | NFR-008 |
| 7 | **Test debt from T015/T017:** the two `crates/app/tests/mcp.rs` tests that compare `by=day` with UTC goldens (`usage_breakdown_equals_the_goldens_for_each_dimension`, `requests_sent_back_to_back_are_all_answered`) spawn the binary with the inherited `TZ` and fail under `TZ=Pacific/Kiritimati` and `TZ=Pacific/Pago_Pago`: pin `TZ=UTC` on the spawned child (and any other spawned-binary test that reads offset-less bounds or `by=day`). Also update the stale comments: `rust/crates/core/tests/rich_parity.rs` lines 7-9 and the `println!` near line 392 ("not testable in Rust yet"), and `rust/crates/core/tests/plan_parity.rs` line 5 (they say the filters are not ported; `scope_parity.rs` compares all 41). Comment and message text only: no assertion changes in those two core files | Intent |
| 8 | The whole `cargo test --workspace` passes under TZ unset, `TZ=Pacific/Kiritimati` and `TZ=Pacific/Pago_Pago` (paste all three); `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass; the Python tree check is empty before and after; the oracle hash (310 files, as in `tasks/TASK_REVIEW_T011.md`; starts `aa6fcaed`, ends `bff92`) is the same before and after; the fixture bytes are unchanged (no `-wal` / `-shm`) | Intent |
| 9 | A mutation test: move one event to the other account in a temp copy of the rich store and an HTTP scoped comparison fails | RS-003 |
| 10 | `rust/README.md`: the `serve` route list and the `mcp` paragraph say the four routes / tools take `workspace` and `account`; the 501 line and the "filters are not available" sentences are gone; `/api/usage/sessions` and `/api/usage/outliers` stay documented as not ported until T020 | stale-reference sweep |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output in the goldens named above. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden-rich/manifest.json, rust/parity/golden-rich/api__usage_summary__workspace_acme.json, rust/parity/golden-rich/api__usage_summary__account_unknown.json, rust/parity/golden-rich/api__usage_breakdown__workspace_acme_account_work.json, rust/parity/golden-rich/api__warnings__workspace_acme_account_work.json, rust/parity/golden-rich/api__warnings__workspace_unknown.json, rust/parity/golden-rich/api__plan__workspace_personal.json, rust/parity/golden-rich/mcp__warnings__workspace_acme.json, rust/parity/golden-rich/mcp__improvement_plan__account_work.json, rust/parity/golden/mcp__tools_list.json, rust/parity/store-rich/store-v11.sqlite

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the real `serve` binary over a temp copy of the rich store | the 8 + 11 scoped usage goldens equal, the unknown-scope ones are 400 | spawn test |
| 2 | the in-process router and `run_tool` with the frozen clock | the scoped warnings and plan goldens equal over HTTP (12) and MCP (10) | in-process test |
| 3 | `TZ=Pacific/Kiritimati` | the whole suite is green | `cargo test --workspace` |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && TZ=Pacific/Kiritimati cargo test --workspace && TZ=Pacific/Pago_Pago cargo test --workspace && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `api.rs` (`serve`, `Query`, the failure mappers) and `mcp.rs` (`run_tool`, the clock parameter, `with_store`); `rust/crates/core/tests/scope_parity.rs` for how the 41 entries are listed and counted.
**Vital slice**: `/api/usage/summary` with a workspace end to end (spawn test), then the other three routes, then the MCP tools, then the error mapping and mutation test, then the TZ pinning and the stale comments.
**Cut list**: no new dependency; no change to the guard, the transport or the core; no generated schemas; no caching.

Build the `Scope` from the query values with `Some(value)` only when the value is non-empty (the core also treats empty as no filter, but do not rely on it for the 501-style check you are removing). Keep the order Python uses: an invalid `by` first, then the scope. Over HTTP keep the checks for the period (`since` / `until`) where T014 has them.

---

## Edge Case Checklist

- [ ] An unknown workspace together with a bad `since`: which error wins in Python? Run it once and match it (note the result in the report)
- [ ] An unknown workspace on `/api/warnings` when the store has too little history to measure (the core refuses up front: the golden or a Python run shows the status)
- [ ] `account=%20Claude%20work%20` (stripped) and `account=claude+WORK` (case-insensitive) resolve
- [ ] A repeated `workspace=` key: the last value wins
- [ ] The tool schemas still accept `null` for both parameters, and a wrong type is a tool error like the existing ones
- [ ] The token never appears in a body, header or log; no caller text in any error
- [ ] Stale references: `rust/README.md`; the module comments at the top of `api.rs` and `mcp.rs`; `api_tests.rs` names that mention 501

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/api.rs`, `api_tests.rs` | scope from the query, the 501 path removed, error mapping |
| `rust/crates/app/src/mcp.rs` | scope from the arguments, the tool error removed, error mapping |
| `rust/crates/app/tests/server.rs`, `tests/mcp.rs` | scoped goldens, the 501 pin tests replaced, TZ pinned |
| `rust/crates/core/tests/{rich_parity,plan_parity}.rs` | comment and message text only (criterion 7) |
| `rust/README.md` | the routes and tools |
| Pre-existing pin tests | the tests that pin the 501 / the tool error: list each one you change or delete |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/core/src/**`, `rust/crates/core/tests/sessions_parity.rs` | the core; T018 owns the new files |
| `rust/parity/**` | the oracle |
| `rust/spikes/**` | the spikes |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: the scoped comparisons fail while the routes answer 501 and the tools answer the tool error. Implement slice by slice. Mutation (criterion 9). Run the live probes once against a running `serve` over a temp copy of the rich store: one scoped request per route, an unknown workspace, an empty one, and the token-less request, and paste them. Run the MCP binary by hand with an unknown workspace and paste the transcript. Finish with the whole suite under the three zones.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into the report (the Supervisor writes `tasks/TASK_REVIEW_T019.md`)
- [ ] Live probes and the MCP transcript pasted
- [ ] Supervisor notified: task ready for Stage 4 review
