# TASK_REVIEW — HUN-61: Store foundation: versioned migrations, FK + busy_timeout, time-first usage query + indexes

> Sibling of `tasks/TASK_GUIDE_HUN-61.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `tests/core/test_migrations.py` (runner, v0 upgrade with rows, rollback, newer-store refusal, PRAGMAs, equality on summary/breakdown/outliers/timeline/contexts, EXPLAIN plan, 1M perf) |
| Verification command run | pass | `uv run pytest tests/core -q` -> 64 passed, 1 deselected; `uv run pytest tests/core -m perf -q` -> 1 passed (229-237 ms); `uv run pytest -q` -> 451 passed, 1 deselected (after AC 8 fix; red step: 2 failed incl. test_leaking_tool_fails_the_suite); `uv run ruff check .` -> All checks passed! |
| Negative cases hold | pass | failing migration rolls back; newer store refused; no backup for a new store |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | pass | 451 passed, 1 deselected (perf test runs only with `-m perf`) |
| easy-verifier score (advisory) | ☐ recorded / ☐ N/A | [overall result, abstaining dimensions, report path — or `N/A — <reason>` if the server is down; never blocks Done] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured before any implementation commit, temp store built from `tests/fixtures/claude_code`, 31 events; `date -u` then sqlite3 on the temp store):

```
$ date -u
Mon Oct 26 09:02:53 AM UTC 2026
0
0
QUERY PLAN
|--CO-ROUTINE (subquery-1)
|  `--SEARCH events USING INDEX events_message (message_id>?)
|--SCAN (subquery-1)
`--USE TEMP B-TREE FOR ORDER BY
$ sqlite3 store.sqlite 'PRAGMA user_version; PRAGMA foreign_keys;'   (output above: user_version=0, foreign_keys=0)
$ sqlite3 store.sqlite 'EXPLAIN QUERY PLAN <current metrics._messages SQL, 2026-01 window>'
```

Note: SQLite does not print a literal `SCAN events` here. It reads the whole `events_message` index
(`SEARCH events USING INDEX events_message (message_id>?)`, i.e. every event with a message id) and then
filters the time after grouping, so the time bound prunes nothing.

**AFTER** (the same BEFORE store upgraded in place by `store.connect()`, then the same queries; 2026-10-26 09:05 UTC):

```
(2,) (1,) (5000,) (31,)        <- user_version, foreign_keys, busy_timeout (ms), events rows kept
Mon Oct 26 09:05:39 AM UTC 2026
after.txt  store.sqlite  store.sqlite.bak-v0      <- restore copy made before the upgrade
0                          <- bak-v0 user_version
31                         <- bak-v0 events rows
QUERY PLAN
|--CO-ROUTINE (subquery-1)
|  |--SEARCH events USING INDEX events_ts_message (ts_utc>? AND ts_utc<?)
|  `--USE TEMP B-TREE FOR GROUP BY
|--SCAN (subquery-1)        <- scans the already-grouped 30-day subquery, not `events`
`--USE TEMP B-TREE FOR ORDER BY
```

Perf (`uv run pytest tests/core -m perf -q -s`, 1,000,000 events over 365 days, no ANALYZE, 30-day
`usage_summary`): `[229, 232, 237] ms` (target < 500 ms; a run with ANALYZE gave `[230, 229, 240] ms`).

**DELTA**: The store upgrades itself through numbered migrations (with a restore copy), every connection enforces
foreign keys and waits up to 5 s for a lock, and a 30-day usage summary over 1M events returns in ~0.23 s because
the time bound now prunes through an index before grouping.

Migration-safety self-check (Supervisor re-runs the skill at Stage 4):
* Reversibility: no down-migrations (cut list). Before the first pending migration on a non-empty store, `connect()`
  writes `store.sqlite.bak-v<N>` (SQLite online backup, WAL-safe). To roll back: stop the timer and dashboard, copy
  `store.sqlite.bak-v<N>` over `store.sqlite`, delete `store.sqlite-wal` and `store.sqlite-shm`.
* Backward compatibility: v0 store with rows and a pre-HUN-42 `verifier_scores` upgrades with every event kept
  (`test_v0_store_upgrades_keeping_every_row_and_leaves_a_backup`, `test_pre_hun42_verifier_scores_is_rebuilt_by_the_baseline`).
  The only dropped data is the pre-HUN-42 `verifier_scores` cache, as before (the next ingest refills it).
* No silent loss: m0002 only creates `events_ts_message` and drops the now-redundant `events_ts`.
* Ordering/atomicity: each migration plus its `user_version` bump runs in one `BEGIN IMMEDIATE`; the version is
  re-read under the lock (concurrent processes); a failure rolls back (tests). A newer store is refused.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-61.jsonl`, never the
implementing agent alone]

AC 8 (no test opens the real data dir): `tests/conftest.py` autouse `_isolated_data_dir` (tmp `EASY_PERSONAL_HOME`) and
`_never_open_the_real_store` (patched `sqlite3.connect` raises under the real `~/.easy-personal`); guard tests in
`tests/test_real_home_guard.py`. Red first: with only the guard, `uv run pytest -q` gave 2 failed (the leak test and
`test_data_dir_is_never_the_real_home`); after the fixture, 451 passed. Perf 1 passed; ruff clean.
