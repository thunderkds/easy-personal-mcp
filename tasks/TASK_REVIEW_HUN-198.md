# TASK_REVIEW — HUN-198: memory on vs off block in the easy-memory panel

> Sibling of `tasks/TASK_GUIDE_HUN-198.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | tests/core/test_easymem.py (AC1, AC2, edge cases), tests/surfaces/test_api_memory_graph.py (AC3), tests/core/test_memgraph.py, tests/leak/canaries.py + tests/leak/test_leak_suite.py (AC8), web/src/MemoryView.test.jsx (AC4-AC7). 166 passed (pytest group); vitest 502 passed. |
| Verification command run | pass | Exit 0. pytest group: `166 passed, 1 warning in 59.17s`; vitest: `Test Files 18 passed (18) / Tests 502 passed (502)`; build: `built in 340ms`; full: `1301 passed, 6 deselected, 1 xfailed, 1 warning in 197.46s`. (CSS spacing tweak afterwards: vitest re-run 502 passed, rebuilt.) |
| Negative cases hold | pass | tests/core/test_easymem.py: empty, `[]`, `{`, string, null, binary, deep nesting, 65 KB file, directory, missing file/folder, symlink outside the folder, `v: 2`, bool/string/negative/float counts, NaN pct, wrong `too_few` all return none/unsupported without raising; vitest: future/unparseable `generated`, pct shown only when `too_few` false. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| easy-verifier score (advisory) | ☐ recorded / ☐ N/A | [overall result, abstaining dimensions, report path — or `N/A — <reason>` if the server is down; never blocks Done] |
| **UI: Visual regression (diff or verdict pasted)** | pass | Headless Google Chrome 156 over CDP on a scratch copy of ~/.easy-memory, project easy-memory-mcp, Memory view. Block matches the mockup under the hit-rate line (real report: 3 on / 5 off, too_few). Fixture too_few false, pct -12: "Saves 12%". Screenshots: scratchpad/shots/too-few-1280x720.png, too-few-375x812.png, saves-1280x720.png, saves-375x812.png (scratchpad = /tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-memory-mcp/a28160ca-b136-45de-b0d3-bc36a9479dd9/scratchpad). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | pass | Reuses .memory-easy, .muted, compact(), plural(), ago(); the only CSS is two rules using --space-2 and --color-heading (no raw colour; the existing memory token test still passes). Block is role=group aria-label="memory on vs off" (vitest). |
| **UI: Responsiveness at target viewports** | pass | Chrome CDP: 1280x720 scrollWidth 1265 = clientWidth 1265 (group right 928 < panel right 949); 375x812 scrollWidth 375 = clientWidth 375 (group right 338 < panel right 359). No horizontal scroll. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-03T11:42:47Z, on branch HUN-198 before any implementation commit; store = scratch copy of `~/.easy-memory`, `EASY_MEMORY_HOME` pointed at the copy, project `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-memory-mcp`; the real store was not written):

```
EASY_MEMORY_HOME=<scratch copy of ~/.easy-memory> project_graph(easy-memory-mcp)['easy_memory']:
{"notes": 141, "abstained": 0, "days": "<30 days>", "prompts_on": 52, "injected": 35, "files_cited": 31, "files_not_drawn": 1, "outside": 0}
has 'report' key: False
```

**AFTER** (same capture, after the change):

```
2026-11-03T11:53:15Z
EASY_MEMORY_HOME=<scratch copy of ~/.easy-memory> project_graph(easy-memory-mcp)['easy_memory']:
{"notes": 141, "abstained": 0, "days": "<30 days>", "prompts_on": 52, "injected": 35, "files_cited": 31, "files_not_drawn": 1, "outside": 0, "report": {"state": "ok", "generated": "2026-11-03T11:41:35Z", "min_sessions": 10, "too_few": true, "on_vs_off_pct": null, "on": {"sessions": 3, "tokens_all_median": 23230411}, "off": {"sessions": 5, "tokens_all_median": 16553412}}}
report: {"state": "ok", "generated": "2026-11-03T11:41:35Z", "min_sessions": 10, "too_few": true, "on_vs_off_pct": null, "on": {"sessions": 3, "tokens_all_median": 23230411}, "off": {"sessions": 5, "tokens_all_median": 16553412}}
```

**DELTA**: The Memory view's easy-memory panel now shows sessions and median tokens with memory on and off (and the saving when there are enough sessions), read from easy-memory's report.json.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-198.jsonl`, never the
implementing agent alone]
