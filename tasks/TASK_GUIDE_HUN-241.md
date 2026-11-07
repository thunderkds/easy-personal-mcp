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
6. Read `PRD.md` FR-089, FR-087, FR-088, FR-065, US-035; `tasks/TASK_GUIDE_HUN-240.md` (the exact response shape); `web/src/WorkspaceKit.jsx`, `web/src/WorkspaceKit.test.jsx`, `web/src/KitView.jsx`, `web/src/Loading.jsx`, `web/src/api.js`, `web/src/tokens.css`, `web/src/app.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-07: "you can depend on the path of projects in the workspace, and mapping the folder reports, report, or something like that to the dashboard, we just show to easy refer, not manage it in anyway". Owner choice: relative path.

**Restated intent**:

> **Kit view → block "Kit by workspace" → report folders.** Under each workspace row, one muted line per report folder from `GET /api/kit/reports` (HUN-240): `app/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05`. `files_capped` shows `2000+ files`. `easy_ui_sessions` 0 is left out of the line. A workspace with `folders: []` shows "no report folders". The text is plain: **no link, no button, no click action** on a folder (reference only, nothing is managed).
>
> The read is fetched in parallel with the block's own read and the Kit view's read; neither waits for it. It has its own `useLoad`. While it loads, the block shows no extra pill (the block's `Loading` pill rules of FR-065 stay: one pill per view). If it fails, each row shows one small inline "report folders unavailable" text and the rest of the block is unchanged. Rows are matched by workspace `name`; a workspace missing from the reports answer shows "no report folders".

**Out of scope**: backend (HUN-240); opening, copying or acting on a folder; absolute paths; sorting or filtering; alerts; the Workspaces page; any MCP tool or CLI command.

**Requirement Refs**: FR-089, US-035, FR-065

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner message and owner choice, 2026-11-07)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: HUN-240 (the route) merged, or its branch merged into this task's branch by the Supervisor before this task goes In Progress

**Entry point**: the Kit view (`web/src/KitView.jsx`) → block "Kit by workspace" (`web/src/WorkspaceKit.jsx`), reached from the sidebar "Kit"

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | With two folders for WS-A in the fake answer, two lines render under WS-A: `app/reports · 12 files · 3 easy-ui sessions · newest 2026-11-05` and `app/smoke-reports · 1 file · newest …` (singular "file"; no "0 easy-ui sessions" text) | FR-089 |
| 2 | `files_capped true` → "2000+ files"; `newest_utc null` → no "newest" part; `folders []` or a workspace missing from the answer → "no report folders" | FR-089 |
| 3 | No folder line is a link, button or has a click handler: `getAllByRole('link')` and a button query inside the report lines return nothing; the text is not in a tab stop | FR-089 |
| 4 | The workspaces block and the Kit project list render while `/api/kit/reports` is pending; when it fails (HTTP 500), each row shows "report folders unavailable" and the block's own data still shows; no second `Loading` pill appears | FR-065 |
| 5 | The request is `GET /api/kit/reports` with no query string and the page token header (through `api.js`); changing the period does NOT refetch it (it is not a period figure) | FR-089 |
| 6 | A folder or repo name with markup (`<img onerror>`) renders as text (React escapes; one test with a planted string); a long folder text is ellipsised with the full text in `title` | FR-089, NFR-002 |
| 7 | Existing vitest + pytest pass; the build passes | — |

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest -q
```

Plus a real run: dashboard from the branch on a COPY of the owner store with HUN-240 merged into the branch and a temp tree with `reports/` and `smoke-reports/` folders; headless Chrome at 1280×720 and 375×812: Kit view with the lines; screenshots under `tasks/evidence-HUN-241/`.

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Kit view "Kit by workspace" with report lines, 1280×720 and 375×812 | headless screenshot BEFORE/AFTER | lines readable under each row; the rows and the project list unchanged |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours |
| Accessibility | vitest | report lines are a list inside the row; text only; the error text uses the same pattern as the block's other inline messages |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720, 375×812 | screenshot + `scrollWidth` | no horizontal page scroll; long text ellipsised with `title` |

---

## Approach

**Vital slice**: a small `useLoad('/api/kit/reports')` in `WorkspaceKit.jsx` (or its child), a lookup by workspace name, a `ReportFolders` list inside the row, tests, styles. **Cut list**: no sort, no link, no copy button, no refetch on period.

## Edge Case Checklist

- [ ] Very long folder text → ellipsis, full text in `title`
- [ ] 375 px width: lines wrap or ellipsise, no horizontal scroll
- [ ] Markup in a name never rendered as HTML (one test)
- [ ] Reports answer slower than the block's answer → the rows show first, the lines appear when they arrive
- [ ] Reports answer fails while the block's answer works (and the reverse)

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/WorkspaceKit.jsx` | the report lines and the parallel read |
| `web/src/app.css` | line styles |
| `web/src/WorkspaceKit.test.jsx` | tests |
| `web/src/test/fixtures.js` | a fake `/api/kit/reports` answer, only if the file already holds the fake answers |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | Backend is HUN-240 |
| `web/src/WorkspacesView.jsx` | other task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows filled
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-07 owner choice: show the relative path `<repo folder>/<report folder>` → relative (PRD question 31)
