# BRAINSTORMING_LOG — Terminal in the app (build after the T130 spike)
**Date**: 2026-11-20 · **Tier**: Standard (the spike answered the technical unknowns; what is left is the build path) · **Board**: `rust/PROJECT_KANBAN.md`

## Owner request (verbatim)
"how about when the application new session, we can show the terminal inside the app, and it will work like the terminal, it mean that, the app will wrap the terminal, so the normal flow will going in the app, and the app don't care too much, that is my idea, let check that" → "go with both" (T130, spike first).

## Owner decisions (2026-11-20, AskUserQuestion, after the spike)
| Question (SPIKE.md § Open questions) | Answer |
|---|---|
| 1 Build? / 2 instead of or beside Chat / 6 `npm ci` for xterm | **Build, beside Chat**: New session gets Terminal or Chat; Chat keeps T126/T127 features; `@xterm/xterm` + `@xterm/addon-fit` approved |
| 3 Lifetime | **End with the tray** (no daemon) |
| 4 Several pages on one PTY | **All can type**; the last resize wins |
| 5 ~1.5 min of owner hooks per turn | **Yes, full parity**: no setting or hook skipped |

## Verified facts (claim gate)
- App sessions: `rust/crates/app/src/app_session.rs` — `MAX_RUNNING = 4` (`:53`), `child_env` (`:263`), `pre_exec` with `PR_SET_PDEATHSIG` from the supervising thread (`:1445`). Routes `rust/crates/app/src/app_session_routes.rs:61-65`, `start` (`:430`).
- Guard: `rust/crates/app/src/server.rs` `guard_layer` (`:139`) checks `Host` (`:104`) and `Origin` (`:110`).
- `axum` 0.8.9 without the `ws` feature (`rust/Cargo.toml:16`); `libc` 0.2.191 is a workspace dependency (`rust/Cargo.toml:27`); `nix` 0.29 is only in `Cargo.lock` (transitive), not a direct dependency.
- `scripts/check_static.py` `ALLOWED_URL_PREFIXES` (`:26`); the spike saw `check_no_remote` fail on the raw xterm files (`sourcemap present`, URL strings).
- Page: `web/src/NewSession.jsx` (kinds `new` / `resume` / `continue`, `:43`), `NewSessionSheet.jsx`, `LiveView.jsx`, `SessionPanel.jsx`; `web/package.json` has only `react`, `react-dom` as runtime dependencies.

## Three paths
| Path | PTY | Transport | Invasiveness | Why it might fail |
|---|---|---|---|---|
| **A. libc PTY + WebSocket (recommended)** | `libc::openpty` + `std::process::Command` with `pre_exec` (`setsid`, `TIOCSCTTY`, `PR_SET_PDEATHSIG`), same pattern as `app_session.rs`; no new PTY crate | axum `ws` feature (adds `tungstenite`, `tokio-tungstenite`, `data-encoding`, `utf-8`); first-frame token; guard on the upgrade | Medium: one new module + one route file; app sessions untouched | `openpty` + controlling tty by hand is easy to get subtly wrong (no SIGWINCH, orphan group); not measured in the spike → Step 0 of the build re-runs spike Q1/Q2/Q6 on it, falls back to Path B if it fails |
| B. `portable-pty` + WebSocket | `portable-pty` 0.9.0 (spike-proven; 5 new crates, second `nix` 0.28, last release 2025-02) | same as A | Medium | Unmaintained-ish crate on the most dangerous write path; two `nix` versions |
| C. libc PTY + SSE out / POST in | as A | out on an `EventSource` (query-token carve-out like `/api/live`); in = queued POSTs, one at a time | Medium, no new crates | Spike: unqueued POSTs reorder keys (186/186 bursts reordered); a queue adds a round trip per key (7.5 ms median, worse on a phone); token in a URL for the stream |

**50% less code**: reuse instead of copy. The terminal session joins the app-session registry and its cap (one `MAX_RUNNING` for both kinds), reuses `child_env`, the FR-109 form check and `stop` (close master → SIGKILL group after 5 s). Live already lists it through `sessions/<pid>.json` (spike Q5), so no new Live list code. The page reuses `NewSessionSheet` with one Terminal/Chat toggle; Terminal sends the same fields minus the prompt.

## Adversarial review (recommended path)
- **Typed bytes = any command.** Strongest write the app has. Mitigation: guard on the upgrade, token only in the first frame (5 s timeout, no data before it), `confirmed: true` on start, input cap (64 KiB/s per session), no start without a click.
- **All pages can type (owner choice)**: two pages interleave keys; the last `resize` shrinks the other page. Accepted by the owner; the panel shows "N pages attached".
- **Ring buffer wrap** tears the replay and drops private modes (bracketed paste): keep the first 4 KiB of output beside the 1 MiB ring; after replay send a resize nudge.
- **Secrets on screen**: raw bytes (tool output, env dumps) in memory only; sixth NFR-002 exception; leak suite plants a secret in the PTY output and proves it absent from store, logs, MCP, CLI, `/api/alerts`, tray.
- **WebKitGTK window**: untested in the spike → the frontend task's verify runs the app window, not only Chrome.
- **Phone keys**: no Esc/Tab/Ctrl on a phone → on-screen key row; UX check at 375 px.
- **Auto mode hides prompts** in a PTY (no FR-111 cards): stated on the Terminal choice ("prompts appear in the terminal").
- **Hooks delay** (~1.5 min per turn on this account): owner accepted; nothing to build.

## Surgical scope
- Should touch: new `rust/crates/app/src/term_session.rs`, `term_routes.rs`; `rust/crates/app/src/server.rs` (route mount only); `rust/Cargo.toml` (axum `ws`); `web/src/NewSessionSheet.jsx`, `NewSession.jsx`, new `web/src/TermPanel.jsx`, `LiveView.jsx` (open the panel); `web/package.json`; `scripts/check_static.py` (allow-list / sourcemap strip); `PRD.md`; leak suite.
- Must not touch: stream-json app sessions' behaviour (`app_session.rs` logic beyond sharing the cap), the answer bridge, any `~/.claude*` file, the Python app, MCP tools (no MCP tool drives a terminal).

## Recommended path
Path A, with Path B as the measured fallback named in the backend guide's Step 0. Two tasks: **T133** backend (PTY owner, routes, WebSocket, ring, leak gate; C3, High) and **T134** page (Terminal choice, xterm panel, key row, static check; C2, Medium; depends on T133). The owner provides the UI design at implementation (memory `owner-provides-ui-design`), so T134 starts by asking for it.

## Edge case checklist (into both guides)
- Start refused at the cap (4 running across both kinds) → 409 with the fixed text.
- Claude exits by itself (`/exit`, crash) → `{"t":"exit","code":N}` to every page; the panel stays readable until closed.
- Tray quits → PDEATHSIG ends `claude` ≤ 6 s; no `sessions/<pid>.json` left (spike Q6).
- Page reloads → re-attach replays the ring + 4 KiB prefix, then a resize nudge.
- Zero-size resize (hidden tab) ignored; sizes clamped 10..500 cols, 5..200 rows.
- Non-UTF-8 output: frames are binary; never decoded on the server.
- First frame missing, wrong or late (5 s) → close with no data (spike guard tests).
- Folder-trust dialog on a new repo appears in the terminal (spike Q2); nothing to answer for the owner.
