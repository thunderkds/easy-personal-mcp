# TASK_REVIEW — HUN-71: QA end-to-end across fake accounts

> Sibling of `tasks/TASK_GUIDE_HUN-71.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | tests/e2e/test_workspaces_flow.py, tests/e2e/e2e_support.py, tests/e2e/e2e_filekeyring.py |
| Verification command run | pass | `uv run pytest tests/e2e tests/leak -q && uv run pytest -q`: 47 passed, 4 xfailed, 1 warning in 81.83s (0:01:21) / 730 passed, 2 skipped, 4 deselected, 4 xfailed, 1 warning in 150.55s (0:02:30) |
| Negative cases hold | pass | refusals (dup name, orphan pick, Codex pick, move clash, bad alias, wrong token) asserted on CLI and API |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | pass | full suite: 730 passed, 2 skipped, 4 deselected, 4 xfailed, 1 warning in 150.55s (0:02:30); ruff check: All checks passed |
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

**BEFORE** (captured before the first commit):

```
$ date -u
Mon Oct 26 10:38:50 AM UTC 2026
$ ls tests/e2e
ls: cannot access 'tests/e2e': No such file or directory
$ uv run pytest -q | tail -1
709 passed, 2 skipped, 4 deselected, 1 warning in 51.75s
```

**AFTER**: `uv run pytest tests/e2e tests/leak -q` -> see Evidence (tests/e2e now exists).

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
