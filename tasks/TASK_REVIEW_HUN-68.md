# TASK_REVIEW — HUN-68: Dashboard Agents page

> Sibling of `tasks/TASK_GUIDE_HUN-68.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `web/src/AgentsView.test.jsx` (13 tests: AC1-AC6 + edge cases) |
| Verification command run | pass | `npm --prefix web test`: Test Files 7 passed (7), Tests 63 passed (63); `npm --prefix web run build`: built in 606ms; `uv run pytest tests/surfaces -q tests/test_web_toolchain.py`: 126 passed, 1 warning |
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

**BEFORE**:
```
$ date -u
Mon Oct 26 10:16:16 AM UTC 2026
$ grep -n "Agents" web/src/App.jsx
(exit 1: no match, no Agents tab)
$ ls web/src/AgentsView.jsx
ls: cannot access 'web/src/AgentsView.jsx': No such file or directory
```

**AFTER**: `grep -n Agents web/src/App.jsx` now shows the import and the `{ id: 'agents', label: 'Agents', View: AgentsView }` tab; `web/src/AgentsView.jsx` exists; Vitest 63 passed.

**DELTA**: The owner can see agent accounts by account or by provider, confirm or ignore detected folders, and pick workspaces for a Claude account with a plan shown before anything is written.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
