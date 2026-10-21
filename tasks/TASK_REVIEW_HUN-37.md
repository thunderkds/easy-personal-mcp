# TASK_REVIEW — HUN-37: Rules engine + 5 usage rules + warnings + improvement_plan

> Sibling of `tasks/TASK_GUIDE_HUN-37.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | New: `tests/core/test_rules.py` (AC1 boundary fire/ok for all 5 rules: cache_hit_low, session_huge_no_compact 150001/150000, tokens_jump, project_dominates 61/60, records_unreadable 3/2; AC2 `test_hit_carries_id_label_measured_threshold_fix`; AC3 `test_threshold_comes_from_config`, unknown/missing config; AC4 `test_plan_groups_by_priority_and_is_deterministic`; abstain on short history / empty store), `tests/surfaces/test_warnings.py` (MCP/CLI parity for warnings and plan, config changes CLI output, bad config → exit 2 ×4). Leak registry updated in `tests/leak/harness.py`. |
| Verification command run | ☒ pass / ☐ fail | 2026-10-21T11:13Z Supervisor, after merging main (HUN-38): `uv run pytest -q` → `142 passed, 2 skipped in 19.33s`; `uv run ruff check` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | `cache_hit_low: abc` in config → `easy-personal: config thresholds must be numbers: <path>`, exit 2; `no_such_rule: 1` → `easy-personal: config <path>: thresholds for unknown rules: no_such_rule`, exit 2; empty window / short history → rule listed in `abstained` with a reason, never a pass; values exactly at a threshold do not fire (boundary tests) |
| verify | ☒ pass / ☐ fail / ☐ N/A | 2026-10-21T11:13Z Supervisor, verify recipe (temp `EASY_PERSONAL_HOME`, fixtures): `ingest` → 5 files, 31 events; `warnings` → hits [], all 5 rules abstained with reasons (fixtures are dated 2026-01, outside today's 7-day window — correct abstain, not pass); `plan` → `{"groups": []}`; bad config cases → exit 2 one-line errors; MCP stdio `tools/list` → `['ping', 'usage_summary', 'usage_breakdown', 'session_outliers', 'warnings', 'improvement_plan']`, `warnings` and `improvement_plan` isError False; 0 `INVENTED_` hits. Firing is proven by the boundary tests — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | 11 files in `467be92...cc54d7b` + callers (`surfaces/cli.py`, `surfaces/mcp.py`, `core/metrics.py` users). lane_check: `C2, 11 files, 645 lines, EXCEEDS (lines 645 > 500)`, accepted at C2. code-review 0 P0/P1, 2 P2 (compaction inferred from a context drop — owner to confirm; bad-config traceback — fixed in cc54d7b), 1 P3 (fix text hard-codes 150k) — `reports/code-review_HUN-37_20261021T181402.html`; security (inline) pass — `reports/security-review_HUN-37_20261021T181402.html` (main checkout) |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `142 passed, 2 skipped`; `uv run pytest tests/leak -q` → `18 passed` with HUN-37 and HUN-38 surfaces registered |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-21T11:14Z, server at 127.0.0.1:8766 called directly over HTTP, `score` repo=HUN-37 worktree, scope=changes, ref=`467be92...cc54d7b`: overall **73** (7 of 13: architecture 100, requirement-fidelity 67, code-quality 87, security 18, test-strategy 75, blast-radius 65, reliability 100); abstained: solution-fit, supply-chain, api, data, operations, infrastructure. Security 18 = known false positives: `sink_hits_observed` 9 (f-string SQL inserts only the `_period` predicate; values bound — the 2 new ones in `session_contexts` / `record_readability` checked) and `redaction_hits_observed` 121 (lockfile pack). Report `~/.easy-verifier-reports/easy-personal-mcp-HUN-37/evidence-report-changes-20261021T111402-403471Z.html`. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task (no UI component) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task (no UI component) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task (no UI component) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-21 before any implementation commit (temp EASY_PERSONAL_HOME):

```
2026-10-21T11:03:54Z
$ uv run pytest tests/core/test_rules.py tests/surfaces -q
Creating virtual environment at: .venv
   Building easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-37
      Built easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-37
Installed 43 packages in 14ms
ERROR: file or directory not found: tests/core/test_rules.py


no tests ran in 0.00s
$ uv run easy-personal warnings
usage: easy-personal [-h] [--version] {ping,mcp,ingest,usage,outliers} ...
easy-personal: error: argument command: invalid choice: 'warnings' (choose from ping, mcp, ingest, usage, outliers)
```

**AFTER**: same commands post-change (fixtures only):

```
2026-10-21T11:07:23Z
$ uv run pytest tests/core/test_rules.py tests/surfaces -q
.........................................                                [100%]
41 passed in 0.90s
$ easy-personal ingest; easy-personal warnings (fixtures)
{
  "abstained": [
    {
      "reason": "no prompt tokens in the window",
      "rule_id": "cache_hit_low"
    },
    {
      "reason": "fewer than two projects with tokens in the window",
      "rule_id": "project_dominates"
    },
    {
      "reason": "no records in the window",
      "rule_id": "records_unreadable"
    },
```

**DELTA**: The owner can run `easy-personal warnings` / `plan` (or the MCP `warnings` / `improvement_plan`) and see which of the 5 declared usage rules fired, with value, threshold and fix line.

**WITNESS**: Supervisor, 2026-10-21T11:13–11:14Z — verify recipe and test runs in the HUN-37 worktree (attributed to HUN-37 via the active_task state file; `memory/event-trace/HUN-37.jsonl`).
