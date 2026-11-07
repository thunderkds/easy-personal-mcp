**Date**: 2026-11-07
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Workspace, orphan workspace, project, kit part; Critical Constraints: metadata only, read-only sources)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-089, FR-087, FR-054, FR-057, NFR-002, NFR-013, open question 31; `src/easy_personal/core/kit.py` (`workspace_rollup`: how it resolves a workspace to repo roots), `src/easy_personal/core/workspaces.py` (`project_paths`, `list_all`), `src/easy_personal/core/memo.py`, `src/easy_personal/surfaces/api.py` (route list, token and host guard, how `/api/kit/workspaces` is wired), `tests/leak/harness.py` (`API_CALLS`), `tests/core/test_kit_workspaces.py` and `tests/surfaces/test_api_kit_workspaces.py` (the test style to copy)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-07: "you can depend on the path of projects in the workspace, and mapping the folder reports, report, or something like that to the dashboard, we just show to easy refer, not manage it in anyway". Owner choice (question): show the **relative** path, not the full path.

**Restated intent**:

> Add a read `kit.report_folders(conn)` and route `GET /api/kit/reports` (no query parameters; same token and host guard as the other `/api/*` routes; `GET` only). It returns:
>
> ```
> {"workspaces": [
>    {"name": str,
>     "folders": [
>       {"repo": <repo folder name>, "folder": <report folder name>,
>        "files": int, "files_capped": bool, "easy_ui_sessions": int, "newest_utc": str|None}, ...]}, ...]}
> ```
>
> One entry per non-orphan workspace, in the order of `workspaces.list_all` (same list as `workspace_rollup`). A workspace with no repo or no report folder has `"folders": []`.
>
> **What counts as a report folder**: a directory that sits **directly inside** a repo root of the workspace (depth 1), found with `os.scandir`, whose lowercase name contains `report` (`reports`, `report`, `smoke-reports`, `Test-Reports`). A symlink is never followed and never listed (`entry.is_dir(follow_symlinks=False)`), so a link to a folder outside the repo is skipped. A repo whose folder is gone (FR-054) is skipped. A repo in two workspaces is listed in both. At most 8 report folders per repo (sorted by name; the rest are dropped).
>
> **Per folder**: walk it (no symlink followed, `followlinks=False`) to depth 3 and count regular files, to a cap of 2000 entries: past the cap, `files` is the cap and `files_capped` is true. `easy_ui_sessions` = regular files in that walk whose name matches `session-*.json` (name only, never opened). `newest_utc` = the newest file `st_mtime` as an ISO UTC string, or null for an empty folder.
>
> **Hard rules**: no file is opened or read; nothing is written, moved or deleted; no store read or write, no migration; no absolute path, no file name and no sub-folder name appears in the response, logs or errors (only the repo folder name, the report folder name and numbers). A folder that cannot be listed (permission) is skipped, never an error for the whole call. Answers in < 2 s on a synthetic tree of 160 repos (NFR-013); add `core/memo.py` caching (short TTL) only if AC 9 fails without it.

**Out of scope**: the page (HUN-241); any MCP tool or CLI command; opening or parsing a report; ingest of easy-ui sessions per workspace; any change to `sources/`, the store or migrations; attributing easy-ui check results to a workspace.

**Requirement Refs**: FR-089, FR-087, FR-054, FR-057, NFR-002, NFR-013

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner message and owner choice, 2026-11-07)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria

---

## Dependencies & Reachability

**Depends on**: None (HUN-238 is merged; this task adds a new route and does not change it)

**Entry point**: `GET /api/kit/reports` in `src/easy_personal/surfaces/api.py`, read by HUN-241

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | WS-A has repo `app` with `reports/` (3 files, 2 named `session-*.json`) and `smoke-reports/` (1 file), and repo `lib` with no report folder; → one `folders` entry per report folder with `repo "app"`, `folder`, `files`, `easy_ui_sessions`, `newest_utc` = the newest file time; `lib` adds nothing | FR-089 |
| 2 | Name match is by lowercase `report` substring at depth 1: `Reports`, `report`, `test-reports` and `reporting-tools` match (substring rule, as the PRD says); `docs/reports/` (depth 2), `src` and a plain FILE named `reports` do not (a file is not a folder) | FR-089 |
| 3 | A symlink `reports -> /outside` is not listed and its target is never walked; a symlinked file inside a report folder is not counted | FR-089, NFR-002 |
| 4 | A workspace with no repos, a repo whose folder is gone, and an unreadable folder (chmod 000) → no error; `folders` is `[]` or skips that folder; the orphan workspace is not listed | FR-089, FR-054 |
| 5 | A folder with more than 2000 entries → `files` 2000 and `files_capped true`; an empty folder → `files 0`, `newest_utc null`; depth is limited to 3 (a file at depth 4 is not counted) | FR-089, NFR-013 |
| 6 | A repo in two workspaces appears in both; a repo with more than 8 report folders lists 8 (sorted by name) | FR-089, FR-087 |
| 7 | Route: 200 with the shape above and the token; 401/403 as the other routes; `POST` → 405; no workspaces → `{"workspaces": []}` | FR-089 |
| 8 | Leak: a canary in a report file NAME, in a report file CONTENT, in a sub-folder name, and the absolute temp path of the repo appear nowhere in the response, the logs or an error body; the response holds only `repo`, `folder`, numbers and times; add the route to `API_CALLS` in `tests/leak/harness.py` | NFR-002 |
| 9 | Speed: on a synthetic tree of 160 repos, 10 workspaces, 2 report folders each with 50 files, the read answers in < 2 s (median of 3); paste the timing | NFR-013 |
| 10 | No file is opened: a test patches `builtins.open` and `Path.read_text`/`read_bytes` to fail, and the read still works; no write: the store file hash and the tree are unchanged after the call | FR-089 |
| 11 | Existing tests pass; `uv run pytest -q` passes. Real run: temp `EASY_PERSONAL_HOME` with fixtures, `easy-personal dashboard`, `curl` the route with the token (header `X-Dashboard-Token`), paste the JSON | — |

---

## UI / Design Acceptance Criteria

N/A — no page change (HUN-241). UI Evidence rows ☐ N/A.

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_HUN-240.md`.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core tests/surfaces tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: one pure function over `workspaces` repo roots plus `os.scandir`/`os.walk` → route → tests. Reuse the way `workspace_rollup` turns a workspace into repo roots; do not write a second resolver.
**Cut list**: no recursion into a report folder's contents beyond counting, no file names, no MCP tool, no CLI, no cache unless AC 9 needs it.

---

## Edge Case Checklist

- [ ] Symlinked report folder or file → skipped
- [ ] Plain file named `reports` → not a folder
- [ ] Gone repo, unreadable folder → skipped, no error
- [ ] Orphan workspace left out; workspace with no repos → entry with `folders []`
- [ ] Cap of 2000 entries and depth 3 hold on a deep or huge folder
- [ ] Repo in two workspaces → in both
- [ ] Two repos with one folder name → two entries, same `repo` text (not merged)
- [ ] Folder names with odd characters stay plain strings (the page escapes them)
- [ ] Planted secrets (NFR-002) never in the response

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/kit.py` | `report_folders` (+ small helper) |
| `src/easy_personal/surfaces/api.py` | the route |
| `tests/core/test_kit_reports.py` (new) | AC 1–6, 9, 10 |
| `tests/surfaces/test_api_kit_reports.py` (new) | AC 7 |
| `tests/leak/harness.py`, `tests/leak/` | AC 8 |
| `README.md`, `site/` | only if a drift test requires the new route to be listed |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/sources/`, `src/easy_personal/core/migrations/`, `src/easy_personal/core/store.py` | No ingest or store change |
| `web/` | The page is HUN-241 |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows ☐ N/A
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-07 owner choice: show the relative path `<repo folder>/<report folder>`, not the full path → relative (PRD question 31)
