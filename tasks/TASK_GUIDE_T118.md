# TASK_GUIDE — T118: Live like Claude 4 — one panel for every session
**Date**: 2026-11-18
**Complexity Level**: C3
**Risk Level**: Medium (page only; uses the guarded routes)
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `PRD.md` question 45, US-043, FR-128, FR-123, FR-124, FR-125, FR-110, FR-111, FR-114, FR-122, NFR-002 (fifth exception), NFR-008
2. `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVENATIVE.md` (Edge Case Checklist: all of it applies)
3. `web/src/LiveView.jsx`, `AppSessions.jsx` (SessionPanel, MessageBox, cards), `PendingAnswer.jsx`, `PromptCard.jsx`, `ReplyBox.jsx`, `api.js`, `App.jsx` (the agent-run account dialog of FR-104 that covered a card in the owner's test), `web/src/test/fixtures.js`, `tokens.css`
4. The T115/T116 route shapes in their TASK_GUIDEs (mock them in tests)
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-18: "should check the live, cause i need the work like claude native, where user can input, choose the option, trace the logs, and manage the sections, [admin], supervisor". Owner answers the same day: full transcript (shown, never stored); queue until the turn end for a busy terminal session; one place for all sessions; stop a terminal session; past sessions + resume; filters and names. This task is the page: one panel, conversation, composer, Stop.

## Dependencies & Reachability
**Depends on**: T115, T116 (route shapes; the page can be built against mocks in parallel, merged after them)
**Entry point**: Live view (`web/src/LiveView.jsx`)

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Live lists app sessions with terminal ones; the App sessions tab keeps only *New session*, which opens the new session's panel in Live (`App.test.jsx` nav updated) | FR-128 |
| 2 | Selecting a session opens one panel: header (name, project, branch, account, kind, busy/idle, time), conversation from FR-123 polled every 2 s while open (stops when closed), *Load earlier*; tool calls folded to the label, a click opens input and result (pre-wrap, scroll inside, `cut` marked) | FR-128, FR-123 |
| 3 | Cards (question, permission, plan) pinned above the composer, for app and terminal sessions; no modal dialog of the app (FR-104 one-time prompt included) covers the Live view while a card is pending: the one-time prompt waits (test) | FR-128 |
| 4 | Composer by state: app → send (FR-110 rules); terminal holding a reply → reply (FR-122, existing ReplyBox behaviour kept); terminal busy + bridge → queue (shows the queued text, Edit, Cancel, "sent at the turn end"); bridge off → a line linking Settings; Enter / Shift+Enter / IME as FR-122 | FR-128, FR-125 |
| 5 | Controls: app → Interrupt / Stop / Close; terminal → **Stop** with one confirm naming project + short id (409 line shown); Resume / Fork as today | FR-128, FR-124 |
| 6 | Text only as React text, never HTML; nothing of the transcript in `localStorage`; token never in a URL | FR-123, NFR-008 |
| 7 | Keyboard: every control reachable; Esc closes the panel; focus returns to the row | FR-128 |
| 8 | `ux-evaluate` at 375 and 1280 (terminal panel with tool calls open, app panel with a card, queue shown): no page-wide horizontal scroll; touch targets ≥ 44 px at 375; tokens only | FR-128 |
| 9 | `cd web && npx vitest run && npm run build`; `python3 scripts/check_static.py` ok | all |

## Approach
A `SessionPanel` component shared by both kinds (move from `AppSessions.jsx`), a `Transcript` component, a `Composer` that picks its action from the snapshot; LiveView renders the panel beside (1280) or instead of (375) the list.

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/LiveView.jsx` | list + panel |
| `web/src/SessionPanel.jsx` | new (from AppSessions) |
| `web/src/Transcript.jsx` | new |
| `web/src/Composer.jsx` | new |
| `web/src/AppSessions.jsx` | New session only |
| `web/src/App.jsx` | one-time prompt waits for cards |
| `web/src/api.js` | transcript, stop, queue calls |
| `web/src/*.test.jsx` | tests |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/**` | T115-T117 own the server |
| `src/easy_personal/**` | Python app |

## Test plan
vitest per AC with mocked routes and fake timers; ux-evaluate screenshots.

## Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: open a terminal session, read its tool output, queue a message)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
