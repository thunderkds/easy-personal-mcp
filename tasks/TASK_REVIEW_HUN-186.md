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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
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

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
