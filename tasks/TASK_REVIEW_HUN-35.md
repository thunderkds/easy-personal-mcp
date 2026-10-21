# TASK_REVIEW — HUN-35: usage_breakdown + session_outliers

> Sibling of `tasks/TASK_GUIDE_HUN-35.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
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

**BEFORE**: captured 2026-10-21T10:52:01Z in the HUN-35 worktree, before any implementation commit:

```
$ uv run pytest tests/core/test_metrics.py tests/core/test_prices.py tests/surfaces -q
ERROR: file or directory not found: tests/core/test_metrics.py
no tests ran in 0.00s
$ uv run easy-personal usage --by day
easy-personal: error: unrecognized arguments: --by day
$ MCP tool list (create_app().list_tools())
['ping', 'usage_summary']
```

**AFTER**: captured 2026-10-21T10:53:34Z, same worktree, fixtures ingested into a temp store:

```
$ uv run pytest tests/core/test_metrics.py tests/core/test_prices.py tests/surfaces -q
29 passed in 0.59s
$ uv run easy-personal usage --by model   # (abridged)
groups[0]: key claude-fixture-model-1, input_tokens 15, output_tokens 8, messages 3,
  api_equivalent_cost {usd: null, price_table_date: "2026-10-21", abstained_models: ["claude-fixture-model-1"]}
total: same tokens as usage_summary
$ uv run easy-personal outliers   -> {"sessions": [], "thresholds": {"context_tokens": 200000, "tool_calls": 90}}
MCP tools: ping, usage_summary, usage_breakdown, session_outliers
```

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-35.jsonl`, never the
implementing agent alone]
