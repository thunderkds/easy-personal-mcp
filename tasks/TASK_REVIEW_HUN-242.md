# TASK_REVIEW — HUN-242: report files list and read-only serve (FR-090)

> Sibling of `tasks/TASK_GUIDE_HUN-242.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | tests/core/test_kit_report_files.py (AC 1-3, 4, 5, 6, 7, 10), tests/surfaces/test_api_kit_report_files.py (AC 1 route, 4 headers, 5-8), tests/leak/test_kit_report_files_leak.py (AC 9), harness API_CALLS entries; tests/core/test_kit_reports.py updated for repo_no. `uv run pytest tests/core tests/surfaces tests/leak -q`: 1272 passed, 5 deselected |
| Verification command run | ☑ pass | `uv run pytest tests/core tests/surfaces tests/leak -q`: 1272 passed, 5 deselected. `uv run pytest -q`: 1749 passed, 2 skipped, 6 deselected, 1 xfailed |
| Negative cases hold | ☑ pass | path attacks, resolution attacks, extensions, 413 / exact 10 MB, chmod 000, FIFO, symlinks, 403/405, token query param: all in the new tests, passing; real-run traversal gave 404 and no token gave 403 (see Demonstration AFTER) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | full `uv run pytest -q`: 1749 passed |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
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
