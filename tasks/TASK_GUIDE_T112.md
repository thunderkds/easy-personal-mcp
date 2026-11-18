# TASK_GUIDE — T112: Reply to idle sessions 2 — the reply box and *Reply from the app* in the Live view
**Date**: 2026-11-18
**Complexity Level**: C2
**Risk Level**: Medium (page only; sends the owner's text through T111's guarded routes)
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `PRD.md` FR-122, FR-121, FR-114, FR-115, FR-108
2. `tasks/TASK_GUIDE_T111.md` (the routes and the `pending_reply` shape) and T111's README bridge section
3. `web/src/LiveView.jsx` (session rows, `pending`, `PendingAnswer`), `web/src/PendingAnswer.jsx`, `web/src/PromptCard.jsx`, `web/src/api.js` (`answerPrompt`, `releasePrompt`), `web/src/AnswerBridge.jsx` (Settings text), `web/src/LiveAnswer.test.jsx`, `web/src/test/fixtures.js`, `web/src/tokens.css`

## Requirement
Owner 2026-11-18: "if the agent is idle, I can input the request to the live session also" (PRD question 42; FR-122). The page half of T111.

## Dependencies & Reachability
**Depends on**: T111 (routes and `pending_reply`); branch `T112` is cut from `T111`.
**Entry point**: Live view session row (`web/src/LiveView.jsx`)

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | A session with `pending_reply` shows in its row: the `last` text (muted, wrapped, no HTML), a message box, **Send**, **Hand back to terminal**, and "waits Ns more" from `until` (updates each second; formatted like the page's other durations) | FR-122 |
| 2 | Enter sends, Shift+Enter makes a new line, IME composition (keyCode 229) does not send; Send is disabled for an empty box and for more than 8,000 characters (counted as typed), with the count shown near the limit | FR-122 |
| 3 | Send → `POST /api/reply {id, text, confirmed: true}`; on 200 the box shows "sent" and the row returns to normal when `pending_reply` goes away; 409 → "This session is no longer waiting"; other errors show the server's one line and keep the text | FR-122 |
| 4 | **Hand back to terminal** → `POST /api/reply/release {id}` | FR-122 |
| 5 | While the row is open or the box has focus, the page sends `POST /api/reply/watch {session_id}` every 20 s (fake timers test); closing the row or blur + closed row stops it; no call for a session without `pending_reply` | FR-121, FR-122 |
| 6 | A bridge-enabled terminal session that is not your turn shows the toggle **Reply from the app** (`aria-pressed`) → `POST /api/reply/mark {session_id, on}`; a marked idle session without a hold says "marked: it waits for you at its next turn end" | FR-122 |
| 7 | The token is never in a URL; nothing of the text in `localStorage`; text typed during a failed send is kept | FR-115, NFR-008 |
| 8 | Settings bridge block (`AnswerBridge.jsx`) names the third entry and shows `on (no reply)` with **Turn on** to add it | FR-113 |
| 9 | `ux-evaluate` at 375 and 1280 (row with a hold, marked row, sent, 409): no horizontal page scroll; touch targets ≥ 44 px at 375; tokens only | FR-122 |
| 10 | `cd web && npx vitest run && npm run build`; `python3 scripts/check_static.py` ok | all |

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/LiveView.jsx` | reply box in the row, mark toggle, watch timer |
| `web/src/ReplyBox.jsx` (new) + test | the box |
| `web/src/api.js` | `sendReply`, `releaseReply`, `watchReply`, `markReply` |
| `web/src/AnswerBridge.jsx` | third entry, `on (no reply)` |
| `web/src/app.css` or `appsession.css` | layout with existing tokens |
| `web/src/test/fixtures.js` | `pending_reply` fixtures |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/**` | T111 |
| `src/easy_personal/**` | Python app |

## Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes (web has no lint script)
- [ ] Tests pass
- [ ] UI Evidence rows (ux-evaluate 375/1280)
- [ ] Smoke test confirmed (owner: reply from the app reaches a real terminal session)
- [ ] TASK_GUIDE marked complete
