# TASK_REVIEW — HUN-192: Store failed tool results per user record: migration 7 → 8 + re-read (FR-059)

> Sibling of `tasks/TASK_GUIDE_HUN-192.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_tool_errors.py` (AC1,2,3,6 + v8 no-op + old-binary refusal), `tests/sources/test_claude_code_tool_errors.py` (AC4), `tests/leak/test_leak_suite.py::test_failed_tool_result_was_counted_and_its_text_not_stored` + planted `INVENTED_failed-tool-result-*` in `tests/fixtures/leak/session.jsonl.tmpl` (AC5). Before the code: 25 failed; after: `25 passed` (targeted), leak suite `34 passed`. AC7: real run below. |
| Verification command run | ☑ pass | `uv run pytest tests/core/test_migrations.py tests/sources -q && uv run pytest tests/leak -q && uv run pytest -q` exit 0; final stage: `1189 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 160.25s (0:02:40)` |
| Negative cases hold | ☑ pass | `is_error` `"true"`/1/missing not counted; text-only, string content, non-list content, assistant/system records -> NULL; deleted file keeps NULL; re-read never turns a known count back to NULL; v8 store opened by a v7 program refused (`newer than this program`). |
| verify | ☑ pass | Stage 5 2026-11-03 03:03 UTC @ 5b52ebe (Supervisor, worktree code, `.backup` copy of the live store in `scratchpad/v192`): before v7 / 386360 events; `EASY_PERSONAL_HOME=<copy> uv run easy-personal ingest` 17.48 s, 71 MB; after v8 / 391488 events (+5128 new transcript lines), 0 duplicate (machine_id, source, source_path, inode, byte_offset) keys, `store.sqlite.bak-v7` written; user rows `56480|47397|2379` (count, filled, failed); non-user rows with a count: 0. Migration-safety (Supervisor, Stage 4): **GO** — additive nullable column, no destructive op, `ADD COLUMN` rewrites no row, re-read idempotent; rollback tested: current main code reads the restored `.bak-v7` (`easy-personal usage` ok) and refuses the v8 store (`schema v8, newer than this program (v7): upgrade easy-personal`). No in-place down migration (project pattern); rollback = restore `.bak-v7` + 0.4.0 binary, next ingest re-reads lines since. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed `m0008_tool_errors.py`, `store.save_file` upsert, `claude_code._tool_errors`/`parse_record`; callers `claude_code.ingest`, `core/live.py` (uses `parse_record` → `_empty`, so the new key is always present), tests. `lane: C2, 11 files, 280 lines, FITS` (unpredicted = test files only). Code review: 0 P0, 0 P1, 1 P2 (process: agent skipped guide step 6 reads `BRAINSTORMING_LOG_KITPERF.md`/PRD; Permanent-Rule reads done; no effect on the diff), 0 P3. Security (Medium risk; Supervisor inline review, built-in `security-review` not run — it reviews the main checkout's branch and needs `origin/HEAD`): only `type` + `is_error` of blocks are read, integer stored, planted `INVENTED_failed-tool-result-*` absent from store/dump (leak suite) — 0 findings. |
| Full smoke suite still green (no regression) | ☑ pass | full `uv run pytest -q`: 1189 passed (3 older tests updated for v8: `test_store.py` allowed columns, `test_entrypoint.py` version check, `test_connectors_schema.py` v5 fixture column). |
| **Companion score (easy-verifier)** | ☑ N/A | Neither companion tool run for this backend-only C2 task. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Backend-only task, no UI. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Backend-only task, no UI. |
| **UI: Responsiveness at target viewports** | ☑ N/A | Backend-only task, no UI. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-03 02:48:17 UTC, on a `.backup` copy of `~/.easy-personal/store.sqlite`, before any implementation commit):

```
$ date -u; sqlite3 <copy> "PRAGMA user_version; SELECT name FROM pragma_table_info('events') WHERE name='tool_error_count'; SELECT 'rows_with_column=' || COUNT(*) FROM pragma_table_info('events') WHERE name='tool_error_count'; SELECT COUNT(*) AS events FROM events;"
Tue Nov  3 02:48:17 AM UTC 2026
7                      <- PRAGMA user_version (store is v7)
rows_with_column=0     <- events.tool_error_count column absent (name query printed nothing)
386360                 <- events rows
$ uv run python -c "from easy_personal.core.migrations import MIGRATIONS; print(max(n for n,_ in MIGRATIONS))"
code latest migration: 7
$ sqlite3 <copy> "SELECT COUNT(*), SUM(byte_offset>0) FROM source_files WHERE source='claude_code'"
1198|1197          <- 1198 claude_code files, 1197 with a non-zero offset
```

**AFTER** (real run 2026-11-03 02:57:16-02:58:01 UTC, copy of the owner's store, `EASY_PERSONAL_HOME=<scratch>/home`; live store never touched):

```
events before: 386360 v7
$ /usr/bin/time -v uv run easy-personal ingest      # migrates 7 -> 8 (backup store.sqlite.bak-v7), then re-reads every transcript
  files 1214 ... Elapsed (wall clock) time: 0:44.69 ... Maximum resident set size: 71472 kB
$ sqlite3 <copy> "PRAGMA user_version; SELECT COUNT(*) FROM events; SELECT COUNT(*), SUM(tool_error_count IS NOT NULL), SUM(tool_error_count) FROM events WHERE record_type='user'; SELECT COUNT(*) FROM events WHERE record_type<>'user' AND tool_error_count IS NOT NULL;"
8
391123
56432|47356|2376          <- user rows | with a known count | total failed tool results
0                        <- non-user rows with a count
$ ls: ingest.lock  store.sqlite  store.sqlite.bak-v7
# the 386360 original rows keep their ids (id <= bak-v7 max id: 386360 rows; 55707 user rows, 46700 now filled);
# the +4763 events are new transcript lines written after the snapshot (ingest reported new_events 4763), not duplicates.
# source_files claude_code: 1214 files, 1213 with offset > 0 after ingest (was reset to 0 by the migration).
$ second ingest: new_events 1 (+5 other sources), 1.14 s; user count totals 56433|47357|2376 (unchanged, idempotent)
```

**DELTA**: The store (v8) now records, per user record, how many tool results failed (`events.tool_error_count`, NULL = unknown), so the next task can compute a tool-error rate per session.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
