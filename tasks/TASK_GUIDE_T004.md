# TASK_GUIDE — T004: P1 slice — loopback HTTP server, Host/Origin/token guard, `/api/version`
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: High
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
5. Read `rust/docs/spike-T001.md` (axum/tokio pins), `BRAINSTORMING_LOG_RUSTPORT.md` (edge cases) and `rust/docs/SPEC.md` (RS-001, RS-005)
6. Read the Python guard: `src/easy_personal/surfaces/api.py` (`Guard`, `TOKEN_HEADER`, `CSP`, `LIVE_PATH`) and the goldens `rust/parity/golden/api__guard__*.json` with their entries in `rust/parity/golden/manifest.json`

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): the Rust app runs all features; the version is visible in the app; the window (later) loads the dashboard from an in-process server on `127.0.0.1`.

**Restated intent**:
> The Rust binary can serve HTTP on `127.0.0.1` only, with the same Host/Origin/token guard and CSP header as the Python app, and answers `GET /api/version`. It is the trust boundary every later route and the window sit behind.

**Out of scope**: any other route; the React bundle and static files (P1b); `/api/live` and its token-in-query exception; websockets; the window; MCP; store access; CORS; TLS.

**Requirement Refs**: RS-001 (API part), RS-005 (server side) in `rust/docs/SPEC.md`; Python `PRD.md` NFR-001, NFR-008.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T001 — verified crate pins (`axum` =0.8.9, `tokio` =1.54.1); T003 — the `api__guard__*` goldens
**Entry point**: `easy-personal-rs serve` (the CLI subcommand) and `GET /api/version`

**Parallel task**: T005 runs at the same time in the `core` crate. You own `rust/crates/app/**`; T005 owns `rust/crates/core/**`. Both of you edit `rust/Cargo.toml` and `rust/Cargo.lock`: re-read `rust/Cargo.toml` immediately before every edit, change only your own lines, and expect `cargo` to print "waiting for file lock" while the other task builds.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `easy-personal-rs serve` binds **only** `127.0.0.1` on a free port (port 0), prints exactly the two lines Python's `serve()` prints (`src/easy_personal/surfaces/api.py`): `dashboard: http://127.0.0.1:<port>/#<token>` and `Ctrl-C to stop.`, and stops on Ctrl-C. No browser is opened in this task | NFR-001, RS-005 |
| 2 | The token is 32 bytes from the OS random source, URL-safe base64 without padding (the shape of Python's `secrets.token_urlsafe(32)`), generated per run. It appears only in that printed URL fragment (the page reads it from `#`, so it is never sent in a request path) and is never logged, echoed in an error or put in a response | NFR-008 |
| 3 | A request whose `Host` header is not exactly `127.0.0.1:<port>` gets `403` with body `{"error": "forbidden"}` equal (as parsed JSON) to `api__guard__bad_host.json` | NFR-008 |
| 4 | A request carrying an `Origin` header other than `http://127.0.0.1:<port>` gets `403` equal to `api__guard__bad_origin.json`; a request with no `Origin` header is not refused for that reason | NFR-008 |
| 5 | A request to a path starting `/api/` without the Python `TOKEN_HEADER` header equal to the token gets `403` equal to `api__guard__no_token.json`. A wrong token also gets 403. The comparison is constant-time | NFR-008 |
| 6 | `GET /api/version` with a good Host and token returns `200` and JSON `{"version": "<Cargo package version>"}`; `Content-Type: application/json` | RS-001 |
| 7 | Every response, including 403 and 404, carries the same `Content-Security-Policy` value as Python's `CSP` constant | NFR-008 |
| 8 | A request to `/api/version` with a token in the query string but no header is refused (the query-token exception is for `/api/live` only, which is not built) | NFR-008 |
| 9 | Negative: the server socket is not reachable through a non-loopback address (test binds, then asserts the bound address is `127.0.0.1`) | NFR-001 |
| 10 | `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python guard's behaviour captured in `rust/parity/golden/api__guard__*.json` and `manifest.json` (status codes, request shapes). The implementing agent must not edit those files.
> **Locked oracle**: rust/parity/golden/api__guard__bad_host.json, rust/parity/golden/api__guard__bad_origin.json, rust/parity/golden/api__guard__no_token.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | good Host, good token, `GET /api/version` | 200 `{"version":"0.1.0"}` | test over a raw TCP client |
| 2 | `Host: localhost:<port>` | 403 equal to the golden body | test |
| 3 | `Origin: http://evil.example` | 403 | test |
| 4 | no token header / wrong token | 403 | test |
| 5 | any response | CSP header equals Python's string | test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `src/easy_personal/surfaces/api.py` `Guard._allowed` (order of checks: Host, then Origin if present, then token for `/api/`) and `rust/crates/app/tests/cli.rs` (how tests run the binary and use temp dirs).
**Vital slice**: the guard (Host, Origin, token, CSP) in front of one route, `/api/version`. Token shape and the printed line follow `serve()` in `src/easy_personal/surfaces/api.py` lines 737-749.
**Cut list**: no static files, no `/api/live`, no websockets, no other route, no logging framework, no graceful-shutdown beyond Ctrl-C, no HTTP client crate in tests (use a raw `TcpStream`).

Use `axum` =0.8.9 and `tokio` =1.54.1 with the same feature set as the spike. The guard is one middleware layer so later routes inherit it. For OS randomness add the smallest well-known crate (check it with `cargo info`; pin with `=`) — do not read `/dev/urandom` directly (Windows is a later target). Constant-time compare: a small crate or ~5 lines; no `==` on the token.

---

## Edge Case Checklist

- [ ] `Host` header missing or with different case/spacing/port → refused
- [ ] Two `Host` headers, or an absolute-form request line → refused or handled like Python (state which)
- [ ] `Origin: null` → refused (not equal to the allowed origin)
- [ ] Token header name matched case-insensitively (HTTP headers are)
- [ ] The token never appears in any response body, error message or log line
- [ ] IPv6 `[::1]` is not bound
- [ ] Server shuts down on Ctrl-C and frees the port
- [ ] DNS-rebinding case: a `Host` of another name pointing at 127.0.0.1 is refused
- [ ] Stale references: after the change, `README.md` in `rust/` lists the new command

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/Cargo.toml` | add `axum`, `tokio` and the OS-random crate to `[workspace.dependencies]` (re-read first; T005 edits it too) |
| `rust/crates/app/Cargo.toml` | use them |
| `rust/crates/app/src/server.rs` | new — router, guard layer, `/api/version` |
| `rust/crates/app/src/main.rs` | add the `serve` subcommand only (T005 does not edit it) |
| `rust/crates/app/tests/server.rs` | new — raw-socket tests |
| `rust/README.md` | document `serve` |
| Pre-existing pin tests | None — the Rust code has no pin tests; `rust/crates/app/tests/cli.rs` must still pass |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/core/**` | T005 owns it |
| `rust/parity/**` | the oracle; locked |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Write the failing guard tests first (red run), then implement. Tests start the server in-process on port 0 and talk to it with a raw `TcpStream`, comparing JSON bodies with the three golden files parsed as JSON.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (Risk High)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T004.md`
- [ ] Run the binary: `serve`, then a request with and without the token
- [ ] Supervisor notified: task ready for Stage 4 review
