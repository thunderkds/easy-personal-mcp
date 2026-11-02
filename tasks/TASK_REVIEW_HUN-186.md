# TASK_REVIEW — HUN-186: [Short Title]

> Sibling of `tasks/TASK_GUIDE_HUN-186.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/surfaces/test_dashboard_speed.py` (17 tests: golden equality AC4, gone x3 lists AC5, gone history counts AC6, sessions detail/size AC3, no per-project git or trace scan, lighter token queries equal the full ones), fixture `tests/surfaces/hun186_fixture.py`, golden `tests/surfaces/hun186_golden.json` (captured from main de96ca4); updated for the new shape: `tests/core/test_board.py`, `tests/surfaces/test_api.py`, `test_api_board.py`, `test_api_memory_graph.py`, `test_scope_warnings.py` |
| Verification command run | ☑ pass | `uv run pytest -q && uv run ruff check src tests` -> `1138 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 258.33s (0:04:18)` then `All checks passed!` (leak suite included) |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | same run as above; one earlier full run had `tests/e2e/test_workspaces_flow.py::test_nothing_outside_the_temp_machine_is_touched[cli]` fail while other agents used the real ~/.easy-personal (its guard reports a change of the real store); it passed alone and in the final full run |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-02 15:21-15:26 local, BEFORE any implementation commit; main code de96ca4; owner store COPY with schema v7, 370k events, 161 stored projects, 330 MB; dashboard app run from this worktree against the copy, requests by `curl -w '%{time_total} %{size_download}'` with the `x-dashboard-token` header, 3 runs each; `uptime` before the run: `15:22:06 up 7:04, load average: 4.11, 13.52, 21.18`; after: `15:23:22 ... load average: 6.66, 12.23, 20.13`; the 7-day usage requests ran at `15:25:36 ... load average: 11.76, 11.53, 18.76`). Load average is not "normal" but far below the ~60 of the Supervisor's measure, so these numbers are a LOWER bound.

| Route (as the views call it) | run 1 / 2 / 3 (s) | median (s) | bytes |
|---|---|---|---|
| /api/board | 0.30 / 0.34 / 0.44 | 0.34 | 133,370 |
| /api/kit | 7.78 / 8.66 / 7.09 | **7.78** | 98,424 |
| /api/memory-graph (list) | 0.36 / 0.43 / 0.39 | 0.39 | 7,545 |
| /api/warnings | 2.81 / 2.94 / 2.43 | **2.81** | 1,524 |
| /api/plan | 3.48 / 3.64 / 3.41 | **3.48** | 1,310 |
| /api/usage/summary (no since = all time) | 1.21 / 1.08 / 1.18 | 1.18 | 648 |
| /api/usage/summary?since=7d | 1.29 / 2.05 / 0.76 | 1.29 | 479 |
| /api/usage/summary previous 7d | 1.26 / 0.68 / 0.35 | 0.68 | 575 |
| /api/usage/outliers (all time) | 1.35 / 1.37 / 1.35 | 1.35 | 20,980 |
| /api/usage/outliers?since=7d | 0.62 / 0.60 / 0.57 | 0.60 | 9,885 |
| /api/usage/sessions (no since = all time) | 8.58 / 7.22 / 7.11 | **7.22** | **7,670,516** |
| /api/usage/sessions?since=7d (the page default) | 3.78 / 3.09 / 2.41 | **3.09** | **3,785,348** |
| /api/usage/breakdown?by=day / model / project / tool, 7d | day 0.63/0.94/0.79; model 0.36/0.33/0.34; project 0.57/0.79/0.89; tool 0.58/0.60/0.36 | 0.79 / 0.34 / 0.79 / 0.58 | 2,970 / 1,948 / 32,436 / 23,291 |
| /api/first-run, /api/accounts, /api/workspaces | all < 0.3 | - | - |

Profile breakdown (cProfile in-process on the same copy, `profile_before.txt`, load ~6.5):
- kit 7.70 s: 646 SQL executes = 4.73 s (`_kernel` = 2 queries x 161 projects = 4.42 s: full scan of `kit_trace_events` (44k rows, no `project_path` index) for every project); `git_part` = 161 `git log` subprocesses = 2.84 s (134 of the 161 folders do not exist); `scoped_projects` `SELECT DISTINCT project_path` = 0.40 s.
- usage/sessions 5.79 s (profile): `_parse_utc` (`datetime.strptime`) called 216,152 times for the idle-gap check = 4.81 s; SQL only 0.25 s. Size: 349 sessions / 21,269 points in 7 days; the page draws the newest 50 only (`MAX_SESSIONS` in `web/src/Timeline.jsx`): their points are 290 KB of the 3.79 MB; the other 299 sessions' points are 3.62 MB and gaps 12 KB, and the page never reads them (it reads their totals only).
- warnings 3.65 s / plan 6.91 s (same `rules.evaluate`; plan ran at higher load): 17 SQL executes = 1.85 s; `usage_breakdown` called 3 times = 2.61 s (`tokens_jump` runs it by day twice just to read `total`: 1.73 s, of which `strptime` in `_keys` for 28,026 messages = 0.64 s); the same 7-day message query is run by 4 rules.
- memory-graph list 0.84 s: `scoped_projects` 0.44 s, easy-memory note counts 0.13 s.
- board 0.50 s: `scoped_projects` 0.44 s; board files 0.05 s.
- usage/summary 0.84 s and outliers 0.89 s: SQL only (event scans), no Python hot spot.

**AFTER** (captured 2026-11-02 15:48 local on the same store COPY and the same harness; code = commit 0549c01; `uptime` before: `15:48:16 up 7:31, load average: 4.90, 6.81, 10.24`, after: `15:48:46 ... load average: 5.38, 6.76, 10.11`; no migration, schema unchanged at v7):

| Route (as the views call it) | run 1 / 2 / 3 (s) | median (s) BEFORE -> AFTER | bytes BEFORE -> AFTER |
|---|---|---|---|
| /api/board | 0.26 / 0.25 / 0.30 | 0.34 -> **0.26** | 133,370 -> 121,207 (134 gone projects dropped, `gone` added) |
| /api/kit | 0.71 / 0.65 / 0.72 | 7.78 -> **0.71** | 98,424 -> 73,741 |
| /api/memory-graph (list) | 0.44 / 0.49 / 0.51 | 0.39 -> **0.49** (see note) | 7,545 -> 5,624 |
| /api/warnings | 1.38 / 1.30 / 1.27 | 2.81 -> **1.30** | 1,524 -> 1,526 |
| /api/plan | 1.21 / 1.39 / 1.32 | 3.48 -> **1.32** | 1,310 -> 1,310 |
| /api/usage/summary (all time) | 0.68 / 0.59 / 0.59 | 1.18 -> **0.59** | 648 |
| /api/usage/summary?since=7d | 0.44 / 0.44 / 0.42 | 1.29 -> **0.44** | 479 |
| /api/usage/outliers (all time) | 0.81 / 1.01 / 0.89 | 1.35 -> **0.89** | 20,980 |
| /api/usage/outliers?since=7d | 0.43 / 0.44 / 0.44 | 0.60 -> **0.44** | 9,885 |
| /api/usage/sessions (all time) | 1.13 / 1.18 / 1.03 | 7.22 -> **1.13** | 7,670,516 -> 472,618 |
| /api/usage/sessions?since=7d (page default) | 0.50 / 0.52 / 0.51 | 3.09 -> **0.51** | 3,785,348 -> **389,062** (< 1 MB) |
| /api/usage/breakdown by day / model / project / tool, 7d | day 0.43/0.43/0.54; model 0.34/0.31/0.35; project 0.33/0.31/0.32; tool 0.31/0.31/0.34 | 0.79 / 0.34 / 0.79 / 0.58 -> 0.43 / 0.34 / 0.32 / 0.31 | same |

Note: memory-graph list is not faster (0.39 BEFORE was measured at load 4; AFTER at load 5-9 it ran 0.44-0.86). Its cost is the `SELECT DISTINCT project_path` scan (0.26-0.28 s) plus easy-memory folder reads; it was already far below 2 s, so no fix was made.
Also measured at load 8-9 (other agents running tests, 15:45): warnings 2.14-2.85, plan 2.04-2.49, kit 1.66-2.14, memory-graph 0.85, sessions all-time 1.1-1.6: the same code is above 2 s for warnings and plan when the load average is above ~8. Every other route stays below 2 s at that load.
Gone counts on the copy: `/api/kit` gone 133 (28 listed), `/api/board` gone 121 (28 listed; worktree paths are skipped before counting, as before), `/api/memory-graph` gone 24 (31 listed; counts only gone projects that had memory notes).

**DELTA**: Every dashboard read opens in under 2 s on the owner's store copy at load average ~5 (kit 7.8 s -> 0.7 s, warnings 2.8 s -> 1.3 s, plan 3.5 s -> 1.3 s, sessions 3.1 s / 3.8 MB -> 0.5 s / 0.39 MB), and the Board, Kit and Memory lists no longer show the 133 projects whose folder is gone, each with a `gone` count.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
