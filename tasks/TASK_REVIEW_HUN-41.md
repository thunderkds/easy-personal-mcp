# TASK_REVIEW — HUN-41: `context` read-only MCP client of easy-context + index freshness + `context_index_stale`

> Sibling of `tasks/TASK_GUIDE_HUN-41.md`. Everything here is **filled by the reviewer at Stage
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

**Migration-safety checklist (done by hand, HUN-41 adds one table)**: additive only (`CREATE TABLE IF NOT EXISTS context_index (project_path PRIMARY KEY, last_index_at_utc)` in `core/store.py` SCHEMA, the project's migration mechanism; no ad-hoc SQL); no existing table altered; no DROP/RENAME/NOT NULL change/data rewrite; rollback = old code ignores the extra table (`DROP TABLE context_index` is lossless, rows are re-fetched at the next ingest); old and new code coexist (old code never reads it); no backfill, table is tiny (one row per project) so no lock concern; verified by `tests/core/test_store.py` table list and `tests/sources/test_context.py`. Verdict: GO.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-22T04:03:42Z, main code, before any implementation commit):

```
$ uv run python -c "import easy_personal.sources.context"
ModuleNotFoundError: No module named 'easy_personal.sources.context'
$ grep -c context_index_stale src/easy_personal/data/rules.yaml
0
$ kit_summary parts for one project (python -c ...)
['kernel', 'git']
```

**AFTER**: `uv run pytest tests/sources/test_context.py -q` -> `22 passed`; `rules.yaml` has `context_index_stale`; `kit_summary` parts are `kernel`, `git`, `context`.

**DELTA**: The owner sees per project when easy-context last indexed it, and a `context_index_stale` hint (call `index_path`) when commits are newer, without easy-personal ever triggering indexing.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
