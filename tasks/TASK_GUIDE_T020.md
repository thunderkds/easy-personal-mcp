# TASK_GUIDE — T020: `/api/usage/sessions`, `/api/usage/outliers` and the MCP `session_outliers` tool
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
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T014, T015, T017, T018, T019)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/app/src/{api.rs,api_tests.rs,mcp.rs}`, `rust/crates/app/tests/{server,mcp}.rs`, and the signatures of `session_outliers_scoped` and `session_timeline_scoped` in `rust/crates/core/src/sessions.rs`
6. Read the Python: `src/easy_personal/surfaces/api.py` lines 56 (`TIMELINE_DETAIL_SESSIONS = 50`), 215-255 (`_scope`) and 680-690 (the two routes); `src/easy_personal/surfaces/mcp.py` `session_outliers` (the tool takes only `since` and `until`)
7. Read the oracle: both manifests (`rust/parity/golden/manifest.json`, `rust/parity/golden-rich/manifest.json`) and the goldens: HTTP `api__usage_outliers__*` (default 1, rich 7), `api__usage_sessions__*` (default 3 including `bad_since`, rich 9 including `bad_since`), MCP `mcp__session_outliers__*` (default 1, rich 2), and `mcp__tools_list.json` for the tool's `name`, `description`, `inputSchema`, `outputSchema`
8. Do not open `~/.easy-personal` or any `~/.claude*` folder, and do not write that path in any command, not even as a guarded `ls`. Tests use `EASY_PERSONAL_HOME` pointing at a temp copy of a parity store

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> The Rust `serve` answers `GET /api/usage/sessions` (the dashboard's timeline, 50 detailed sessions) and `GET /api/usage/outliers`, and the Rust `mcp` lists and runs `session_outliers`, with the same JSON and status codes as Python, through the T018 core functions, with `workspace` and `account` on the two routes as in Python.

**Out of scope**: any change to `rust/crates/core/src/**`; every other route and tool (kit, board, reports, accounts, workspaces, memory graph); the `investigate` prompt; the static page and the window; adding `workspace` / `account` to the MCP `session_outliers` schema (Python's tool has only `since` and `until`: keep the schema equal to the golden).

**Requirement Refs**: RS-003 (parity), RS-004 / NFR-008 (guard), NFR-002 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T018 (the core functions), T019 (scope in `api.rs` / `mcp.rs`), T014, T015
**Entry point**: `easy-personal-rs serve` (`/api/usage/sessions`, `/api/usage/outliers`) and `easy-personal-rs mcp` (`session_outliers`)
**Parallel task**: none. You own `rust/crates/app/**`, the README's `serve` and `mcp` paragraphs. `cargo` may print "waiting for file lock".

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | Over HTTP, with a valid token and the manifest parameters, `/api/usage/outliers` equals the default golden and the 7 rich ones, and `/api/usage/sessions` (the core called with `detail_sessions = Some(50)`) equals the default `all` and `window` and the rich `all`, `window`, `window_mid`, `account_default`, `account_work`, `workspace_acme`, `workspace_acme_account_work`, `workspace_personal`; status 200, `Content-Type: application/json`, the CSP stays on every response. Spawned binary under `TZ=UTC` over a temp copy of each store (`rust/parity/store-v11.sqlite`, `rust/parity/store-rich/store-v11.sqlite`). The test counts the entries from the manifests so a missing one fails | RS-003 |
| 2 | `since=bad` on `/api/usage/sessions` is 400: the default golden's exact body, the rich one by status and the `error` key only (its manifest entry says why); the same for `/api/usage/outliers` and for a bad `until`. A bad period is reported before an unknown scope, as in Python. An unknown `workspace` or `account` is 400 with Python's text, as on the other routes; an empty value is no filter | RS-003 |
| 3 | Over MCP, `tools/list` returns six tools and for `session_outliers` the `name`, `description`, `inputSchema` and `outputSchema` equal the golden entry (the five others unchanged). `tools/call session_outliers` equals `structuredContent` and the parsed `content[0].text` of `mcp__session_outliers__all` (default store), and `mcp__session_outliers__all` and `__window` (rich store). No clock is involved: these also run through the spawned binary under `TZ=UTC`. `since` / `until` accepted as null, absent, string; a wrong type is a tool error like the other tools; a bad `since` is a tool error `Error executing tool session_outliers: <Python's text>` (run Python once over a scratch copy of the default store, `rust/parity/.venv/bin/python -B -I`, scratch `EASY_PERSONAL_HOME`, to confirm the exact wording and `isError`, and paste what you saw) | RS-003 |
| 4 | Store states as the other routes and tools: no store 503 / a tool error without a path; a newer store; a directory in its place; the server never creates a store. No path, SQL, row text or caller input in any body (NFR-002) | RS-002, NFR-002 |
| 5 | The guard covers the new routes exactly as before: no token, wrong token, bad Host, bad Origin, a token in the query string -> `403 {"error":"forbidden"}` with the CSP. The existing server tests (including the encoded-path probes) pass unchanged except where they count routes or pin the old 404 for these two paths: list each change | NFR-008 |
| 6 | `Query` handling: percent-decoding, `+` as a space, last value wins, apply to the new routes through the existing `Query`. `POST` on the new routes is 405 with the CSP | RS-003 |
| 7 | A mutation test: change one `ts_utc` in a temp copy and an HTTP comparison for `/api/usage/sessions` fails naming the entry | RS-003 |
| 8 | The server's stdout and stderr hold only the two startup lines after the new routes were called (no query value, no token outside the URL line) | NFR-002 |
| 9 | `cargo test --workspace` passes under TZ unset, `TZ=Pacific/Kiritimati` and `TZ=Pacific/Pago_Pago` (paste all three); `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` pass; the Python tree check is empty before and after; the oracle hash (310 files as in `tasks/TASK_REVIEW_T011.md`; starts `aa6fcaed`, ends `bff92`) is unchanged | Intent |
| 10 | `rust/README.md`: the `serve` route list has six routes and the `mcp` paragraph six tools; the sentences "`/api/usage/sessions` and `/api/usage/outliers` are not ported and answer 404" are gone; stale references swept (module comments of `api.rs` and `mcp.rs`, test names) | stale-reference sweep |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output in the goldens named above. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden/api__usage_outliers__all.json, rust/parity/golden/api__usage_sessions__all.json, rust/parity/golden/api__usage_sessions__window.json, rust/parity/golden/api__usage_sessions__bad_since.json, rust/parity/golden/mcp__session_outliers__all.json, rust/parity/golden/mcp__tools_list.json, rust/parity/golden-rich/api__usage_outliers__workspace_acme.json, rust/parity/golden-rich/api__usage_sessions__window_mid.json, rust/parity/golden-rich/api__usage_sessions__workspace_acme_account_work.json, rust/parity/golden-rich/api__usage_sessions__bad_since.json, rust/parity/golden-rich/mcp__session_outliers__window.json, rust/parity/golden-rich/manifest.json, rust/parity/golden/manifest.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the real `serve` binary over a temp copy of each store | the 20 session/outlier HTTP goldens (default 4 incl. `bad_since`, rich 16 incl. `bad_since`) equal, counted from the manifests | spawn test |
| 2 | the real `mcp` binary | `tools/list` has six tools equal to the golden; `session_outliers` equals the 3 goldens | spawn test |
| 3 | no token / a bad period / an unknown scope | 403 / 400 / 400 | spawn test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && TZ=Pacific/Kiritimati cargo test --workspace && TZ=Pacific/Pago_Pago cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: the four routes in `api.rs` and the usage entries test helper in `tests/server.rs` (the T019 `scoped` flag); the `usage_summary` tool in `mcp.rs` and its spawn tests.
**Vital slice**: `/api/usage/outliers` end to end (spawn test), then `/api/usage/sessions` with 50, then the error cases, then the MCP tool, then the guard cases and the mutation test.
**Cut list**: no new dependency; no change to the guard, the transport or the core; no generated schemas (keep the hand-written one, equal to the golden); no caching.

`/api/usage/sessions` always passes `Some(50)` (Python's `TIMELINE_DETAIL_SESSIONS`); the route has no `detail_sessions` parameter. The MCP tool takes no scope (Python's does not) and the T019 helper that reads `workspace` / `account` is not used there.

---

## Edge Case Checklist

- [ ] A bad `until` and a bad `since` together: which error wins in Python? Run it once and match it
- [ ] A very long query string or many repeated keys does not panic
- [ ] The MCP `session_outliers` over an empty store (no events): Python's answer; match it
- [ ] `tools/list` order: Python's order in the golden
- [ ] The token never appears in a body, header or log; no caller text in any error
- [ ] Stale references: `rust/README.md`; the "404 for now" comments in `api.rs` / `tests/server.rs`

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/api.rs`, `api_tests.rs` | two routes |
| `rust/crates/app/src/mcp.rs` | the sixth tool |
| `rust/crates/app/tests/server.rs`, `tests/mcp.rs` | the goldens, counts, guard cases |
| `rust/README.md` | routes and tools |
| Pre-existing pin tests | any test that expects five tools or a 404 for these two paths: list each change |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/core/**` | the core is done; if it lacks something, stop and report |
| `rust/parity/**` | the oracle |
| `rust/spikes/**` | the spikes |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: the HTTP comparisons fail while the routes answer 404 and `tools/list` has five tools. Implement slice by slice. Mutation (criterion 7). Run the live probes once against a running `serve` over a temp copy of the rich store (one request per route, a bad `since`, an unknown workspace, no token) and paste them; run the MCP binary by hand once and paste the transcript. Finish with the whole suite under the three zones.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into the report (the Supervisor writes `tasks/TASK_REVIEW_T020.md`)
- [ ] Live probes and the MCP transcript pasted
- [ ] Supervisor notified: task ready for Stage 4 review
