**Date**: 2026-11-05
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints; glossary)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` NFR-013, FR-064; `BRAINSTORMING_LOG_PERF.md`; then `web/src/UsageView.jsx` + its test, `web/src/Rail.jsx` + its test, `web/src/useLoad.js`, `web/src/api.js`, `web/src/app.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05: "the data loading take so long, investigate the root cause and propose the fix", then "yes, investigate and fix them \[auto\]". Supervisor measurement (store copy, 462k events): the Usage page sends \~14 reads at once and shows nothing until all return (`Promise.all` in `UsageView.jsx` `load`), 5.1–6.0 s; the first card could show after \~0.4 s. The side rail computes its own `since` (`Date.now()`), so its `/api/usage/summary` and `/api/usage/breakdown?by=model` URLs differ by milliseconds from the Usage view's and the server cannot share their work.

**Restated intent**:

> On the Usage page, each part renders as soon as the data it needs has arrived, instead of after all requests: KPI widgets (summary + day breakdown; the previous-period badges fill in when `previous` arrives, "no previous data" only after that request ends or fails), each Breakdown (day, model, project, tool), WorkSplit (day), Outliers, Totals (summary + day), Timeline (sessions). A part still waiting shows a small per-part loading state; a part whose request failed shows its own error line; the other parts still render. Changing period / workspace / account drops older answers (as now). All requests of one Usage load, and the side rail's Cache reuse card, use one `since` computed the same way: `now − days`, rounded **down to the whole minute** (ISO, UTC), so the same question gives the same URL. No API change, no new endpoint.

**Out of scope**:

* Any server change (the server task shares work behind the API)
* Other pages (Warnings & plan, Kit, Board, Memory…)
* Changing what any card shows when all data is there

**Requirement Refs**: NFR-013 (page side), FR-064

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (auto (Supervisor), 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None (API shapes unchanged; runs in parallel with the server task)

**Entry point**: dashboard sidebar → Usage (`UsageView`), side rail `CacheReuse`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | With `/api/usage/sessions` held pending in a test, the KPI widgets, the four breakdowns, Outliers and Totals are on screen; the Timeline shows its own loading state | NFR-013 |
| 2 | With `/api/usage/outliers` failing, every other part renders and Outliers shows its own error line | NFR-013 |
| 3 | Previous-period badges appear when the previous request resolves; "no previous data" only after it fails or for All time | FR-064 |
| 4 | Switching period while requests are pending: answers of the old period never appear | FR-064 |
| 5 | The Usage view's `/api/usage/summary?…` and the rail's `/api/usage/summary?…` in the same minute are the same URL; `since` has seconds `00` and ms `000` | NFR-013 |
| 6 | Existing Usage and Rail tests still pass (changed only where they assert all-at-once loading; say which in the review file) | FR-064 |
| 7 | Real run on a store copy: time to first KPI and time to full page, before and after, pasted | NFR-013 |

---

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage, 1280×720, all data loaded | headless screenshot BEFORE/AFTER | identical layout |
| Usage, 1280×720, sessions still loading | headless screenshot AFTER | cards shown, Timeline loading state in its place |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours; reuse the existing `Loading` component and `.error` style |
| Accessibility | vitest | each per-part loading state has an accessible label (e.g. "Loading sessions"); error lines use `role="alert"` |

### 3. Responsiveness

| Width | Expected |
| -- | -- |
| 1280 px | layout unchanged |
| 375 px | per-part loading states do not cause horizontal scroll |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run src/UsageView.test.jsx src/Rail.test.jsx && npx vitest run && npm run build
```

## Approach

**Vital slice**: start each request at once and keep one state per request (or per part) in `UsageView`; a tiny shared `sinceOf(days)` helper (minute-rounded) used by `UsageView` and `Rail`.
**Cut list**: no request de-duplication layer in the browser, no caching in the browser, no new endpoint, no change to other pages.

Rebuild the web on `main` after merge (static build is gitignored). Real run: `sqlite3 ~/.easy-personal/store.sqlite ".backup <dir>/store.sqlite"`, then `EASY_PERSONAL_HOME=<dir> uv run easy-personal dashboard`; the token is after `#` in the printed URL. Screenshot helpers exist in the Supervisor scratchpad: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h224-shoot.mjs` (adapt; do not edit it in place).

---

## Edge Case Checklist

- [ ] All time (`days === null`): no `since`, no previous request
- [ ] A late answer from an old period / scope is dropped
- [ ] Workspace with no repos: the existing note still shows
- [ ] Planted secret/text fixtures never appear on the page

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/UsageView.jsx` | per-part loading |
| `web/src/UsageView.test.jsx` | AC 1–5 |
| `web/src/Rail.jsx` | shared minute-rounded `since` |
| `web/src/Rail.test.jsx` | AC 5 |
| `web/src/app.css` | only if a per-part loading state needs a rule |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | server is the other task |
| Other views in `web/src/` | out of scope |
| `~/.easy-personal/store.sqlite` (live) | real run on a copy only |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Mock `fetch` per URL with controllable promises; assert parts render as each resolves, per-part error, stale answers dropped, identical summary URLs; existing tests kept; screenshots and timings from a real run on a store copy.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review