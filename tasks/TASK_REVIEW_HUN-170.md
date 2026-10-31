# TASK_REVIEW — HUN-170: Live trace report: combined run, E5 docs-only result, REPORT.md

> Sibling of `tasks/TASK_GUIDE_HUN-170.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `experiments/live-trace/tests/test_report.py` (join/summary maths with a miss shown as n/N, aggregate, integrity check on the real RESULT.md/REPORT.md/e5.md, planted-secret grep). Script under test: `experiments/live-trace/report/check_report.py`. Live combined run: `experiments/live-trace/report/combined.py` (5 sessions, `report/runs/combined-1..5.json`). `uv run pytest experiments/live-trace/tests -q -k report` -> `23 passed, 58 deselected in 0.07s` (2026-10-31T16:23:35Z) |
| Verification command run | ☑ pass | `uv run pytest experiments/live-trace/tests -q -k report 2>&1 | tail -3` -> `.......................  [100%]` / `23 passed, 58 deselected in 0.07s`. Also `uv run python experiments/live-trace/report/check_report.py` -> `report check: ok (0 problem(s))` |
| Negative cases hold | ☑ pass | `test_report_check_fails_when_result_lacks_a_section[6 sections]`, `test_report_check_fails_when_comparison_lacks_a_source_row[E1..E5]`, `..._lacks_section_or_missing_file`, `..._e5_lacks_docs_only_marker`, `test_report_secret_grep_catches_a_leak_but_skips_tests_folder` all pass (check script fails when a section or a source row is removed). A source miss is kept in N: `test_report_summary_shows_n_of_N_and_never_hides_a_miss`; live run 5 shows 45/46 for E2 and E3 |
| verify | ☑ pass | Supervisor 2026-10-31: verify recipe from the HUN-170 worktree — `easy-personal --version` → `0.2.0`, `ping` → `easy-personal core ok`, MCP stdio `tools/call ping` → `easy-personal core ok`; diff touches only `experiments/live-trace/` + review files (no `src/`), so product behaviour is unchanged; spike suite `81 passed in 11.41s` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor code-review 2026-10-31 over git diff main..HUN-170 (only experiments/live-trace/ + this review file). Re-ran check_report.py → ok (0 problems); content scan of all stored files outside tests/ for emails, scratch text, planted key → 0 hits; raw per-source files stay in /tmp outside the repo (0 email hits). Findings: P0 0, P1 0; P3: combined.py monkey-patches capture.build_cmd to add --settings instead of extending E4's signature — acceptable for a spike; REPORT.md written by heredoc after the Write tool refused it in the subagent — content reviewed by Supervisor — pass |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest experiments/live-trace/tests -q` -> `81 passed in 11.93s` |
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

**BEFORE**: captured 2026-10-31T16:14:46Z by qa-expert, before any implementation commit:
```
$ ls experiments/live-trace/report experiments/live-trace/REPORT.md 2>&1
ls: cannot access 'experiments/live-trace/report': No such file or directory
ls: cannot access 'experiments/live-trace/REPORT.md': No such file or directory
$ uv run pytest experiments/live-trace/tests -q -k report 2>&1 | tail -3
Installed 43 packages in 7ms
.                                                                        [100%]
1 passed, 58 deselected in 0.15s
```
(the 1 passing test matched `report` by name from an earlier task; there is no report test file, report folder or REPORT.md.)

**AFTER**: 2026-10-31T16:23:35Z, same commands:
```
$ ls experiments/live-trace/report experiments/live-trace/REPORT.md 2>&1
experiments/live-trace/REPORT.md

experiments/live-trace/report:
__pycache__  check_report.py  combined.py  e5.md  runs
$ uv run pytest experiments/live-trace/tests -q -k report 2>&1 | tail -3
.......................                                                  [100%]
23 passed, 58 deselected in 0.07s
```

**DELTA**: The owner can read one recommendation for the live-collector source stack, backed by 5 combined sessions (E1+E2+E3+E4 on at once, 46 tool calls joined by tool_use_id), and a check script fails if a RESULT.md section or a REPORT.md source row goes missing.

**WITNESS**: qa-expert (HUN-170 agent), 2026-10-31T16:14Z (BEFORE) and 16:23Z (AFTER); the Supervisor should re-derive from `memory/event-trace/HUN-170.jsonl`.
