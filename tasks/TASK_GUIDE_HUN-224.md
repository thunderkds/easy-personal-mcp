**Date**: 2026-11-05
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` in full (glossary: Saved report, Rule snapshot, Before / now)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and **no** `limit` — the whole file
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-074, FR-078, FR-079; `BRAINSTORMING_LOG_REPORTS.md` (follow-up section); `web/src/WarningsPlanView.jsx` (`Card`, `Causes`, `NotChecked`, `CHIPS`, `byPriority`, `steps`), `web/src/SavedReports.jsx`, `web/src/WarningsPlanView.test.jsx`, `web/src/app.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05: "the report content is full, but the showup information is not good, can we follow the same format with our warning and plan". Owner chose the "Warning cards" mockup: summary, fixes with citation chips, "1 to watch then" cards like the main page with a "Now:" line, then "Other rules (6)" closed.

**Restated intent**:

> When a saved report is opened, show, in order: **Summary**; **Fixes** (numbered, each with its cited tool and number as a chip, e.g. `session_outliers · 645,301`); **"N to watch then"** — one card per rule whose `before.state` is `fired`, sorted by `before.priority`, rendered by the **same** `Card` **component** as Warnings & plan (export it from `WarningsPlanView.jsx`; do not copy its markup), fed with `before` as the hit; under each card's body a **"Now:"** line: the now value + "still firing" when `now.state` is `fired`, "clear now" when `clear`, the reason when `abstained` / `not_measured`; then a closed **"Other rules (N)"** disclosure (same look as "Not checked") listing every other rule as `title  before → now` with the existing value formatting. When no rule fired then, the heading says "Nothing fired then" and only the Other rules list shows. The report list rows stay as they are. The card works for an old report whose `before` has no `steps` (it then shows `fix`, as `Card` already does).

API shape (from HUN-223, being built in parallel): a fired rule's `before` = the full warning hit (`rule_id`, `label`, `priority`, `measured`, `threshold`, `window_days`, `fix`, `title`, `why`, `steps`, `summary`, optional `detail`, `causes`, `diagnosis`, optional `cache_read_share`) plus `state`, `fired`. Old reports: `label`, `priority`, `why`, `fix`, `window_days` filled from config, no `steps`. Write fixtures in that shape.

**Out of scope**:

* Any server change (HUN-223)
* The report list rows, the warnings list above, the hint
* Delete / edit, charts

**Requirement Refs**: FR-079 (UI part)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner chose "Warning cards", 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-223 for the real run only (fixtures carry the shape; merge `main` into your branch before the real run if HUN-223 has merged; otherwise do the real run on a store copy after it merges, or report it as pending)

**Entry point**: dashboard sidebar → Warnings & plan → Saved reports → open a report

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Opened report order: Summary, Fixes (chips `tool · number`), "N to watch then", Other rules | FR-079 |
| 2 | A fired-then rule renders with the same `Card` as the warnings list (same title, label chip, summary, Why it matters, "Why this happened" causes, cache-read share, steps); asserted by shared test ids / roles, and `Card` is imported, not copied | FR-079, FR-074 |
| 3 | "Now:" line shows `223.1% · still firing`, `40% · clear now`, or the abstain / not-measured reason | FR-079 |
| 4 | Two fired rules sort by priority (1 first) | FR-079 |
| 5 | Other rules disclosure is closed by default, shows "(N)", lists `title before → now` | FR-079 |
| 6 | Nothing fired then → "Nothing fired then" + Other rules only | FR-079 |
| 7 | Old-shape `before` (no `steps`) shows the `fix` line, no crash | FR-079 |
| 8 | Warnings list, hint and report list rows unchanged (existing tests green) | FR-074, FR-078 |
| 9 | Real run on a store copy: screenshots 1280×720 and 375×812 with report open | FR-079 |

---

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Warnings & plan, one report open, 1280×720 | headless screenshot BEFORE/AFTER | card identical in look to the warnings list card above it; Now line under it; Other rules closed |
| Same, 375×812 | headless screenshot | readable; no page-wide horizontal scroll |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours |
| Reuse | code read | `Card` / `Causes` imported from `WarningsPlanView.jsx`; disclosure uses the `NotChecked` look |
| Accessibility | vitest | disclosure is `<details>`/`<summary>` or a button with `aria-expanded` |

### 3. Responsiveness

| Width | Expected |
| -- | -- |
| 1280 px | card full width of the report panel |
| 375 px | no horizontal scroll; chips wrap |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run src/WarningsPlanView.test.jsx && npx vitest run && npm run build
```

## Approach

**Vital slice**: export `Card` (and what it needs) from `WarningsPlanView.jsx`; rewrite `ReportOpen` in `SavedReports.jsx`; tests; CSS only for the Now line and chips.
**Cut list**: no new route, no change to the list rows, no charts.

---

## Edge Case Checklist

- [ ] `before.causes` empty → no "Why this happened" block (Card already handles)
- [ ] `now` missing value (abstained) → reason text, never `0`
- [ ] Report text renders as text (no `dangerouslySetInnerHTML`)
- [ ] Planted secret/text fixtures never appear on the page

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/SavedReports.jsx` | card layout |
| `web/src/WarningsPlanView.jsx` | export `Card` (no behaviour change) |
| `web/src/WarningsPlanView.test.jsx`, `web/src/test/fixtures.js` | AC 1–8 |
| `web/src/app.css` | Now line, fix chips |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | server is HUN-223 |
| Other views in `web/src/` | out of scope |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Fixtures in the HUN-223 shape (new and old report); assert order, card reuse, Now line states, priority sort, closed disclosure, empty-fired state, old shape; screenshots from a real run on a store copy.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review