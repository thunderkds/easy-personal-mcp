# TASK_GUIDE — T095: Waiting sessions 2 — Live view shows *your turn*, the question text, and the session the app asks for
**Date**: 2026-11-16
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Attention state, Your turn, Question text, Waiting session) and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/frontend.md` and `skills/ux-evaluate/SKILL.md`
5. Read `BRAINSTORMING_LOG_WAITING.md` (Option A, Edge Case Checklist), `docs/ddr/0005-live-question-text-show-never-store.md`, the `PRD.md` rows FR-050, FR-072, FR-105, FR-106, FR-108
6. Read `web/src/LiveView.jsx` (`statesOf`, `pillText`, the session row and its side panel, the stream hook at ~l.298-350, the footnote at ~l.414), `web/src/LiveView.test.jsx`, `web/src/App.jsx` (the one stream for the page, `onOpenLive`), `web/src/StatusChips.jsx`, `web/src/tokens.css`, `docs/design/graphite-mint/README.md`, and `tasks/TASK_GUIDE_T077.md` (how the last view task built and copied the page)
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders).

---

## Requirement

> **OPEN — do not start this task.** How a click tells the page which session to show is not decided: the Debate of 2026-11-16 ended with three different proposals (`tasks/TASK_REVIEW_T095.md` `Paused:`). The text below still describes the Supervisor's draft, a `focus` message on `GET /api/live`; the Supervisor rewrites this guide after the owner's answer.

**Original request** (owner, 2026-11-16): "For the dashboard let show the question, or stop point to the live section" · "go [admin], with question text".

**Restated intent** (FR-108):
> 1. **Badge.** The attention state `your_turn` shows as a badge with the words `your turn`. It counts in the Live header count, gives the row the same "needs you" sort and border rule as the other states, and such a session is never idle. State order: `failing`, `waiting`, `your_turn`, `needs_attention`. An unknown state name is still left out.
> 2. **Question text.** A `waiting` state that has the key `questions` (a list of strings, from T094) shows each question as plain text under the badge, in the session row and in its side panel. It is React text, never `dangerouslySetInnerHTML`. Long text wraps and never widens the page. A `questions` value that is not a list of strings shows nothing and throws nothing. Without the key the badge is as today.
> 3. **Focus.** The page's one stream accepts a new message `event: focus` whose data is `{"session_id": "<id>"}` or `{"session_id": null}` (sent by T096). On it, the page opens the Live view. With an id that is in the list, that session's row is selected: scrolled into view, its side panel open, and marked for assistive technology. With `null`, or an id that is not in the list, nothing is selected and no error shows. A second `focus` for another session moves the selection.
> 4. **Footnote.** The Live view footnote names the one text it shows: "Only tool names, skill, sub-agent, MCP and command names, counts and times are shown, and the question a waiting session asks; never other message or tool content."
> 5. The page works out no state itself (FR-050): it renders what the stream sends.
> 6. The built page is copied where `scripts/check_static.py` expects it, as T077 did.

**Out of scope**: answering a question from the page (no text input, no button that writes to a session); `header`, option labels, the plan text; the server side of `focus` (T096); the tray; Python code; a new design artboard.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16, `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `PRD.md` US-040, FR-108, FR-050, FR-072, FR-106; `docs/ddr/0005-live-question-text-show-never-store.md`

---

## Dependencies & Reachability
**Depends on**: T094
**Entry point**: `LiveView` (`web/src/LiveView.jsx`), nav item `Live`

## Dependency rule
No new npm package. No change to `web/package-lock.json`.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Test: a session with `your_turn` renders the badge `your turn`, is counted in the header count, is sorted with the sessions that need the owner, and is not idle past the active window | intent 1 |
| 2 | Test: state order `failing`, `waiting`, `your_turn`, `needs_attention`; an unknown state renders nothing | intent 1 |
| 3 | Test: `questions: ["A?", "B?"]` renders both under the `waiting` badge in the row and in the side panel; no `questions` key renders the badge as before | intent 2 |
| 4 | Test: a question `<img src=x onerror=alert(1)>` is shown as that text (no `img` element in the DOM); `questions` as a string, a number, `null`, a list with a non-string item render no question and throw nothing | intent 2 |
| 5 | Test: a `focus` message with a listed id opens the Live view from another view and selects that row (`aria-current` or `aria-selected`, panel open); with `null` it only opens the Live view; with an unknown id nothing is selected and no error text shows; a second `focus` moves the selection | intent 3 |
| 6 | Test: the footnote text of intent 4 | intent 4 |
| 7 | `grep -n "dangerouslySetInnerHTML\|innerHTML" web/src` prints nothing; no colour literal outside `web/src/tokens.css` | intent 2 |
| 8 | `cd web && npm test` passes; `npm run build` passes; the static check passes (`python3 scripts/check_static.py`, or the command T077 used) | intents 5, 6 |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd web && npm test 2>&1 | tail -5 && npm run build 2>&1 | tail -3 && cd .. && python3 scripts/check_static.py; echo exit=$?
```

### Evidence (filled by reviewer at Stage 4/5)
> Moved to `tasks/TASK_REVIEW_T095.md`.

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T095.md`. BEFORE (before your first implementation commit): a screenshot of the Live view fed a stream snapshot that holds a `your_turn` session and a `waiting` session with `questions` (fixture server, as `tasks/evidence-T068/fixture_server.py` does): no `your turn` badge, no question text. AFTER: the same feed shows both.

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen / Component | Verification method | Expected result |
|-------------------|---------------------|-----------------|
| Live view, session rows and side panel | screenshot at 1280 px, LLM vision vs `docs/design/graphite-mint/` | the new badge and the question lines use the existing badge and text styles; nothing else moved |

### 2. Design-System Compliance

| Criterion | Verification method | Expected result |
|-----------|---------------------|-----------------|
| Colours match design tokens | grep for raw hex / `rgb(` outside `web/src/tokens.css` | none |
| Typography matches | computed style of the question line | the page's body text token, no new font size literal |
| Spacing / layout | visual at both widths | the question sits under its badge with the row's existing spacing scale |

### 3. Layout / Responsiveness

| Viewport | Verification method | Expected result |
|----------|---------------------|-----------------|
| Mobile (375 px) | headless Chrome screenshot | a 300-character question with no space wraps; `document.documentElement.scrollWidth <= innerWidth` |
| Desktop (1280 px) | headless Chrome screenshot | row and panel show the questions; no overflow |

### 4. UX Acceptance Criteria

| User story | User goal | Flow / state | Pass condition (observable) |
|------------|-----------|--------------|-----------------------------|
| US-040 | See which session is my turn | a `your_turn` session in the feed | the badge `your turn` shows on its row at 375 px and 1280 px, and the header count includes it |
| US-040 | Read what a waiting session asks | a `waiting` session with 2 questions | both questions show as text under the badge at both widths |
| US-040 | Land on the session I clicked | a `focus` message for a listed session while another view is open | the Live view opens and that row is marked selected, its panel open |
| US-040 | Not be confused by a session that is gone | a `focus` for an id not in the list | the Live view opens, no row is selected, no error text |
| US-040 | Empty state | no session | the existing "No active sessions" text, unchanged |

---

## Approach

**Pattern reference**: `web/src/LiveView.jsx` `pillText` / the badge list at ~l.263, and `StatusChips` `onOpenLive` in `web/src/App.jsx` — how a view is opened from outside it.

**Vital slice**: the badge, the question lines and the focus handling. No new component file unless a test needs it.

Test first (`tdd`). The stream is one `EventSource` for the whole page (T068): add the `focus` listener where the other listeners are, and lift "which view is open" the way `onOpenLive` already does.

## Edge Case Checklist
- Long question text wraps at 375 px and never widens the page; it is React text, never HTML
- `questions` with 4 items of 300 characters: the row stays usable (the row may show the first question and the panel all of them, if you say so in your report and the tests match)
- A `focus` that arrives before the first `snapshot`: the selection is applied when the session appears in the next snapshot, or dropped; say which and test it
- A selected session that leaves the list: the selection clears, no error
- The memoized row (`SAME_KEYS`, the `attention` JSON compare) must redraw when only `questions` changed
- Keyboard: the selected row is reachable and announced; do not steal focus on every snapshot
- A click for a session that left the list: the Live view opens, nothing is selected, no error

## Files to Change (Predicted)
`web/src/LiveView.jsx`, `web/src/LiveView.test.jsx`, `web/src/App.jsx`, `web/src/App.test.jsx`, `web/src/app.css`, the built files under `src/easy_personal/dashboard/static/` (build output only), `tasks/TASK_REVIEW_T095.md`, `tasks/evidence-T095/**` (screenshots)

## Files Must NOT Touch
`src/easy_personal/**` except the built static files, `tests/**` except a test that pins the footnote or the static build, `rust/**`, `web/package.json`, `web/package-lock.json`, `web/src/api.js`, `PRD.md`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `memory/**`, `.claude/**`, `docs/**`

## Test Plan
AC 1-6: Vitest output. AC 7: the grep output. AC 8: the three commands' last lines. UI rows: screenshots at 375 px and 1280 px with the verdict table of `ux-evaluate` in your report.

## Completion Checklist
- [ ] Every AC and every UI row with pasted output or a screenshot path in `tasks/TASK_REVIEW_T095.md`
- [ ] The `ux-evaluate` verdict table in the final report
- [ ] Commits on branch `T095` only; final report with `Status:`

## Questions Log
