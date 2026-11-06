# TASK_REVIEW — HUN-237: Live tail warm start — active sessions listed with full totals after a dashboard restart (FR-086)

> Sibling of `tasks/TASK_GUIDE_HUN-237.md`. Everything here is **filled by the reviewer at Stage
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
Owner instruction (not a decision): "yes, start HUN-237 [auto]" (2026-11-06), after the Supervisor proposed: on start, load the sessions that wrote in the last 5 minutes, with their full totals.
1. Rebuild state by replaying transcripts (publish-to-state only), not by seeding from the store — FACT: src/easy_personal/core/live.py:14 "labels, sub-agent types and the kit stage live in memory and are never stored." (the store cannot give the stage, block ids or open calls)
2. Replay files modified within FORGET_AFTER_S (1 h), last WARM_MAX_BYTES = 64 MiB each — DEBATED: 1 h + cap (Advocate 1: 1 h + 64 MiB cap; Advocate 2: 5 min, whole file) ⚠ decided for you — please review. Deciding Citation: PRD.md:132 "A session is *active* when it wrote a record in the last 5 minutes, or when it has an *open call* (FR-047)"; cap kept by rule 2 (bounded start); cost: totals of a file > 64 MiB are partial
3. Replay changes in-memory state only: the feed and the store stay as today — FACT: web/src/LiveView.jsx:169 "No tool calls seen since this page opened." (the feed is defined as since-page-open); PRD.md:159 "The event count in the store is the same as with one writer."
4. C2 / Risk Medium — RULE: agents/backend.md:92 "| **C2** Moderate | 3+ files, *or* a design choice, *or* a new component |"; agents/backend.md:106 "Risk Medium/High (auth, schema, shared services)" (the live tail shares the store write path, FR-071)
5. spawned on claude · C2 → claude-sonnet-5-5 — FACT: PROJECT_SPEC.md:165 "| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` | `claude-sonnet-5-5` | `claude-opus-5-5` |"
Checks: code-review (1 P1 fixed in 2cf9724), security-review (no findings), tests 190 + 1625 passed, ruff clean, /verify pass (owner-invoked, 2026-11-06 16:14 UTC)
Not done: merge (yours)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | tests/core/test_live.py: 11 `test_warm_start_*` tests (AC 1-6 + early-return edge + state_version bump); tests/leak/test_live_leak.py::test_warm_start_replay_leaks_no_canary_and_reads_only_what_the_live_path_reads (AC 7). `uv run pytest tests/core/test_live.py -q -k warm` → `12 passed, 106 deselected in 0.66s` |
| Verification command run | ☑ pass | `uv run pytest tests/core/test_live.py tests/surfaces/test_api_live.py tests/leak -q` → `190 passed, 1 warning in 62.19s`; `&& uv run pytest -q` → `1625 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 213.19s`; `uv run ruff check src tests` → `All checks passed!` (2026-11-06 16:14 UTC) |
| Negative cases hold | ☑ pass | real run: a file 2 h old (sess-old) not listed; unauthenticated `/api/live` → 403; replay buffers no event (`last_id: 0` first snapshot) and stores nothing (store 45 before dashboard → 46 after one appended line); canary `VERIFYCANARY237` in Skill args, Bash input, tool results, sub-agent meta → 0 hits in SSE + dashboard log; tests: window, cap mid-line, early-return no double replay |
| verify | ☑ pass | owner-invoked `/verify HUN-237`, 2026-11-06 ~16:12 UTC, isolated `uv tool install` (0.4.0, ping ok, MCP stdio 12 tools). Dashboard started after ingest: first `/api/live` snapshot, no new line yet, lists sess-warm `tool_calls 23`, `tokens_in 2300`, `stage 4 code-review`, attention `waiting AskUserQuestion`, `last_id 0`; appended line → events id 1 (tool_use Read) and id 2 (usage), tool_calls 24 — pass. Script: scratchpad v237.sh |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed live.py diff (_scan, _warm, _publish, _append, _jsonl_files) + callers (surfaces/api.py /api/live, cli dashboard) + tests/core/test_live.py, tests/surfaces/test_api_live.py, tests/leak; skipped store/parser/web (not touched, Files Must NOT Touch) |
| Full smoke suite still green (no regression) | ☑ pass | `1625 passed, 2 skipped, 6 deselected, 1 xfailed` |
| **Companion score (easy-verifier)** | ☑ N/A | not run: easy-verifier refuses worktree paths (seen HUN-233..236); advisory only |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no page change (web/ untouched); the page renders the snapshot it already receives |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no page change |
| **UI: Responsiveness at target viewports** | ☑ N/A | no page change |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-06 14:49:31 UTC (`date -u`), AC 1-6 tests written first, no implementation yet:

```
$ date -u; uv run pytest tests/core/test_live.py -q -k warm
Fri Nov  6 02:49:31 PM UTC 2026
FAILED tests/core/test_live.py::test_warm_start_lists_a_stored_session_with_whole_totals_before_any_new_line
FAILED tests/core/test_live.py::test_warm_start_appends_no_event_and_the_next_line_gets_id_one
FAILED tests/core/test_live.py::test_warm_start_of_a_partly_stored_file_replays_below_the_cursor_and_stores_the_rest_once
FAILED tests/core/test_live.py::test_warm_start_rebuilds_stage_waiting_and_sub_agent_type_and_leaves_answered_calls_closed
FAILED tests/core/test_live.py::test_warm_start_rebuilds_a_failure_streak - V...
FAILED tests/core/test_live.py::test_warm_start_of_a_huge_file_replays_the_last_cap_bytes_from_a_line_start[boundary]
FAILED tests/core/test_live.py::test_warm_start_of_a_huge_file_replays_the_last_cap_bytes_from_a_line_start[inside-line-4]
FAILED tests/core/test_live.py::test_warm_start_of_a_huge_file_replays_the_last_cap_bytes_from_a_line_start[whole-line-4]
FAILED tests/core/test_live.py::test_warm_start_does_not_replay_a_file_twice_when_the_first_scan_ended_early
9 failed, 2 passed, 106 deselected in 1.20s
```
(Typical failure: `ValueError: not enough values to unpack (expected 1, got 0)` in `warm_session` = the stored session is absent from `snapshot()` after the first poll; the cap tests: `AttributeError: module 'easy_personal.core.live' has no attribute 'WARM_MAX_BYTES'`. The 2 passing tests are the "stores nothing" and the "outside the window / first seen later" guards, which hold today and must keep holding.)

**AFTER**: 2026-11-06 16:14:33 UTC (`date -u`), same command after the change:

```
$ uv run pytest tests/core/test_live.py -q -k warm
12 passed, 106 deselected in 0.66s
```
(12 = the 11 BEFORE tests + the Stage 4 state_version test.)

**DELTA**: After a dashboard restart, the Live view lists every session active in the last 5 minutes (or with an open call) at once, with whole-session totals and its kit stage, instead of only after its next line with totals counted from the restart.

**WITNESS**: Supervisor (main session, not the implementing agent), 2026-11-06 16:10–16:14 UTC, on the owner's `/verify HUN-237`: isolated real run (v237.sh) + verification command; Bash calls attributed to HUN-237 in `memory/event-trace/HUN-237.jsonl` via the active-task state file.
