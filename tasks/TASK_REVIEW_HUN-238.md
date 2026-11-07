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
Paused: auto-mode check failed — 1 `DEBATED` line(s) need ≥2 Advocate `Agent` calls (description starting `Advocate`); the trace has 0

## Auto decisions

Mode: auto
1. Kept the easy-ui check results (passed, failed) overall, one `ui_checks` entry for the whole response, and showed per-workspace easy-ui use only from stored events — DEBATED: keep overall (Advocate 1: keep overall, `src/easy_personal/sources/ui.py:6` "Sessions are overall, never attributed to a project."; Advocate 2: attribute by time overlap, conceded low confidence) ⚠ decided for you — please review
2. Ran HUN-238 (backend) in parallel with HUN-207 (web): their file lists do not overlap — FACT: tasks/TASK_GUIDE_HUN-207.md:127 "| `src/easy_personal/` | backend is HUN-206 |"
3. Did not run `/verify` (only the user can) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule`
4. Did not merge, push or move the task to Done — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule`
Checks: tests `uv run pytest tests/core tests/surfaces tests/leak -q && uv run pytest -q` — 1181 passed, then 1658 passed, 2 skipped (agent run 2026-11-06T23:5xZ); reviews code-review — 0 P0, 0 P1, 0 P2, 2 P3; security-review — no finding at confidence 8 or above; easy-verifier changes score — overall 60, change_risk 100 (advisory); real run — `easy-personal dashboard` on a temp store with fixtures, `curl` of `/api/kit/workspaces` printed the JSON in the Demonstration AFTER block, 400 for an unknown account, 403 without a token (auto-checked, not /verify)
Not done: merge (yours); /verify (yours)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_kit_workspaces.py` (24 tests: AC 1-6, 9), `tests/surfaces/test_api_kit_workspaces.py` (8 tests: AC 7), `tests/leak/test_kit_workspaces_leak.py` (1 test: AC 8) + route in `API_CALLS` of `tests/leak/harness.py`. Last run of the three new files: 24 + 8 + 1 passed. AC 9 timing (`pytest -s`): `workspace_rollup, 160 projects, 10 workspaces, 6400 events: runs [0.363, 0.344, 0.333] s, median 0.344 s` (limit 2 s; no memo needed) |
| Verification command run | ☑ pass | `uv run pytest tests/core tests/surfaces tests/leak -q` -> `1181 passed, 5 deselected, 1 warning in 76.29s`; `uv run pytest -q` -> `1658 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 151.05s`; `uv run ruff check src tests` -> `All checks passed!` |
| Negative cases hold | ☑ pass | abstained and unknown-schema rows are no score (never 0); orphan not listed; `/b` vs `/bc`; `mcp__easy-uix__`/`mcp__claude_ai_easy-ui__` not counted; NULL/empty `tool_names`; unknown account -> 400 without echo; no token/wrong token/wrong Host -> 403; POST -> 405; canaries (reason_code, scope, description, email, ui `target`) and full repo paths absent from response and logs |
| verify | ☑ pass | `/verify` run by the Supervisor on 2026-11-07 (user asked for verify and merge) on branch worktree-agent-a05e748ab0a0a8cba, temp home, `uv tool install .`: `easy-personal --version` → `0.4.0`; `ping` → `easy-personal core ok`; MCP stdio initialize, tools/list, ping call → all three answered, `isError:false`; `mcp --transport http --host 0.0.0.0` → `refusing host '0.0.0.0': HTTP must bind 127.0.0.1 only`; `easy-personal dashboard` then `GET /api/kit/workspaces` with `X-Dashboard-Token` → `200 {"workspaces":[],"ui_checks":{"status":"no data"}}` (empty store, AC 7 shape); no token → `403`; `?account=nope` → `400`; POST → `405` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `core/kit.py` (new functions), the one-line route in `surfaces/api.py`, and the 4 test files; read the callers it reuses (`workspaces._under`, `owners.owner_map`, `metrics._scope/_period`, `ui_part`). Skipped the HUN-237 files that appear in the diff against the stale `origin/HEAD`. Lane check: `EXCEEDS (lines 598 > 500)` — tests are 500 of the lines. Findings: P3 `tests/surfaces/test_api_kit_workspaces.py` written after the 3-line route edit, not red first (core tests were red first: 24 failed); P3 the guide said one SELECT per workspace, the code uses one `events` query for all (same result, fewer queries). Checked and suppressed: NULL `ts_utc` cannot reach `_ui_calls` (`sources/claude_code.py:160` abstains such events). |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1658 passed, 2 skipped, 6 deselected, 1 xfailed`; README/site drift tests passed with no edit |
| **Companion score (easy-verifier)** | ☑ pass | overall 60 (advisory, no gate), change_risk 100, scope changes, ref `main...HEAD`, report: /home/hungnguyenhuu/.easy-verifier-reports/agent-a05e748ab0a0a8cba/evidence-report-changes-20261107T000023-304074Z.html |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | No UI in this task (the page is HUN-239); the guide's UI section is N/A. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No UI in this task (the page is HUN-239); the guide's UI section is N/A. |
| **UI: Responsiveness at target viewports** | ☑ N/A | No UI in this task (the page is HUN-239); the guide's UI section is N/A. |

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
