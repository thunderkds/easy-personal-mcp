# TASK_REVIEW — HUN-176: Live tail: follow every account's transcripts while the dashboard runs (FR-043)

> Sibling of `tasks/TASK_GUIDE_HUN-176.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_live.py` (20 tests: AC1 `test_tool_use_line_is_stored_and_in_the_buffer_within_500_ms`, AC2 `test_partial_last_line_is_never_parsed_and_is_taken_once_complete`, AC3 `test_a_subagents_file_appearing_mid_session_is_followed`, AC4 `test_the_tail_does_not_hold_the_lock_so_a_scheduled_ingest_completes` + `test_events_are_not_lost_or_doubled_when_a_scheduled_ingest_holds_the_lock`, AC5 `test_buffer_is_bounded_and_ids_keep_counting` + `test_default_buffer_size_is_2000` + `test_events_carry_metadata_only_and_nothing_secret_reaches_store_or_logs`, AC6 `test_stop_ends_the_thread_and_is_repeatable` + the two `test_cli_dashboard_*`), `tests/leak/test_live_leak.py` (planted leak-suite canaries over the tail). Output: `tests/core -k live`: `20 passed, 369 deselected, 1 warning in 3.51s`; `tests/leak`: `30 passed, 1 warning in 24.27s` |
| Verification command run | ☑ pass | 2026-11-01T02:09:50Z `uv run pytest tests/core -q -k live && uv run pytest tests/leak -q` -> `20 passed, 369 deselected, 1 warning in 3.51s` then `30 passed, 1 warning in 24.27s`. `uv run ruff check src tests` -> `All checks passed!` |
| Negative cases hold | ☑ pass | Covered in `tests/core/test_live.py`: half-written last line never parsed (AC2); lock held by another process -> batch skipped, no event lost or doubled; scheduled ingest reads first -> cursor not rewound, one row per line; replaced file (new inode) and truncated file re-read from 0; vanished file, garbage/blank/non-dict lines do not stop the tail; session idle > 1 h forgotten (bounded memory); planted secret text/tool input/tool output absent from events, snapshot, store dump and log records; leak-suite canaries (`tests/leak/test_live_leak.py`) absent in every form |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` -> `1018 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 105.72s (0:01:45)` |
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

**BEFORE**: captured 2026-11-01T02:00:47Z in the worktree at 3a1f707, before any implementation commit:

```
$ uv run python -c "import easy_personal.core.live"
    import easy_personal.core.live
ModuleNotFoundError: No module named 'easy_personal.core.live'
$ uv run pytest tests/core -q -k live
369 deselected, 1 warning in 1.06s
$ grep -n "live" src/easy_personal/surfaces/cli.py
(no match, exit 1)
```

**AFTER**: captured 2026-11-01T02:09:50Z on branch HUN-176 (commit 8755ff2):

```
$ uv run pytest tests/core -q -k live && uv run pytest tests/leak -q
20 passed, 369 deselected, 1 warning in 3.51s
30 passed, 1 warning in 24.27s
$ uv run python -c "from easy_personal.core import live; print(live.LiveTail().buffer_size, live.POLL_SECONDS)"
2000 0.1
$ grep -n "live" src/easy_personal/surfaces/cli.py
387:        from easy_personal.core import live
394:        tail = live.LiveTail().start()  # FR-043: follows the transcripts while the page is open
```

AC7 real run (2026-11-01, Claude Code 2.1.316, `claude -p --model haiku --allowed-tools Read Bash`, account `~/.claude-hung2`, temp `EASY_PERSONAL_HOME`, real `easy-personal dashboard` command started through a driver that only logs event id/kind/tool/timestamps). Dashboard cold ingest first (505 files, 105792 new events), then `dashboard: http://127.0.0.1:36533/...` was up; the session ran 02:09:02Z-02:09:19Z, exit 0. Metadata only:

```
transcript tool_use: 6 ['Bash', 'Read', 'Bash', 'Bash', 'Read', 'Bash']
buffer tool_use:     6 ['Bash', 'Read', 'Bash', 'Bash', 'Read', 'Bash']
latency ms (buffer visible - transcript timestamp): [1212, 164, 664, 669, 117, 618]
median 641 ms, max 1212 ms, within 2 s: 6 / 6 = 100% (NFR-011 bar: >= 95%)
kinds in buffer for the session: tool_use 6, usage 7, tool_result 6; ids monotonic
store (temp): 65 rows for the session file, 0 duplicate byte_offsets, stored offset 264555 == file size 264555
SIGINT to the dashboard process: process gone within 3 s (no orphan thread/process)
```

**DELTA**: While `easy-personal dashboard` runs, a Claude Code tool call now lands in the store and in a bounded in-memory event buffer within about 100 ms of the transcript write (median 641 ms after Claude Code's own record timestamp), where before nothing followed the transcripts after the start-up ingest.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
