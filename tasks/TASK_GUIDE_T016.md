# TASK_GUIDE — T016: Window spike (Tauri v2 under WSLg, throwaway)
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: common-infrastructure
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T001, T004, T014)
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Read `rust/docs/spike-T001.md` (the Tauri addendum), `rust/spikes/t001-crates/Cargo.toml` and how `rust/Cargo.toml` keeps spikes out of the workspace
6. Read `rust/crates/app/src/server.rs` (the guard and the CSP: `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'`)
7. Do not open `~/.easy-personal` or any `~/.claude*` folder, not even to check that it exists

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): the Rust version runs as a desktop application (a window), Ubuntu first.

**Restated intent**:
> Before building the window for real (P1b), prove on this machine (Ubuntu 26.04 on WSL2, WSLg) that a Tauri v2 window opens, that its webview loads a page from a `127.0.0.1:<port>` server, and that the page can read the token from the URL fragment and call the server with the `x-dashboard-token` header under the dashboard's CSP. Throwaway code only.

**Out of scope**: any product code; touching `rust/crates/**`; the real dashboard, static serving, tray, packaging; macOS and Windows (record what you can read about them, build nothing).

**Requirement Refs**: RS-005 (window loads the dashboard from the loopback server with the token guard), RS-006 (no display must not break the CLI and MCP paths), NFR-001 / NFR-008 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T001 (Tauri compiled after the owner's apt install: 41 s `cargo check` of a scratch crate)
**Entry point**: `cargo run` in `rust/spikes/t016-window/` (a throwaway binary), and the report `rust/docs/spike-T016.md`
**Parallel task**: T017 works in `rust/crates/core/**`. Do not edit those. You own `rust/spikes/t016-window/**` and `rust/docs/spike-T016.md`.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | A separate Cargo project `rust/spikes/t016-window/` (its own `Cargo.lock`, outside the main workspace, `target/` already gitignored) builds with `tauri` pinned `=2.12.5` (or the version `cargo info` shows as latest stable if the T001 note is out of date: say which) and the system packages already installed. If a package is missing, stop and report the exact `apt install` line; do not run `sudo` | RS-005 |
| 2 | The spike starts its own tiny loopback server on `127.0.0.1:<free port>` that mimics the dashboard guard: `GET /` serves a small HTML page; `GET /api/version` answers 200 `{"version":"spike"}` only with the exact `Host`, no foreign `Origin` and the `x-dashboard-token` header equal to a random token, otherwise 403 `{"error":"forbidden"}`; every response carries the dashboard CSP above. It opens a Tauri window at `http://127.0.0.1:<port>/#<token>` | RS-005, NFR-008 |
| 3 | The page's own JavaScript (inline scripts are not allowed by that CSP, so serve it as `/app.js` from the same origin) reads the token from `location.hash`, calls `fetch('/api/version', {headers: {'x-dashboard-token': token}})`, and also tries a call **without** the token, then posts a JSON report to `/report` (with the token header): window opened (`document.readyState`), the hash was present, the two fetch statuses, `location.origin`, `navigator.userAgent`, whether `window.__TAURI__` or `window.__TAURI_INTERNALS__` exists (a remote origin may not get IPC: record the fact), and any `securitypolicyviolation` events | RS-005, NFR-008 |
| 4 | The spike process prints the received report as one JSON line and exits 0 within 20 s; it exits non-zero with a clear message if no report arrives in 20 s. No browser tab is opened and nothing but `127.0.0.1` is contacted | NFR-001 |
| 5 | It proves the window really opened on this machine: the report arrives from inside the webview (so the page loaded and ran its script), and the spike also logs the Tauri window-created and page-load events. State whether `DISPLAY`/`WAYLAND_DISPLAY` were set and which backend GTK used | RS-005 |
| 6 | Failure modes recorded with evidence: run once with `DISPLAY` and `WAYLAND_DISPLAY` unset (does it fail fast with a clear error or hang? use a timeout), and once with the usual WSLg workaround variables if the normal run fails (for example `WEBKIT_DISABLE_COMPOSITING_MODE=1`, `GDK_BACKEND=x11`, `LIBGL_ALWAYS_SOFTWARE=1`): list which variables were needed, if any | RS-006 |
| 7 | Measurements, taken from a real run and stated as such: time from process start to the report, the peak resident memory of the process tree, and the size of the release binary (`cargo build --release` once; state the build time) | NFR-005 |
| 8 | `rust/docs/spike-T016.md` (about 60 lines) answers: does a Tauri window open here; what the webview is (WebKitGTK version); whether the fragment token and the custom header work under the CSP; whether a remote origin gets Tauri IPC (and what capability config would be needed if the product wants it); the env variables needed; the measurements; what P1b must therefore do (the product server must serve the static page and `/app.js`-style assets from the same origin, and the window must open the URL with the fragment); and anything blocking | Intent |
| 9 | Product code is untouched: `rust/crates/**`, `rust/Cargo.toml` and `rust/Cargo.lock` have no change (compare a file list and hashes before and after); `git status --short -- src web tests pyproject.toml uv.lock` is empty before and after | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The evidence is a real run. A claim that the window opened without the report coming from inside the webview does not count.
> **Locked oracle**: none (a spike has no golden)

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | `cargo run` in the spike | one JSON line with the in-webview report, exit 0 | command output |
| 2 | the report | `fetch` with the token 200, without 403, hash present | the JSON line |
| 3 | no display variables | a clear failure or a documented hang with a timeout | command output |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs a display (WSLg) and the Tauri system packages on the owner's machine
. "$HOME/.cargo/env" && cd rust/spikes/t016-window && timeout 60 cargo run 2>&1 | tail -20
```

---

## Approach

**Pattern reference**: `rust/spikes/t001-crates/` (a spike project) and the Tauri addendum of `rust/docs/spike-T001.md`.
**Vital slice**: first a window that loads a static page; then the token/CSP behaviour; then the failure modes and measurements.
**Cut list**: no frontend framework; no bundler; no Tauri CLI; no icons beyond what the build needs; no code reuse from the product crates.

Tauri 2 can open a window on an external URL with `WebviewUrl::External`. Keep the Tauri `build.rs`, `tauri.conf.json` and capability files minimal. If the build demands an icon, generate the smallest valid one in the spike folder.

---

## Edge Case Checklist

- [ ] The token is random per run and never printed in full (print only its length) except inside the URL the window opens, which is not logged
- [ ] The spike server binds `127.0.0.1` only
- [ ] No traffic leaves the machine (no update check, no telemetry: say how you checked, for example by reading what the Tauri config enables)
- [ ] The process always exits (a window left open after the report arrives counts as a failure); no zombie `WebKitWebProcess` remains (check `ps`)
- [ ] Do not use `pkill -f` with a pattern that appears in your own command line; stop a stuck run with `kill <saved pid>`
- [ ] Tests and probes never open `~/.easy-personal` or `~/.claude*`

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/spikes/t016-window/**` | new throwaway project |
| `rust/docs/spike-T016.md` | new, the findings |
| Pre-existing pin tests | None |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/**`, `rust/Cargo.toml`, `rust/Cargo.lock` | the product; T017 works there |
| `rust/parity/**` | the oracle |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Build, run, read the report line, then the failure-mode runs and the measurements. The spike is its own test: no unit tests.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review done
- [ ] Real-run output pasted into `tasks/TASK_REVIEW_T016.md` (by the Supervisor from your report)
- [ ] No product file changed (hashes before and after)
- [ ] Supervisor notified: task ready for Stage 4 review
