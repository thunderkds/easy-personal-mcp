# TASK_REVIEW — HUN-240: [Short Title]

> Sibling of `tasks/TASK_GUIDE_HUN-240.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_kit_reports.py` (AC 1-6, 9, 10: 10 tests), `tests/surfaces/test_api_kit_reports.py` (AC 7: 5 tests), `tests/leak/test_kit_reports_leak.py` (AC 8: 1 test) + route added to `API_CALLS` in `tests/leak/harness.py`. New tests: 16 (1181 -> 1197 in the verification command). AC 9 timing, `pytest -s`: `report_folders on 160 repos / 10 workspaces / 2 folders x 50 files: [0.0324, 0.0321, 0.0320] median 0.0321s` (limit 2 s; no memo needed) |
| Verification command run | ☑ pass | 2026-11-07T08:10:58Z `uv run pytest tests/core tests/surfaces tests/leak -q` -> `1197 passed, 5 deselected, 1 warning in 74.52s`; then `uv run pytest -q` -> `1674 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 154.64s (0:02:34)` |
| Negative cases hold | ☑ pass | symlinked folder/file/dir not listed or counted; plain file named `reports` ignored; depth-2 `docs/reports` ignored; gone repo, chmod 000 folder, empty workspace -> no error; 2001 files -> 2000 + capped; depth-4 file not counted; no token/wrong token/wrong Host -> 403; POST/PUT/PATCH/DELETE -> 405; `builtins.open`, `os.open`, `Path.open/read_text/read_bytes` patched to fail and the read still works; tree and store hash unchanged |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff main...HEAD`: `core/kit.py`, `surfaces/api.py`, `tests/leak/harness.py` and the 3 new test files (7 files, 394 lines). `python3 scripts/lane_check.py` -> `lane: C2, 6 files, 394 lines, FITS`. Direct caller of `report_folders`: only the new route. Not reviewed: `workspaces.list_all` (unchanged). Stage 4 `code-review` (4 always-on personas + security, adversarial, over-engineering): 0 P0, 1 P1, 0 P2, 2 P3 (cost of up to 2000 `stat` calls per folder on every request, no cache; repo-name basename relies on normalised repo paths). The P1: `_report_stats` held one open `scandir` handle per sub-folder, so a report folder with many sub-folders could use up the process file descriptors and the `OSError` guard hid it as an undercount. Fixed in `8efbe99`; new test `test_many_subfolders_never_exhaust_file_descriptors` ran red (103 of 300 files counted) before the fix and green after. After the fix: `17 passed` in the 3 HUN-240 test files. Startup-reads check: the report line names PROJECT_SPEC.md, the guide and `agents/backend.md` -> no finding. `security-review` (Risk Medium): no finding at confidence >= 8 (token and Host guard as the other routes, GET only, no query input, directory roots only from stored repos, `follow_symlinks=False` everywhere, nothing opened, only names, counts and times returned, `OSError` swallowed with no text to the client). |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1674 passed, 2 skipped, 6 deselected, 1 xfailed` (above) |
| **Companion score (easy-verifier)** | ☑ pass | `overall` 56 (advisory floor 80, not met), `change_risk` 100 (value as the tool returned it, change kind confirmed as `feature`: the tool guessed `bugfix` from the review-fix commit subject), scope `changes`, ref `main...HEAD`. The low dimensions (security 18, reliability 36, blast-radius 30) come from rules that could not measure this change (`no number given — achieved coverage is below the declared floor`; one rating had 35% of rule weight measured, 50% needed); no finding was raised by the tool. The advisory score does not block. report: `/home/hungnguyenhuu/.easy-verifier-reports/agent-a5d68d8dab3c0e6e7/evidence-report-changes-20261107T083009-651260Z.html` |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no page change (HUN-241) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no page change (HUN-241) |
| **UI: Responsiveness at target viewports** | ☑ N/A | no page change (HUN-241) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-07, before any implementation commit; temp `EASY_PERSONAL_HOME`, fixtures: workspace `acme` with repos `app` (has `reports/`, `smoke-reports/`) and `lib`; `easy-personal dashboard` on a free port):

```
== 2026-11-07T07:56:40Z GET /api/kit/reports   (X-Dashboard-Token sent)
HTTP/1.1 404 Not Found
server: uvicorn
content-length: 9
content-type: text/plain; charset=utf-8
content-security-policy: default-src 'self'; frame-ancestors 'none'

Not Found
== POST -> 404
== no token -> 403
```

Baseline `uv run pytest tests/core tests/surfaces tests/leak -q` at 2026-11-07T07:59:38Z:

```
1181 passed, 5 deselected, 1 warning in 89.97s (0:01:29)
```

**AFTER** (2026-11-07T08:14:54Z, same fixtures, real `easy-personal dashboard`, temp `EASY_PERSONAL_HOME`; `app/reports` holds `session-1.json`, `session-2.json`, `a.html` and an empty sub-folder; `app/smoke-reports` holds one file; `lib` has no report folder):

```
== 2026-11-07T08:14:54Z GET /api/kit/reports
HTTP/1.1 200 OK
content-type: application/json
content-security-policy: default-src 'self'; frame-ancestors 'none'

{"workspaces":[{"name":"acme","folders":[{"repo":"app","folder":"reports","files":3,"files_capped":false,"easy_ui_sessions":2,"newest_utc":"2026-11-07T08:14:54Z"},{"repo":"app","folder":"smoke-reports","files":1,"files_capped":false,"easy_ui_sessions":0,"newest_utc":"2026-11-07T08:14:54Z"}]},{"name":"empty","folders":[]}]}
== POST -> 405
== no token -> 403
```

**DELTA**: `GET /api/kit/reports` now returns, per workspace, the report folders of its repos (repo folder name, report folder name, file count, easy-ui session file count, newest time) with no path or file name.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
