**Date**: 2026-11-07
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Workspace, orphan workspace, kit part)
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Apply the C2 process from your role guide
6. Read `PRD.md` FR-087, FR-088, FR-065, US-034, open question 30; `BRAINSTORMING_LOG_KITWS.md`; `tasks/TASK_GUIDE_HUN-238.md` (the exact response shape); `web/src/KitView.jsx`, `web/src/KitView.test.jsx`, `web/src/Loading.jsx`, `web/src/api.js`, `web/src/Scope.jsx`, `web/src/App.jsx` (how `period`, `workspace` and `account` reach the Kit view), `web/src/tokens.css`, `web/src/app.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-07: "for verifier mcp, let collect and show the report by workspace to the dashboard" and "for ui mcp, do the same stuffs with 1" (auto mode; placement is the Supervisor's choice, PRD open question 30).

**Restated intent**:

> **Kit view → block "By workspace".** Under the project list (or above it, the implementer chooses what reads best at 1280×720 and notes why), a block fetched from `GET /api/kit/workspaces` (HUN-238) with the page's `period` and `account`. It is fetched in parallel with the Kit view's own read; the Kit view never waits for it; it uses the `Loading` pattern; its failure shows inline in the block only. When the page has a workspace filter selected, the block still lists all workspaces and marks the selected one.
>
> One row per workspace: colour dot + name; **Verifier** cell: "2 of 3 repos scored", mean as a number with a bar (token colours only), "lowest <folder> 60" and the newest report date (local, YYYY-MM-DD), or "no data"; **easy-ui** cell: "2 sessions · 4 calls · last YYYY-MM-DD", or "not used". Below the rows one line from `ui_checks`: "easy-ui checks (all workspaces): N sessions, P passed, F failed — not split by workspace", or nothing when `ui_checks.status` is "no data". No score is ever drawn as 0: "no data" is text. Empty list: "No workspaces yet." with the link/hint the page already uses to reach Workspaces.

**Out of scope**: backend changes (HUN-238); a per-repo drill-down; sorting controls; alerts; the Workspaces page (HUN-207); an MCP tool or CLI command.

**Requirement Refs**: FR-087, FR-088, FR-065, US-034

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (auto (Supervisor), 2026-11-07)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: HUN-238 (the route) merged, or its branch merged into this task's branch by the Supervisor before this task goes In Progress

**Entry point**: the Kit view (`web/src/KitView.jsx`), reached from the sidebar "Kit"

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | With two workspaces in the fake response, two rows render with name, verifier text ("2 of 3 repos scored", mean, lowest folder + value, date) and easy-ui text ("2 sessions · 4 calls · last <date>") | FR-087, FR-088 |
| 2 | A workspace with `verifier.status "no data"` shows the text "no data" and no bar or number; `ui.status "not used"` shows "not used" | FR-087, FR-088 |
| 3 | `ui_checks` with sessions shows the one line "not split by workspace" once, under the rows (not per row); `ui_checks.status "no data"` → no line | FR-088 |
| 4 | The Kit project list renders while `/api/kit/workspaces` is pending; a failed read shows an inline error inside the block only; the block uses the `Loading` pill pattern (one pill per view) | FR-065 |
| 5 | The request carries the page's `period` and `account`; changing the period refetches the block | FR-087 |
| 6 | The selected workspace filter marks its row (`aria-current`) and the block still lists all rows | FR-087 |
| 7 | Empty `workspaces` → "No workspaces yet." text; no crash on missing optional fields | FR-087 |
| 8 | Existing vitest + pytest pass; the build passes | — |

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest -q
```

Plus a real run: dashboard from the branch on a COPY of the owner store with HUN-238 merged into the branch, headless Chrome at 1280×720 and 375×812: Kit view with the block; screenshots. If the owner store has no verifier rows, use fixture reports in a temp `EASY_PERSONAL_VERIFIER_REPORTS`.

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Kit view with the new block, 1280×720 and 375×812 | headless screenshot BEFORE/AFTER | block readable below/above the project list; project list unchanged |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours |
| Accessibility | vitest + keyboard | rows are a table or list with a header/label; bars have text equivalents; error uses `role="alert"` as FR-065 says |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720, 375×812 | screenshot + `scrollWidth` | no horizontal page scroll; long workspace names ellipsised with `title` |

---

## Approach

**Vital slice**: `WorkspaceKit.jsx` (new) + tests → wire into `KitView.jsx` with a parallel fetch → styles. **Cut list**: no drill-down, no sorting, no alert colours beyond a token bar.

## Edge Case Checklist

- [ ] A very long workspace name → ellipsis, full name in `title`
- [ ] Mean 0 as a real score vs "no data" are drawn differently
- [ ] 375 px width: the two cells stack, no horizontal scroll
- [ ] Planted secret text in a workspace name/description never rendered as HTML (React escapes; add one test)
- [ ] Period change while a request is in flight → the late answer does not overwrite the new one

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/WorkspaceKit.jsx` (new) | the block |
| `web/src/KitView.jsx` | wire the block |
| `web/src/app.css` | block styles |
| `web/src/WorkspaceKit.test.jsx` (new), `web/src/KitView.test.jsx` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | Backend is HUN-238 |
| `web/src/WorkspacesView.jsx` | HUN-207 |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [x] Implementation done
- [x] Tests written AND pass — output pasted into the review file's Evidence table
- [x] UI Evidence rows filled
- [x] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-07 DEBATE place the block on the Kit view or the Workspaces page → Kit view (Advocate 1 argued Kit view, high confidence: `web/src/KitView.jsx:339` takes `scope`, US-034/FR-087 name the Kit view, avoids a clash with HUN-207; Advocate 2 argued Workspaces page and conceded low confidence: `web/src/WorkspacesView.jsx:639` takes no scope or period, which FR-087 requires; decided by rule 1: only the Kit view has the scope and period) ⚠ decided for you — please review
