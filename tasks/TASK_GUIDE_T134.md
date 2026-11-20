# TASK_GUIDE — T134: Terminal 2 — Terminal choice and the xterm panel (page)
**Date**: 2026-11-20
**Complexity Level**: C2
**Risk Level**: Medium (new runtime dependency in the page; a panel that sends typed bytes)
**Priority**: P2
**Assigned agents**: frontend-developer
**Agent guides**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `PRD.md` FR-109, **FR-136..FR-138**, NFR-008, question 47
2. `docs/ddr/0009-terminal-in-the-app.md`, `BRAINSTORMING_LOG_TERMINAL.md`, `tasks/TASK_GUIDE_T133.md` (the route contract)
3. `tasks/evidence-T130/SPIKE.md` (branch `T130` until merged) — Q3 (render, resize), Q7 (page cost, `check_static` failures)
4. `web/src/NewSessionSheet.jsx`, `web/src/NewSession.jsx` (kinds `:43`), `web/src/LiveView.jsx`, `web/src/SessionPanel.jsx`, `web/src/appsession.css`, `scripts/check_static.py` (`ALLOWED_URL_PREFIXES` `:26`, `check_no_remote`)
- **Before any layout code: ask the owner for the design** (memory `owner-provides-ui-design`: the owner gives the tray/dashboard design at implementation; no up-front mockups from the agent).

## Requirement
Owner 2026-11-20: "when the application new session, we can show the terminal inside the app, and it will work like the terminal"; build it beside Chat; all attached pages can type; `npm ci` for `@xterm/xterm` + `@xterm/addon-fit` approved.

## Dependencies & Reachability
**Depends on**: T133 (routes and WebSocket)
**Entry point**: Live → New session → **Terminal** / **Chat** choice → Start → the terminal panel opens in Live

## Acceptance Criteria
| # | Criterion | Evidence |
|---|---|---|
| 1 | New session has a Terminal / Chat choice (Chat default, today's form unchanged); Terminal hides the prompt field, keeps repo, account, model, mode, shows "Prompts appear in the terminal, not as cards", and starts through `POST /api/term-sessions` with `confirmed: true` | vitest |
| 2 | The panel opens a WebSocket to `/api/term/{id}`, sends the auth frame first (token never in the URL), draws with `@xterm/xterm` + `@xterm/addon-fit`, sends `resize` on fit (debounced), sends typed bytes as binary | vitest with a fake WebSocket |
| 3 | Key row: Esc, Tab, Ctrl-C, ↑ ↓ ← →, Enter send the right bytes; usable at 375 px | vitest + `ux-evaluate` 375/1280 |
| 4 | `exit` frame → panel shows "Ended (code N)", input off, the screen stays readable; `pages` frame → "N pages attached" when N > 1; Stop button calls `POST /api/term-sessions/{id}/stop` with `confirmed` | vitest |
| 5 | Re-opening the session (reload, other page) re-attaches and shows the replay | vitest + `/verify` |
| 6 | Terminal sessions appear in Live like other sessions (the `sessions/<pid>.json` path) and open the terminal panel, not the Chat panel | vitest |
| 7 | `web/package.json` adds exactly `@xterm/xterm` and `@xterm/addon-fit` (pinned, `npm ci` lock updated); `scripts/check_static.py` passes on the built bundle (source maps stripped by the build; any URL string kept is allow-listed with a comment saying it is never fetched); CSP unchanged | `npm run build` + `python3 scripts/check_static.py` |
| 8 | The app window (WebKitGTK, `app`), not only Chrome, draws the panel and takes keys | `/verify` screenshots |
| 9 | Web suite and build pass | commands below |

## Approach
Reuse `NewSessionSheet` with one toggle; new `web/src/TermPanel.jsx` (+ test) holds xterm, fit, the WebSocket and the key row; `LiveView.jsx` picks `TermPanel` for a terminal session. Existing styles and the owner's design; no new design system.

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/NewSessionSheet.jsx`, `web/src/NewSession.jsx` + tests | Terminal / Chat choice |
| `web/src/TermPanel.jsx` (new) + test, `web/src/appsession.css` | panel, key row |
| `web/src/LiveView.jsx` + test | open the right panel |
| `web/package.json`, `web/package-lock.json` | two dependencies |
| `scripts/check_static.py`, `web/vite.config.js` (if needed for source maps) | static check |
| `README.md` sessions part | one paragraph |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/**` | T133 |
| Chat panel behaviour (`SessionPanel.jsx` cards, steering) | Chat stays as it is |
| CSP header | unchanged (DDR-0009) |

## Test plan
vitest for ACs 1–6; `ux-evaluate` 375/1280; `/verify`: real `serve` with the T133 stub as claude, start a Terminal session from the page, type, resize the window, reload, stop.

## Verification Command
```bash
cd web && npm ci && npm test -- --run && npm run build && cd .. && python3 scripts/check_static.py   # web/ has no lint script (found at T131 Stage 4)
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes (N/A for web/: no lint script)
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
