# TASK_REVIEW — HUN-169: Live trace E4: stream-json capture from headless runs

> Sibling of `tasks/TASK_GUIDE_HUN-169.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `experiments/live-trace/tests/test_e4.py` (12 tests: per-line record, result fields and 2 result lines, planted secret, stream_event names, killed run, evaluate capture %/latency, latency summary, lock, forbidden env, cmd flags, bad lines) |
| Verification command run | ☑ pass | `uv run pytest experiments/live-trace/tests -q -k e4` (2026-10-31T16:09:30Z): ............                                                             [100%] 12 passed, 12 deselected in 2.81s |
| Negative cases hold | ☑ pass | killed run (partial last line, no result), unparsed and non-dict lines, forbidden OTEL_LOG env refused, planted secret and prompt sentence absent from stored record, e4/runs/ and RESULT.md |
| verify | ☑ pass | 8 live Haiku sessions (5 measured, 2 partial, 1 resume): 45/45 and 18/18 tool_use ids captured, 100 % — pass; numbers in `experiments/live-trace/e4/RESULT.md` |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Only `experiments/live-trace/e4/` and `tests/test_e4.py` added; imports harness read-only; `src/`, `.claude*` untouched |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q`: 994 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 139.01s |
| **Companion score (easy-verifier)** | ☑ N/A | not run (spike task, no companion tool run) |
| **UI: Visual regression** | ☑ N/A | no UI component |
| **UI: Design-system compliance** | ☑ N/A | no UI component |
| **UI: Responsiveness at target** | ☑ N/A | no UI component |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured before any implementation commit):

```
# 2026-10-31T15:48:47Z
$ ls experiments/live-trace/e4 2>&1
ls: cannot access 'experiments/live-trace/e4': No such file or directory
$ uv run pytest experiments/live-trace/tests -q -k e4 2>&1 | tail -3
Installed 43 packages in 30ms

12 deselected in 0.04s
```

**AFTER** (2026-10-31T16:09:30Z):

```
$ ls experiments/live-trace/e4
RESULT.md  capture.py  runs
$ uv run pytest experiments/live-trace/tests -q -k e4 2>&1 | tail -3
............                                                             [100%] 12 passed, 12 deselected in 2.81s
```

**DELTA**: The spike can now capture a headless claude -p stream-json run as metadata-only records and compare it with the transcript: 100 % of tool_use ids, median 1.8 ms and p95 4.4 ms latency, result-line fields and interactive answer in RESULT.md.

**WITNESS**: qa-expert agent (HUN-169), 2026-10-31T16:09:30Z; commands run in the HUN-169 worktree. Not derived from memory/event-trace (spike worktree).
