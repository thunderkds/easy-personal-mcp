# TASK_GUIDE — T021: Serve the dashboard's static files from `serve` (`--static-dir`)
**Date**: 2026-11-10
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P0
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md` (the guard constraint: the guard keys on the raw path; the CSP; the window requirements from T016)
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T004, T014, T016)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/app/src/{server.rs,main.rs}` and `rust/crates/app/tests/server.rs` (the guard layer, `json()`, the raw-path tests, the spawn helpers), and `rust/docs/spike-T016.md`
6. Read the Python you match: `src/easy_personal/surfaces/api.py` lines 66-130 (`static_dir`, `Guard`) and 720-728 (the `Mount("/", StaticFiles(directory=static, html=True))`). Starlette's `StaticFiles` is the behaviour to follow for the cases below
7. Read `rust/spikes/t016-window/src/main.rs` for how the spike served `/` and `/app.js` (throwaway code: do not copy it, read what it needed)
8. Do not open `~/.easy-personal` or any `~/.claude*` folder, and do not write that path in any command, not even a guarded `ls`. Tests use temp directories you create

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): the Rust version runs as a desktop application (a window), Ubuntu first.

**Restated intent**:
> `easy-personal-rs serve --static-dir <DIR>` also serves the files of `<DIR>` (the built dashboard) from the same origin as the API, behind the same Host / Origin guard and with the same CSP on every response, so the window of P1b can open `http://127.0.0.1:<port>/#<token>`. Without the flag the server behaves exactly as today.

**Out of scope**: building the dashboard (`npm`, `vite`: nothing is installed or downloaded); embedding the files in the binary; the window; any change to `rust/crates/core/**`; the routes that are not ported (they stay 404); `Range` requests, conditional requests (`If-None-Match`, 304), compression, directory listings, redirects for a directory without a trailing slash.

**Requirement Refs**: RS-004 / NFR-008 (guard on the origin), RS-005 (window loads the dashboard from the loopback server), NFR-002 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T004 (server and guard), T014 (routes), T016 (what the window needs)
**Entry point**: `easy-personal-rs serve --static-dir <DIR>`
**Parallel task**: T022 works in `rust/crates/core/src/**` (a new `workspaces.rs`), `rust/crates/app/src/api.rs` and `api_tests.rs` (two GET routes) and the new tests for them. You own `rust/crates/app/src/{server.rs,main.rs}`, a new `rust/crates/app/src/static_files.rs` if you want one, `rust/crates/app/tests/server.rs` (append; do not edit the existing tests except where a criterion says so), a new `rust/crates/app/tests/static_files.rs` if you want one, and `rust/README.md`'s `serve` paragraph. Do not edit `api.rs`. Re-read `Cargo.toml` and `Cargo.lock` before editing; `cargo` may print "waiting for file lock"; if a build fails only inside the core or `api.rs`, that is T022 in progress: wait a minute and retry.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `serve` takes an optional `--static-dir <DIR>` (a clap argument). The directory is resolved once at start; a path that is not a directory makes `serve` exit non-zero with a clear one-line message that does not print the path's content; nothing is created. Without the flag every existing behaviour and test is unchanged (the 13 original server tests and the T014 ones pass unchanged), including `GET /` answering what it answers today | Intent |
| 2 | With the flag: `GET /` serves `index.html`; `GET /<dir>/` serves that directory's `index.html` when it exists (Starlette `html=True`); `GET /assets/app.js` serves the file. Status 200, `Content-Length` of the body, a `Content-Type` by extension for at least `.html` (`text/html; charset=utf-8`), `.js` (`text/javascript; charset=utf-8`), `.css` (`text/css; charset=utf-8`), `.json`, `.svg`, `.png`, `.ico`, `.woff2`, `.txt`, and `application/octet-stream` otherwise; the CSP header is on every response (hit, miss, error). `HEAD` answers the same headers with no body. Other methods answer 405 with the CSP | RS-005 |
| 3 | A miss is `404` (a fixed body, no path echoed, the CSP); a directory with no `index.html` is 404, never a listing; a path that is a directory without a trailing slash is 404 (no redirect: cut list). An `/api/...` path that no route serves is still the JSON 404 it is today, never a static file, even when a file of that name exists in the directory | NFR-008 |
| 4 | The guard covers static files as Python's `Guard` does: a wrong `Host` or a foreign `Origin` on any static path is `403 {"error":"forbidden"}` with the CSP; **no token is needed** for a non-`/api/` path (the page must load before it can read the fragment); `/api/*` still needs the token exactly as before; a token in the query string is still refused on `/api/*` routes and is simply ignored on a static path | NFR-008 |
| 5 | **Path safety** (High risk, test every item with a fixture directory you create and a file *outside* it holding a marker string): `..`, `%2e%2e`, `%2E%2E`, `..%2f`, `%2e%2e%2f`, `..\\`, `%5c`, double-encoded `%252e%252e`, a NUL byte (`%00`), an absolute-looking path (`//etc/passwd`, `/%2fetc/passwd`), and a trailing-dot or overlong path never return the outside file (404 or 403, never 200) and never panic; a **symlink inside the directory that points outside it** is 404 (canonicalise the resolved path and require it to stay under the canonical root; do not follow it); a symlink that points to a file inside the directory may be served; a path that names a non-regular file (a FIFO, a socket, a device) is 404 without blocking; a file whose name starts with `.` (for example `.web-stamp`, `.env`) is 404 (differs from Python, which serves it: say so in the code comment and the report). The marker string of the outside file never appears in any response | NFR-008, NFR-002 |
| 6 | The raw-path rule of the guard still holds: `/%61pi/usage/summary`, `//api/usage/summary`, `/api%2fusage%2fsummary` and the encoded forms T014 added are never served as an API route **and never as a static file either** (the static fallback must not decode a path into `/api/...` and serve a file of that name; test with a file `api/usage/summary` inside the fixture directory) | NFR-008 |
| 7 | Files are read per request (a file changed between two requests is served new; no cache), by `tokio::fs` or inside `spawn_blocking` (never a blocking read on an async worker); a file larger than 32 MiB is 404 (say why in a comment: the dashboard's biggest file is far smaller); the fixture directory's bytes are unchanged after the tests; nothing is written | Intent |
| 8 | The T016 findings hold with the real server: a spawn test starts `serve --static-dir <fixture>`, reads the printed `dashboard:` URL, and (with plain HTTP requests, no browser) shows that `GET /` and `/app.js` load without a token, that `GET /api/version` with the token header is 200 and without it 403, and that the CSP is the dashboard CSP on all of them. The printed URL carries the fragment; the server's stdout and stderr hold only the two startup lines after those requests | RS-005 |
| 9 | No new runtime dependency (use `tokio::fs`, `axum`, std; a MIME table by extension written in the code is fine, the `mime_guess` crate is not needed); no change to `rust/crates/core/**`, `api.rs`, `mcp.rs`; `cargo test --workspace` passes under TZ unset, `TZ=Pacific/Kiritimati` and `TZ=Pacific/Pago_Pago` (paste all three); `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` pass; the Python tree check (`git status --short -- src web tests pyproject.toml uv.lock`) is empty before and after; the oracle hash (310 files as in `tasks/TASK_REVIEW_T011.md`; starts `aa6fcaed`, ends `bff92`) is unchanged | Intent |
| 10 | `rust/README.md`: the `serve` paragraph documents `--static-dir`, the no-token rule for static paths, the dotfile and symlink rules and the 32 MiB cap; stale references swept (module comments of `server.rs`) | stale-reference sweep |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle here is Starlette's `StaticFiles` behaviour for the cases the criteria list, plus the T016 findings. A path-safety test that returns 200 for a file outside the directory is a failed task. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/docs/spike-T016.md, rust/parity/golden/api__guard__bad_host.json, rust/parity/golden/api__guard__bad_origin.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | `serve --static-dir <fixture>` | `/` and `/assets/x.js` load with no token, correct types, the CSP | spawn test |
| 2 | every traversal form of criterion 5 | never 200, outside marker never in a body | spawn test |
| 3 | `serve` with no flag | exactly the current behaviour | existing tests |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && TZ=Pacific/Kiritimati cargo test --workspace && TZ=Pacific/Pago_Pago cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: the fallback and guard layer in `server.rs` and the spawn / raw-path tests in `tests/server.rs`.
**Vital slice**: `GET /` and one file end to end with the CSP (spawn test); then the content types and HEAD; then the no-token rule and the guard; then the path-safety table; then the large-file cap and the T016 spawn test.
**Cut list**: no embedding; no cache or conditional requests; no Range; no compression; no redirects; no new dependency; no Tauri.

Resolve a request path like this: take the **raw** path from the request target; refuse (404) anything containing a backslash, a NUL, an encoded slash or backslash (`%2f`, `%5c`) or `%25` after decoding, or any segment equal to `.` or `..` after percent-decoding once; split on `/`, ignore empty segments only for the trailing slash; join under the canonical root; canonicalise and require the result to start with the canonical root; `symlink_metadata` the final component and refuse a symlink whose target leaves the root (or simply refuse every symlink and say so: the simpler choice is fine). Decide and state which. Do the check on the resolved path, not on the text alone.

---

## Edge Case Checklist

- [ ] A directory named like a file extension; a file with no extension; a file with a UTF-8 name (the dashboard has none, but percent-decoded UTF-8 must work or be a clean 404, never a panic)
- [ ] A very long path (60,000 characters) and 3,000 `/` segments do not panic
- [ ] `index.html` missing from the directory: `GET /` is 404
- [ ] The fixture directory given with a trailing slash, as a relative path, and through a symlink (the canonical root is what counts)
- [ ] An empty file; a file of exactly 32 MiB and one byte more
- [ ] No absolute path or file content in any error body or log line (NFR-002)
- [ ] The token never appears in a response, header or log
- [ ] Stale references: `rust/README.md`; the "one route" / "no static" wording in `server.rs`

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/server.rs` | the optional static fallback, wiring of the directory |
| `rust/crates/app/src/static_files.rs` | new (optional): path resolution, MIME table, response |
| `rust/crates/app/src/main.rs` | `serve --static-dir` |
| `rust/crates/app/tests/server.rs` or `tests/static_files.rs` | new cases, spawn test |
| `rust/README.md` | the `serve` paragraph |
| Pre-existing pin tests | any test that pins `GET /` or an unknown non-API path as 404: it must still pass with no flag (state each) |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/app/src/api.rs`, `api_tests.rs`, `mcp.rs` | T022 owns `api.rs`; the MCP server is not involved |
| `rust/crates/core/**` | T022 works there |
| `rust/parity/**` | the oracle |
| `rust/spikes/**` | the spikes |
| `web/**`, `src/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app and the dashboard sources; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: the spawn tests fail while `--static-dir` does not exist and `/` is 404. Implement slice by slice. The path-safety table is a table-driven test over the fixture directory and a sibling "outside" directory with a marker file. Mutation: temporarily remove the canonical-root check and see the symlink and `..` cases fail (say what you did and put it back). Run the live probes once by hand against a running `serve --static-dir <fixture>`, pasting the requests (token redacted): `/`, a file, a miss, a traversal, a symlink, no token on `/api/version`, a wrong Host. The dashboard token is base64url (`A-Za-z0-9_-`), not hex. Finish with the whole suite under three zones.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into the report (the Supervisor writes `tasks/TASK_REVIEW_T021.md`)
- [ ] Live probes pasted
- [ ] Supervisor notified: task ready for Stage 4 review
