# TASK_REVIEW — HUN-166: Live trace E1: live tail of Claude Code transcripts

> Sibling of `tasks/TASK_GUIDE_HUN-166.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `experiments/live-trace/tests/test_e1.py` (11 tests: half line, fields, subagent file mid-session, EOF start, unknown type, resume by offset, truncation, planted secret, thread + score, scan, >1 MB line). `uv run pytest experiments/live-trace/tests -q -k e1` -> `11 passed, 12 deselected in 0.06s` (2026-10-31, QA agent). AC 2-4 are live runs: `e1/runs/*.json`, `e1/RESULT.md` |
| Verification command run | pass | `uv run pytest experiments/live-trace/tests -q -k e1` -> `11 passed, 12 deselected in 0.06s`; whole spike folder `uv run pytest experiments/live-trace/tests -q` -> 23 passed (12 harness + 11 e1) |
| Negative cases hold | pass | half line never parsed (0 bad JSON), unknown record type abstained and counted, bad JSON line counted not emitted, planted `sk-ant-FAKE-SECRET-123` and fake prompt sentence absent from events/stats; live: 8 unknown record types abstained, 36-67 per run |
| verify | ☑ pass | Supervisor 2026-10-31: verify recipe from the HUN-166 worktree — `easy-personal --version` → `0.2.0`, `ping` → `easy-personal core ok`, MCP stdio `tools/call ping` → `easy-personal core ok`; diff touches only `experiments/live-trace/` + review files (no `src/`), so product behaviour is unchanged; spike suite `23 passed in 0.06s` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor code-review 2026-10-31 over the branch diff vs HUN-165 (only experiments/live-trace/ + this review file). Content scan of stored runs for emails, scratch-file text, planted key: 0 hits. Inline security pass (Medium risk): subprocess list args, no shell=True; no credential reads; no ~/.claude* writes. Findings: P0 0, P1 0; P3: 16 ruff style findings (SIM115 etc.) in spike files, left — no lint gate on experiments/. Tail reuses src reader read-only (no src diff) — pass |
| Full smoke suite still green (no regression) | pass | `uv run pytest -q` -> `994 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 136.92s` |
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

**BEFORE** (captured 2026-10-31T15:48:52Z, before any HUN-166 implementation commit; HEAD 696e23d):

```
$ ls experiments/live-trace/e1 2>&1
ls: cannot access 'experiments/live-trace/e1': No such file or directory
$ uv run pytest experiments/live-trace/tests -q -k e1 2>&1 | tail -3
Installed 43 packages in 28ms

12 deselected in 0.05s
```

**AFTER** (2026-10-31T16:10:26Z):

```
$ ls experiments/live-trace/e1
RESULT.md  __pycache__  run_e1.py  runs  tail.py
$ uv run pytest experiments/live-trace/tests -q -k e1 2>&1 | tail -3
...........                                                              [100%]
11 passed, 12 deselected in 0.14s
```

**DELTA**: The spike can tail live transcripts and report capture (45/45 tool_use) and latency (median 196.5 ms, p95 1352.8 ms) against the transcript reference.

**WITNESS**: Supervisor re-ran `uv run pytest experiments/live-trace/tests -q` in the HUN-166 worktree on 2026-10-31 → `23 passed in 0.06s`, and grepped the stored run data for emails, scratch-file text and the planted key → 0 hits.
