# TASK_REVIEW — HUN-160: Work view rail regroup and first-run checklist

> Sibling of `tasks/TASK_GUIDE_HUN-160.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/surfaces/test_first_run.py` (10 tests: AC2/AC3 + edge cases), `web/src/FirstRun.test.jsx`, `web/src/App.test.jsx` (rail groups, badge, buttons, GET-only), `web/src/shell.test.js` (hidden group, token-only CSS), `tests/leak/harness.py` (route leak-scanned). Supervisor run 2026-10-31T15:19:47+07:00 on 1c456d9: `Tests  276 passed (276)`; `839 passed, 5 deselected, 1 xfailed, 1 warning in 163.92s (0:02:43)` |
| Verification command run | ☑ pass | `npm --prefix web test -- --run` → `Test Files  13 passed (13)` / `Tests  276 passed (276)`; `npm --prefix web run build` → `✓ built in 125ms`; `uv run pytest -q` → `839 passed, 5 deselected, 1 xfailed, 1 warning in 163.92s (0:02:43)`; `uv run ruff check src tests` → `All checks passed!` |
| Negative cases hold | ☑ pass | runtime: `POST /api/first-run -> 405`; no token `-> 403`; tests: unconfirmed accounts / orphan-only workspace / no unit file / no systemd → not done |
| verify | ☑ pass | Supervisor /verify recipe 2026-10-31T15:19:11+07:00, isolated `uv tool install --reinstall --no-cache .` in temp home on fixtures: `0.2.0`, `easy-personal core ok`, ingest ok, dashboard bound `127.0.0.1` only, `GET /api/first-run -> 200 {"steps":[{"id":"first_ingest","done":true,…},{"id":"ingest_timer","done":false,"hint":"easy-personal schedule install"},…]}` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor code-review 2026-10-31 on `main...HUN-160` (1c456d9): 11 changed files + direct callers `core/accounts.list_accounts`, `core/workspaces.list_all`, `core/schedule.{unit_dir,systemd_available,cron_snippet}`, CLI `schedule install` parser. lane_check: `lane: C2, 11 files, 450 lines, FITS; unpredicted: …` (guide predicted with `…` placeholders; no scope creep). Entry points `GROUPS`, `/api/first-run` found. Startup reads line present (spec, guide, role guide). Findings: P0 0, P1 0, P2 1 — `schedule.systemd_available()` runs `systemctl --user show-environment` with no timeout on each `/api/first-run` call while the timer is absent, and App re-reads on every view change; suggest `timeout=2` + TimeoutExpired → False (in `schedule.py`, outside this task's files; follow-up). P3 0. No safe fixes needed. |
| Full smoke suite still green (no regression) | ☑ pass | full suites above, no regression vs baseline (829 → 839 passed) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier MCP not run for this task (advisory, no hook); no report path. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Supervisor 2026-10-31, headless Chrome over CDP (easy-ui MCP not registered in this session, same method as learnings), HUN-160 1c456d9 dashboard on fixture `tests/fixtures/claude_code/2.1.302`, temp home: `reports/HUN-160/First_run_{1280,768,375}.png`. LLM verdict vs `BRAINSTORMING_LOG_WORKVIEW.md` Rail sketch — pass: rail shows `// usage`, `// kit`, `// connect`, `// system`; `// work` present but hidden (`hidden=true`, no tabs yet); First run active with badge `3 left`; 4 cards in dashboard card style, done step in accent, not-done steps with command box / Open Agents / Open Workspaces buttons. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Computed style (CDP): body `"IBM Plex Sans", system-ui, sans-serif`; code `"IBM Plex Mono", ui-monospace`; vitest `shell.test.js` asserts no raw colour literals in `.badge` / `.first-run*` rules (tokens only); console errors `[]`; request hosts `["127.0.0.1:47859"]` only (NFR-001). |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280 / 768 / 375 px: `scrollWidth <= clientWidth` true at all three (no horizontal scroll); at 375 the rail collapses behind the menu button and the cards stack full width. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, 2026-10-31T14:49:29+07:00, main de84e68, before any implementation commit):
```
$ grep -n "label: '//" web/src/App.jsx
15:    label: '// insights',
24:    label: '// setup',
$ grep -rn "first-run\|first_run" src web/src
(no output — no /api/first-run route, no first-run view)
```

**AFTER** (Supervisor, 2026-10-31T15:19:11+07:00, HUN-160 1c456d9):
```
$ grep -n "label: '//" web/src/App.jsx
  { id: 'work', label: '// work', tabs: [] },
    label: '// usage',
  { id: 'kit', label: '// kit', ...
    label: '// connect',
  { id: 'system', label: '// system', tabs: [{ id: 'first-run', label: 'First run', View: FirstRun }] },
$ curl -H 'X-Dashboard-Token: …' $BASE/api/first-run
200 {"steps":[{"id":"first_ingest","done":true,…}, … 4 steps]}
```

**DELTA**: The owner sees the views grouped into work / usage / kit / connect / system and a First run checklist with a `N left` badge that shows which setup steps are still missing and how to finish each.

**WITNESS**: Supervisor, 2026-10-31 (verify recipe + full suites run with `active_task` = HUN-160).
