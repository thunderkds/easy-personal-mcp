# TASK_REVIEW — HUN-49: Dashboard server — on demand, free port, token + Host/Origin guard, CSP, read API

> Sibling of `tasks/TASK_GUIDE_HUN-49.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE**: Captured 2026-10-21T12:31:56Z in the HUN-49 worktree, before any implementation commit.
`uv run easy-personal dashboard` ->
```
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,warnings,plan} ...
easy-personal: error: argument command: invalid choice: 'dashboard' (choose from ping, mcp, ingest, schedule, usage, outliers, warnings, plan)
```
`uv run pytest tests/surfaces/test_api.py -q` -> `no tests ran in 0.00s` (file absent).

**AFTER**: Runtime check with fixture transcripts (BROWSER=true, 6 s timeout):
`easy-personal dashboard` printed `http://127.0.0.1:43661/#<token>`; `GET /` -> 200 with `content-security-policy: default-src 'self'`; `/api/usage/summary` without token -> 403; with `Host: evil.example` -> 403; with token -> 200 `{"sessions":4,"messages":3,...}`. Token appears only in the printed URL (stdout), not stderr. `uv run pytest tests/surfaces/test_api.py tests/leak -q` -> 35 passed.

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
