# TASK_REVIEW — HUN-244: Read-only store health check (CLI and API)

> Sibling of `tasks/TASK_GUIDE_HUN-244.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_store_health.py` (AC 1-7, 10; 24 tests, 1 perf), `tests/surfaces/test_api_store_health.py` (AC 8; 11), `tests/leak/test_store_check_leak.py` + `tests/leak/harness.py` `CLI_CALLS`/`API_CALLS` (AC 9). Implementer run (Sonnet agent), not yet reviewer-confirmed. |
| Verification command run | ☑ pass | `uv run pytest tests/core tests/surfaces tests/leak -q` -> `1316 passed, 6 deselected`; `uv run pytest -q` -> `1800 passed, 2 skipped, 7 deselected, 1 xfailed`. (One earlier run showed `tests/core/test_kit_workspaces.py::test_160_projects_and_10_workspaces_answer_in_under_two_seconds` failing under load from a parallel agent; it passes alone, 24 passed, and in both later full runs.) |
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

**BEFORE** (captured on `c245d89` = `main`, before any HUN-244 implementation commit; temp `EASY_PERSONAL_HOME`):

```
2026-11-13T08:19:37Z
$ EASY_PERSONAL_HOME=/tmp/tmp.GGjiwJXgeO uv run easy-personal store-check
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,workspace,warnings,kit,plan,kit-findings,dashboard,exec,headers,connector,account,doctor} ...
easy-personal: error: argument command: invalid choice: 'store-check' (choose from ping, mcp, ingest, schedule, usage, outliers, workspace, warnings, kit, plan, kit-findings, dashboard, exec, headers, connector, account, doctor)
rc=2

2026-11-13T08:19:41Z
$ (in-process TestClient of api.create_app, valid token) GET /api/store/health
GET /api/store/health -> 404 Not Found
```

**AFTER** (same two surfaces, after the change; temp `EASY_PERSONAL_HOME`, fixture transcripts, own dashboard on a free port):

```
2026-11-13T08:32:05Z
$ easy-personal store-check
file: store.sqlite 176128 bytes (wal 0 bytes)
schema: user_version 11, code has 11, migration pending: no
integrity_check: ok
foreign_key_check: ok
tables: account_registrations 0, ..., events 39, ..., meta 1, source_files 6, ..., workspaces 1
sources: claude_code last 2026-01-15T10:21:00.000000Z, abstained 3; kit_trace no data; context no data; ui no data; verifier no data
backups: none
verdict: healthy
rc=0

2026-11-13T08:40:32Z
$ curl -H 'X-Dashboard-Token: <token>' http://127.0.0.1:<port>/api/store/health
{"store_file":"store.sqlite","exists":true,"readable":true,"size_bytes":176128,"wal_bytes":0,"user_version":11,"latest_migration":11,"migration_pending":false,...,"integrity_check":{"ok":true,"problems":0,"mode":"full"},"foreign_key_check":{"ok":true,"problems":0},...} HTTP 200
$ curl (no token)                  -> {"error":"forbidden"} HTTP 403
$ curl -X POST with token          -> Method Not Allowed HTTP 405
$ curl "?token=<token>" (query)    -> {"error":"forbidden"} HTTP 403
```

**DELTA**: The owner can run `easy-personal store-check` (or open `GET /api/store/health`) and see, without opening SQLite, whether the store is intact, migrated and holding rows, instead of guessing from an empty dashboard.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
