# TASK_GUIDE — T023: The window — `easy-personal-rs app` behind a `window` Cargo feature (Tauri v2)
**Date**: 2026-11-10
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P0
**Assigned agent**: common-infrastructure
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md` (the guard constraint, the window requirements from T016)
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T001, T004, T016, T021)
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Read `rust/docs/spike-T016.md` and `rust/docs/spike-T001.md` (the Tauri addendum) completely, then the whole spike: `rust/spikes/t016-window/{Cargo.toml,build.rs,tauri.conf.json,src/main.rs}`, its `icons/` and `no-capabilities` handling
6. Read `rust/crates/app/src/{server.rs,main.rs,static_files.rs}`, `rust/crates/app/Cargo.toml`, `rust/Cargo.toml`, and `rust/crates/app/tests/server.rs` (the spawn helpers)
7. Do not open `~/.easy-personal` or any `~/.claude*` folder, and do not write that path in any command, not even a guarded `ls`. Tests use temp stores

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): the Rust version runs as a desktop application (a window), Ubuntu first; macOS and Windows later.

**Restated intent**:
> `easy-personal-rs app --static-dir <DIR>` starts the loopback server (T021) in the same process, opens one Tauri v2 window on `http://127.0.0.1:<port>/#<token>`, and ends cleanly when the window closes. Tauri is behind a Cargo feature `window` that is **off by default**, so the CLI, `serve` and `mcp` builds never link it and still run on a machine with no display.

**Out of scope**: building the dashboard (`npm`: nothing is installed or downloaded); embedding the static files in the binary; a tray, an updater, a menu, notifications, auto-start; packaging (`.deb`, AppImage); macOS and Windows (write down what you see in the code that differs, build nothing); any change to `rust/crates/core/**`, `api.rs`, `mcp.rs`; any IPC capability for the page.

**Requirement Refs**: RS-005 (window loads the dashboard from the loopback server with the token guard), RS-006 (no display must not break the CLI and MCP paths), NFR-001 (nothing but `127.0.0.1`), NFR-008 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T016 (the spike), T021 (static serving), T004 / T014 (the server)
**Entry point**: `cargo build --features window` then `easy-personal-rs app --static-dir <DIR>`
**Parallel task**: none. You own `rust/crates/app/{Cargo.toml,build.rs,tauri.conf.json,icons/,capabilities handling,src/window.rs,src/server.rs,src/main.rs}`, `rust/Cargo.toml` (`[workspace.dependencies]` only), `rust/Cargo.lock`, new tests, and `rust/README.md`'s window paragraph. `cargo` may print "waiting for file lock". The Tauri build is slow and large (the spike's `target/` is 3.7 GB, gitignored): that is expected.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | **Feature gate.** `rust/crates/app/Cargo.toml` has `[features] window = [...]` (off by default) with `tauri` pinned `=2.12.5` (or what `cargo info` shows as the latest stable if the T001 note is out of date: say which), `tauri-build` pinned (as the spike: `=2.7.4`) as an optional build-dependency, and `gtk` only if you need it, all `optional = true`; pins use `=` and go in `[workspace.dependencies]`. Without the feature: `cargo build` and `cargo test --workspace` compile **no** Tauri, GTK or WebKit crate (show `cargo tree -p easy-personal-rs -e normal --prefix none | grep -ci "tauri\|wry\|webkit\|gtk"` printing `0`, and the time of a default build), and every existing test passes unchanged. The `build.rs` runs `tauri_build` only when `CARGO_FEATURE_WINDOW` is set | RS-006 |
| 2 | **Server reuse.** `server::run` is split so the window can start the same server: a function (for example `server::start(static_dir: Option<&Path>) -> Result<Running, String>`) binds `127.0.0.1:0`, makes the token, and returns the port, the token and a way to stop it (graceful shutdown); `run` (the `serve` subcommand) is that function plus the two printed lines and the Ctrl-C wait. `serve`'s stdout, stderr, exit codes and every `tests/server.rs` test are unchanged | Intent |
| 3 | **`app` subcommand.** `easy-personal-rs app --static-dir <DIR>` exists in every build. Without the feature it prints one line (`this build has no window: rebuild with --features window`, no path) and exits with a non-zero code (state which); with the feature it starts the server, opens the window and runs until the window closes. `--static-dir` is required for `app` (clap error otherwise); the directory is validated as in T021 before anything is shown. The window is titled `easy-personal`, about 1280x800, resizable, one window only | RS-005 |
| 4 | **URL and token.** The window opens `http://127.0.0.1:<port>/#<token>` (the fragment carries the token: nothing else does). The token is never put in the query string, never printed, never logged; the page URL in any log line is printed **without the fragment** (T016: Tauri's page-load payload includes it). stdout stays empty in normal operation; stderr gets at most one line per notable event (start, window closed) and no URL fragment | RS-005, NFR-008 |
| 5 | **No IPC, no extra power.** The window gets no IPC capability: `tauri.conf.json` has `withGlobalTauri: false`, no plugin, no `shell`, `http`, `fs`, `updater` or `opener` anything, an empty capabilities set (as the spike's `no-capabilities`), and devtools are not enabled in a release build. Show (a unit test on the config file or `cargo tree` plus the file) that no capability file grants the remote origin a command. The `security.csp` of the Tauri config is left null: the CSP comes from the server's header (T016) | NFR-008 |
| 6 | **Navigation lock.** The webview may navigate only within `http://127.0.0.1:<port>` (the origin of the server); any other navigation and any new-window request is blocked (Tauri's `on_navigation` / `on_new_window` or the Tauri v2 equivalent; the dashboard has plain external `<a href>` links in its sidebar: clicking one must do nothing, not load a remote site in the app). A blocked URL is logged without query and fragment, host only. Unit-test the decision function with: same origin with path and fragment (allowed), another port, `localhost`, another host, `https`, `file:`, `data:`, `javascript:`, an empty string (all blocked) | NFR-001, NFR-008 |
| 7 | **Display preflight with a timeout.** Before Tauri is touched, `app` checks that a display can be reached, without risking T016's hang (a dead `DISPLAY=:99` made GTK init hang; an unset display made tao panic with exit 101): no `DISPLAY` and no `WAYLAND_DISPLAY` -> one clear line, exit 5, within one second; a `WAYLAND_DISPLAY` is checked by connecting to `$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY` (a unix socket, absolute `WAYLAND_DISPLAY` allowed) with a timeout; a `DISPLAY` is checked by connecting to the X11 socket (`/tmp/.X11-unix/X<N>` for `:N`, or TCP `host:6000+N` with a timeout of 2 s; use the T016 notes) — when either works, go on; when both fail, exit 5 with a message that names the variable and the problem but not the socket path. The whole preflight never takes longer than 5 s. Unit-test the parsing (`:0`, `:0.0`, `host:1`, `unix:/path`, empty, garbage) and the connect logic against a temp unix socket and a closed one. Run it for real with `DISPLAY=:99 WAYLAND_DISPLAY=` and with both unset (a Supervisor will repeat these) | RS-006 |
| 8 | **Clean exit.** Closing the window stops the server and exits 0; SIGINT and SIGTERM do the same; no `WebKitWebProcess`, `WebKitNetworkProcess` or `easy-personal-rs` process is left (check with `ps` by pid / parent, never `pkill -f` with a pattern in your own command line). No panic on a display that goes away is required, but say what happens | RS-005 |
| 9 | **Self-test mode (so the Supervisor can verify without eyes).** A hidden flag `app --self-test` (not in `--help`) makes the process exit 0 with the single stdout line `self-test: ok` as soon as the page has made one **authenticated** `/api/*` request (a request that passed the guard with the token header), and exit 3 with one stderr line after 20 s if it did not; it closes the window either way. The guard layer records that an authenticated request was seen in a small shared flag (an `AtomicBool`); nothing else changes in the guard. A fixture page in the test directory reads the token from `location.hash` and calls `fetch('/api/version', {headers: {'x-dashboard-token': token}})`. Run it for real under WSLg: `cargo run --features window -- app --self-test --static-dir <fixture>` printing `self-test: ok`, exit 0, time, and the peak memory if you can measure it | RS-005 |
| 10 | **No-display paths still work with the feature built.** With `--features window` built and `DISPLAY`, `WAYLAND_DISPLAY` and `XDG_RUNTIME_DIR` unset: `serve` (start, one request, stop), `mcp` (initialize, `tools/list`, `tools/call usage_summary` over a temp store) and `store-info` behave as before and never load GTK. Show the three runs | RS-006 |
| 11 | **Network containment.** Nothing but `127.0.0.1` is contacted: read the Tauri config and the enabled Tauri features and say what could reach out (updater, telemetry, webview auto-update checks, `about:blank` pages); the server binds `127.0.0.1` only. Confirm with `ss -tnp` (or `lsof -i`) during the self-test that the process and its children hold only loopback sockets | NFR-001 |
| 12 | `cargo test --workspace` (no feature) passes under TZ unset, `TZ=Pacific/Kiritimati` and `TZ=Pacific/Pago_Pago` (paste all three); with `--features window`, `cargo test --workspace --features window` (unit tests of the preflight, the navigation decision and the config) passes too; `cargo clippy --workspace --all-targets -- -D warnings` and the same with `--features window` are clean; `cargo fmt --check` clean; the Python tree check (`git status --short -- src web tests pyproject.toml uv.lock`) is empty before and after; the oracle hash (310 files as in `tasks/TASK_REVIEW_T011.md`; starts `aa6fcaed`, ends `bff92`) is unchanged | Intent |
| 13 | `rust/Cargo.lock` is updated additively; state how many packages it gained, and list any new direct dependency with its license (`cargo tree` / `Cargo.lock`: the Tauri tree is the T001 one). `rust/README.md`: a window paragraph (the feature, `app`, `--static-dir`, `--self-test` is hidden, the display preflight, the navigation lock, the Ubuntu packages needed from T001 and that macOS and Windows are not done); stale references swept (the `serve`-only wording) | stale-reference sweep |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The evidence is a real run: a claim that the window works without `self-test: ok` from the real binary does not count. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/docs/spike-T016.md

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | default build | no Tauri crate compiled, all existing tests green | `cargo tree`, `cargo test` |
| 2 | `--features window`, WSLg display, a fixture page | `self-test: ok`, exit 0 | real run |
| 3 | no display / dead display | exit 5 in <= 5 s with a clear line, no hang, no panic | real runs with a timeout |
| 4 | the navigation decision table | only the server's origin allowed | unit test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain and, for the real run, a display (WSLg) with the Tauri system packages (T001)
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check && cargo build --features window && cargo test --workspace --features window && timeout 60 cargo run --features window -- app --self-test --static-dir <fixture folder>
```

---

## Approach

**Pattern reference**: `rust/spikes/t016-window/src/main.rs` (window creation, `gtk::init` preflight, the page-load logging) and `rust/crates/app/src/server.rs`.
**Vital slice**: first the feature gate and the default build proof (criterion 1); then `server::start`; then the window opening on the fixture with `--self-test`; then the preflight; then the navigation lock; then the no-display runs and the containment check.
**Cut list**: no tray, menu, updater, packaging; no embedding; no macOS or Windows; no frontend; no IPC; no new crate beyond the Tauri tree.

Keep every Tauri `use` inside `src/window.rs` under `#[cfg(feature = "window")]`, as `rmcp` stays inside `mcp.rs`. Tauri's `generate_context!` needs a `frontendDist`: use a tiny stub folder with a name other than `dist` (gitignored), as the spike did. If the product needs a capability file to compile, it is an **empty** set.

If a criterion cannot be met (for example the navigation hook does not exist in the pinned Tauri version), stop that criterion, say why with evidence, and propose the nearest safe alternative; do not weaken the lock silently.

---

## Edge Case Checklist

- [ ] The port or the token never appears in `ps` arguments (they are not passed as arguments)
- [ ] The server stops when the window is closed even if a request is in flight
- [ ] Two `app` runs at once get two free ports and two tokens
- [ ] `--static-dir` missing, not a directory, a file: one clear line, exit non-zero, no window, no panic
- [ ] `WAYLAND_DISPLAY` set to a name with a slash or `..` is only connected to, never read as a file
- [ ] The preflight reads no file from `~` and opens no `~/.claude*` or `~/.easy-personal` path
- [ ] Stale references: `rust/README.md`; the "one binary, one subcommand" wording; module comments

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/Cargo.toml`, `rust/Cargo.toml`, `rust/Cargo.lock` | the feature, pinned optional deps |
| `rust/crates/app/build.rs`, `tauri.conf.json`, `icons/`, `stub-frontend/` | Tauri build (only with the feature) |
| `rust/crates/app/src/window.rs` (new), `src/display.rs` (new, the preflight, compiled always so it is testable) | the window, the preflight and the navigation decision |
| `rust/crates/app/src/server.rs`, `main.rs` | `start`, the authenticated-request flag, the `app` subcommand |
| `rust/crates/app/tests/` | spawn tests that do not need a display (no-display exit, missing `--static-dir`, no-feature message) |
| `rust/README.md` | the window paragraph |
| Pre-existing pin tests | none expected: list any |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/core/**`, `rust/crates/app/src/{api,mcp,static_files}.rs` | not part of this task (a one-line visibility change in `static_files.rs` only if unavoidable: say so) |
| `rust/parity/**` | the oracle |
| `rust/spikes/**` | the spikes (read only) |
| `web/**`, `src/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app and the dashboard sources; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first for what can be red without a display: the default-build proof, the preflight parsing and connect tests, the navigation table, the no-feature message, the missing `--static-dir`. Then the feature build and the real `--self-test` run under WSLg; then the failure runs (`DISPLAY=:99` with `WAYLAND_DISPLAY` unset and `GDK_BACKEND=x11`, both unset) each under `timeout 30`; then `serve`, `mcp` and `store-info` with no display. Take the memory and time numbers from a real run and say so. Finish with the suite under three zones, with and without the feature.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Tests written AND pass — output pasted into the report (the Supervisor writes `tasks/TASK_REVIEW_T023.md`)
- [ ] Real runs pasted (self-test, no-display, dead display, `serve` / `mcp` / `store-info` without a display)
- [ ] Supervisor notified: task ready for Stage 4 review
