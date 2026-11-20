# 0009. Terminal in the app: the real `claude` TUI in a PTY, beside Chat sessions

**Status**: Accepted
**Date**: 2026-11-20
**Deciders**: thunderkds (owner), Supervisor
**Related**: FR-109..FR-116, FR-136..FR-138 · NFR-002, NFR-008 · DDR-0006 (amends its "Not done" item), DDR-0007 · PRD question 42 (note), question 47 · T130 spike `tasks/evidence-T130/SPIKE.md` · `BRAINSTORMING_LOG_TERMINAL.md`

---

## Context

DDR-0006 put "an embedded PTY" on the cut list: a terminal emulator inside the app was too much for what stream-json app sessions already gave. On 2026-11-20 the owner asked for exactly that: "when the application new session, we can show the terminal inside the app, and it will work like the terminal ... the app will wrap the terminal". The T130 spike (Claude Code 2.1.329) answered all 7 technical questions GO: the interactive `claude` runs in a PTY, keys and its own dialogs work, xterm.js draws it (colours, cursor, resize), the session is saved as a normal `cli` session that Live lists and `--resume` reopens, the answer-bridge hooks fire, closing the PTY ends it cleanly, and the page cost is about 124 KB gzip.

The owner answered the spike's open questions the same day: build it **beside** Chat sessions, sessions **end with the tray**, **all attached pages can type**, and the owner's hooks run with **full parity** (about 1.5 min per turn on this account is accepted).

Gate: (1) hard to reverse: no, it is one new session kind; (2) surprising without context: yes, it reverses a DDR-0006 cut item; (3) genuine trade-off: yes (three paths weighed). 2 of 3: DDR.

---

## Decision

1. **A second session kind, Terminal.** New session offers *Terminal* or *Chat*. Terminal starts the owner's own `claude` in a PTY in a stored repo (or a worktree), with the FR-109 form fields (repo, account, model, mode) and `child_env`; no prompt field, no free arguments. Chat (stream-json, DDR-0006) is unchanged.
2. **Typed bytes are allowed on one route only.** A guarded WebSocket `GET /api/term/{id}`: the guard checks Host and Origin on the upgrade, the first frame carries the token and the size (never the URL, 5 s timeout, no data before it). Binary frames in are stdin bytes; this amends FR-116 ("no free text") for that route alone.
3. **Lifetime: the tray's.** The PTY child gets `PR_SET_PDEATHSIG` like app sessions; quitting the tray ends it. No daemon. One cap of 4 running sessions covers both kinds (question 41).
4. **All attached pages can type**; the last resize wins; the panel shows how many pages are attached.
5. **Screen bytes shown, never stored.** Sixth NFR-002 exception: a 1 MiB in-memory ring (plus the first 4 KiB of output) per session, replayed on attach, dropped when the session ends and at exit. Never stored, logged, or sent to MCP, the CLI, `/api/alerts`, the tray or a popup.
6. **Full parity**: no setting, hook, statusline or MCP server is skipped or narrowed in a terminal session.
7. **Still not done**: typing into a terminal the app did not start (tmux, other terminals: question 42 stands), a daemon that outlives the tray, an MCP tool that drives a terminal, any start without an owner click.

---

## Alternatives Considered

| Alternative | Why not chosen |
|---|---|
| Terminal instead of Chat | Loses structured cards, model/mode/effort steering, interrupt and queue, Allow/Deny cards (T126/T127); owner chose beside |
| `portable-pty` crate | Spike-proven, but 5 new crates, a second `nix`, last release 2025-02; kept as the build's fallback if `libc::openpty` fails Step 0 |
| SSE out + POST in | No new crates, but keys reorder unless queued (spike: 186/186 bursts), a round trip per key, and a URL token for the stream |
| Sessions outliving the tray (daemon) | A separate process and lifecycle; owner chose end with the tray |
| One controlling page, others read-only | Owner chose all can type |

---

## Consequences

- PRD: FR-116 amended (the terminal route carries typed bytes); NFR-002 gains its sixth exception; the "Write sessions" cut list drops "embedding the interactive terminal UI (a PTY)"; FR-136..FR-138 added; question 47 records the owner's answers; question 42's "terminal input stays out" now reads "input to terminals the app did not start stays out".
- The guard now protects a route that types into a shell-capable process. Kept against it: Host/Origin on the upgrade, first-frame token, `confirmed` on start, an input cap per session, no start without a click. The leak suite covers the ring.
- New dependencies: axum's `ws` feature (`tungstenite` family) in Rust; `@xterm/xterm` and `@xterm/addon-fit` in the page (owner-approved `npm ci`). `scripts/check_static.py` must accept the built bundle (strip source maps; allow-list any URL string the bundle keeps, each one never fetched).
- Auto-mode sessions show their dialogs only inside the terminal, never as FR-111 cards; the Terminal choice says so.
- The Python app is not changed.
