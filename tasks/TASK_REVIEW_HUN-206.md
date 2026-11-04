# TASK_REVIEW — HUN-206: Scratch row + unassigned-repos read route

> Sibling of `tasks/TASK_GUIDE_HUN-206.md`. Everything here is **filled by the reviewer at Stage
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

## Auto decisions

Mode: auto
Auto mode began after Stage 4 (owner, 2026-11-04: "you can run [auto]"); Stages 0.5–4 ran with the owner's answers.
1. Stage 4 security review done by hand, not by the built-in skill — FACT: the built-in `security-review` failed with `fatal: ambiguous argument 'origin/HEAD...': unknown revision or path not in the working tree.` (this repo's only remote is `github`), as recorded in `memory/MEMORY.md` "built-in security-review needs an `origin` remote"
2. P2/P3 findings left as suggestions, not fixed — RULE: `skills/code-review/SKILL.md` § Phase 4 "Do not apply P2/P3 fixes automatically"
3. No Ready for Review move and no auto `/verify` — FACT: the Linear team's statuses are Backlog, Todo, In Progress, Done, Canceled, Duplicate (no review state; `.claude/board.json` `status_map` is `{}`), and the auto-verify hook acts only on a `### Ready for Review` row (`docs/claude-md/tracks.md` `### Auto verify (Stop hook)` condition 1); the row stays In Progress
4. The run stops after HUN-206; HUN-207 (needs HUN-206 merged) stays in Todo until the owner merges — DEBATED: stop after HUN-206 (Advocate 1: stack HUN-207 on the HUN-206 branch; Advocate 2: stop after HUN-206) ⚠ decided for you — please review
Checks: tests `uv run pytest -q` — `1352 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning`; reviews code-review — P0 0 / P1 0 / P2 2 / P3 1, security (manual) — no finding, easy-verifier overall 56 / change_risk 100; real run `uv run easy-personal usage --by project` on a store COPY — printed 13 groups (was 127), Scratch 115 folders, totals identical; speed 1.37 s (main) vs 1.38 s (HUN-206) (auto-checked, not /verify)
Not done: merge (yours); HUN-207 (after the merge); /verify (yours) — the board has no Ready for Review state, so the auto-verify hook cannot fire

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_scratch.py` (AC 1-6; 15 tests), `tests/surfaces/test_api_unassigned.py` (AC 6-7; 4 tests), `tests/test_readme.py::test_scratch_group_is_documented_and_in_the_mcp_description` (AC 9), `tests/leak/harness.py` `API_CALLS` entry for `/api/workspaces/unassigned` (AC 8, run by `tests/leak/`). `uv run pytest -q tests/core/test_scratch.py tests/surfaces/test_api_unassigned.py` -> `19 passed` |
| Verification command run | ☑ pass | `uv run pytest -q` -> `1352 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 243.55s (0:04:03)` (2026-11-04, worktree HUN-206, commit after `HUN-206: existing tests ...`). Real run on the store COPY: see Demonstration AFTER. |
| Negative cases hold | ☑ pass | no/wrong token and wrong Host -> 403 on the new route (`test_route_refuses_no_token_wrong_token_and_wrong_host`; also real server: curl without token -> 403); `/…/repo` does not cover `/…/repo2`; relative / empty / NULL stored path -> Scratch / `(unknown)`, no crash; a folder under the temp folder runs no git call (`test_ac4_...`); `by=day/model/tool` outputs byte-identical to the pre-change code on the store copy (`cmp`). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor `code-review` 2026-11-04: `git diff main...HEAD` (22 files) + direct callers of `owner_map`/`tokens_by_project`/`usage_breakdown` (`rules._top_project_share`, `workspaces.project_paths`, `kit.py`, `kitperf.py` — unchanged, still `owner_map`). Lane: `C2, 21 files, 480 lines, EXCEEDS (files 21 > 14); unpredicted: tests/conftest.py, tests/test_readme.py` — the extra files are test opt-ins, not product code. Findings: P0 0, P1 0, P2 2, P3 1 (below). Security (Medium risk, manual — built-in `security-review` needs an `origin` remote): new route is GET-only through `_query` (token + Host), SQL takes no request input, response = repo paths/counts/cost (dashboard-only), leak `API_CALLS` entry added — no finding. Speed: `usage --by project` on the store copy 1.37 s (main) vs 1.38 s (HUN-206), warm. |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` → `1352 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 243.55s (0:04:03)` |
| **Companion score (easy-verifier)** | ☑ pass | overall 56 (advisory — no blocking minimum set, `docs/claude-md/pipeline-stages.md` step 4b), change_risk 100, scope changes, ref `main...HUN-206`; security 18 = the known SQL/lockfile false positives (memory: easy-verifier score output); 6 dimensions abstain. report: `/home/hungnguyenhuu/.easy-verifier-reports/HUN-206/evidence-report-changes-20261104T041806-780491Z.html` |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | backend only (UI is HUN-207) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | backend only (UI is HUN-207) |
| **UI: Responsiveness at target viewports** | ☑ N/A | backend only (UI is HUN-207) |

### Code-review findings (Supervisor, 2026-11-04)

| Sev | File | Finding | Conf | Action |
| -- | -- | -- | -- | -- |
| P2 | `src/easy_personal/core/workspaces.py` `unassigned` | Re-states the per-message query of `metrics._messages` (same filters, `GROUP BY message_id`) instead of reusing it | 75 | Suggestion only; keep — the shapes differ (per path, all time) |
| P2 | `tests/conftest.py` `real_projects` | 7 modules (incl. `test_dashboard_speed`) patch `_is_scratch` to False, so the speed test does not time the scratch rule | 75 | Accepted: measured on the real store copy instead (1.37 s → 1.38 s); the Scratch rule has its own 15 tests |
| P3 | `src/easy_personal/core/metrics.py` | A real repo whose folder is named `Scratch` merges into the Scratch row | 100 | Accepted, noted |

Process: the agent wrote code before tests (it says so); every AC has a test and BEFORE was captured before the first implementation commit — Gate 5 met. Startup reads line names `PROJECT_SPEC.md`, the guide and `agents/backend.md` — no process finding.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-04T03:49:38Z, before any implementation commit; store COPY made with `sqlite3 ~/.easy-personal/store.sqlite ".backup <worktree>/.verify-copy/store.sqlite"`, `EASY_PERSONAL_HOME=<worktree>/.verify-copy`):

```
$ uv run easy-personal usage --by project        # group count = len(groups)
127
first 15 keys: ['orderly', 'personal-agentic-claude', 'easy-verifier-mcp', 'easy-personal-mcp', 'idea-radar',
'easy-memory-mcp', 'easy-context-mcp', 'kitchd', 'idea-ladder', 'easy-ui-mcp', 'java-training',
'hungnguyen111', 'proj', 'tapchat-apis', 'kit']        (total.messages 49517)

$ GET /api/workspaces/unassigned  (TestClient on the copy, valid token, 2026-11-04T03:49:48Z)
405 Method Not Allowed      # no such route: only /api/workspaces/{id} (PATCH/DELETE) matches the path
routes with "workspaces": /api/workspaces, /api/workspaces, /api/workspaces/{id}, /api/workspaces/{id}, /api/accounts/{id}/workspaces
```

**AFTER** (2026-11-04T04:00:23Z, same store COPY, `EASY_PERSONAL_HOME=<worktree>/.verify-copy`, worktree code):

```
$ uv run easy-personal usage --by project         # 4.8 s wall
groups: 13   (was 127)
first keys: ['orderly', 'personal-agentic-claude', 'easy-verifier-mcp', 'easy-personal-mcp', 'idea-radar',
 'easy-memory-mcp', 'easy-context-mcp', 'kitchd', 'idea-ladder', 'easy-ui-mcp', 'java-training',
 'Scratch', 'tapchat-apis']
Scratch: scratch=true, folders=115, messages=1091, tokens=34,827,268, api-equivalent cost usd 23.010823
  top folders: hungnguyen111 4,565,152 | proj 3,578,874 | kit 1,556,900 | v149 1,433,597 | vt 913,837
total identical to BEFORE: True; sum(group tokens) == total tokens: True; sum(group messages) == total messages: True
by=day / model / tool: byte-identical to the pre-change code (git archive HEAD~1, `cmp`: 19518 / 2896 / 47943 bytes)

$ real uvicorn server on the copy (127.0.0.1:48206, token header), 2026-11-04T04:01:19Z
GET /api/workspaces/unassigned  -> 200, 10 repos:
 easy-verifier-mcp, personal-agentic-claude, easy-memory-mcp, idea-ladder, idea-radar, easy-ui-mcp,
 kitchd, easy-context-mcp, tapchat-apis, java-training     (newest last_used_utc first: 2026-11-03 ... 2026-10-05)
GET without token -> 403
```
Note: the guide expected about 9 repos; the copy gives 10 (`tapchat-apis` is the extra one, 8 sessions, $3.05).

**DELTA**: A project list now shows 13 rows instead of 127 (116 throwaway folders sit in one openable `Scratch` row with the same totals), and the dashboard API can list the 10 real repos that no workspace covers.

**WITNESS**: backend-developer agent, 2026-11-04 03:49Z (BEFORE) and 04:00-04:01Z (AFTER); to be confirmed by the Supervisor from `memory/event-trace/HUN-206.jsonl`.
