# TASK_REVIEW — HUN-225: easy-context not in use → rule and kit part left out (FR-080 server)

> Sibling of `tasks/TASK_GUIDE_HUN-225.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_context_in_use.py` (22 tests: AC 1-6), plus `tests/sources/test_context.py`, `tests/core/test_rules.py`, `tests/core/test_reports.py`, `tests/surfaces/test_warnings.py`, `tests/sources/test_kit_trace.py` updated and the context lines of `tests/surfaces/scope_golden.json` / `hun186_golden.json` (AC 7). Implementer run: `uv run pytest tests -q -k "rules or plan or warnings or kit or report or context or leak"` -> `426 passed, 1095 deselected`; `uv run pytest -q` -> see next row |
| Verification command run | ☑ pass | implementer, 2026-11-05: full suite `1512 passed, 2 skipped, 6 deselected, 1 xfailed` (baseline before the change: `1490 passed, 2 skipped, 6 deselected, 1 xfailed`; +22 new tests) |
| Negative cases hold | ☑ pass | call 8 days old, call after `now`, `mcp__easy-context-old__`, `mcp__easy-contextual__`, `mcp__easy-memory__`, abstained row, call in another workspace/account: all "not in use" |
| verify | ☑ pass | Supervisor combined real run 2026-11-05T07:45-08:00Z: detached scratch tree of HUN-226 + HUN-225 src files, dashboard on a fresh backup of ~/.easy-personal/store.sqlite. /api/warnings abstained: []; /api/reports/1 rules = cache_hit_low, project_dominates, records_unreadable, session_huge_no_compact, tokens_jump, tool_errors_high (no context_index_stale); Kit page: 'Code index' in text false — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor Stage 4 code-review inline: src diff (context.py in_use, rules.py left_out + evaluate skip, reports.py NOT_IN_USE/drop, kit.py) + tests + goldens (only context 'no data'->'not in use' and dropped context abstain). lane_check: `lane: C2, 12 files, 339 lines, FITS; unpredicted: tests/... and goldens` (test files only). P0-P3: none. Security-review N/A beyond inline (Low risk, read-only query, scoped by metrics._scope). Pass |
| Full smoke suite still green (no regression) | ☑ pass | Supervisor rerun on branch 146b9d7: `uv run pytest -q` -> `1512 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 259.82s` |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier score not run for this C2 Low-risk change; review done inline |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | backend only (page proof under HUN-226) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | backend only |
| **UI: Responsiveness at target viewports** | ☑ N/A | backend only |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-05T05:21Z UTC, on a `.backup` copy of the owner store, `EASY_PERSONAL_HOME` = that copy's dir, code at commit 18ae74b, before any implementation commit):

`easy-personal warnings` (as_of 2026-11-05T05:21:26Z), `abstained` list:
```
"abstained": [
  {"reason": "no easy-context data (service down or no index for a known project)",
   "rule_id": "context_index_stale", "title": "Code index is out of date"}
]
```
`reports.get_report(conn, 1)`, the `context_index_stale` entry (rule ids in the report: cache_hit_low, context_index_stale, project_dominates, records_unreadable, session_huge_no_compact, tokens_jump, tool_errors_high):
```
{"rule_id": "context_index_stale",
 "before": {"state": "abstained", "reason": "no easy-context data (service down or no index for a known project)"},
 "now":    {"state": "abstained", "reason": "no easy-context data (service down or no index for a known project)"}}
```
`kit.kit_summary(conn)`, project `easy-personal-mcp` (144 projects; all 144 had the same part): `"context": {"status": "no data"}`

**AFTER** (captured 2026-11-05T05:33Z UTC, fresh `.backup` copy of the owner store, same `EASY_PERSONAL_HOME` method):

`easy-personal warnings`: `"abstained": []`; the text `context_index_stale` appears 0 times in it; `easy-personal plan` also 0 times.
`GET /api/reports/1` (Starlette TestClient on `api.create_app`, token header) -> 200; rule ids: `['cache_hit_low', 'project_dominates', 'records_unreadable', 'session_huge_no_compact', 'tokens_jump', 'tool_errors_high']`; `context_index_stale present: False` (it was abstained then and is not in use now).
`GET /api/kit`: 20 projects (dashboard view, gone folders hidden); `easy-personal-mcp` -> `"context": {"status": "not in use"}`; the only distinct context part is `{"status": "not in use"}`.

**DELTA**: With no easy-context tool call in the last 7 days, the owner no longer sees the "Code index is out of date" warning, plan item, abstained entry, saved-report line or Kit context part; they all return on their own after the next call.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
