# TASK_REVIEW — HUN-215: MCP prompt investigate (FR-075)

> Sibling of `tasks/TASK_GUIDE_HUN-215.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/surfaces/test_mcp_investigate.py` (11 tests, AC 1-5 + edge cases); `uv run pytest tests/surfaces/test_mcp_investigate.py -q` -> `11 passed in 1.56s` |
| Verification command run | ☑ pass | `uv run pytest tests -q -k mcp` -> `47 passed, 1340 deselected, 1 warning in 7.97s`; `uv run pytest -q` -> `1378 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 255.59s` |
| Negative cases hold | ☑ pass | quotes/newlines/backslash scope values stay on one JSON-quoted line; unknown workspace still returns; no scope names no scope; socket.connect guarded |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Stage 4 code-review (Supervisor, 2026-11-04): diff 2a23cf0..HEAD = src/easy_personal/surfaces/mcp.py (one constant + one prompt), tests/surfaces/test_mcp_investigate.py, README.md; no caller change. lane_check: `lane: C1, 3 files, 131 lines, FITS`. Startup reads line has the three Permanent-Rule reads. Findings: P0 0, P1 0, P2 1: with a scope given, `usage_breakdown` and `session_outliers` take no scope, so the model may mix scoped warnings with all-account numbers; the Scope line should say those two cover every account (accepted deviation from AC 3: those tools have no scope argument, and tool changes are out of scope). Security: fixed text, scope values JSON-quoted, no model/network call (test guards socket.connect), no new dependency — no finding. Supervisor rerun `uv run pytest tests -q -k mcp` → `47 passed, 1340 deselected, 1 warning in 2.78s`. |
| Full smoke suite still green (no regression) | ☑ pass | agent: `uv run pytest -q` → `1378 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 255.59s` |
| **Companion score (easy-verifier)** | ☐ N/A | not run for this C1 prompt-only change: the two runs today (HUN-212, HUN-213) returned the same repo-level 50 for unrelated diffs, so it carries no signal here |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-11-04T11:50:02Z, current code, no implementation commit yet): stdio probe
`EASY_PERSONAL_HOME=<empty temp dir> uv run python <scratchpad>/stdio_prompts.py uv run easy-personal mcp`
(initialize, then `prompts/list`, then `prompts/get investigate`):

```
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "prompts": []
  }
}
stderr tail: ... ValueError: Unknown prompt: investigate
```

First new test, failing (2026-11-04T11:50:45Z):
`uv run pytest tests/surfaces/test_mcp_investigate.py -q -x` ->
`E       KeyError: 'investigate'`
`FAILED tests/surfaces/test_mcp_investigate.py::test_prompt_is_listed_with_two_optional_arguments`
(without `-x`: `11 failed in 2.08s`)

**AFTER** (2026-11-04T11:55:18Z, isolated `uv tool install --reinstall --no-cache .` with UV_TOOL_DIR/UV_TOOL_BIN_DIR in a temp dir, EASY_PERSONAL_HOME an empty temp dir; same stdio probe):
`prompts/list` -> `{"prompts":[{"name":"investigate","description":"Investigate your Claude Code usage: causes of each warning and up to 5 ranked fixes.","arguments":[{"name":"workspace","required":false},{"name":"account","required":false}]}]}`
`prompts/get investigate` with workspace=`Acme "x"\nline2`, account=`work` -> one `user` text message: the fixed 6-step text, ending with
`Scope: pass workspace="Acme \"x\"\nline2" and account="work" to every tool call that takes a scope (`warnings`, `improvement_plan`).`
`prompts/get investigate` with no arguments -> the same text without the Scope line.

**DELTA**: In Claude Code the owner can run `/mcp__easy-personal__investigate` and have their own session investigate the usage data and suggest ranked fixes; before, `prompts/list` was empty.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-215.jsonl`, never the
implementing agent alone]
