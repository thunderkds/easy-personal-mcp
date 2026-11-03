**Date**: 2026-11-03
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints: metadata only, read-only on sources, 127.0.0.1 only, token + Host/Origin on dashboard routes)
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`; this task also adds one small read-only API route, so skim `agents/backend.md` § testing and security too
5. Apply the C2 process from your role guide
6. Read `PRD.md` FR-068, FR-065; `src/easy_personal/surfaces/api.py` (`create_app`, `_query`, the token middleware); `web/src/WorkspacesView.jsx` (`WorkspaceForm`, Repos fieldset); `web/src/app.css` (`.actions`, `.group`, `.repo-row`)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-03: "the repos should be a button to open the current directory and choose the path, by the way the button Add repo with the button Save below are no space"

**Restated intent**:

> A browser cannot return an absolute path from the OS folder dialog, so the dashboard server lists folders and the page shows a folder browser.
>
> **API** — `GET /api/dirs?path=<absolute path>` (path optional). Behind the same per-launch token + Host check as every `/api/*` route. Without `path`: the dashboard process's current directory (`os.getcwd()` at request time — where `easy-personal dashboard` was started). Answer: `{"path": "<resolved absolute>", "parent": "<abs>" | null, "home": "<abs>", "dirs": [{"name": "...", "repo": true|false}]}`.
>
> * Names of sub-directories only — never file names, sizes, contents or symlink targets. Hidden entries (name starts with `.`) left out. `repo` = the sub-folder contains a `.git` entry (existence check only).
> * Sorted case-insensitively; at most 500 entries, plus `"truncated": true` when cut.
> * A relative path, a path that does not exist or is not a directory → 400 `{"error": "..."}`; a folder it cannot read → 403 `{"error": "Cannot read this folder"}`. No path echoed back in an error beyond what the client sent. Never follows into anything to write; read-only (`os.scandir`, `is_dir(follow_symlinks=True)` is fine).
> * Never logged with the path.
>
> **Page** — in `WorkspaceForm`'s Repos group:
>
> * "+ Add repo" becomes "Browse…" (opens the folder browser) next to "+ Type a path" (adds an empty text row, today's behaviour). Each existing repo row keeps its text input and ✕.
> * Folder browser: a modal `<dialog>` (focus moves in, Esc closes, focus returns to the button), title "Choose a repo folder", the current path shown (mono), an "Up" button (disabled when `parent` is null), a "Home" button, a list of sub-folders as buttons (click = go into it; git repos marked "git" badge), a "Choose this folder" primary button that adds the current path as a repo row (skip if already listed) and closes, and "Cancel". Loading uses the existing `Loading` pill pattern or an inline status inside the dialog; an error shows inline in the dialog (`role="alert"`). Phone width: dialog fits 375 px with a 16 px gutter, list scrolls inside.
> * Spacing: a visible gap (at least `--space-4`) between the Repos group's buttons and the form's Save/Back buttons.

**Out of scope**: picking files; creating folders; Add connector / agents forms; any MCP tool (tool surface frozen); remembering the last folder.

**Requirement Refs**: FR-068, US-028

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, 2026-11-03; "current directory" read as the dashboard process's working directory — stated to the owner)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: HUN-202 (merged)

**Entry point**: `Route("/api/dirs", ...)` in `src/easy_personal/surfaces/api.py`; `Browse…` button in `WorkspaceForm` (`web/src/WorkspacesView.jsx`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `GET /api/dirs` without `path` answers the process cwd; with a tmp folder holding `a/`, `B/`, `.hidden/`, `file.txt`, `r/.git/` it lists exactly `a`, `B`, `r` (case-insensitive order), `r.repo == true`, no file names | FR-068 |
| 2 | `parent` is the parent folder, `null` at `/`; `home` is the user's home | FR-068 |
| 3 | Relative path / missing path / a file → 400; unreadable folder → 403; no token or wrong Host → refused like other `/api/*` routes | FR-068 |
| 4 | More than 500 sub-folders → 500 entries and `truncated: true` | FR-068 |
| 5 | Leak test: planted file names and file contents in the listed folder never appear in the response | FR-068, NFR (metadata only) |
| 6 | Page: Browse… opens the dialog at the cwd, clicking a folder goes in, Up goes to parent, Choose this folder adds the path as a repo row (no duplicate) and closes; Cancel / Esc close and return focus | FR-068 |
| 7 | Page: "+ Type a path" still adds an empty editable row; an API error shows inline in the dialog with `role="alert"` | FR-068 |
| 8 | The Repos buttons and Save/Back have a gap ≥ `--space-4` (CSS rule test + screenshot) | FR-068 |
| 9 | Existing tests still pass | FR-065 |

---

## Evaluation & Acceptance

### Verification Command (exact, runnable)

```bash
uv run pytest -q && cd web && npx vitest run && cd .. && npm --prefix web run build
```

Plus a real run: dashboard from the branch on a COPY of the owner store, headless Chrome at 1280×720 and 375×812: open Browse…, go into a folder, choose it, see the row; screenshot.

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Workspace form with dialog open, 1280×720 and 375×812 | headless screenshot | dialog readable, list scrolls, buttons reachable |
| Workspace form bottom | screenshot BEFORE/AFTER | visible gap between Repos buttons and Save |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours |
| Accessibility | vitest + keyboard | dialog labelled, focus in/out, Esc, list buttons reachable |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720, 375×812 | screenshot + `scrollWidth` | no horizontal page scroll |

---

## Approach

**Vital slice**: `/api/dirs` (core helper in `src/easy_personal/core/` or inside `api.py` if ≤ \~40 lines) with tests → `FolderPicker` dialog in `WorkspacesView.jsx` (or `web/src/FolderPicker.jsx`) with tests → wire Browse… → spacing rule.
**Cut list**: no file picking, no folder creation, no recent-folders memory, no keyboard type-ahead in the list.

## Edge Case Checklist

- [ ] Symlink to a folder → listed as a folder; symlink loop or broken link → skipped, no crash
- [ ] Path with spaces / unicode → round-trips through the query string
- [ ] Folder with a sub-folder it cannot stat → skipped, others listed
- [ ] `/` → `parent: null`
- [ ] Planted secret/text fixtures never appear in any response, page or log

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/surfaces/api.py` | `/api/dirs` route |
| `src/easy_personal/core/` (optional new helper) | folder listing |
| `tests/surfaces/` + `tests/leak/` | route + leak tests |
| `web/src/WorkspacesView.jsx`, `web/src/FolderPicker.jsx` (new) | Browse…, dialog |
| `web/src/app.css` | dialog styles, gap above Save |
| `web/src/*.test.jsx` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/surfaces/mcp*` | Tool surface frozen |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows filled
- [ ] Supervisor notified: task ready for Stage 4 review