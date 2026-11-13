# TASK_REVIEW — T051: Rust start-up ingest, MCP --http, context ingest

> Sibling of `tasks/TASK_GUIDE_T051.md`. Everything here is **filled by the reviewer at Stage
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

**BEFORE** (captured on `c245d89`, binary built from it, temp `EASY_PERSONAL_HOME` and agent root, before any T051 file existed; `store-info` lines are the transcript count of the store: a second transcript was added after the first ingest and `serve` did not copy it):

```
== 2026-11-13T08:21:12Z git rev: c245d89 (no implementation commit yet)
--- 1. ingest on an empty temp home
  "not_ingested": [
    "context"
  ],
  "skipped": 0,
exit=0
--- 2. mcp --transport http
error: unexpected argument '--transport' found

Usage: easy-personal-rs mcp

exit=2
--- 3. serve (3 s) over a store with a transcript not yet ingested
  events: 8
dashboard: http://127.0.0.1:45903/#FbbO1F1--G9OiyZdeY2TnUfiY0fsrnu3zP1WoNJ342M
Ctrl-C to stop.
events after serve started:
  events: 8
  kit_trace_events: 0
--- 4. app without --features window
this build has no window: rebuild with --features window
```

**AFTER** (same checks on the working tree of branch `T051`, a closed port as the easy-context URL so no real service is touched; `store-info` count: the transcript is now ingested by `serve` itself, 8 events with no `ingest` run first):

```
== 2026-11-13T10:01:05Z AFTER: working tree of branch T051 (debug binary built from it)
--- 1. ingest on an empty temp home
{
  "context": {
    "status": "no data"
  },
  "files": 1,
--- 2. mcp --transport http (port 0), one tools/list over HTTP, Ctrl-C
MCP over HTTP: http://127.0.0.1:34839/mcp
tools over HTTP: 12
mcp --host 0.0.0.0:
easy-personal-rs: refusing host "0.0.0.0": HTTP must bind 127.0.0.1 only
exit=2
--- 3. serve (3 s) over an empty home with a transcript
ingesting before start (waits if another ingest runs)...
dashboard: http://127.0.0.1:40229/#Xlyk3CJkPK4VBmJEmh62g0q3g
events in the store after serve started:
  events: 8
  kit_trace_events: 0
```

**DELTA**: `serve` and `app` ingest before they start (with `--no-ingest` to skip), `mcp --transport http` serves the 12 tools over loopback HTTP, and `ingest` reads easy-context's index freshness (`context` key) instead of listing it as not ingested.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
