# TASK_GUIDE — T137: Git diff 2 — Changes tab and the Live *Changes* button (page)
**Date**: 2026-11-26
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Supervisor (DDR-0011)
**Agent guide**: `agents/frontend.md`

> Order note: this guide was written after the first draft of the page code, before any commit. Hard-Stop Gate 1 asks
> for it first.

---

## Mandatory Startup (Do Not Skip)

Read `PROJECT_SPEC.md`, this file, `docs/ddr/0010-git-diff-view-show-never-store.md`.

---

## Requirement (Pillar 1)

"the git diff for the repos is important, so I want to have this feature in this our application" (owner, 2026-11-26). Owner choices: full compare, Changes tab and Live button, view only, file list then one file.

**Restated intent**:
> In the app the owner picks a repo (or opens it from a Live session), picks what to compare, sees the changed files and reads one file's changes.

**Out of scope**: any git write; side-by-side; highlight; line numbers; auto refresh; storing the content.

**Requirement Refs**: FR-141 (over FR-139), US-045.

### Requirement Fidelity Gate
- [x] Restated intent matches the request: auto (Supervisor), from the owner's four answers
- [x] Terms align with the glossary
- [x] Every criterion traces to the requirement
- [x] Refs exist in `PRD.md`

---

## Dependencies & Reachability

**Depends on**: T136 — the `/api/diff/*` routes
**Entry point**: `ChangesView`

---

## Acceptance Criteria

| # | Criterion | Traces to |
|---|-----------|-----------|
| 1 | A *Changes* tab lists workspace repos (the global workspace scope narrows them) and the repo's work trees | FR-141 |
| 2 | The four compare modes ask the routes with the picked refs | FR-141 |
| 3 | The file list shows area, status and counts; a picked file's patch marks added and removed lines; *Refresh* reads again | FR-141 |
| 4 | *Changes* on a Live panel opens the tab on that session's folder | FR-141 |
| 5 | Content is drawn as text only; empty, loading, error, cut, binary and filters-off states are said in words | FR-141, NFR-002 |

## Evaluation & Acceptance

### Verification Command

```bash
cd web && npx vitest run && npm run build
```

## UI / Design Acceptance Criteria

### 4. UX Acceptance Criteria

| User story | User goal | Flow / state | Pass condition (observable) |
|------------|-----------|--------------|-----------------------------|
| US-045 | read changes | open Changes, pick a file | files grouped by area; added lines green with `+`, removed red with `-`; no sideways page scroll at 375px and 1280px |
| US-045 | compare two refs | *Two refs* | From and To selects and the file list show at 375px and 1280px |

## Approach

**Pattern reference**: `web/src/BoardView.jsx` (a work view with pickers), `web/src/api.js` (guarded reads).
**Vital slice**: pickers, file list, patch.
**Cut list**: side-by-side, highlight, line numbers, auto refresh.

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `web/src/ChangesView.jsx`, `changes.css`, `ChangesView.test.jsx` | new |
| `web/src/{App,LiveView,SessionPanel}.jsx`, `api.js`, `tokens.css` | tab, button, guarded read, one colour |
| Pre-existing pin tests | `App.test.jsx` pins the tab list: add *Changes* |

## Files Must NOT Touch

`rust/`, `src/easy_personal/`, `.claude/`, `skills/`.
