# TASK_REVIEW — HUN-214: Warning hits carry causes + diagnosis (FR-073)

> Sibling of `tasks/TASK_GUIDE_HUN-214.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-04T11:50:21Z, before any implementation commit; `EASY_PERSONAL_HOME=<scratchpad>/home` = `sqlite3 .backup` copy of the owner store, run from the worktree with `uv run easy-personal warnings`; abstained: ['context_index_stale']). Only `tokens_jump` fired; its steps are the generic ones, no `causes`, no `diagnosis`:

```
[
 {
  "fix": "Open Usage and find which project or session drove the rise. Decide if the rise is intended before it becomes your baseline.",
  "label": "aware",
  "measured": 2.026,
  "priority": 3,
  "rule_id": "tokens_jump",
  "steps": [
   "Open Usage and find which project or session drove the rise.",
   "Decide if the rise is intended before it becomes your baseline."
  ],
  "summary": "Token use rose 202.6% in the last 7 days against the 7 days before; target 50% or less",
  "threshold": 0.5,
  "title": "Token use jumped",
  "why": "A sudden rise is easy to miss and quickly becomes the new normal, along with its cost.",
  "window_days": 7
 }
]
```

Timings of the same command, 3 runs incl. `uv` start: 4.29 s, 3.47 s, 5.12 s. In-process `rules.evaluate` on the copy, 3 runs: 2.09 s, 1.56 s, 1.53 s (median 1.56 s); `plan.improvement_plan`: 1.75 s, 1.90 s, 2.43 s (median 1.90 s).

First new test failing (2026-11-04, before the change): `uv run pytest tests/core/test_rules.py -q -k more_sessions`

```
>       assert hit["diagnosis"] == "more_sessions"
               ^^^^^^^^^^^^^^^^
E       KeyError: 'diagnosis'
tests/core/test_rules.py:428: KeyError
FAILED tests/core/test_rules.py::test_tokens_jump_more_sessions_names_the_three_projects_with_the_largest_rise
1 failed, 44 deselected in 0.77s
```

**AFTER** (2026-11-04, same store copy, same command `uv run easy-personal warnings`; the same single hit, `measured` 2.026 unchanged; abstained ['context_index_stale']):

```
[
 {
  "cache_read_share": 0.9691,
  "causes": [
   {
    "kind": "project",
    "name": "easy-verifier-mcp",
    "new": false,
    "sessions_before": 2,
    "sessions_now": 11,
    "text": "easy-verifier-mcp: 8.6M \u2192 429.8M tokens, 2 \u2192 11 sessions",
    "tokens_before": 8644070,
    "tokens_now": 429793012
   },
   {
    "kind": "project",
    "name": "personal-agentic-claude",
    "new": false,
    "sessions_before": 39,
    "sessions_now": 89,
    "text": "personal-agentic-claude: 500.6M \u2192 919.4M tokens, 39 \u2192 89 sessions",
    "tokens_before": 500612368,
    "tokens_now": 919382006
   },
   {
    "kind": "project",
    "name": "idea-radar",
    "new": true,
    "sessions_before": 0,
    "sessions_now": 42,
    "text": "idea-radar: 0 \u2192 376.1M tokens, 0 \u2192 42 sessions (new)",
    "tokens_before": 0,
    "tokens_now": 376132068
   }
  ],
  "diagnosis": "more_sessions",
  "fix": "The rise comes from more sessions (100 \u2192 395), not bigger ones: tokens per session went from 12.5M to 9.6M. New since the period before: idea-radar. Check that it should run this often. Biggest rise: easy-verifier-mcp (+421.1M tokens). Decide if that pace is intended before it becomes your baseline.",
  "label": "aware",
  "measured": 2.026,
  "priority": 3,
  "rule_id": "tokens_jump",
  "steps": [
   "The rise comes from more sessions (100 \u2192 395), not bigger ones: tokens per session went from 12.5M to 9.6M.",
   "New since the period before: idea-radar. Check that it should run this often.",
   "Biggest rise: easy-verifier-mcp (+421.1M tokens). Decide if that pace is intended before it becomes your baseline."
  ],
  "summary": "Token use rose 202.6% in the last 7 days against the 7 days before; target 50% or less",
  "threshold": 0.5,
  "title": "Token use jumped",
  "why": "A sudden rise is easy to miss and quickly becomes the new normal, along with its cost.",
  "window_days": 7
 }
]
```

Timings: CLI 3 runs incl. `uv` start 2.16 s, 2.10 s, 2.21 s (before 4.29, 3.47, 5.12 s on a busier machine). In-process `rules.evaluate`: 1.43 s, 1.04 s, 1.04 s (median 1.04 s; before 1.56 s); `plan.improvement_plan`: 1.03 s, 0.94 s, 0.95 s (median 0.95 s; before 1.90 s). The token rules now share two window scans, so the run is faster than before although it names causes.

**DELTA**: The owner sees, for the `tokens_jump` hit, that the rise is 100 -> 395 sessions (not bigger ones), which 3 projects drove it and that `idea-radar` is new, with fix lines that say so, instead of the generic line.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/HUN-214.jsonl`, never the
implementing agent alone]
