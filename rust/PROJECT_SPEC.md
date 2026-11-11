# PROJECT_SPEC.md — Rust port (`rust/`)
**Last updated**: 2026-11-10
**Version**: 0.1

> **Scope of this document**: *How* to build it safely — architecture, constraints, risk areas, task state, learnings.
> Product intent lives in `rust/docs/SPEC.md` (stand-in for a PRD); the full requirement set is the Python app's `PRD.md` at the repo root. The repo-root `PROJECT_SPEC.md` still describes the Python app.

---

## Project Identity

- **Name**: easy-personal-rs
- **Repo**: this repo (`easy-personal-mcp`), folder `rust/`. Owner decision 2026-11-10: one repo, no sibling
- **Primary tech**: Rust (stable); desktop window shell (candidate: Tauri v2); SQLite
- **Type**: native desktop app + CLI + MCP server in one binary — a staged port of `easy-personal-mcp` (Python)
- **Deployment target**: local machine only. Ubuntu first, then macOS, Windows last
- **Key stakeholders**: owner (quannhm1604)

---

## Architecture Summary

A Rust core replaces the Python implementation slice by slice. It opens the same SQLite store (`~/.easy-personal/store.sqlite`), serves the same `/api/*` and MCP contracts, and shows the existing React bundle in a desktop window loaded from the in-process `127.0.0.1` server. The Python project stays the parity oracle until each surface flips, then becomes maintenance-only and is removed. Full reasoning: `BRAINSTORMING_LOG_RUSTPORT.md` (repo root).

---

## Readiness

> The files that prove this project is ready to code. `Approved by` is written only on the user's explicit confirmation, never in auto mode.

- **Requirements**: rust/docs/SPEC.md
- **Direction**: BRAINSTORMING_LOG_RUSTPORT.md
- **Codebase map**: none — blank scaffold
- **Tech stack** (verified by T001, `rust/docs/spike-T001.md`): Rust stable 1.99.1, MSRV 1.90; `rusqlite` 0.41.1 (bundled SQLite); `clap` 4.6.7; `axum` 0.8.9 on `tokio` 1.54.1; `jiff` =0.2.42 (OS zone and `TZ`, T009: `rust/docs/spike-T009.md`); `rmcp` =3.9.0 (stdio proven, one module); `keyring` 4.2.0 (guard with `Entry::store_status()`); `tauri` 2.12.5 (compiles on Ubuntu 26.04.1 after the owner's apt install: `cargo check` of a scratch crate finished in 41 s, 2026-11-10); no YAML crate for the shipped data (Python parses `catalog/*.yaml`, `data/*.yaml` with its own line parser — hand-port it); React/Vite bundle reused from this repo
- **Main flow**: rust/docs/main-flow.md
- **Approved by**: user 2026-11-10
- **Waived**: [none]

---

## Critical Constraints

- **Do not modify the Python app** (`src/`, `web/`, `tests/`, `pyproject.toml`, `PRD.md`) from Rust tasks. It is the parity reference. Read its fixtures and data files by path only. **Exception (owner, 2026-11-10: "if forced, we can replace the python")**: if keeping both becomes impractical, the Python app may be replaced earlier than P6. That needs a TASK_GUIDE of its own that names what is replaced and why, and the owner's confirmation in that task; it is never a side effect of another task.
- **Store**: the Rust app must open the same store file. It refuses a store whose `PRAGMA user_version` is newer than it knows. Only one implementation writes at a time (the Python lock `core/lock.py` contract). Python migrations are frozen at the P0 number (read from `src/easy_personal/core/migrations/`); new migrations are Rust-only, continuing the numbers.
- **NFR-001 / NFR-008**: no traffic leaves the machine; HTTP binds `127.0.0.1` only; the Host/Origin/token guard stays on every `/api/*` route; the window loads the page with the token.
- **NFR-006**: the `exec` launcher adds < 200 ms and touches only the OS keychain.
- **Secrets** never reach argv, logs, the store or the page. No plaintext fallback when no keychain exists.
- **DDR-0001**: connector writes go only through `claude mcp add-json --scope user`.
- **The guard keys on the raw request path** (`server.rs`, `starts_with("/api/")`). Python guards the decoded path. Today no route matches an encoded form, so `/%61pi/version`, `//api/version` and `/api%2fversion` are 404 (T004 probe). When P1b adds a static fallback or any nested router, add a test that no encoded or dot-segment form reaches an `/api` handler unguarded.
- **The React page is never ported** — only served.
- **Platform order**: Ubuntu → macOS → Windows. No Windows-only work before P5.
- **No crate or SDK choice is final** until the T001 spike verifies it.

---

## Known Risk Areas

| Area | Risk Level | Reason | Files |
|------|-----------|--------|-------|
| Shared store with Python | High | Two writers or a newer `user_version` could corrupt or lock history | `rust/crates/core/src/store.rs` (to be created) |
| JSON parity with Python | High | Float rounding, key order, local-day boundaries, abstain counts | parity tests (to be created) |
| Per-OS scheduling and keychain | Medium | systemd vs launchd vs Task Scheduler; Secret Service vs Keychain vs Credential Manager | P3–P5 |
| Window webview | Medium | WebKitGTK / WKWebView / WebView2 differ per OS | P1b |

---

## Task State

See `PROJECT_KANBAN.md`.

---

## Memory / Insights

| Date | Insight | Source task |
|------|---------|------------|
| 2026-11-10 | The shipped `*.yaml` data files are not real YAML (own line parser; `claude_ai_connector: yes` would parse as a string in a YAML 1.2 crate). Hand-port the parser; do not add a YAML crate for them | T001 |
| 2026-11-10 | `rmcp` ships a new major about every 6 weeks and had 2026 advisories (fixed in 3.9.0; HTTP/OAuth ones do not apply to stdio). Pin `=3.9.0`, keep it behind one module | T001 |
| 2026-11-10 | Python code is Linux-only in practice (systemd timer, Secret Service); macOS/Windows are new work, not a recompile | Brainstorming |
| 2026-11-10 | A Tauri v2 window opens under WSLg (Wayland), and its webview (WebKitGTK 2.52.6) loads `http://127.0.0.1:<port>/#<token>`, reads the fragment and calls the guarded API under the dashboard CSP. P1b must serve the static page from the same origin, open the URL with the fragment, ship no IPC capability, and preflight the display with a timeout (a dead `DISPLAY` hangs GTK init) | T016 |
| 2026-11-10 | The `workspace` / `account` scope is in the Rust core (`scope.rs`; scoped usage, rules and plan equal all 41 scoped goldens and a 1,140-case Python cross-check). HTTP and MCP still answer 501 / tool error until T019. Run the whole workspace under non-default zones: it exposed two T015 tests that do not pin TZ | T017 |
| 2026-11-10 | The scope is wired: four HTTP routes and four MCP tools honour `workspace` / `account` (T019, 501 gone); the core has `session_outliers` / `session_timeline` (T018, 23 goldens). Open for the owner: Python's MCP usage tools ignore the scope, ours honour it. Next: T020 serves sessions and outliers | T018, T019 |
| 2026-11-10 | Six HTTP read routes and six MCP tools are served from Rust with the same JSON as Python (T020: sessions, outliers, `session_outliers`). Left of the Python surface: kit, board, reports, accounts, workspaces, memory graph, connectors, `save_report`. Next: P1b (the window: static page from the same origin, token in the URL fragment) | T020 |
| 2026-11-10 | `serve --static-dir <DIR>` serves the built dashboard from the API's origin (T021, High risk, path-safety table, dotfiles 404); the workspace list and unassigned projects are served (T022). The real dashboard is not built yet (`npm ci` needs the owner's OK) and calls ~40 routes of which 10 are ported. Next: T023, the window (`window` Cargo feature, `app` subcommand, display preflight with a timeout) | T021, T022 |
