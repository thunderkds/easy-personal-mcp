# TASK_REVIEW — HUN-43: `ui` source: easy-ui UI checks overall (never `target`) + `kit_summary` part

> Sibling of `tasks/TASK_GUIDE_HUN-43.md`. Everything here is **filled by the reviewer at Stage
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

Migration-safety note (checklist done by hand, `skills/migration-safety/SKILL.md`): schema change is one additive `CREATE TABLE IF NOT EXISTS ui_sessions` (5 columns) in `core/store.py` SCHEMA, the project's only schema mechanism (like `context_index`); no ALTER/DROP/RENAME, no NOT NULL added to an existing table, no backfill, no index; old code ignores the table, new code on an old store creates it on connect (expand only, zero downtime). Rollback = `DROP TABLE ui_sessions` (nothing else reads it; data re-derivable from the report folder). Dry run: new tests build the table on a fresh store and the end-to-end run above created it on an empty home; the full suite passes. Verdict: GO.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-22T04:13:44Z, before any implementation commit (empty temp home, `EASY_PERSONAL_HOME=$D/h EASY_PERSONAL_CLAUDE_PROJECTS=$D/p uv run easy-personal kit`):

```
{
  "projects": []
}
```
No `ui` key in `kit_summary`; `ui_sessions` appears nowhere in `core/store.py`.

**AFTER**: 2026-10-22T04:16:14Z, one synthetic session file (target and an action label hold "SECRET"), `EASY_PERSONAL_UI_REPORTS` set, `easy-personal ingest` then `easy-personal kit`:

```
"ui": {"actions": 1, "failed": 0, "first_utc": "2026-10-01T10:00:00.000000Z", "last_utc": "2026-10-01T10:00:00.000000Z", "other": 0, "passed": 1, "sessions": 1, "status": "ok"}
```

**DELTA**: `kit_summary` now reports easy-ui checks overall (counts, actions, first/last time) and never stores the caller-authored `target`.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
