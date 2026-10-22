# TASK_REVIEW — HUN-45: Connector tracer: Slack catalog entry + `connector add/remove` CLI → keychain → `claude mcp add-json` (snippet fallback) + `doctor`

> Sibling of `tasks/TASK_GUIDE_HUN-45.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE**: captured 2026-10-22T04:36:55Z in worktree easy-personal-mcp-HUN-45 before any implementation commit:

```
$ uv run easy-personal connector add slack
easy-personal: error: argument command: invalid choice: 'connector' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, kit, plan, dashboard, exec, headers)
rc=2
$ uv run easy-personal doctor
easy-personal: error: argument command: invalid choice: 'doctor' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, kit, plan, dashboard, exec, headers)
rc=2
```

**AFTER**: `uv run pytest tests/connectors -q` -> 50 passed; `uv run pytest tests/leak -q` -> 24 passed; `uv run pytest -q` -> 280 passed, 2 skipped (fake `claude` on PATH and in-memory keychain; the real ones are never run).

**DELTA**: `easy-personal connector add|remove slack` and `easy-personal doctor` now exist: secrets go to the keychain and `claude mcp add-json --scope user` registers `easy-personal-launch exec slack`.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
