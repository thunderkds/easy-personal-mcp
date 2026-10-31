# TASK_REVIEW — HUN-167: Live trace E2: native async http hooks to a local collector

> Sibling of `tasks/TASK_GUIDE_HUN-167.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `experiments/live-trace/tests/test_e2.py` (9 tests: collector keys/2xx (AC1), settings template (AC2), analyze capture/latency (AC3), planted-secret + large payload + two sessions + subagent agent_id (edge cases)). AC3/AC4 live numbers in `experiments/live-trace/e2/RESULT.md`, run records in `e2/runs/`. `uv run pytest experiments/live-trace/tests -q -k e2` -> `9 passed, 12 deselected in 3.57s` |
| Verification command run | ☑ pass | `uv run pytest experiments/live-trace/tests -q -k e2` -> `9 passed, 12 deselected in 3.57s`; whole live-trace folder: `21 passed in 4.59s` |
| Negative cases hold | ☑ pass | `test_e2_bad_requests_rejected_without_storing` (not JSON/array -> 400, wrong path -> 404, nothing stored); collector-down runs: exit 0, 5 of 5 steps, 0 events |
| verify | ☑ pass | Supervisor 2026-10-31: verify recipe from the HUN-167 worktree — `easy-personal --version` → `0.2.0`, `ping` → `easy-personal core ok`, MCP stdio `tools/call ping` → `easy-personal core ok`; diff touches only `experiments/live-trace/` + review files (no `src/`), so product behaviour is unchanged; spike suite `21 passed in 4.60s` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor code-review 2026-10-31 over the branch diff vs HUN-165 (only experiments/live-trace/ + this review file). Content scan of stored runs for emails, scratch-file text, planted key: 0 hits. Inline security pass (Medium risk): subprocess list args, no shell=True; no credential reads; no ~/.claude* writes; collector binds 127.0.0.1, drops tool_input/tool_response/prompt/last_assistant_message before writing, >1 MB body dropped. Findings: P0 0, P1 0; P2 (doc, not code): settings use async: true on http hooks, which Claude Code ignores — recorded in RESULT.md as a finding — pass |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest tests -q -x` -> `994 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 139.54s` |
| **Companion score (easy-verifier)** | ☑ N/A | spike code under `experiments/`, not product code; companion not run |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI: headless spike scripts |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI: headless spike scripts |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI: headless spike scripts |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-31, before the first implementation commit):

```
2026-10-31T15:48:51Z
$ ls experiments/live-trace/e2 2>&1
ls: cannot access 'experiments/live-trace/e2': No such file or directory
$ uv run pytest experiments/live-trace/tests -q -k e2 2>&1 | tail -3
Installed 43 packages in 10ms

12 deselected in 0.07s
```

**AFTER**: see the Evidence table (same two commands, post-change): `e2/` holds collector.py, settings.json, run_e2.py, summarize.py, RESULT.md, runs/; 9 e2 tests pass.

**DELTA**: The spike can now run the standard session with `http` hooks posting to a local collector, and has measured capture, latency, overhead and collector-down behaviour for that source.

**WITNESS**: qa-expert agent, 2026-10-31 (BEFORE 15:48Z; sessions 15:50Z to 16:1xZ); Supervisor to confirm from the event trace.
