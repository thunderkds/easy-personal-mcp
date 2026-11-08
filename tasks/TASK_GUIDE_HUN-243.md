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
6. Read `PRD.md` FR-089, FR-090, US-036, NFR-002; `tasks/TASK_GUIDE_HUN-242.md` (the exact route shapes); `web/src/WorkspaceKit.jsx`, `web/src/WorkspaceKit.test.jsx`, `web/src/KitView.jsx`, `web/src/api.js` (how the token header is added; add a helper there if it only has JSON), `web/src/Loading.jsx`, `web/src/tokens.css`, `web/src/app.css`, `web/src/test/fixtures.js`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-07: "should be clickable, not only show as readonly, it's useless". Owner choice: a click lists the files; a click on a file opens it in a new tab.

**Restated intent**:

> **Kit view → block "Kit by workspace" → report folders.** Each report folder line (HUN-241) becomes a `<button type="button">` with `aria-expanded`. The line text stays as it is (`app/reports · 12 files · …`). Click: the line opens in place and shows the file list from `GET /api/kit/report-files?workspace=<name>&repo_no=<n>&folder=<folder>` (HUN-242); click again: it closes. Only one fetch per open (fetch on first open, keep the answer while the page stays; no refetch on close/open).
>
> **File list**: one row per file: relative `path`, size (`12 KB`), `newest`-style date `YYYY-MM-DD`. `mtime_utc` is **nullable** (a file whose time `datetime` cannot represent gives `null`, see the DRAFT-mtime-500 record in `tasks/MINIMAL_LOG.md`): render a dash for it, never `Invalid Date`, and cover it with a test. A file with `openable` true is a `<button type="button">`; a file with `openable` false is plain muted text with `title="this file type is not opened here"`. While loading: a small inline "loading files" text, no extra `Loading` pill. On failure (any non-200): one inline "files unavailable" text. Empty: "no files". `truncated` true: a last line "first 200 files shown".
>
> **Open a file**: the file button opens a blank tab **synchronously inside the click handler** (`window.open('', '_blank')`, then `win.opener = null`) so the popup blocker allows it, then fetches `GET /api/kit/report-file?workspace=&repo_no=&folder=&path=` through `api.js` with the page token header, builds a `Blob` from the response bytes with the response `Content-Type`, and sets the tab's location to `URL.createObjectURL(blob)`. Revoke the object URL after 60 s. The token is never put in a URL. If the fetch fails (404 / 413 / network), close the blank tab and show an inline "cannot open this file" next to the file row (`role="status"`). If `window.open` returns null (blocked), show "allow pop-ups to open the file".
>
> The `repo_no` comes from the `GET /api/kit/reports` entry. All query values go through `URLSearchParams`.

**Out of scope**: backend (HUN-242); rendering a report inside the dashboard; downloads; copy buttons; sorting or filtering; the Workspaces page; absolute paths; any MCP tool or CLI command.

**Requirement Refs**: FR-090, US-036, FR-089

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner message and owner choice, 2026-11-07)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: HUN-242 (the routes) merged into the base of this task's branch before it goes In Progress

**Entry point**: the Kit view (`web/src/KitView.jsx`) → block "Kit by workspace" (`web/src/WorkspaceKit.jsx`), reached from the sidebar "Kit"

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Each report folder line is a button named by its text, `aria-expanded="false"`; a click sets it `true` and requests exactly one `GET /api/kit/report-files?workspace=ws-a&repo_no=0&folder=reports` with the page token header; a second click closes it and a third opens it without a second request | FR-090 |
| 2 | The open list shows one row per file with path, size and date; an `openable` file is a button, a non-openable file is plain text with a `title` and no button | FR-090 |
| 3 | While the file list is pending the line shows "loading files" and no second `Loading` pill appears; on HTTP 500 it shows "files unavailable"; an empty answer shows "no files"; `truncated` shows "first 200 files shown" | FR-090 |
| 4 | A click on a file calls `window.open` once, synchronously in the click (before the fetch resolves), sets `opener` to null, requests `GET /api/kit/report-file?...path=...` with the token header, then sets the new tab's location to the object URL built from the response; `URL.revokeObjectURL` runs after 60 s (fake timers); no token appears in any URL | FR-090 |
| 5 | On a failed file fetch the blank tab is closed and "cannot open this file" shows beside the row; when `window.open` returns null "allow pop-ups to open the file" shows and no fetch is made | FR-090 |
| 6 | A folder, repo, file path or workspace name with markup (`<img onerror>`, `&`, `#`, `?`, spaces) renders as text and reaches the request correctly encoded (assert the exact query string); a long text is ellipsised with the full text in `title` | FR-090, NFR-002 |
| 7 | The rows and the project list of the block render while the report-files request is pending; changing the period refetches nothing for report files; keyboard: Enter/Space on the folder button and on a file button work (native button) and both show a visible focus ring from tokens | FR-090 |
| 8 | Real-run security check (headless Chrome, not jsdom): a fixture report `evil.html` holds `<script>` that sets `document.title="PWNED"`, calls `fetch("/api/workspaces")` and writes `window.opener`/`localStorage` reads to the page. After the click, the new tab's title is NOT `PWNED`, no request to `/api/*` is made from it, and the tab's `window.opener` is null. If the script runs (the blob tab does not inherit the CSP), STOP and report `Status: blocked` with `Question: [choice]` — do not ship an HTML open that can run a repo's script in the dashboard origin | FR-090, NFR-002 |
| 9 | Existing vitest + pytest pass; the build passes | — |

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest -q
```

Plus a real run: dashboard from the branch (HUN-242 merged into it) on a temp `EASY_PERSONAL_HOME` with a workspace whose repo has `reports/` (one `.html` report with inline CSS and a `.md`, one `.json`) and `smoke-reports/`; headless Chrome at 1280×720 and 375×812: Kit view with a folder open, and the opened report tab; screenshots under `tasks/evidence-HUN-243/`. In the report, state in words how the opened HTML report looks (is the inline CSS applied or not), because PRD question 32 depends on it.

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Kit view "Kit by workspace", folder closed and open, 1280×720 and 375×812 | headless screenshot BEFORE/AFTER | the folder line looks like a text row with a small chevron; the open list is indented under it; rows and project list unchanged |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours |
| Accessibility | vitest | native buttons, `aria-expanded`, list inside the row, status text with `role="status"` |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720, 375×812 | screenshot + `scrollWidth` | no horizontal page scroll; long paths ellipsised with `title` |

---

## Approach

**Vital slice**: make the HUN-241 `ReportFolders` item a disclosure button; a small `ReportFiles` child with its own `useLoad`-style fetch on first open; an `openReportFile` helper (window.open → fetch → blob → location); tests; styles. **Cut list**: no sort, no search, no download, no copy, no preview pane.

## Edge Case Checklist

- [ ] Popup blocked → message, no fetch
- [ ] Tab opened but fetch fails → tab closed, message
- [ ] Folder closed while its list is loading → no state update after unmount
- [ ] File names with `#`, `?`, `&`, spaces, unicode → encoded once
- [ ] 375 px width: no horizontal scroll
- [ ] Markup in a name never rendered as HTML (one test)
- [ ] Two clicks on one file → two tabs, no crash

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/WorkspaceKit.jsx` | folder button, `ReportFiles`, `openReportFile` |
| `web/src/api.js` | a bytes/blob GET helper with the token header, only if it has none |
| `web/src/app.css` | styles |
| `web/src/WorkspaceKit.test.jsx`, `web/src/KitView.test.jsx` | tests (the HUN-241 "no button" tests change to match FR-090) |
| `web/src/test/fixtures.js` | fake `report-files` and `report-file` answers |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | Backend is HUN-242 |
| `web/src/WorkspacesView.jsx` | other task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows filled
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-07 owner choice: a click on a report folder lists its files; a click on a file opens it in a new tab → FR-090 (PRD question 32)
