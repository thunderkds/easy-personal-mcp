# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_kit_workspaces.py` (24 tests: AC 1-6, 9), `tests/surfaces/test_api_kit_workspaces.py` (8 tests: AC 7), `tests/leak/test_kit_workspaces_leak.py` (1 test: AC 8) + route in `API_CALLS` of `tests/leak/harness.py`. Last run of the three new files: 24 + 8 + 1 passed. AC 9 timing (`pytest -s`): `workspace_rollup, 160 projects, 10 workspaces, 6400 events: runs [0.363, 0.344, 0.333] s, median 0.344 s` (limit 2 s; no memo needed) |
| Verification command run | ☑ pass | `uv run pytest tests/core tests/surfaces tests/leak -q` -> `1181 passed, 5 deselected, 1 warning in 76.29s`; `uv run pytest -q` -> `1658 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 151.05s`; `uv run ruff check src tests` -> `All checks passed!` |
| Negative cases hold | ☑ pass | abstained and unknown-schema rows are no score (never 0); orphan not listed; `/b` vs `/bc`; `mcp__easy-uix__`/`mcp__claude_ai_easy-ui__` not counted; NULL/empty `tool_names`; unknown account -> 400 without echo; no token/wrong token/wrong Host -> 403; POST -> 405; canaries (reason_code, scope, description, email, ui `target`) and full repo paths absent from response and logs |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1658 passed, 2 skipped, 6 deselected, 1 xfailed`; README/site drift tests passed with no edit |
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

**BEFORE** (captured before any implementation commit; real `easy-personal dashboard` on a temp `EASY_PERSONAL_HOME` with fixtures: 3 workspaces, 4 repos, 2 verifier reports, 1 easy-ui session file, 6 transcript events; `curl` with the per-launch token):

```
# 2026-11-06T23:50:47Z  GET /api/kit/workspaces
HTTP/1.1 404 Not Found
server: uvicorn
content-length: 9
content-type: text/plain; charset=utf-8
content-security-policy: default-src 'self'; frame-ancestors 'none'

Not Found
# 2026-11-06T23:50:47Z  GET /api/kit/workspaces?account=nope
HTTP/1.1 404 Not Found
...
Not Found
# no token
HTTP/1.1 403 Forbidden
```

**AFTER** (same fixtures and command, real `easy-personal dashboard`, `curl` with the token):

```
# 2026-11-06T23:58:40Z  GET /api/kit/workspaces
HTTP/1.1 200 OK
content-type: application/json
{"workspaces":[
 {"name":"ws-a","color":"#336699","repos":3,
  "verifier":{"status":"ok","repos_scored":2,"repos_total":3,"mean":70.0,"lowest":{"project":"beta","value":60.0},"newest_utc":"2026-11-05T09:00:00.000000Z"},
  "ui":{"status":"ok","sessions":2,"calls":4,"last_utc":"2026-11-06T10:03:00.000000Z"}},
 {"name":"ws-b","color":null,"repos":1,
  "verifier":{"status":"no data","repos_scored":0,"repos_total":1,"mean":null,"lowest":null,"newest_utc":null},
  "ui":{"status":"ok","sessions":1,"calls":1,"last_utc":"2026-11-06T10:06:00.000000Z"}},
 {"name":"ws-empty","color":null,"repos":0,
  "verifier":{"status":"no data","repos_scored":0,"repos_total":0,"mean":null,"lowest":null,"newest_utc":null},
  "ui":{"status":"not used","sessions":0,"calls":0,"last_utc":null}}],
 "ui_checks":{"status":"ok","sessions":1,"passed":1,"failed":0,"other":0,"actions":2,"first_utc":"2026-11-06T09:00:00.000000Z","last_utc":"2026-11-06T09:00:00.000000Z"}}
# 2026-11-06T23:58:40Z  GET /api/kit/workspaces?account=nope
HTTP/1.1 400 Bad Request
{"error":"unknown account; valid labels: (none)"}
# no token
HTTP/1.1 403 Forbidden
```

(Fixture: ws-a = alpha 80 + beta 60 + delta without a report; ws-b = gamma, no report; 4 `mcp__easy-ui__*` calls in two ws-a sessions, one of them a sidechain; one `mcp__easy-verifier__score` call not counted.)

**DELTA**: the dashboard API now answers, per workspace, the mean and lowest easy-verifier score with the repo folder name and the easy-ui sessions, calls and last call time, while easy-ui check results stay overall.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-238.jsonl`, never the
implementing agent alone]
