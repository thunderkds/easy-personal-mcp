# TASK_REVIEW — HUN-69: Dashboard Workspaces page, form, legacy banner

> Sibling of `tasks/TASK_GUIDE_HUN-69.md`. Everything here is **filled by the reviewer at Stage
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
| | **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `web/src/WorkspacesView.test.jsx` (31 tests: cards AC1, form AC2, add connector + planted secret AC3/AC7, move/alias/delete/assign AC4, delete workspace AC5, legacy banner AC6); `web/src/test/fixtures.js`. `web vitest`: Test Files 7 passed (7), Tests 91 passed (91) |
| | Verification command run | pass | `npm --prefix web test && npm --prefix web run build && uv run pytest tests/surfaces tests/leak -q` -> vitest 91 passed; vite built (index-C520mTQ-.js 255.83 kB); pytest `144 passed, 1 warning in 24.18s`. Also `uv run pytest tests/test_web_toolchain.py tests/test_site_content.py tests/test_readme.py -q` -> `50 passed` |
| | Negative cases hold | pass | invalid name shows API message and keeps form; refused move shows API message, no Confirm; refusal clears secret; double click sends one request; legacy check failure shows no banner |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| | Full smoke suite still green (no regression) | pass | AgentsView/UsageView/App tests unchanged and green (91/91 vitest) |
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

**BEFORE** (captured 2026-10-26, before any implementation commit):

```
$ date -u
Mon Oct 26 10:39:04 AM UTC 2026
$ grep -n "Workspaces\|Connectors" web/src/App.jsx
3:import ConnectorsView from './ConnectorsView.jsx'
12:  { id: 'connectors', label: 'Connectors', View: ConnectorsView },
$ ls web/src/WorkspacesView.jsx
ls: cannot access 'web/src/WorkspacesView.jsx': No such file or directory
```

**AFTER** (2026-10-26, after implementation, worktree HUN-69):

```
$ ls web/src/WorkspacesView.jsx web/src/ConnectorsView.jsx
web/src/WorkspacesView.jsx   (ConnectorsView.jsx: removed)
$ grep -n "Workspaces\|Connectors" web/src/App.jsx
5:import WorkspacesView from './WorkspacesView.jsx'
12:  { id: 'workspaces', label: 'Workspaces', View: WorkspacesView },
```

**DELTA**: The owner can create, edit, rename and delete workspaces, add, alias, move and delete connectors per workspace, assign orphan connectors and adopt legacy ones from the Workspaces tab, each write showing its plan before Confirm.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
