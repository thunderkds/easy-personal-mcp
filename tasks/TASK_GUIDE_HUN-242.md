**Date**: 2026-11-07
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Workspace, orphan workspace; Critical Constraints: metadata only, read-only sources)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-089, FR-090, US-036, NFR-002, NFR-008, open question 32; `src/easy_personal/core/kit.py` (`report_folders`, `_repo_report_folders`, `_is_real_dir`, `_report_stats`: reuse the folder rule, do not write a second one), `src/easy_personal/core/workspaces.py` (`project_paths`, `list_all`), `src/easy_personal/surfaces/api.py` (the `Guard` middleware and its CSP, `_query`, how `/api/kit/reports` is wired), `tests/leak/harness.py` (`API_CALLS`), `tests/core/test_kit_reports.py`, `tests/surfaces/test_api_kit_reports.py`, `tests/leak/test_kit_reports_leak.py` (test style to copy)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-07, about the report folder lines of FR-089: "should be clickable, not only show as readonly, it's useless". Owner choice (question): a click lists the files of the folder; a click on a file opens it in a new tab.

**Restated intent**:

> Three changes, all read-only.
>
> **(a)** In `kit.report_folders` / `GET /api/kit/reports`, add `repo_no` (int) to each folder entry: the 0-based place of the repo in that workspace's repo list (the same order `_repo_report_folders` walks). Nothing else in the FR-089 shape changes.
>
> **(b)** `kit.report_files(conn, workspace, repo_no, folder)` and route `GET /api/kit/report-files?workspace=<name>&repo_no=<n>&folder=<report folder name>`. It returns `{"files":[{"path","size","mtime_utc","openable"}],"truncated":bool}`: the regular files of that report folder to depth 3 (no symlink followed), newest first, at most 200 (`truncated` true when more exist). `path` is relative to the report folder, with `/` separators. `openable` is true when the extension is in the allow-list `html htm md txt json png jpg jpeg pdf` (lowercase compare) and the size is ≤ 10 MB.
>
> **(c)** `kit.report_file(conn, workspace, repo_no, folder, path)` and route `GET /api/kit/report-file?workspace=&repo_no=&folder=&path=`. It returns the bytes of one file as a `Response` with: the content type from a fixed extension table (`html`/`htm` → `text/html; charset=utf-8`, `md` → `text/markdown; charset=utf-8`, `txt` → `text/plain; charset=utf-8`, `json` → `application/json`, `png`, `jpg`/`jpeg`, `pdf`), never from the content; `X-Content-Type-Options: nosniff`; `Cache-Control: no-store`; `Content-Disposition: inline` (no file name in it).
>
> **Resolution (both b and c)**: the server finds the repo itself: non-orphan workspace named `workspace`, repo number `repo_no` of its repo list, then the report folder named `folder` under the FR-089 rule (depth 1, lowercase name contains `report`, a real directory, not a symlink; the same 8 per repo cap applies to the lookup). Any miss (unknown or orphan workspace, repo_no out of range or not an integer, repo gone, folder not a report folder) gives **404** with the fixed body `{"error":"not found"}` and no echo of the input.
>
> **File rules for (c)**: `path` must be a relative path: not empty, no leading `/`, no `\`, no NUL, no `.` or `..` component, at most 8 components and 512 characters. Walk it component by component from the report folder with `os.open(..., dir_fd=..., O_NOFOLLOW)` (or `os.lstat` on every level, plus a `realpath` containment check); a symlink at any level, a non-regular file, or a depth over 3 is a 404. Extension not in the allow-list is a 404. Size over 10 MB (10 * 1024 * 1024) is **413** with `{"error":"too large"}`. Open the file read-only with `O_NOFOLLOW`, check `fstat` is a regular file and the size, then read at most the cap + 1 bytes. Read errors (permission, vanished) are 404, never a traceback.
>
> **Hard rules**: `GET` only (other methods 405 as the other routes); the same Guard (Host, Origin, header token) as every `/api/*` route, no change to the Guard and no token in a URL; nothing is written, moved, deleted or stored; no store write, no migration; the file bytes appear only in the (c) response body; no absolute path, no file name and no sub-folder name in logs, error bodies or the (a) answer (the (b) answer carries relative paths inside the report folder: that is the point of FR-090); `uvicorn` access log stays off (it already is). The Guard's CSP header stays on the (c) response as it is.

**Out of scope**: the page (HUN-243); any MCP tool or CLI command; parsing or rendering a report; svg; writing or deleting; a one-time link route; changing the Guard; caching.

**Requirement Refs**: FR-089, FR-090, US-036, NFR-002, NFR-008

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner message and owner choice, 2026-11-07)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria

---

## Dependencies & Reachability

**Depends on**: None (HUN-240 is merged)

**Entry point**: `GET /api/kit/report-files` and `GET /api/kit/report-file` in `src/easy_personal/surfaces/api.py`, read by HUN-243

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `GET /api/kit/reports`: every folder entry has `repo_no`; two repos of one workspace with the same folder name get different `repo_no`; the rest of the shape is unchanged (existing tests still pass) | FR-089 |
| 2 | Files list: a folder with `a.html` (newest), `sub/b.md`, `sub/deep/c.json`, `x.bin` → 4 entries newest first, `path` relative with `/`, `openable` true for the first three and false for `x.bin`; a file at depth 4 is absent; 201 files → 200 listed and `truncated true` | FR-090 |
| 3 | Files list excludes a symlinked file, a symlinked sub-folder (not walked) and a non-regular file; an empty folder → `{"files":[],"truncated":false}` | FR-090 |
| 4 | File read happy path: bytes of `a.html` equal the file; `Content-Type` `text/html; charset=utf-8`; `X-Content-Type-Options: nosniff`; `Cache-Control: no-store`; `Content-Disposition` is `inline` with no file name; one test per allowed extension checks the content type | FR-090 |
| 5 | Path attacks each give 404 with the fixed body and no echo: `../x`, `a/../../x`, `/etc/passwd`, `a\\b`, a NUL byte, an empty path, `.`, 9 components, 600 characters, a symlinked file whose target is outside, a symlinked sub-folder on the path, a FIFO | FR-090, NFR-002 |
| 6 | Resolution attacks each give 404 with the fixed body: unknown workspace, orphan workspace, `repo_no` `-1` / `99` / `abc`, a folder name that is not a report folder (`src`), a folder name with `/` or `..`, a symlinked report folder, a gone repo | FR-090 |
| 7 | Extension `svg`, `py`, no extension, `.html.bak` → 404; a file over 10 MB → 413 `{"error":"too large"}`; a 10 MB file exactly → 200; `chmod 000` file → 404 | FR-090 |
| 8 | Routes: 200 with the token; 401/403 as the other routes without it, with a wrong Host or a foreign Origin; `POST` → 405; the token is not accepted from a `token` query parameter on these paths | FR-090, NFR-008 |
| 9 | Leak: a canary in a file NAME outside the opened path, in a file CONTENT that is not opened, and the absolute temp path never appear in the list answer, any error body, or the log output; the file CONTENT canary appears only in the (c) response of its own file; add both routes (with a fixture workspace) to `API_CALLS` in `tests/leak/harness.py` where the harness allows | NFR-002 |
| 10 | No write: after calls to (b) and (c) the store file hash and the file tree (names, mtimes, sizes) are unchanged | FR-090 |
| 11 | Existing tests pass; `uv run pytest -q` passes. Real run: temp `EASY_PERSONAL_HOME`, a workspace with a repo that has `reports/a.html`, `reports/sub/b.md`, `smoke-reports/s.txt`; `easy-personal dashboard`; `curl` with `X-Dashboard-Token`: list, read `a.html`, a traversal try, a no-token try; paste the real output | — |

---

## UI / Design Acceptance Criteria

N/A — no page change (HUN-243). UI Evidence rows ☐ N/A.

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_HUN-242.md`.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core tests/surfaces tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: reuse the FR-089 folder resolver; one safe-open helper (`dir_fd` + `O_NOFOLLOW`) used by both the lister and the reader; two routes; tests that attack the path. **Cut list**: no cache, no MIME sniffing, no range requests, no streaming (10 MB cap makes a plain read fine), no MCP tool, no CLI.

---

## Edge Case Checklist

- [ ] `..`, absolute path, backslash, NUL, empty, too long, too deep → 404
- [ ] Symlink at the file, at a sub-folder on the path, at the report folder → 404 / not listed
- [ ] FIFO or device file → not listed, 404 on read (never blocks the server: open with `O_NONBLOCK` or `lstat` first)
- [ ] File changes between list and read (vanished, grown past the cap) → 404 / 413, no traceback
- [ ] Two repos with one folder name → told apart by `repo_no`
- [ ] Orphan workspace, gone repo, workspace renamed → 404
- [ ] Odd file names (spaces, `#`, `?`, unicode) round-trip through the query string
- [ ] Planted secrets (NFR-002) never in logs or error bodies

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/kit.py` | `repo_no` in `report_folders`; `report_files`; `report_file`; a small safe-open helper |
| `src/easy_personal/surfaces/api.py` | the two routes |
| `tests/core/test_kit_reports.py` | `repo_no` test |
| `tests/core/test_kit_report_files.py` (new) | AC 2, 3, 4, 5, 6, 7, 10 |
| `tests/surfaces/test_api_kit_report_files.py` (new) | AC 4 (headers), 8 |
| `tests/leak/harness.py`, `tests/leak/` | AC 9 |
| `README.md`, `site/` | only if a drift test requires the new routes to be listed |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/sources/`, `src/easy_personal/core/migrations/`, `src/easy_personal/core/store.py` | No ingest or store change |
| `Guard` class in `api.py` | The token rule stays: header only |
| `web/` | The page is HUN-243 |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows ☐ N/A
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-07 owner choice: a click on a report folder lists its files; a click on a file opens it in a new tab → FR-090 (PRD question 32)
* 2026-11-08 DEBATE Should `GET /api/kit/report-file` serve a `.html` report as `text/html`, as `text/plain`, or as `text/html` plus a `Content-Security-Policy: sandbox` response header? → `text/html; charset=utf-8` from the fixed extension table, with `nosniff`, `Cache-Control: no-store` and `Content-Disposition: inline`, and no sandbox CSP on the route (Advocate 1 argued `text/html`, cited `PRD.md:181`, `tasks/TASK_GUIDE_HUN-242.md:33`; Advocate 2 argued `text/plain`, cited `tasks/TASK_GUIDE_HUN-243.md:67`, `PRD.md:190`; Advocate 3 argued `text/html` + `CSP: sandbox` and disproved its own option, cited `src/easy_personal/surfaces/api.py:98-99`) ⚠ decided for you — please review
* 2026-11-08 Fact found during that Debate: `Guard.send_csp` strips any `content-security-policy` a route sets and overwrites it with the constant `CSP` (`src/easy_personal/surfaces/api.py:98-99`), and a route response CSP never governs a `blob:` document the page builds in JavaScript. A route-level sandbox header is therefore inert; the fallback if HUN-243 AC 8 fails stays the one-time link route with its own sandbox CSP (PRD question 32).
