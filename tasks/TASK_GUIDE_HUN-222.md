**Date**: 2026-11-05
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Saved report, Rule snapshot, Before / now; Critical Constraints)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` US-032, FR-074, FR-078; `BRAINSTORMING_LOG_REPORTS.md` (mockup, Edge Case Checklist); then `web/src/WarningsPlanView.jsx` + its test, `web/src/api.js`, `web/src/tokens.css`, `web/src/app.css`; and the merged HUN-221 `GET /api/reports` / `GET /api/reports/{id}` JSON (read its tests on `main`)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05: "we should log this report to the dashboard also, for easy refer, or trace the improvement or enhance if any". Owner choices 2026-11-05: before / now per rule (no "tried" ticks); a section on the Warnings & plan page (not a new page); brainstorm Option A ("as you recommend").

**Restated intent**:

> Under the warnings list on the Warnings & plan page, add a *Saved reports* section from `GET /api/reports`: newest first; each row shows save date (local time), scope ("all workspaces" or the workspace/account), how many rules fired then, and the summary's first line. Activating a row (click, Enter or Space) opens it in place and loads `GET /api/reports/{id}`: the summary, the fixes (each with its cited tool + number when present), and a before / now list, one line per rule: title, saved value + fired / clear, arrow, now value + fired / clear, labelled "now = last 7 days". A rule abstaining now, or not measured now, shows that text instead of a number. Only one report is open at a time. Empty list text: "No saved reports yet. Run `/mcp__easy-personal__investigate` in Claude Code." No write control on the page.

**Out of scope**:

* Any server change (HUN-221)
* Delete / edit, "tried" ticks, charts, filters, pagination
* Changes to the warnings list above (FR-074 stays)

**Requirement Refs**: FR-078 (UI part), US-032

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner "as you recommend", 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-221 (FR-076, FR-077, FR-078 API) merged into `main`

**Entry point**: dashboard sidebar → Warnings & plan → Saved reports section

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | With 2 reports the section lists both, newest first, each with date, scope, fired count and summary first line | FR-078 |
| 2 | Click / Enter / Space on a row opens it and requests `/api/reports/{id}`; opening another closes the first | FR-078 |
| 3 | Open report shows summary, fixes (tool + number when present), and one before / now line per rule, e.g. `Token use jumped 218% fired → 40% clear` | FR-078 |
| 4 | A rule abstaining now shows its reason; a rule not measured now shows "not measured now"; never a blank or `0` | FR-078 |
| 5 | Empty list shows the exact FR-078 empty text | FR-078 |
| 6 | The warnings list and the investigate hint above are unchanged (existing tests kept); the section has no button that writes | FR-074, FR-078 |
| 7 | Report text renders as text (a `<b>` in a fixture summary shows literally) | NFR-002 |
| 8 | Real run on a store copy with one report saved via MCP: screenshots at 1280×720 and 375×812, list closed and one report open | FR-078 |

---

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Warnings & plan with 1 saved report, 1280×720, closed + open | headless screenshot BEFORE/AFTER | section under the warnings list; before / now lines aligned |
| Same, 375×812 | headless screenshot | readable; before / now wraps; no page-wide horizontal scroll |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours; fired uses the existing warning colour, clear the existing ok colour |
| Accessibility | vitest | rows are buttons with `aria-expanded`; before / now is a list; the arrow has text for screen readers ("then … now …") |

### 3. Responsiveness

| Width | Expected |
| -- | -- |
| 1280 px | section full width of the main column |
| 375 px | each before / now line stacks then above now; no horizontal scroll |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run src/WarningsPlanView.test.jsx && npx vitest run && npm run build
```

## Approach

**Vital slice**: a `SavedReports` component in `WarningsPlanView.jsx` (or a sibling file if it passes \~120 lines), two `api.js` calls, CSS in `app.css`.
**Cut list**: no new route or page, no charts, no filters, no write control.

Start only after HUN-221 is merged into `main`; merge `main` into your branch first. Rebuild the web on `main` after merge (static build is gitignored).

---

## Edge Case Checklist

- [ ] `/api/reports/{id}` 404 (report gone) → inline "Report not found", list stays
- [ ] Load error → existing error state, warnings list still shown
- [ ] Very long summary first line → truncated with ellipsis in the row, full in the open view
- [ ] Planted secret/text fixtures never appear on the page

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/WarningsPlanView.jsx` | Saved reports section |
| `web/src/WarningsPlanView.test.jsx` | AC 1–7 |
| `web/src/api.js` | two GET calls |
| `web/src/app.css` | section rules |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | server is HUN-221 |
| Other views in `web/src/` | out of scope |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Mock `/api/reports` and `/api/reports/{id}` shaped like HUN-221's JSON; assert list order, open/close, before / now lines, abstain / not-measured text, empty state, literal text rendering; screenshots from a real run on a store copy.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review