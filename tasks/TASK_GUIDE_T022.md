# TASK_GUIDE — T022: `workspaces` list and `unassigned` projects (core) and their two GET routes
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T005, T012, T014, T017, T019)
3. Read this file completely
4. Read `agents/backend.md`
5. Read the Python you port: `src/easy_personal/core/workspaces.py` (`_COLUMNS`, `_dict`, `list_all`, `_under`, `unassigned`), `core/owners.py` (`project_map`, `SCRATCH`, how a project key is made), `core/price_table.py` (`cost`, `as_of`), and the route lines in `src/easy_personal/surfaces/api.py` (`/api/workspaces`, `/api/workspaces/unassigned`: both GET handlers use `_query`, ignore the query string, and return `{"workspaces": [...]}` and `{"repos": [...]}`)
6. Read the Rust you extend: `rust/crates/core/src/{metrics,scope,prices}.rs` (`project_map`, `under`, `PriceTable`, `round_py` and how `usage_breakdown` costs a message and abstains a model), `rust/crates/app/src/api.rs` and `api_tests.rs`, `rust/crates/app/tests/server.rs`
7. Read the oracle: both manifests (`rust/parity/golden/manifest.json`, `rust/parity/golden-rich/manifest.json`) and the four goldens `api__workspaces__all.json`, `api__workspaces_unassigned__all.json` in `golden/` and in `golden-rich/`
8. Do not open `~/.easy-personal` or any `~/.claude*` folder, and do not write that path in any command, not even a guarded `ls`. Tests use temp copies of the parity stores

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips. The dashboard needs the workspace list for its scope picker and the unassigned projects for its workspaces page.

**Restated intent**:
> The Rust core has `workspaces::list_all` and `workspaces::unassigned`, and the Rust `serve` answers `GET /api/workspaces` and `GET /api/workspaces/unassigned` with the same JSON as Python, for the four goldens (default and rich stores), behind the guard.

**Out of scope**: every write (`POST /api/workspaces`, `PATCH`, `DELETE`: the Rust app is read-only; those methods stay 405); `accounts`, `kit`, `reports`, `board`, `dirs`, `jobs`, `sync`, `connectors`, `first-run`, `memory-graph`, `live`; any MCP tool; the static files (T021); anything that edits `server.rs` or `main.rs`.

**Requirement Refs**: RS-003 (parity), RS-004 / NFR-008 (guard), NFR-002 in `rust/docs/SPEC.md`; Python `PRD.md` FR-035, FR-069, FR-070.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T017 (`scope.rs`), T005 (prices, `usage_breakdown`'s costing), T014 (routes), T007 (rich oracle)
**Entry point**: `easy_personal_core::workspaces::{list_all, unassigned}` and `GET /api/workspaces`, `GET /api/workspaces/unassigned` on `easy-personal-rs serve`
**Parallel task**: T021 works in `rust/crates/app/src/{server.rs,main.rs}`, a new `static_files.rs`, `tests/server.rs` (appended) and `tests/static_files.rs`. Do not edit those. You own a new `rust/crates/core/src/workspaces.rs`, `lib.rs`, `rust/crates/core/tests/workspaces_parity.rs`, `rust/crates/app/src/api.rs`, `api_tests.rs`, and the route cases in a new `rust/crates/app/tests/workspaces.rs`. `pub(crate)` on an existing core helper (`scope::under`, `metrics::project_map`) is allowed; change nothing else in `scope.rs` or `metrics.rs`. Re-read `Cargo.toml` and `Cargo.lock` before editing; `cargo` may print "waiting for file lock"; if a build fails only in `server.rs` / `main.rs`, that is T021 in progress: wait a minute and retry.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `workspaces::list_all(conn) -> Result<Value>` returns `{"workspaces": [...]}`: each workspace `{id, name, color, icon, description, email, is_orphan (bool), repos (sorted)}`, ordered `is_orphan DESC, name` (SQLite's collation order, as Python). `workspaces::unassigned(conn) -> Result<Value>` returns `{"repos": [...]}`: real projects with stored usage that no workspace repo covers, newest use first (`(last_used_utc, path)` descending), each `{path, name, sessions, api_equivalent_cost: {usd, abstained_models, price_table_date}, last_used_utc}`, with Python's rounding (`round(usd, 6)`, `None` when nothing was priced and a model abstained) | RS-003 |
| 2 | Over `rust/parity/store-v11.sqlite` and `rust/parity/store-rich/store-v11.sqlite` the two functions equal the four goldens (parsed JSON; integers stay integers and floats stay floats). A test counts the entries from the manifests so a missing one fails | RS-003 |
| 3 | Over HTTP, with a valid token, `/api/workspaces` and `/api/workspaces/unassigned` equal the same four goldens (spawned binary over a temp copy of each store, `TZ=UTC`), status 200, `Content-Type: application/json`, the CSP on every response. The query string is ignored as in Python (a test shows `?workspace=nope` changes nothing). `POST`, `PATCH` and `DELETE` on `/api/workspaces` answer 405 with the CSP | RS-003 |
| 4 | Store states as the other routes: no store 503 with the fixed text, a newer store, a directory in its place; the server never creates a store; no path, SQL or row text in any body (NFR-002). The routes sit behind the existing guard exactly like the others: add them to the guard tests' route list (no token, wrong token, bad Host, bad Origin, token in the query -> `403 {"error":"forbidden"}` with the CSP) and to the raw-path tests (`/%61pi/workspaces`, `//api/workspaces`, `/api%2fworkspaces` never served) | NFR-008, RS-002 |
| 5 | The costing equals `usage_breakdown`'s (the same price table, the same abstaining rule, `price_table_date` equals the table's `as_of`); reuse the existing helpers, do not write a second cost function or a second `project_map`. The path-aware `under` of `scope.rs` is reused (`/a/b` never covers `/a/bc`) | RS-003 |
| 6 | The queries bind parameters only (the two statements have no caller input, so say so in a comment); the store is opened read-only; the store bytes are unchanged after the tests and no `-wal` / `-shm` appears; no absolute path or row text in any error | RS-002, NFR-002 |
| 7 | A mutation test: remove one workspace repo row (or change one `project_path`) in a temp copy of the rich store and a comparison fails naming the entry | RS-003 |
| 8 | Edge cases checked against Python (run it once over a scratch copy with `rust/parity/.venv/bin/python -B -I`, a scratch `HOME` and `EASY_PERSONAL_HOME`, and embed the expected values in the test): a store with no workspace row (`{"workspaces": []}`) and no usage (`{"repos": []}`); a project whose path is the scratch bucket (skipped); a gone worktree counted for its owner project; a model missing from the price table (abstained, `usd` null when nothing priced, a number when something was); ties in `last_used_utc` ordered by path descending | RS-003 |
| 9 | `cargo test --workspace` passes under TZ unset, `TZ=Pacific/Kiritimati` and `TZ=Pacific/Pago_Pago` (paste all three); `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` pass (if the workspace check fails only on T021's files, check your own with `rustfmt --check --edition 2024 <your files>` and say so); the Python tree check is empty before and after; the oracle hash (310 files as in `tasks/TASK_REVIEW_T011.md`; starts `aa6fcaed`, ends `bff92`) is unchanged | Intent |
| 10 | `rust/README.md`: the core paragraph lists the two functions and the `serve` route list the two routes; the writes stay documented as absent (405) | stale-reference sweep |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output in the goldens named above. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden/api__workspaces__all.json, rust/parity/golden/api__workspaces_unassigned__all.json, rust/parity/golden-rich/api__workspaces__all.json, rust/parity/golden-rich/api__workspaces_unassigned__all.json, rust/parity/golden-rich/manifest.json, rust/parity/golden/manifest.json, rust/parity/store-rich/store-v11.sqlite

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | both stores, the two core functions | the four goldens equal | test |
| 2 | the real `serve` binary over a temp copy of each store | the four goldens equal over HTTP | spawn test |
| 3 | a repo row removed in a temp copy | a comparison fails and names the entry | mutation test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && TZ=Pacific/Kiritimati cargo test --workspace && TZ=Pacific/Pago_Pago cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `usage_breakdown`'s `by=project` path in `metrics.rs` (the project key, the costing, the abstained models) and `rust/crates/core/tests/scope_parity.rs` (manifest-driven comparison, a child process under `TZ=UTC`, mutation on a temp copy); `api.rs` `serve` for the routes.
**Vital slice**: `list_all` against the two goldens, then `unassigned`, then the two routes with the spawn test, then the guard and store-state cases, then the Python edge cases.
**Cut list**: no writes; no caching; no new dependency; no new MCP tool.

`unassigned` costs each message once (`GROUP BY message_id`, the maxima of each token column), as `usage_breakdown` does; take Python's SQL as it is.

---

## Edge Case Checklist

- [ ] Float formatting of `usd` (six places, Python `round`: use `round_py`) and `null` versus `0.0`
- [ ] A workspace with no repos; two workspaces with the same repo
- [ ] `name` of a project path with a trailing slash or the empty string (`Path(key).name or key`)
- [ ] Non-ASCII workspace names and paths sort as SQLite sorts them (byte order, `BINARY` collation)
- [ ] No absolute path of the real home or row text in a test message
- [ ] Stale references: `rust/README.md`; the module comments of `api.rs`

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/workspaces.rs` (new), `lib.rs` | the two functions |
| `rust/crates/core/src/{scope,metrics}.rs` | `pub(crate)` on a helper only |
| `rust/crates/core/tests/workspaces_parity.rs` | new |
| `rust/crates/app/src/api.rs`, `api_tests.rs` | two routes |
| `rust/crates/app/tests/workspaces.rs` | new spawn tests |
| `rust/README.md` | the functions and routes |
| Pre-existing pin tests | any test that pins these two paths as 404: list each |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/app/src/{server,main,mcp}.rs`, `rust/crates/app/tests/{server,mcp}.rs` | T021 owns `server.rs`, `main.rs` and `tests/server.rs`; the MCP server is not involved |
| `rust/parity/**` | the oracle |
| `rust/spikes/**` | the spikes |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: `workspaces_parity.rs` and the spawn tests fail while the functions are stubs and the routes answer 404. Implement slice by slice. Mutation (criterion 7). Run the live probes once against a running `serve` over a temp copy of the rich store (both routes, a write method, no token) and paste them; the dashboard token is base64url (`A-Za-z0-9_-`), not hex. Finish with the whole suite under three zones.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into the report (the Supervisor writes `tasks/TASK_REVIEW_T022.md`)
- [ ] Live probes pasted
- [ ] Supervisor notified: task ready for Stage 4 review
