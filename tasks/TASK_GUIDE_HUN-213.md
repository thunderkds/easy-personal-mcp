**Date**: 2026-11-04
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Dashboard, Live tail, Attention state; Critical Constraints: metadata only)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-045, FR-050, FR-072, NFR-011, NFR-015, then `web/src/LiveView.jsx`, `web/src/LiveView.test.jsx`, the `.live-*` rules in `web/src/app.css`, and `web/src/tokens.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-04: "in the live here, can we show just a shorten with the visualize also, I mean the bar are running with token, cost, for the tools call, we may boot to detail for the live session of tools will reduce all the session are showing, it will make the dashboard lag also".

Owner choices (AskUserQuestion, 2026-11-04): layout **rows + side panel**; bar scale **largest session = full bar**.

Supervisor investigation (read-only, 2026-11-04) of why the page lags:

* Every SSE `live` event dispatches once and re-renders the whole `LiveView`: every session card and every feed, with no `React.memo`.
* Every session card draws up to `FEED_MAX` (50) feed rows: 8 active sessions ≈ 400 list items, each feed its own `aria-live` region.

**Restated intent**:

> Replace the session cards with one *session row* per active session, grouped by account: project, short session id, attention badges (as today), active/idle + time ago, and three bars, for tokens (in + out), API-equivalent cost and tool calls, each with its number. A full bar = the largest value of that measure among listed sessions; an unpriced cost shows "unknown price" and no $0 bar. Order: sessions needing you first, then newest. A row is a button: click / Enter / Space opens the *session panel* (that session's `Totals` + `Feed`); one open at a time; a close button; when the open session leaves the list the panel says "This session is no longer active." Panel right of the rows; below them under 768 px. Keep the reducer's per-session feed (bounded `FEED_MAX`) so opening a session shows its recent calls. Apply SSE events in batches at most every 250 ms; memoize rows so an untouched row does not re-render; draw at most one feed.

**Out of scope**:

* Any server change (`core/live.py`, `surfaces/api.py`, SSE payload) — the backend fix for missing sessions is a separate task (FR-071)
* Sparklines or history charts; sorting controls; remembering the open session across reloads
* Other views

**Requirement Refs**: FR-072, NFR-015 (FR-045, FR-050 kept)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner choices recorded above)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None (works with today's `/api/live`)

**Entry point**: `LiveView` default export in `web/src/LiveView.jsx`, reached from the `Live` tab in `web/src/App.jsx`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | With 3 sessions in a snapshot, the page shows 3 session rows, each with project, 8-char id, active/idle text and three bars with their numbers; no feed is drawn | FR-072 |
| 2 | Bar widths: the session with the most tokens has a 100% tokens bar; one with half has 50%; same rule for cost and tool calls; each bar has an accessible name with the number (e.g. "tokens 1.2M") | FR-072 |
| 3 | A session with an unknown price shows "unknown price" and no cost bar width; a session with $0 priced cost shows a 0 bar | FR-072 |
| 4 | Click, Enter and Space on a row open its panel with `Totals` and the feed (newest first, at most `FEED_MAX`); the row has `aria-expanded`/`aria-controls`; opening another row closes the first; the close button closes it | FR-072 |
| 5 | Live events for the open session appear in its panel feed; the open session leaving the list shows "This session is no longer active." | FR-072 |
| 6 | Attention badges, the header "N sessions need you" count, needing-you-first order and account grouping still work (existing FR-050 tests adapted, not deleted) | FR-050, FR-072 |
| 7 | 100 `live` events sent within 250 ms (fake timers) are applied in at most 2 batches; a row whose session got no event in a batch does not re-render (React `Profiler` count or a render spy) | NFR-015 |
| 8 | At most one `.live-feed` list is in the DOM at any time | NFR-015 |
| 9 | Real run: `easy-personal dashboard` with 3+ running sessions; screenshot of the rows and an open panel at 1280×720 and 375×812 | FR-072 |

---

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Live, 3+ sessions, panel closed and open, 1280×720 | headless screenshot BEFORE/AFTER | rows with bars; panel right of rows |
| Live, panel open, 375×812 | headless screenshot | panel below rows; no page-wide horizontal scroll |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours; bars use existing accent/series tokens |
| Accessibility | vitest + keyboard | rows are buttons with `aria-expanded`; bars have accessible names; one `aria-live` feed only |

### 3. Responsiveness

| Width | Expected |
| -- | -- |
| ≥ 768 px | rows + panel side by side |
| < 768 px, 375 px | panel below rows; long project names truncate with full name on hover |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run src/LiveView.test.jsx && npx vitest run && npm run build
```

## Approach

**Vital slice**: `SessionRow` (memoized) + `Bar` + `SessionPanel` inside `LiveView.jsx`; batched dispatch in `useLive` (collect parsed events, flush with a `setTimeout`/`requestAnimationFrame` ≤ 250 ms as one `batch` action); CSS in `app.css`; rebuilt static bundle if the repo commits it (follow how HUN-207/earlier UI tasks shipped `src/easy_personal/dashboard/static/`).
**Cut list**: no new component files unless `LiveView.jsx` passes \~400 lines; no chart library; no stored selection.

Keep reducer state shape stable so memoized rows get the same object when their session did not change. Keep the footnote on API-equivalent cost and the metadata-only line.

---

## Edge Case Checklist

- [ ] 0 sessions → today's empty text; no panel
- [ ] All sessions at 0 tokens/cost/calls → bars at 0, no divide-by-zero
- [ ] Snapshot after a server restart (ids restart) → panel feed cleared, as the reducer does today
- [ ] Stream reconnect with the panel open → panel stays on the same session
- [ ] Planted secret/text fixtures never appear on the page (only tool names, counts, times — FR-045)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/LiveView.jsx` | rows, bars, panel, batching, memo |
| `web/src/LiveView.test.jsx` | AC 1–8 (adapt card-based helpers) |
| `web/src/app.css` | `.live-*` rules for rows, bars, panel |
| `src/easy_personal/dashboard/static/` | rebuilt bundle, only if the repo commits it |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/`, `src/easy_personal/surfaces/` | Server unchanged (FR-071 is another task) |
| Other views in `web/src/` | Out of scope |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Adapt `LiveView.test.jsx` (FakeEventSource): rows and bars from a snapshot; widths by measure; unknown price; open/close/switch by click and keyboard; live events into the open panel; session leaving; attention tests kept; fake-timer burst for batching; render spy for memo; DOM count of `.live-feed`; screenshots at two widths from a real run.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] All three UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] Real run done and screenshots attached
- [ ] Supervisor notified: task ready for Stage 4 review