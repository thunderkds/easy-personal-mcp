# TASK_REVIEW — HUN-242: report files list and read-only serve (FR-090)

> Sibling of `tasks/TASK_GUIDE_HUN-242.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. `GET /api/kit/report-file` serves a `.html` report as `text/html; charset=utf-8` from the fixed extension table, with `nosniff`, `Cache-Control: no-store` and `Content-Disposition: inline`, and no sandbox CSP on the route — DEBATED: text/html (Advocate 1: `text/html` as the guide's table says; Advocate 2: `text/plain` so no script can ever parse; Advocate 3: `text/html` plus `Content-Security-Policy: sandbox`) ⚠ decided for you — please review
2. Advocate 3's option was eliminated on fact, not preference: the Guard strips any content-security-policy a route sets and overwrites it with its own constant, so the route cannot emit sandbox without the Guard change this task forbids — FACT: src/easy_personal/surfaces/api.py:95 "async def send_csp(message):" — that middleware rebuilds every response's headers, dropping any content-security-policy the route set and appending the constant CSP (lines 98-99)
3. The implementer kept the task after the step-limit hook stopped it once, instead of the task pausing — RULE: `docs/claude-md/tracks.md` `### Stops that stay` (it pauses on "the same sub-agent hitting the step limit twice", not the first)
4. The implementer was told not to run code-review itself; Stage 4 ran at Supervisor level — RULE: `AGENTS.md` § 5-Stage Agentic Pipeline (Stage 4 is the Supervisor's stage)
5. The P1 startup-reads finding (the implementer skipped two of the three Permanent-Rule reads) is recorded, not fixed — RULE: `.claude/skills/code-review/SKILL.md` § Phase 4 — Apply Safe Fixes (it applies code fixes that are unambiguous and safe, and a process finding has none). Mitigation checked — FACT: agents/backend.md:92 "plan before coding" is the C2 process, with brainstorming only when more than one approach is viable, so none was owed here, and code-review and verify are not the implementer's to run
6. The two P3 findings were left unapplied — RULE: `.claude/skills/code-review/SKILL.md` § Phase 4 — Apply Safe Fixes (Do not apply P2/P3 fixes automatically)
7. The companion score row is N/A — RULE: `tasks/TASK_REVIEW_HUN-242.md` § Evidence (the row's own rule: N/A with a reason when neither companion tool was run); the easy-verifier MCP server is not loaded in this session
8. HUN-243 was not started this run — FACT: tasks/TASK_GUIDE_HUN-243.md:50 "**Depends on**: HUN-242 (the routes) merged into the base of this task's branch before it goes In Progress", and a merge is never done in auto mode — RULE: `docs/claude-md/tracks.md` § Never in auto mode
Paused: partly — the board log could not be written. `python3 scripts/board_sync.py push comment` and `decide` both failed with "LINEAR_API_KEY is not set", a secret the Supervisor does not have and that is never stored in a file. The Debate is logged in this file and under the guide's `## Questions Log` instead; the board comment and the Decision Log entry are still owed. Nothing else paused: the step-limit stop was the first one, so it did not pause the task.
Checks: tests `uv run pytest tests/core tests/surfaces tests/leak -q` — 1272 passed, 5 deselected, and `uv run pytest -q` — 1749 passed, 2 skipped, 6 deselected, 1 xfailed; reviews `code-review` — 0 P0, 0 P1 code findings, 1 P1 process finding, 2 P3, 0 fixes applied; `security-review` — "No findings at or above the reporting bar", with the auth channel, the `O_NOFOLLOW` walk, the extension allow-list and the error/log paths each verified sound; real run `scratchpad/realrun.sh` under `timeout 100` — printed `repo_no` on each folder entry, `a.html` and `sub/b.md` in the file list, `200` with `content-type: text/html; charset=utf-8`, `x-content-type-options: nosniff`, `cache-control: no-store`, `content-disposition: inline` for the read, `404 {"error":"not found"}` for the traversal try and `403` with no token (auto-checked, not /verify)
Reports: `code-review` → report: `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/reports/code-review_HUN-242_20261108T064527.html` (Risk 11%, Quality 90%, Effort 12%, Healthy); `security-review` → report: `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/reports/security-review_HUN-242_20261108T064605.html` (Risk 6%, Quality 94%, Effort 15%, Healthy). `reports/` is gitignored, so both stay on this machine.
Not done: merge (yours); /verify: ☑ pass — user-invoked `/verify HUN-242`, run against head 78fe629 on 2026-11-07T23:57Z. It supersedes the hook's earlier ☐ blocked run against 9c83a77, which could not reach the worktree (easy-personal-mcp-wt/ is not an allowed directory for a headless session) and had $0.71 of its $2 budget left. All written Acceptance Criteria pass; one pre-existing P1 defect was found outside them and filed as tasks/TASK_GUIDE_HUN-248.md.

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | tests/core/test_kit_report_files.py (AC 1-3, 4, 5, 6, 7, 10), tests/surfaces/test_api_kit_report_files.py (AC 1 route, 4 headers, 5-8), tests/leak/test_kit_report_files_leak.py (AC 9), harness API_CALLS entries; tests/core/test_kit_reports.py updated for repo_no. `uv run pytest tests/core tests/surfaces tests/leak -q`: 1272 passed, 5 deselected |
| Verification command run | ☑ pass | `uv run pytest tests/core tests/surfaces tests/leak -q`: 1272 passed, 5 deselected. `uv run pytest -q`: 1749 passed, 2 skipped, 6 deselected, 1 xfailed |
| Negative cases hold | ☑ pass | path attacks, resolution attacks, extensions, 413 / exact 10 MB, chmod 000, FIFO, symlinks, 403/405, token query param: all in the new tests, passing; real-run traversal gave 404 and no token gave 403 (see Demonstration AFTER) |
| verify | ☑ pass | user-invoked `/verify HUN-242` (not the hook), run by the Supervisor in the task worktree against head 78fe629 on 2026-11-07T23:57Z. Recipe: `uv tool install` into a temp dir, `--version` 0.4.0, `ping` "easy-personal core ok", MCP stdio handshake (ids 1/2/3 answered, 12 tools listed, stdout protocol-only), NFR-001 `--host 0.0.0.0|localhost|::1` all exit 2. MCP HTTP skipped: the recipe marks it opt-in and this task adds no MCP surface. Live route checks on a temp `EASY_PERSONAL_HOME` with a fixture workspace: AC 1 `/api/kit/reports` 200 with `repo_no` on every folder entry; AC 2 `report-files` 200 `{"files":[{"path":"a.html","size":12,"openable":true,...},{"path":"sub/b.md",...}],"truncated":false}`; AC 3 a symlinked file not listed and a symlinked directory not descended; AC 4 `a.html` 200 with `content-type: text/html; charset=utf-8`, `x-content-type-options: nosniff`, `cache-control: no-store`, `content-disposition: inline`, Guard CSP intact; AC 5/6/7 all twelve attacks 404 with the fixed body `{"error":"not found"}` (`../`, `/etc/passwd`, `a\b`, `.`, a symlinked file, a symlinked sub-folder, `x.bin`, `a.html.bak`, unknown workspace, `repo_no` 99 and `abc`, a non-report folder `src`); AC 8 no token 403, token in the query only 403, foreign Origin 403, wrong Host 403, POST 405; AC 10 no store write. No absolute path in any response body. **pass** on every written Acceptance Criterion. One defect found outside the ACs and filed as P1 in tasks/TASK_GUIDE_HUN-248.md: a file with an out-of-range `st_mtime` makes both `/api/kit/reports` and `/api/kit/report-files` answer 500 with a traceback. It is NOT a regression from this task — `main` at 75a2d08 answers 500 on `/api/kit/reports` for the same fixture, so the flaw is in the merged FR-089 code and this task repeats it. Recommend fixing before merge: it is a small change and one odd file breaks a whole workspace's Kit block. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed: the 9 files of `git diff main...HEAD` (`src/easy_personal/core/kit.py`, `src/easy_personal/surfaces/api.py`, `tests/leak/harness.py`, 3 new test files, 2 updated test files) plus the direct callers of the changed functions — `_query` and the `Guard` class in `api.py` (read, not changed, to confirm the new routes match the existing endpoint shape and the token rule), and `workspaces.list_all` (read, the only source of a repo path). Skipped: the rest of the repo, `web/` and the other sources, because nothing outside the diff calls `report_files`, `report_file` or `_report_names` — `grep -rn "report_files\|report_file\|_report_names\|ReportNotFound\|ReportTooLarge" --include="*.py" --include="*.js" --include="*.jsx" src web tests scripts` returns, outside `kit.py`, `api.py` and the three new test files, only `tests/leak/harness.py:122` — which is itself in the reviewed set, so nothing outside the reviewed set calls the new code. `scripts/lane_check.py`: `lane: C2, 8 files, 604 lines, EXCEEDS (lines 604 > 500); unpredicted: tests/surfaces/test_api_kit_reports.py` — warn-only; 416 of the 604 lines are the new tests, and the unpredicted file is the one-line `repo_no` assertion AC 1 itself requires, so the C2 label stands |
| Full smoke suite still green (no regression) | ☑ pass | full `uv run pytest -q`: 1749 passed |
| **Companion score (easy-verifier)** | ☑ N/A | The easy-verifier MCP server is not loaded in this session (no `score` tool is available; the server at `127.0.0.1:8766` was not reachable), so no scoring session ran. Per the row's own rule the merge gate blocks on ☑ fail only, never on an N/A row. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no page change (HUN-243) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no page change (HUN-243) |
| **UI: Responsiveness at target viewports** | ☑ N/A | no page change (HUN-243) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: real run (AC 11 script: temp EASY_PERSONAL_HOME, workspace acme with repo app having reports/a.html, reports/sub/b.md, smoke-reports/s.txt; `easy-personal dashboard`; curl with X-Dashboard-Token), captured before any implementation commit:

```
date: 2026-11-07T23:17:49Z
--- list folders
HTTP/1.1 200 OK
{"workspaces":[{"name":"acme","folders":[{"repo":"app","folder":"reports","files":2,"files_capped":false,"easy_ui_sessions":0,"newest_utc":"2026-11-07T23:17:48Z"},{"repo":"app","folder":"smoke-reports","files":1,"files_capped":false,"easy_ui_sessions":0,"newest_utc":"2026-11-07T23:17:48Z"}]}]}--- list files
HTTP/1.1 404 Not Found
Not Found--- read a.html
HTTP/1.1 404 Not Found
date: Sat, 07 Nov 2026 23:17:49 GMT
server: uvicorn
content-length: 9
content-type: text/plain; charset=utf-8
content-security-policy: default-src 'self'; frame-ancestors 'none'

Not Found
--- traversal
HTTP/1.1 404 Not Found
Not Found--- no token
HTTP/1.1 403 Forbidden
{"error":"forbidden"}```

(`/api/kit/reports` has no `repo_no`; both new routes are 404.)

**AFTER**: same script, post-change:

```
date: 2026-11-07T23:32:12Z
--- list folders
HTTP/1.1 200 OK
{"workspaces":[{"name":"acme","folders":[{"repo":"app","repo_no":0,"folder":"reports","files":2,"files_capped":false,"easy_ui_sessions":0,"newest_utc":"2026-11-07T23:32:11Z"},{"repo":"app","repo_no":0,"folder":"smoke-reports","files":1,"files_capped":false,"easy_ui_sessions":0,"newest_utc":"2026-11-07T23:32:11Z"}]}]}--- list files
HTTP/1.1 200 OK
{"files":[{"path":"a.html","size":12,"openable":true,"mtime_utc":"2026-11-07T23:32:11Z"},{"path":"sub/b.md","size":4,"openable":true,"mtime_utc":"2026-11-07T23:32:11Z"}],"truncated":false}--- read a.html
HTTP/1.1 200 OK
date: Sat, 07 Nov 2026 23:32:12 GMT
server: uvicorn
content-type: text/html; charset=utf-8
x-content-type-options: nosniff
cache-control: no-store
content-disposition: inline
content-length: 12
content-security-policy: default-src 'self'; frame-ancestors 'none'

<h1>hi</h1>

--- traversal
HTTP/1.1 404 Not Found
{"error":"not found"}--- no token
HTTP/1.1 403 Forbidden
{"error":"forbidden"}```

**DELTA**: A user can list the files of a report folder and open one file in a browser tab, with every path attack answered by a fixed 404.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
