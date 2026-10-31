# TASK_REVIEW — HUN-163: Work view: scheduled jobs (systemd timers + cron)

> Sibling of `tasks/TASK_GUIDE_HUN-163.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_jobs.py` (first word only, allow-list, own timer, no data per kind, read-only subcommands, timeouts, cron next run), `tests/surfaces/test_api_jobs.py` (secret in cron line + ExecStart nowhere in response; token), `tests/leak/{conftest,harness,test_leak_suite}.py` (canary in fake argv; route scanned), `tests/fakes/jobs_fakes.py`, `web/src/JobsView.test.jsx` (8), `web/src/App.test.jsx`. Supervisor run 2026-10-31T15:31:45+07:00 on 2ed8192: `Tests  285 passed (285)`; `899 passed, 5 deselected, 1 xfailed, 1 warning in 216.52s (0:03:36)` |
| Verification command run | ☑ pass | `npm --prefix web test -- --run` → `Test Files  14 passed (14)` / `Tests  285 passed (285)`; build `✓ built in 519ms`; `uv run pytest -q` → `899 passed, 5 deselected, 1 xfailed, 1 warning in 216.52s (0:03:36)`; ruff `All checks passed!` |
| Negative cases hold | ☑ pass | runtime: planted `SECRET_X_PLANTED` in a fake cron line → `SECRET in response: 0`, not in screenshots; unrelated timer → `hidden: 1`; fake argv log shows only `crontab -l`, `systemctl --user list-timers …`, `systemctl --user show -p …` |
| verify | ☑ pass | Supervisor runtime check 2026-10-31 on 2ed8192: `easy-personal dashboard` (temp home, fake `systemctl`/`crontab` first on PATH), `GET /api/jobs` → `easy-personal-ingest.timer` (systemd, `easy-personal`, `*-*-* 03:17:00`, last_exit 0) + `(crontab) claude` (`0 7 * * 1-5`, next `2026-11-02T00:00:00Z` = Mon 07:00 local), `hidden: 1`, sources ok/ok; bound 127.0.0.1 — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor code-review + inline security-review 2026-10-31 on `1c456d9...HUN-163` (2ed8192): 14 files + callers (`api.Guard`, `config.config_path`, `rules._read_flat`). Branch holds only HUN-163 files after the agent's stash repair (verified `--stat`). lane_check: `lane: C2, 14 files, 1033 lines, EXCEEDS (lines 1033 > 500)` — ~580 lines are tests/fakes. Findings: P0 0, P1 0. P2 1: default allow-list adds `easy-verifier`, `easy-context`, `easy-ui`, `easy-memory` (guide said 'kit tools', not enumerated) — owner to confirm. P3 2: `config.jobs_allow` imports private `rules._read_flat`; `sh -c "cd x && claude"` counts as hidden (documented cut). Security (Medium): single `_run` gate allows only `systemctl --user list-timers|show` and `crontab -l`, 3 s timeout, stdin DEVNULL, unit names validated against an option-safe regex; only the command basename leaves the module; route behind `Guard` (token test 403). |
| Full smoke suite still green (no regression) | ☑ pass | baseline 839 → 899 passed; web 276 → 285 |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier MCP not run for this task (advisory, no hook); no report path. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | `reports/HUN-163/Scheduled_{1280,768,375}.png` (CDP, fake tools). Verdict vs sketch: pass — `// work › Scheduled`, table Job / Kind / Schedule / Next run / Last run / Last exit, `✓ 0`, cron `not recorded`, footer `1 other job hidden · /loop jobs and cloud routines are not included`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | no new CSS (reuses `table-wrap table-frame`, `muted`, `error`); Plex Sans / Plex Mono computed; console errors `[]`; request hosts only `127.0.0.1` |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280/768/375: no horizontal page scroll; at 375 the table scrolls inside its frame (guide: wide content scrolls in its box) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, 2026-10-31T15:05:19+07:00, main a491d1d, before any implementation commit):
```
$ ls src/easy_personal/core/jobs.py
ls: cannot access 'src/easy_personal/core/jobs.py': No such file or directory
$ grep -nE "/api/(board|jobs|memory-graph)" src/easy_personal/surfaces/api.py
(no output — no /api/jobs route)
$ ls web/src | grep -E "Board|Jobs|Memory"
(no output — no JobsView)
```

**AFTER** (Supervisor, 2026-10-31, HUN-163 2ed8192):
```
$ ls src/easy_personal/core/jobs.py
src/easy_personal/core/jobs.py
$ curl -H 'X-Dashboard-Token: …' $BASE/api/jobs   (fake systemctl/crontab)
{"jobs": [{"name": "easy-personal-ingest.timer", "kind": "systemd", "command": "easy-personal", ...}, {"name": "(crontab) claude", "kind": "cron", ...}], "hidden": 1, "sources": {"systemd": "ok", "cron": "ok"}}
```

**DELTA**: The owner sees their agent-related timers and cron jobs with next run, last run and last result, without any command arguments shown.

**WITNESS**: Supervisor, 2026-10-31 (runtime check + full suites with `active_task` = HUN-163).
