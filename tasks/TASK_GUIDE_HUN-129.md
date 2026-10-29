**Date**: 2026-10-29
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
6. Verify on fixtures only: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_AGENT_ROOT`; never open the real `~/.easy-personal/` or `~/.claude*/`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-29: "fix" — reply to the Supervisor's question "Should I fix the "By model" column …", which named the defect: in the Usage "By model" card the cost column, header included, is cut off at the right edge (seen at 1440 px and in the 1920 px HUN-128 screenshot).

Cause (Supervisor, read from code): `web/src/Breakdown.jsx` `ModelTable` sits in `.table-wrap { overflow-x: auto }` inside a half-width `.grid-2` card; the table is wider than the card (long header "API-equivalent cost", 17-char model ids), so the card scrolls sideways and the last column looks clipped.

**Restated intent**:

> In the "By model" card all four columns are fully visible without sideways scrolling at 1440 and 1024 px viewports on the fixture data (model ids like `claude-sonnet-5-5`, totals up to 8 digits with separators, costs like `$14.26`). Use the design's short headers — `Model`, `Total`, `Output`, `Cost` — and keep the full meaning for assistive tech (`abbr title` or a visually-hidden suffix: "Total tokens", "Output tokens", "API-equivalent cost"). Keep the existing `.table-wrap` scroll as the fallback for narrower cards (375 px), never clipped without a scroll. No other card changes.

**Out of scope**: the other breakdown tables; the KPI strip; column order; data.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-29
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (*API-equivalent cost*)
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: HUN-126, HUN-128 (merged). **Entry point**: `web/src/Breakdown.jsx` `ModelTable` (Usage view).

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Header cells read `Model`, `Total`, `Output`, `Cost`; each short header carries its full name for assistive tech ("Total tokens", "Output tokens", "API-equivalent cost") | headers |
| 2 | At 1440 and 1024 px on fixture data, the By model `.table-wrap` has `scrollWidth <= clientWidth` (no sideways scroll) and the cost cell's right edge is inside the card | no clipping |
| 3 | At 375 px the table still scrolls inside `.table-wrap`; no page-level horizontal scroll | fallback |
| 4 | No color or size literal outside `tokens.css`; all Vitest tests pass; build ok; `uv run pytest -q` green | standards |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | Usage with 3 model groups | `columnheader` names: Model, Total tokens, Output tokens, API-equivalent cost (accessible names) and visible text Model/Total/Output/Cost | Vitest |
| 2 | viewport 1440 and 1024, fixture data | By model `.table-wrap` scrollWidth <= clientWidth | host Playwright |
| 3 | viewport 375 | page scrollWidth <= clientWidth | host Playwright |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-129.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-129.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage By model card at 1440 / 1024 / 375 | screenshot | all 4 columns visible at 1440 and 1024 |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens only | grep | no literal outside `tokens.css` |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1440 / 1024 / 375 px | rect checks | as Success Criteria |

---

## Approach

**Pattern reference**: `web/src/Breakdown.jsx` `ModelTable`; `.card th, .card td` padding in `web/src/app.css`.
**Vital slice**: short headers with full accessible names; tighten only if still too wide (e.g. model cell may ellipsize with `title`).
**Cut list**: none.

---

## Edge Case Checklist

- [ ] a long unknown model id → ellipsis + `title`, not a wider table
- [ ] an abstained cost (no price) still renders its existing text inside the column

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/Breakdown.jsx` | short headers |
| `web/src/app.css` | only if needed |
| `web/src/UsageView.test.jsx` | header assertions |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` (except rebuilt static) | no API change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |
| `web/package.json` dependencies | no new dependency |

---

## Test Plan

Vitest for Success 1 (red first). One batched host-Playwright script (`/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-ui-mcp/node_modules`) on a fixture dashboard for Success 2–3, writing rects and screenshots to `reports/<this task ID>/`.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] Supervisor notified: task ready for Stage 4 review