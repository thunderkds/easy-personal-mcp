**Date**: 2026-11-02
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-058 and NFR-013; `src/easy_personal/core/metrics.py` `session_timeline` docstring (`detail_sessions`)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-11-02 after HUN-190 merged: "the Session timeline also" (= top 10 + load more shows the entire list). Supervisor decision (stated to the owner): the API keeps `detail_sessions=50` (HUN-186, NFR-013: the full detail was 7.6 MB / 72 s); sessions past 50 are listed without a chart.

**Restated intent**:

> In `web/src/Timeline.jsx`, show the first 10 sessions (API order, newest first), then the shared `useTopN` button from `web/src/Widgets.jsx` ("Show N more" / "Show fewer", `aria-expanded`) that lists every session. Remove `MAX_SESSIONS` and the "Showing 50 of N sessions" line. A session at index ≥ `data.detail_sessions` (when the API sends that field) renders its card with every total (messages, tool calls, tokens, share bar) but no `SessionChart`, no gap count in the footer and no gap list, plus one muted line "Chart kept for the newest 50 sessions only." (the number from `data.detail_sessions`). With no `detail_sessions` field every session has its chart (as today).

**Out of scope**:

* Any API change (`detail_sessions` stays 50)
* Loading chart detail on demand
* Other views

**Requirement Refs**: US-025, FR-058, NFR-013

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner message 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-190 (merged; `useTopN`)

**Entry point**: `easy-personal dashboard` → Usage → Session timeline (`web/src/UsageView.jsx` → `web/src/Timeline.jsx`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | 60 sessions: 10 cards + "Show 50 more"; click → 60 cards, "Show fewer", `aria-expanded="true"`; heading still says "Session timeline (60)" | FR-058 |
| 2 | ≤ 10 sessions → all, no button; 0 → "No sessions in this period." | FR-058 |
| 3 | With `detail_sessions: 50`, card 51+ has no chart `svg`, no "N gaps" footer item, no gap list, and the line "Chart kept for the newest 50 sessions only."; cards 1–50 unchanged | FR-058 |
| 4 | Share bars use every session's tokens (unchanged when folded) | FR-058 |
| 5 | "Showing 50 of N sessions" text is gone | FR-058 |
| 6 | Real run on the owner's machine: Usage 30 days → 10 cards + "Show N more"; open → all; a card past 50 shows the no-chart line; screenshots at 1280 and 375, no horizontal scroll | US-025 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run Timeline Usage Widgets && cd .. && npm --prefix web run build && uv run pytest tests/test_web_toolchain.py -q
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage → Session timeline at 1280×720, owner data | headless screenshot | 10 cards then the button, same look as the other Usage lists |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors / fonts | reuse `.details-toggle`, `.muted` | no new CSS colour or token |
| Accessibility | vitest | `aria-expanded` matches state; count in the button text |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 and 375 | screenshot + `scrollWidth` | no horizontal page scroll |

---

## Approach

**Vital slice**: `useTopN(data.sessions)` → no-chart card past `detail_sessions` → remove cap text.
**Cut list**: no on-demand chart load, no paging by 10.

---

## Edge Case Checklist

- [ ] `detail_sessions` missing → every card has a chart
- [ ] Exactly 10 / 11 sessions
- [ ] Period / scope change folds again (lists unmount while loading)
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/Timeline.jsx` | fold + no-chart card |
| `web/src/Timeline.test.jsx` | tests (the "caps the list" test is replaced on purpose) |
| `src/easy_personal/core/metrics.py` | docstring line only: the page lists every session, charts for the newest `detail_sessions` |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/surfaces/api.py`, other `src/easy_personal/**` code | No API change |
| `web/package.json`, `web/package-lock.json`, `web/src/tokens.css` | No new dependency or token |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Vitest fixtures with 0, 10, 11, 60 sessions, with and without `detail_sessions`; toggle by click and keyboard; real run + screenshots.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review