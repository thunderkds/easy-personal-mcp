# TASK_GUIDE — T014: Serve usage, warnings and plan over the guarded HTTP server
**Date**: 2026-11-10
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md` (note the guard constraint: the guard keys on the raw path)
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T004, T008, T009, T010, T013)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/app/src/server.rs`, `rust/crates/app/tests/server.rs`, `rust/crates/app/src/main.rs`
6. Read the Python routes: `src/easy_personal/surfaces/api.py` lines 117-127 (`_query`) and 215-255 (`_period`, `_who`, `_scope`) and 678-700 (the routes)
7. Read the core you call: `rust/crates/core/src/{store,clock,metrics,rules,plan}.rs` public items; the goldens `rust/parity/golden/api__usage_*.json`, `api__warnings__all.json`, `api__plan__all.json` and the rich ones in `rust/parity/golden-rich/`, with both manifests
8. Do not open `~/.easy-personal` or any `~/.claude*` folder, not even to check that it exists. Tests use `EASY_PERSONAL_HOME` pointing at a temp copy of a parity store

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10: "start the P1 wiring slice"): serve the ported read functions over HTTP, with the same JSON as Python.

**Restated intent**:
> `GET /api/usage/summary`, `/api/usage/breakdown`, `/api/warnings` and `/api/plan` return the same JSON and status codes as the Python app for the parameters the goldens exercise, behind the existing Host / Origin / token guard, reading the shared store read-only.

**Out of scope**: `/api/usage/sessions` and `/api/usage/outliers` (not ported in the core; they answer 404 for now); every other route; the static page and the window (P1b); the `workspace` and `account` filters (see criterion 5); the MCP tools (T015); creating a store.

**Requirement Refs**: RS-003 (parity), RS-004 / NFR-008 (guard on every `/api/*` route), NFR-002 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T004 (server and guard), T005 / T009 (usage, local zone), T006 (rules), T010 (plan), T013 (`store::open_checked`, `clock::now_micros`)
**Entry point**: `easy-personal-rs serve` (the four routes), and the router builder in the app crate
**Parallel task**: T015 edits `rust/crates/app/src/mcp.rs` and `tests/mcp.rs`. You own `server.rs`, a new `api.rs` if you want one, `tests/server.rs` and the app `Cargo.toml` dev-dependencies. Do not edit `mcp.rs`. Re-read `Cargo.toml` and `Cargo.lock` before editing; `cargo` may print "waiting for file lock".

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | With a valid token, `GET /api/usage/summary` and `/api/usage/breakdown?by=…` over a temp copy of `rust/parity/store-v11.sqlite` (`EASY_PERSONAL_HOME`) return a body whose parsed JSON equals the goldens `api__usage_summary__{all,window}`, `api__usage_breakdown__{day,model,model_window,project,tool}`, with status 200. The query parameters are the manifest's. `Content-Type: application/json`; the CSP header stays on every response | RS-003 |
| 2 | `/api/warnings` and `/api/plan` equal `api__warnings__all` and `api__plan__all` (default store) and the rich ones over `rust/parity/store-rich/store-v11.sqlite`. Because they depend on the current time, these are tested **in-process** against the router with a fixed clock injected through the router's state (the manifests' `frozen_now`); there is **no** environment variable or flag in the binary that changes the clock | RS-003 |
| 3 | Errors match Python: an invalid `by` gives status 400 and the body of `api__usage_breakdown__bad_by.json`; an invalid `since` gives 400 with `{"error": "<message>"}` and the message equals Python's (the core produces it); an empty `since=` is an error as in Python (`Invalid isoformat string: ''`); an empty `workspace=` or `account=` counts as no filter | RS-003 |
| 4 | Query decoding matches Python's Starlette: percent-decoding, `+` is a space, a repeated key uses the **last** value. A test shows each with a `since` value (for example `since=2026-01-15T10%3A05%3A00Z`) | RS-003 |
| 5 | A non-empty `workspace` or `account` (not ported in the core) answers `501` with `{"error":"the workspace and account filters are not available in this build yet"}`. It is never ignored silently. A test covers each route | Intent |
| 6 | Store states: no store -> `503` `{"error":"no store yet: nothing has been ingested into the data folder"}`; a store newer than this build -> `503` with the core's `Newer` text; busy or unreadable -> `503` with a fixed text. No path, row text or SQL ever appears in a body (NFR-002). The server never creates the store | RS-002 |
| 7 | The guard covers the new routes exactly as before: no token, wrong token, bad Host, bad Origin, a token in the query string -> `403 {"error":"forbidden"}` with the CSP; the existing 13 server tests pass unchanged. The raw-path rule still holds: `/%61pi/usage/summary`, `//api/usage/summary` and `/api%2fusage%2fsummary` are not served (404 or 403, never 200) — add these three to the tests | NFR-008 |
| 8 | Store reads run in `spawn_blocking` (not on the async workers), with a new read-only connection per request; one request cannot block the guard or another route for longer than the store's 5 s busy wait | Intent |
| 9 | `HEAD` and other methods: `GET` routes answer `HEAD` like axum does; `POST`/`PUT`/`DELETE` on these routes answer 405 with the CSP (state what Python does in the test comment) | RS-003 |
| 10 | `serve`'s printed lines and `/api/version` are unchanged. No new runtime dependency (dev-dependencies for in-process tests, such as `tower` and `http-body-util`, are allowed: pin with `=` after `cargo info`) | Intent |
| 11 | The fixture bytes are unchanged after the tests (no `-wal`/`-shm`); `git status --short -- src web tests pyproject.toml uv.lock` is empty before and after; `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output in the goldens named above. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden/api__usage_summary__all.json, rust/parity/golden/api__usage_summary__window.json, rust/parity/golden/api__usage_breakdown__day.json, rust/parity/golden/api__usage_breakdown__model.json, rust/parity/golden/api__usage_breakdown__model_window.json, rust/parity/golden/api__usage_breakdown__project.json, rust/parity/golden/api__usage_breakdown__tool.json, rust/parity/golden/api__usage_breakdown__bad_by.json, rust/parity/golden/api__warnings__all.json, rust/parity/golden/api__plan__all.json, rust/parity/golden-rich/api__warnings__all.json, rust/parity/golden-rich/api__plan__all.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the real `serve` binary over a temp store copy | the four usage goldens equal over HTTP | spawn test |
| 2 | the in-process router with the frozen clock | warnings and plan equal, default and rich | in-process test |
| 3 | no token / encoded `/api` path | 403 / not served | spawn test |
| 4 | a missing store | 503, folder stays empty | spawn test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/app/src/server.rs` (the guard layer, `json()`) and `rust/crates/app/tests/server.rs` (binary-spawn pattern); the T008 `mcp.rs` `run_tool` shows how a core call becomes a result.
**Vital slice**: `/api/usage/summary` end to end (spawn test), then breakdown, then the router state with an injectable clock, then warnings and plan, then the error and filter cases.
**Cut list**: no static files; no caching; no JSON struct layer (pass `serde_json::Value` through); no pagination; no CORS; no streaming.

Keep the guard layer as the outermost layer around every route including the fallback. A small `AppState` holding the token guard data, a store-path function and a clock function is enough; the binary passes the real ones, tests pass fixed ones.

---

## Edge Case Checklist

- [ ] Integer and float text in bodies equals Python's (parsed JSON compared; ints stay ints)
- [ ] A very long query string or many repeated keys does not panic
- [ ] Non-UTF-8 percent escapes do not panic (say what is returned)
- [ ] `since` with `+` for an offset (`+07:00`) is a space after decoding in Python — keep the same
- [ ] Zone: tests that read offset-less bounds or `by=day` run under `TZ=UTC` through a child process, as the core tests do (the goldens are UTC)
- [ ] No absolute path, row text, SQL or secret in any body, header or log line (NFR-002)
- [ ] The token never appears in a response or in an error
- [ ] Tests never open `~/.easy-personal`
- [ ] Stale references: `rust/README.md` route list and the "one route so far" sentence

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/server.rs` | state, the four routes, error mapping (or a new `api.rs` for the handlers) |
| `rust/crates/app/src/api.rs` | optional, new |
| `rust/crates/app/tests/server.rs` | new cases appended; the existing 13 unchanged |
| `rust/crates/app/tests/api.rs` | optional, new (in-process tests) |
| `rust/crates/app/Cargo.toml`, `rust/Cargo.toml`, `rust/Cargo.lock` | dev-dependencies only |
| `rust/README.md` | the new routes, the 501 / 503 behaviour |
| Pre-existing pin tests | None — `tests/server.rs` (13), `tests/cli.rs` (6) and the core suites stay green |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/app/src/mcp.rs`, `rust/crates/app/tests/mcp.rs` | T015 owns them |
| `rust/crates/core/**` | the core is done; if it lacks something, stop and report |
| `rust/parity/**` | the oracle |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: the spawn-test comparisons against goldens fail while the routes answer 404. Implement slice by slice. Mutation: change one `output_tokens` in a temp copy and see the comparison fail. Run the three encoded-path probes and the guard probes against a live `serve` by hand and paste them.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T014.md`
- [ ] Live probes pasted (guard, encoded paths, missing store)
- [ ] Supervisor notified: task ready for Stage 4 review
