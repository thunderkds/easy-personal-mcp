# TASK_REVIEW — HUN-39: kit_trace source + tool_errors_high rule + kit_summary (kernel part)

> Sibling of `tasks/TASK_GUIDE_HUN-39.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `tests/sources/test_kit_trace.py` (AC1 schema columns + dump, AC2 `test_untagged_rows_have_no_kit_task_id`, AC3 `test_kit_summary_counts_and_no_data`, AC4 `test_tool_errors_high_boundary`, AC5 `test_incremental_per_file_and_partial_last_line`); leak suite extended (`tests/leak/canaries.py` trace_summary canary, `conftest.py`, `test_leak_suite.py`). Supervisor re-run 2026-10-21T14:55:33Z: `uv run pytest tests/sources/test_kit_trace.py tests/core/test_rules.py tests/leak -q` → `52 passed in 7.30s` |
| Verification command run | ☒ pass / ☐ fail | `52 passed in 7.30s` (Supervisor re-run, worktree HUN-39, 2026-10-21T14:55:33Z) |
| Negative cases hold | ☒ pass / ☐ fail | 10/100 and 9/100 errors do not fire; 8-day-old calls ignored; no calls → abstain; malformed lines abstain; 5 MB `summary` line; deleted project → `missing_on_disk`; summary canary absent from store, MCP, CLI, stdio |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | 17 files in `main...HUN-39` + callers of `store.prune`, `rules.METRICS`, `cli._ingest`. lane_check: `C2, 16 files, 419 lines, EXCEEDS (files 16 > 14)` — 8 are tests; C2 kept. code-review 0 P0/P1 code, 1 P1 process (partial read of `agents/backend.md`, closed by Supervisor conformance check), 1 P3 (`prune` docstring). migration-safety **GO** (additive `kit_trace_events` + `kit_trace_ts`; up→down→up on a fixture store, 31 events kept; down must also delete `source_files` rows with source `kit_trace`). security (inline) pass, no findings. |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `155 passed, 2 skipped in 16.51s` (2026-10-21T14:55Z) |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-21, MCP `score` repo=HUN-39 worktree, scope=changes, ref=`main...HUN-39`: overall **81** (architecture 100, code-quality 100, reliability 100, test-strategy 75, requirement-fidelity 67, blast-radius 65, security 59); abstained: solution-fit, supply-chain, api, data, operations, infrastructure. Security = `redaction_hits_observed` 119 (known false positive: lockfiles + planted canaries). Raw JSON ~/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/dbd9c4da-edf5-49bc-80a5-cb2b8c04f640/tool-results/mcp-easy-verifier-score-1792594495200.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-21T12:33:11Z on the unchanged tree, via `git stash -u`):

```
$ uv run pytest tests/sources/test_kit_trace.py tests/core/test_rules.py tests/leak -q
ERROR: file or directory not found: tests/sources/test_kit_trace.py
no tests ran in 0.00s
$ EASY_PERSONAL_HOME=<tmp> uv run easy-personal kit
easy-personal: error: argument command: invalid choice: 'kit' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, plan)
```

**AFTER** (2026-10-21T12:35:28Z, temp `EASY_PERSONAL_HOME`, fixture repo with one errored trace line):

```
$ easy-personal ingest --source <fixture>   ->  "kit_trace": {"files": 1, "new_events": 1}
$ easy-personal kit   ->  project "repo", parts.kernel: status ok, tool_calls 1, tool_errors 1, error_rate 1.0, kit_tasks 1
$ uv run pytest tests/sources/test_kit_trace.py tests/core/test_rules.py tests/leak -q  ->  52 passed
```

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
