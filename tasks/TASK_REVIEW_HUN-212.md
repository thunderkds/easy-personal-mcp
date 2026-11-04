# TASK_REVIEW — HUN-212: Live tail shows lines another process stored first (FR-071)

> Sibling of `tasks/TASK_GUIDE_HUN-212.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_live.py`: two_tails_on_one_store (AC1, AC3), a_line_stored_by_another_caller_between_two_polls + mixed batch (AC2, AC3), tail_started_with_the_cursor_at_the_end (AC4), replaced_file / truncated_file followed by both tails (AC5), file_created_after_both_tails_started (review fix), new_subagent_file_published_by_both_tails, file_with_history_seen_in_a_tails_first_scan, cursor moved backwards, offset map forgets a vanished file. Supervisor rerun 2026-11-04: `uv run pytest tests -q -k live` → `85 passed, 1291 deselected, 1 warning in 40.39s` |
| Verification command run | ☑ pass | agent, final tree 381913f: `uv run pytest tests -q -k live` → 85 passed; `uv run pytest -q` → `1367 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 267.39s`; Supervisor rerun of the live set: 85 passed |
| Negative cases hold | ☑ pass | lock busy → nothing published, retried next poll (existing busy-lock test); cursor moved backwards → no history replay (new test); tail started late → no history from first-scan files (new test); leak suite `tests/leak/test_live_leak.py` passes |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Stage 4 code-review (Supervisor, 2026-11-04): diff 69791fb..381913f = src/easy_personal/core/live.py, tests/core/test_live.py; callers `LiveTail.poll_once` (dashboard start) and `GET /api/live` read path unchanged. lane_check: `lane: C2, 2 files, 269 lines, FITS`. Startup reads line present with all three Permanent-Rule reads. Findings: P0 0, P1 0 (the first-report gap — a file created while the tail runs started at the store cursor — was fixed in 381913f with tests), P2 1 follow-up: a Claude account confirmed while a dashboard runs has its existing files first seen after priming, so they are read from byte 0; old sessions can reach the page as idle rows until the next snapshot, and one with an open call < 24 h old could show `needs attention`. Rare; suggested fix: prime per account root. Security review (manual; built-in `security-review` needs `origin/HEAD`, absent here): no new input, route or file path; same roots and `_jsonl_files` walk (no dir symlink follow); `_item` unchanged, so metadata only (FR-002); lock still per batch; memory bounded (offset map pruned on vanished files). No finding. |
| Full smoke suite still green (no regression) | ☑ pass | agent full suite on 381913f: `1367 passed, 2 skipped, 6 deselected, 1 xfailed` |
| **Companion score (easy-verifier)** | ☑ pass | advisory: `score` scope=changes ref=69791fb task_id=HUN-212 → overall 50 (blast-radius 46, requirement-fidelity 67, security 30, test-strategy 55; others abstained); change_risk: rating_abstention ("all declared rating metrics abstained"). Same figures as HUN-213, so repo-level, not this diff. Not gating. report: /home/hungnguyenhuu/.claude-hung2/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/92fb1d94-3cb3-4b8e-920b-9d813a38e38e/tool-results/mcp-easy-verifier-score-1793792028877.txt |
| **UI: Visual regression (diff or verdict pasted)** | ☐ N/A | backend only (guide: UI / Design Evidence N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ N/A | backend only (guide: UI / Design Evidence N/A) |
| **UI: Responsiveness at target viewports** | ☐ N/A | backend only (guide: UI / Design Evidence N/A) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-04T11:10:50Z / re-run 2026-11-04T11:11:31Z, before any change to `src/`.
Command: `uv run pytest tests/core/test_live.py -q -k "two_tails or another_caller or cursor_at_the_end or replaced_file_is_followed or truncated_file_is_followed" --tb=line`
Failure of AC 1 (two `LiveTail` objects on one store; the first poll wins the store cursor, the second tail lists no session):

```
>           assert {s["session_id"] for s in tail.snapshot(now=clock())["sessions"]} == {"sess1", "sess2", "sess3"}
E           AssertionError: assert set() == {'sess1', 'sess2', 'sess3'}
E             Extra items in the right set:
E             'sess2'
E             'sess3'
E             'sess1'
tests/core/test_live.py:770: AssertionError
```

Full summary of the run:

```
AssertionError: assert [] == ['Old', 'New']
AssertionError: assert (0 == 3)
FAILED tests/core/test_live.py::test_two_tails_on_one_store_each_list_every_session_and_every_tool_event_once_in_the_store
FAILED tests/core/test_live.py::test_a_line_stored_by_another_caller_between_two_polls_is_still_published_and_not_stored_again
FAILED tests/core/test_live.py::test_a_tail_started_with_the_cursor_at_the_end_of_a_file_publishes_nothing_until_a_new_line
FAILED tests/core/test_live.py::test_a_replaced_file_is_followed_by_both_tails_without_publishing_old_lines_twice
FAILED tests/core/test_live.py::test_a_truncated_file_is_followed_by_both_tails_without_an_error_or_a_second_publish
5 failed, 38 deselected in 1.04s
```

**AFTER**: same pytest command, 2026-11-04 (after the change; the test file also holds extra edge tests). After the Supervisor's review fix (a file created while the tail runs, or a replaced file, is read from byte 0; only files of the tail's first scan start at the store cursor), `uv run pytest tests/core/test_live.py -q` printed `49 passed in 6.16s`. The review-fix tests failed first (2026-11-04T11:24:29Z, 3 failed, 46 passed): new-file-both-tails, new-subagent-file-both-tails, replaced-file-both-tails.
Real run (AC 7, simulated sessions: fixture lines appended to 3 transcript files under a temp account root, plus a 4th session file created AFTER both dashboards started; two real `easy-personal dashboard` processes on one temp store, `/api/live` read from each; started 2026-11-04T11:25:19Z):

```
dashboard 1 (port 43809) snapshot sessions: [('sess1', 'repo-1', 3), ('sess2', 'repo-2', 3), ('sess3', 'repo-3', 3), ('sess4', 'repo-4', 3)]
dashboard 1 live tool_use events: 12 [('sess1', 'Bash'), ('sess2', 'Bash'), ('sess3', 'Bash'), ('sess4', 'Grep')]
dashboard 2 (port 33499) snapshot sessions: [('sess1', 'repo-1', 3), ('sess2', 'repo-2', 3), ('sess3', 'repo-3', 3), ('sess4', 'repo-4', 3)]
dashboard 2 live tool_use events: 12 [('sess1', 'Bash'), ('sess2', 'Bash'), ('sess3', 'Bash'), ('sess4', 'Grep')]
store events rows: 15 (3 seed + 9 new + 3 in the new file = 15 expected)
distinct (path,inode,offset): 15
```

**DELTA**: two dashboards (or a dashboard plus the scheduled ingest) now each show every running session, including a session or subagent file that starts after the dashboards, and the store still holds each line once.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-212.jsonl`, never the
implementing agent alone]
