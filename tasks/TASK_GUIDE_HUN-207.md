**Date**: 2026-11-04
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints; glossary: Workspace, scratch folder, unassigned repo)
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Apply the C2 process from your role guide
6. Read `PRD.md` FR-069, FR-070, FR-065; `BRAINSTORMING_LOG_SCRATCH.md` (UI sketch, Edge Case Checklist); `tasks/TASK_GUIDE_HUN-206.md` (the `Scratch` group shape and `GET /api/workspaces/unassigned` answer); `web/src/Breakdown.jsx` (`ProjectBars`); `web/src/WorkspacesView.jsx` (list, `WorkspaceForm`, how it saves via `PATCH /api/workspaces/{id}`); `web/src/Toast.jsx` (`useNotify`); `web/src/Loading.jsx`; `web/src/api.js`; `web/src/tokens.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-04: "relate to the workspace, I saw the management is not good also, let evaluate and improve that. by the way, I saw a lot of draft folder and project that I do not work on". Chosen: "Auto 'Scratch' group", "List + 'Add to' button", brainstorm Option A ("continue").

**Restated intent**:

> **Usage → By project.** The `Scratch` group (`scratch: true`) shows as one row "Scratch (N folders)" at its sorted place, as a disclosure button (`aria-expanded`). Opening it lists each entry of `folders` indented under it, with the same name / tokens / bar as other rows (bar share of the period total). Closed by default; no request on open (the data is in the group).
>
> **Workspaces page.** A card "Repos not in a workspace (N)" under the workspace list, filled from `GET /api/workspaces/unassigned`, fetched in parallel with the page's other reads: the page never waits for it; its own loading uses the `Loading` pattern; its failure shows inline in the card only. Each row: folder name, full path (mono, ellipsis, `title` = path), "N sessions · $X · last used YYYY-MM-DD" (local date), a `<select>` of non-orphan workspaces (labelled "Add to workspace") and an "Add" button. Add: read the chosen workspace's current repos (fresh), `PATCH /api/workspaces/{id}` with those repos + this path (no duplicate), busy button while it runs, toast on success ("Added <name> to <workspace>") or error (FR-065); on success the row leaves the list and the workspace's repo count updates. With no non-orphan workspace: a hint "Add a workspace first" instead of the controls. Empty list: "Every repo with usage is in a workspace."

**Out of scope**: backend changes (HUN-206), a hide button, bulk add, guessing a workspace, Live view / session timeline names.

**Requirement Refs**: FR-069, FR-070, FR-065, US-025, US-029

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, 2026-11-04; UI sketch in `BRAINSTORMING_LOG_SCRATCH.md` shown to the owner)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: HUN-206 (Scratch group + `GET /api/workspaces/unassigned`) merged

**Entry point**: `<Breakdown by="project">` in `web/src/UsageView.jsx`; the Workspaces view (`web/src/WorkspacesView.jsx`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | By project with a `Scratch` group of 3 folders: one closed "Scratch (3 folders)" row; click → 3 indented rows, `aria-expanded="true"`; click again → closed | FR-069 |
| 2 | Opening Scratch sends no request | FR-069 |
| 3 | No `Scratch` group → By project renders exactly as before | FR-069 |
| 4 | Workspaces page renders its workspace list while `/api/workspaces/unassigned` is still pending; a failed unassigned read shows an inline error in the card only | FR-070 |
| 5 | Rows show name, path, sessions, cost, last-used date; the select lists non-orphan workspaces only | FR-070 |
| 6 | Add: PATCH body = the workspace's current repos + the path, no duplicate; success → toast, row gone, repo count +1; error → error toast, row stays | FR-070, FR-065 |
| 7 | Only orphan exists → hint, no select/button; empty list → "Every repo with usage is in a workspace." | FR-070 |
| 8 | Existing vitest + pytest still pass; the build passes | — |

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest -q
```

Plus a real run: dashboard from the branch on a COPY of the owner store, headless Chrome at 1280×720 and 375×812: Usage → By project (Scratch closed, then open); Workspaces → the new card, Add one repo to a workspace on the COPY; screenshots.

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Usage By project, Scratch closed + open, 1280×720 | headless screenshot BEFORE/AFTER | \~11 rows + Scratch instead of 127 rows |
| Workspaces page with the new card, 1280×720 and 375×812 | headless screenshot | card readable, controls reachable |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours |
| Accessibility | vitest + keyboard | disclosure has `aria-expanded`; select labelled; toast roles per FR-065 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720, 375×812 | screenshot + `scrollWidth` | no horizontal page scroll; long paths ellipsised |

---

## Approach

**Vital slice**: Scratch disclosure in `ProjectBars` + tests → unassigned card component + tests → wire into `WorkspacesView` → Add via PATCH + toast.
**Cut list**: no hide button, no bulk add, no workspace guess, no sort control, no change to the Live view or session timeline.

## Edge Case Checklist

- [ ] Two folders with the same name inside Scratch → both rows shown (keys not by name alone)
- [ ] A very long path → ellipsis, full path in `title`
- [ ] The workspace was edited in another tab → Add reads its repos fresh before PATCH
- [ ] Double-click on Add → one PATCH (busy button)
- [ ] Planted secret/text fixtures never appear in the page

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/Breakdown.jsx` | Scratch disclosure |
| `web/src/WorkspacesView.jsx` (+ optional `web/src/UnassignedRepos.jsx`) | the new card, Add |
| `web/src/app.css` | indent + card styles |
| `web/src/*.test.jsx` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | backend is HUN-206 |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows filled
- [ ] Supervisor notified: task ready for Stage 4 review
## Questions Log

- 2026-11-04 DEBATE HUN-207 needs HUN-206 merged; auto mode never merges — stack on HUN-206 or stop? → (B) stop after HUN-206 (Advocate 1: stack on the HUN-206 branch, Depends on is advisory; Advocate 2: stop, the precondition is a merge and a stacked branch ties the UI to unreviewed backend; safer-option rule 2: smaller change) ⚠ decided for you — please review (Linear comment + decision on HUN-207)
