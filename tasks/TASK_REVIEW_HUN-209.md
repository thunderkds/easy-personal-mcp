# TASK_REVIEW — HUN-209: Store migration 9 clears stale abstained flag on metadata rows

> Sibling of `tasks/TASK_GUIDE_HUN-209.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_migrations.py` (`test_migration_9_clears_stale_metadata_rows_and_nothing_else` = AC 1-4, `test_migration_9_is_idempotent_and_a_v9_store_is_not_touched` = AC 4 + v9 no-op, `test_migration_9_frozen_list_is_part_of_the_parser_list`); `tests/core/test_rules.py::test_records_unreadable_ignores_stale_abstained_metadata_rows_after_migration_9` = AC 5 (30% stale `api-request`: fired, measured 0.3, then no hit after v8->v9). Pass was verified: pass |
| Verification command run | ☑ pass | `uv run pytest tests/core/test_migrations.py tests/core/test_rules.py -q` -> `66 passed, 1 deselected in 1.78s` (before the test_tool_errors edit); after it, with `tests/core/test_tool_errors.py` added: `75 passed, 1 deselected in 2.00s`. Full `uv run pytest -q`: `2 failed, 1354 passed, 2 skipped, 6 deselected, 1 xfailed in 239.01s`; the 2 failures were `tests/core/test_tool_errors.py` tests that hard-coded `user_version == 8` (stale assertions, fixed to use the latest migration number; both then pass in the 75-test run). `uv run ruff check src tests` -> All checks passed. Red first: before the migration existed, the 3 new tests failed. |
| Negative cases hold | ☑ pass | user/assistant/attachment/system/`mystery`/NULL-type abstained `claude_code` rows stay `abstained = 1`; `kit_trace` and `git_local` rows with metadata type names stay `abstained = 1` (test_migration_9_clears_stale_metadata_rows_and_nothing_else); all other columns equal before/after; v9 store opened again makes no second backup; `test_newer_store_is_refused_not_downgraded` covers old binary vs newer store |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor, 2026-11-04: reviewed `m0009_metadata_abstained.py`, its caller `store.migrate`/`MIGRATIONS`, readers `metrics.record_readability` + rule `records_unreadable`, and the 3 changed test files. Not reviewed: surfaces/web (no change). code-review: P0 0, P1 1 (process: startup reads `agents/backend.md`, PRD FR-003/FR-026 not read; `PROJECT_SPEC.md` only line-counted — waived, diff matches guide and all ACs verified independently), P2 0, P3 0. lane_check: `C1, 5 files, 281 lines, EXCEEDS (lines 281 > 250); unpredicted: tasks/TASK_GUIDE_HUN-209.md, tests/core/test_tool_errors.py` — 228 lines are the guide/review files; test_tool_errors.py edit replaces hard-coded `== 8` with latest migration (needed). security-review: built-in skill failed (`fatal: ambiguous argument 'origin/HEAD...'`); manual review — constant SQL with bound params, no input, no file read, no new stored field; no finding — pass. migration-safety: GO (see below). |
| Full smoke suite still green (no regression) | ☑ pass | Supervisor re-run after the test_tool_errors fix, worktree HUN-209 @ 67ad0a0: `uv run pytest -q` -> `1356 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 239.28s`. Leak suite included. Frozen type list == `claude_code.METADATA_TYPES` (19, checked). |
| **Companion score (easy-verifier)** | ☑ pass | overall 76, change_risk 100, scope changes, ref `main...HUN-209` (advisory; needs_input picks/test_pairs/change_kind not answered; change_kind detected `feature`, actual `fix`). report: `/home/hungnguyenhuu/.claude-hung2/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/f2a87310-c14a-429e-aa17-5c1a76ce5c54/tool-results/mcp-easy-verifier-score-1793788917100.txt` |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Pure backend migration; no UI component. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Pure backend migration; no UI component. |
| **UI: Responsiveness at target viewports** | ☑ N/A | Pure backend migration; no UI component. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (copy of the owner store in a scratch dir, `EASY_PERSONAL_HOME=<scratch>/home`; code = main without migration 9; `date -u` = `Wed Nov  4 10:30:02 AM UTC 2026`):

```
user_version (8,)
SELECT record_type, SUM(abstained), COUNT(*) FROM events WHERE record_type IN ('api-request','api-request-shape') GROUP BY 1
  -> [('api-request', 27360, 40410), ('api-request-shape', 987, 1515)]
total events: 434817
$ uv run easy-personal warnings   (as_of_utc 2026-11-04T10:30:03Z)
  hits: records_unreadable, detail {"version": "2.1.312"}, measured 0.1998,
  summary "20% of records from Claude Code 2.1.312 could not be read; target 2% or less, last 7 days"
```

**AFTER** (same copy, migrated by the new code; `date -u` = `Wed Nov  4 10:33:06 AM UTC 2026`; `EASY_PERSONAL_HOME=<scratch>/home`):

```
$ uv run easy-personal warnings   (as_of_utc 2026-11-04T10:33:07Z)
  hits: ['tokens_jump']        # records_unreadable no longer fires
user_version (9,)    backup: store.sqlite.bak-v8 exists
SELECT record_type, SUM(abstained), COUNT(*) ... -> [('api-request', 0, 40410), ('api-request-shape', 0, 1515)]
total events: 434817   (before: 434817)
still abstained (not metadata types, untouched): agent-color 146, agent-setting 40, continued-in 2,
  fork-context-ref 1, relocated 26, session-owner 1753, worktree-state 26
```

**DELTA**: The `warnings` tool no longer reports a false `records_unreadable` for Claude Code 2.1.312 (20% -> not fired) because 28,347 stale metadata rows now read `abstained = 0`.

**WITNESS**: Supervisor, 2026-11-04, independent re-run on a fresh copy of the owner store: before `(8, (436403, 97696))` [user_version, (events, SUM(abstained) all time)] -> up `4.00s` `(9, (436403, 2036))`; `.bak-v8` = `(8, (436403, 97696))`; restored then up again `1.96s` `(9, (436403, 2036))`. All-time cleared: 95,660 metadata rows (28,347 are `api-request*`).

## Migration safety (Supervisor, 2026-11-04)

Verdict: GO. Migration tool: `store.migrate` (`MIGRATIONS`), one transaction, `<store>.bak-v8` written first. Destructive ops: none (UPDATE of one flag; no DROP/RENAME/NOT NULL). Rollback: restore `.bak-v8` (tested up -> restore -> up, counts above); no down script, same as m0007/m0008 — accepted. Coexistence: an old binary refuses a v9 store (`test_newer_store_is_refused_not_downgraded`). Locks: local single-user SQLite, 2-4 s on 436k rows. Backfill: none needed. Data loss: none (row count equal; other columns asserted unchanged).
