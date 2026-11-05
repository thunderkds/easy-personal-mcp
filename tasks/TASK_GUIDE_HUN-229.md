**Date**: 2026-11-05
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints; glossary: Saved report)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-065; then `web/src/SavedReports.jsx`, `web/src/Loading.jsx` + `web/src/Loading.test.jsx`, `web/src/WarningsPlanView.jsx` (line \~153 uses `Loading`) + `web/src/WarningsPlanView.test.jsx`, the `.loading-pill` rules in `web/src/app.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05: "when click on the report, should show the loading indicator also", then "with the loading, should show loading indicator, the loading should random the text message for funny". Today `SavedReports.jsx` `ReportOpen` shows a plain text line `<p className="muted report-wait">Loading report</p>` while `GET /api/reports/{id}` runs, and the section shows `<p className="muted">Loading saved reports</p>` while the list loads. FR-065 says page and list loading shows as the one floating loading indicator (the `Loading` component), not a line of text that pushes the page.

**Restated intent**:

> When the owner clicks a saved report, the floating loading indicator (`<Loading label="Loading report" />`) shows until that report's data arrives or fails, instead of the text line. The Saved reports list loading uses the same indicator (`<Loading label="Loading saved reports" />`). A failed load still shows its inline error line as now ("Report not found" / "Could not load the report: …" / "Could not load saved reports: …"). Nothing else on the page changes. No API change.
>
> Funny line: the shared `Loading` component (all 11 uses: Usage, Board, Jobs, Kit, Warnings, Workspaces, Memory, FirstRun, Agents, App, and the two new saved-report uses) keeps its real label as now and adds one funny line under it, picked at random from a fixed list when the indicator appears. The line stays the same while that indicator is on screen (no rotation). The label stays first in the pill's text, so a screen reader and the existing tests still read "Loading report" etc. The list lives in one small module (e.g. `web/src/loadingLines.js`), \~12 short original lines, friendly and on-theme (tokens, sessions, cache, workspaces), English, no names of people or brands, no jokes about the user. Examples: "Counting tokens one by one…", "Asking the cache nicely…", "Herding stray sessions…", "Dusting off old transcripts…", "Bribing the database with coffee…", "Sorting receipts from your robots…". A test can make the pick deterministic (e.g. a `pick` argument or a stubbed `Math.random`).

**Out of scope**:

* Any server change
* Other views' loading text lines that do not use `Loading`
* Rotating lines, a settings switch to turn the lines off, translations
* A skeleton or placeholder inside the opened report

**Requirement Refs**: FR-065

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: dashboard sidebar → Warnings & plan → Saved reports (`SavedReports`) → click a report row (`ReportOpen`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | With `/api/reports/{id}` held pending, clicking a report row shows `role="status"` "Loading report" rendered by the `Loading` component (class `loading-pill`); the old `.report-wait` text line is gone | FR-065 |
| 2 | When the report resolves, the indicator disappears and the report shows; when it fails (404 / other), the indicator disappears and the existing inline error line shows | FR-065 |
| 3 | With `/api/reports` held pending, the Saved reports section shows the `Loading` indicator "Loading saved reports"; on error the inline error line still shows | FR-065 |
| 4 | `Loading` renders its label plus exactly one line from the fixed list (`.loading-line`); with a stubbed random source, the chosen line is the expected one; the line does not change on re-render while shown; every line in the list is ≤ 40 characters | FR-065 (owner ask) |
| 5 | `Loading.test.jsx` view table still passes: each view's pill still contains its label | FR-065 |
| 6 | Existing Warnings & plan / saved-report tests still pass (changed only where they assert the old text line; say which in the review file) | FR-065 |

---

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Warnings & plan, 1280×720, report clicked with its request held | headless screenshot AFTER | floating pill centred in the content area; no text line pushes the list |
| Warnings & plan, 1280×720, report opened | headless screenshot BEFORE/AFTER | identical |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Reuse | code | the existing `Loading` component; at most one small CSS rule for `.loading-line` using `web/src/tokens.css` variables only (muted colour, smaller size) |
| Accessibility | vitest | indicator has `role="status"` and its label |

### 3. Responsiveness

| Width | Expected |
| -- | -- |
| 1280 px | pill centred in the content area |
| 375 px | pill visible, the funny line wraps inside the pill, no horizontal scroll |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run src/WarningsPlanView.test.jsx src/Loading.test.jsx && npx vitest run && npm run build
```

## Approach

**Vital slice**: replace the two text lines in `SavedReports.jsx` with `<Loading label=… />`; add the line list and a once-per-mount random pick (`useState(() => pick())`) to `Loading`; tests per AC.
**Cut list**: no skeleton, no rotation, no off switch, no delay/debounce for fast loads.

Real run: `sqlite3 ~/.easy-personal/store.sqlite ".backup <dir>/store.sqlite"`, then `EASY_PERSONAL_HOME=<dir> uv run easy-personal dashboard`; the token is after `#` in the printed URL. Screenshot helper to adapt (do not edit in place): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h224-shoot.mjs`. To catch the pill, delay the `/api/reports/{id}` response in the page (e.g. Playwright `page.route`).

---

## Edge Case Checklist

- [ ] Click a report, then click it again (close) before it loads: no indicator left on screen
- [ ] Switch between two reports quickly: only the current one's indicator / data shows
- [ ] Warnings list and Saved reports loading at the same time: two pills may stack — check it looks acceptable and say so in the review file
- [ ] Planted secret/text fixtures never appear on the page

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/SavedReports.jsx` | use `Loading` |
| `web/src/Loading.jsx` | funny line |
| `web/src/loadingLines.js` | new: the line list |
| `web/src/Loading.test.jsx` | AC 4–5 |
| `web/src/app.css` | `.loading-line` rule |
| `web/src/WarningsPlanView.test.jsx` | AC 1–3, 6 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | no server change |
| Other views in `web/src/` (their `<Loading label=…>` calls) | out of scope; they get the line through `Loading` |
| `~/.easy-personal/store.sqlite` (live) | real run on a copy only |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Mock `fetch` per URL with controllable promises; assert the pill while pending, gone after resolve / reject; existing tests kept; screenshot from a real run on a store copy.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review