# TASK_GUIDE — T135: Terminal 3 — roomier terminal panel and Pop out to its own window
**Date**: 2026-11-23
**Complexity Level**: C2
**Risk Level**: Medium (page layout of the terminal panel; the app window may now open a second window of its own origin)
**Priority**: P2
**Assigned agents**: Supervisor (writes the code), one fresh-context reviewer
**Agent guides**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

## Mandatory Startup (Do Not Skip)
1. `PRD.md` FR-136..FR-138, NFR-008; `docs/ddr/0009-terminal-in-the-app.md`
2. `tasks/TASK_GUIDE_T134.md` (the panel), `web/src/TermPanel.jsx`, `web/src/LiveView.jsx`, `web/src/appsession.css`
3. `rust/crates/app/src/window.rs` (`open_window`, `navigation_allowed`, `on_new_window`)

## Requirement
Owner 2026-11-23: "the terminal has run, but the UI is not good, it looks like the app push the terminal too small, from the buttons to the terminal screen, let check and update it with best UX, but can we support the option to move the terminal outside of app". Owner picked (AskUserQuestion): **Own window** — a Pop out button opens the terminal alone in a separate window, in a browser and in the desktop app.

Found (Supervisor, 2026-11-23): the terminal panel sits in the right 3/5 of Live beside the session list; above the screen are the heading, a three-row facts list and a status line; the screen is a fixed `60vh`; below it a row of eight 44 px key buttons, then Stop. On a 1280 px window the screen is about 80 columns and is pushed below the fold.

## Dependencies & Reachability
**Depends on**: T133, T134 (Done)
**Entry point**: Live → a terminal session row → the terminal panel; its **Pop out** button → a window with only that terminal (`/?term=<id>#<token>`)

## Acceptance Criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | With a terminal session open, the panel takes the whole width of Live (the session list is hidden, **Back to sessions** shows at every width); the Chat panel layout is unchanged | vitest + `ux-evaluate` 1280 |
| 2 | One compact head row: Back, the title, the status, **Pop out**, **Stop**; the account and folder are one muted line (the folder in full on hover via `title`); no three-row facts list | vitest |
| 3 | The screen fills the window height left under the head (`100dvh` based, at least 14rem); the key row is one compact row under it at ≥ 40rem wide and stays 44 px touch targets under 40rem | `ux-evaluate` 375/1280 |
| 4 | **Pop out** opens `/?term=<id>#<token>` with `window.open` (no opener) and closes the in-app panel; when the browser blocks it, an alert says so and the panel stays | vitest |
| 5 | `/?term=<id>` alone shows only that terminal, filling the window, with no Back and no Pop out, its title "<project> · terminal"; an unknown or ended-and-gone id shows a short message | vitest |
| 6 | With two pages attached, focusing a page's screen sends its size (the last resize wins, question 47), so the page you type in fits | vitest |
| 7 | The desktop app window opens the pop-out as a second app window with the same navigation guard, link guard, no devtools; every other new window stays denied and logged by host only; the token is never logged | Rust unit tests on the pure URL check + `/verify` |
| 8 | Web suite, build + `check_static.py`, `cargo test` and `cargo clippy --features window` pass | commands below |
| 9 | **Expand** (owner 2026-11-23, after trying it: "it looks like we open another terminal application"): a toggle in the head makes the panel cover the whole app window and a second press restores the layout; the keys stay with the terminal; not shown in the popped window | vitest + screenshot |
| 10 | The popped window reads as part of the app: a bar with "Easy Personal · Terminal", "Closing this window does not stop the session." and **Dock back** (Live opens the terminal in its panel and answers `docked`, then the window closes; if it stays, the bar says it can be closed; with no Live page answering in 500 ms — the app window on another view, or closed — the window stays and says "Open Live in the app window, then press Dock back again.") | vitest |
| 11 | A Live row whose terminal is open in its own window shows "in its own window" until that window closes (same-origin `BroadcastChannel`: ids and five message kinds only (`open`, `closed`, `who`, `dock`, `docked`), never screen bytes or the token) | vitest + two-tab browser check |

## Approach
Page: `TermPanel` gets a compact head (`popped` hides Back and Pop out), a full-width mode (`.live-split[data-term]`), a viewport-height screen and a compact key row. `main.jsx` renders a small `TermWindow` for `?term=<id>`: it reads `GET /api/term-sessions`, finds the id, renders `TermPanel popped`. Rust: a pure `is_term_popout(url, port)` (origin as `navigation_allowed`, then `/?term=<safe id>` and an optional `#…`); `on_new_window` builds a `term-<n>` window through one shared guarded builder on the main thread, then denies the webview's own window.

## Owner direction 2026-11-23 (after installing the build)
Owner: the pop-out "looks like we open another terminal application already, does it make the users is expecting when working in the app". Supervisor's recommendation, owner "approve": the in-app terminal is the main way; **Expand** gives it the whole app window; Pop out stays as a secondary option with app cues (bar, Dock back, row mark, closing note) — kept rather than removed (memory: parity, not narrowing).

## Cut list (not in this task)
- The in-app row does not show "open in its own window".
- No attach from a system terminal app (owner chose Own window).
- No remembered window size or position.

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/TermPanel.jsx` + test | compact head, Pop out, `popped`, resize on focus |
| `web/src/TermWindow.jsx` (new) + test, `web/src/main.jsx` | the pop-out page |
| `web/src/LiveView.jsx` + `LiveTerm.test.jsx` | full-width mode, close after Pop out |
| `web/src/appsession.css` | layout |
| `web/src/test/fakeXterm.js` | focus event stand-in |
| `rust/crates/app/src/window.rs` | the pop-out window |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/crates/app/src/term_*.rs`, server routes, CSP | the contract of T133 stays |
| Chat panel (`SessionPanel.jsx`) | unchanged |

## Verification Command
```bash
cd web && npm test -- --run && npm run build && cd .. && python3 scripts/check_static.py
cd rust && cargo test -p easy-personal-app && cargo clippy -p easy-personal-app --features window -- -D warnings
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes (N/A for web/; clippy for Rust)
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
