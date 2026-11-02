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
6. Read `PRD.md` FR-023, FR-056, FR-058

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-11-02 (after HUN-189 merged): "I think show top 10 will be good for now, for all, load more will show the entire". Supervisor reading (stated to the owner): every ranked list in the Usage view; the By day chart (a timeline) and Kit (already folds, HUN-188) are not changed.

**Restated intent**:

> In the Usage view, By project (bars and its "Show table" table), By model, By tool and Outlier sessions each show their first 10 rows in the order the API sends (already sorted: tokens descending; outliers by tool calls), then a button "Show N more" (N = hidden rows) that shows every row; when open it reads "Show fewer". A list with ≤ 10 rows shows no button. Share bars and totals still use every row. One small shared piece (e.g. a `useTopN` hook or `<ShowMore>` in `web/src/Widgets.jsx`) used by all four lists; match the Kit button (`KitView.jsx` `details-toggle`, `aria-expanded`). Changing period / workspace / account folds the lists again.

**Out of scope**:

* Any API change (the backend still sends all rows)
* By day chart and table; Kit; Board; Memory; Live
* A user setting for the row count (fixed 10)

**Requirement Refs**: US-025, FR-023, FR-058

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner message 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: none

**Entry point**: `easy-personal dashboard` → Usage (`web/src/UsageView.jsx` → `web/src/Breakdown.jsx`, `Outliers`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | By project with 25 groups: 10 bar rows + button "Show 15 more"; click → 25 rows, button "Show fewer", `aria-expanded="true"`; click → 10 again | FR-058 |
| 2 | Same rule for By model, By tool and Outlier sessions; the By project "Show table" table also shows 10 rows + its own button | FR-058 |
| 3 | 10 or fewer rows → all rows, no button; 0 rows → the existing empty text | FR-058 |
| 4 | Bar widths in By project still use the total of every row (row 1 width unchanged when folded) | FR-058 |
| 5 | By day chart and its table show every day (unchanged) | FR-058 |
| 6 | Changing the period folds lists back to 10 | FR-058 |
| 7 | Keyboard: the button is a `<button>`, reachable by Tab, toggles with Enter/Space; the count is in its text | FR-058 |
| 8 | Real run on the owner's machine: Usage (30 days) shows 10 projects + "Show N more"; screenshots at 1280 and 375 | US-025 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run Usage Breakdown Widgets && cd .. && npm --prefix web run build && uv run pytest tests/test_web_toolchain.py -q
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage at 1280×720, 30 days, owner data | headless screenshot (playwright-core from `../easy-ui-mcp/node_modules`, `/usr/bin/google-chrome`; nav items are buttons; at narrow width click `Menu` first) | each list ends with the button under row 10; looks like the Kit button |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors / fonts | reuse `.details-toggle` | no new colour; only `var(--…)` from `tokens.css` |
| Accessibility | vitest | `aria-expanded` matches state; button text has the count |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 and 375 | screenshot + `scrollWidth` | no horizontal page scroll |

---

## Approach

**Vital slice**: one shared fold helper → By project bars → model, tool, outliers → project table.
**Cut list**: no "show 10 more" paging, no row-count setting, no API limit.

---

## Edge Case Checklist

- [ ] Exactly 10 / 11 rows
- [ ] Two lists open at once are independent
- [ ] Folded state resets when data changes (period, workspace, account)
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/Widgets.jsx` | shared fold helper |
| `web/src/Breakdown.jsx`, `web/src/UsageView.jsx` | use it |
| `web/src/Breakdown.test.jsx` or `web/src/UsageView.test.jsx`, `web/src/Widgets.test.jsx` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` | No API change |
| `web/package.json`, `web/package-lock.json`, `web/src/tokens.css` | No new dependency or token |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Vitest with fixture groups of 0, 10, 11 and 25 rows for each list; toggle by click and keyboard; bar width check; period change folds; real run + screenshots.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review