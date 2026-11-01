# TASK_REVIEW — HUN-175: Transcript reader recognises the record types new in Claude Code 2.1.316 (FR-046)

> Sibling of `tasks/TASK_GUIDE_HUN-175.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/sources/test_claude_code.py`: `test_2_1_316_metadata_types_are_known_not_abstained` (AC1), `test_2_1_316_metadata_types_keep_only_type_session_and_time` (AC2), `test_unknown_type_beside_2_1_316_types_still_abstained` (AC3). Leak: `tests/leak/canaries.py` (`ai_title`, `request_blob`) + `tests/fixtures/leak/session.jsonl.tmpl`. Fixture `tests/fixtures/claude_code/2.1.316/meta.jsonl`. `tests/surfaces/test_usage_summary.py` file count 5 to 6 (new fixture file). |
| Verification command run | ☑ pass | `uv run pytest tests/sources -q && uv run pytest tests/leak -q` gave `137 passed, 1 deselected in 6.85s` and `29 passed, 1 warning in 25.79s` |
| Negative cases hold | ☑ pass | `test_unknown_type_beside_2_1_316_types_still_abstained`: invented `future-316-type` is abstained, `by_version` `{"2.1.316": 1}`; planted `INVENTED_*` values absent from the full SQLite dump. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` gave `997 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 115.15s` |
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

**BEFORE**: re-captured 2026-11-01T01:52:48Z by the resuming agent, before any implementation commit (the branch has no commits). Command: `EASY_PERSONAL_HOME=<scratch> PYTHONPATH=<scratch>/before/src uv run --no-sync python <scratch>/demo175.py tests/fixtures/claude_code/2.1.316`, where `<scratch>/before/src` is `git archive HEAD src` (unmodified `claude_code.py`):

```
# 2026-11-01T01:52:48Z module: <scratch>/before/src/easy_personal/sources/claude_code.py
ai-title             abstained=1 count=1
api-request          abstained=1 count=1
api-request-blob     abstained=1 count=1
api-request-shape    abstained=1 count=1
bridge-session       abstained=1 count=1
mode                 abstained=1 count=1
permission-mode      abstained=1 count=1
tools-used           abstained=1 count=1
total abstained: 8
```

(The previous agent's capture of 2026-10-31T16:35:39Z gave the same 8 abstained; it predates the last fixture edit, so it was replaced.)

**AFTER**: same command with the working-tree `claude_code.py` (2026-11-01T01:55:29Z):

```
# 2026-11-01T01:55:29Z module: .../easy-personal-mcp-wt/HUN-175/src/easy_personal/sources/claude_code.py
ai-title             abstained=0 count=1
api-request          abstained=0 count=1
api-request-blob     abstained=0 count=1
api-request-shape    abstained=0 count=1
bridge-session       abstained=0 count=1
mode                 abstained=0 count=1
permission-mode      abstained=0 count=1
tools-used           abstained=0 count=1
total abstained: 0
```

**DELTA**: A Claude Code 2.1.316 transcript no longer shows its 8 new metadata record types as abstained, and no text from them is stored.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
