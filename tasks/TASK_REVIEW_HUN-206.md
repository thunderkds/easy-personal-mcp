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

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_scratch.py` (AC 1-6; 15 tests), `tests/surfaces/test_api_unassigned.py` (AC 6-7; 4 tests), `tests/test_readme.py::test_scratch_group_is_documented_and_in_the_mcp_description` (AC 9), `tests/leak/harness.py` `API_CALLS` entry for `/api/workspaces/unassigned` (AC 8, run by `tests/leak/`). `uv run pytest -q tests/core/test_scratch.py tests/surfaces/test_api_unassigned.py` -> `19 passed` |
| Verification command run | ☑ pass | `uv run pytest -q` -> `1352 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 243.55s (0:04:03)` (2026-11-04, worktree HUN-206, commit after `HUN-206: existing tests ...`). Real run on the store COPY: see Demonstration AFTER. |
| Negative cases hold | ☑ pass | no/wrong token and wrong Host -> 403 on the new route (`test_route_refuses_no_token_wrong_token_and_wrong_host`; also real server: curl without token -> 403); `/…/repo` does not cover `/…/repo2`; relative / empty / NULL stored path -> Scratch / `(unknown)`, no crash; a folder under the temp folder runs no git call (`test_ac4_...`); `by=day/model/tool` outputs byte-identical to the pre-change code on the store copy (`cmp`). |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | backend only (UI is HUN-207) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | backend only (UI is HUN-207) |
| **UI: Responsiveness at target viewports** | ☑ N/A | backend only (UI is HUN-207) |

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
