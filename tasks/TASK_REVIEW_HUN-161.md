# TASK_REVIEW — HUN-161: Typed vs agent usage split (entrypoint migration)

> Sibling of `tasks/TASK_GUIDE_HUN-161.md`. Everything here is **filled by the reviewer at Stage
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

## Migration-safety (Supervisor, 2026-10-31, `m0007_entrypoint.py` at HUN-161 954ffc4)

Verdict: **GO**. Reversible: yes (restore `<store>.bak-v6`, or `ALTER TABLE events DROP COLUMN entrypoint` + `PRAGMA user_version = 6`; tested). Downtime: none. Data-loss risk: none.

- [x] Managed migration (`store.migrate`, numbered `MIGRATIONS`, own `BEGIN IMMEDIATE` + `user_version` bump); no ad-hoc SQL.
- [x] In scope (FR-039, NFR-009). Guide said "every source file"; reset limited to `source = 'claude_code'` — accepted (only Claude carries `entrypoint`; kit_trace re-read gains nothing).
- [x] Destructive ops: none (nullable `ADD COLUMN`; offset reset only).
- [x] Rollback tested up → down → up on a copy of the owner's store: v6 → v7 → v6 (column gone, 267,708 events) → v7 (267,708 events).
- [x] Expand-only; single local process, no multi-version deploy; old program refuses v7 store with a clear message.
- [x] Zero-downtime: additive only.
- [x] Locks: SQLite `ADD COLUMN` is metadata-only — migrate took **147 ms** on the 236 MB copy (267,708 events, 688 Claude files).
- [x] Backfill separate from the schema change (next ingest), per-file commits, idempotent, resumable (`test_interrupted_backfill_resumes_without_duplicates`, `test_backfill_twice_gives_the_same_rows`).
- [x] Dry run on a production-like copy: migrate 147 ms; backfill ingest **16.94 s, maxrss 69,684 KB**; `integrity_check` ok; `foreign_key_check` 0 rows; `.bak-v6` written; kit/other file offsets unchanged (445 files, sum 15,040,956). After backfill (copy also took in 2 days of new sessions): 314,096 events; entrypoint `cli` 291,164 · `sdk-cli` 18,546 · `claude-desktop` 521 · NULL 3,865.
- [x] No silent data loss. Note (P3): with `--retention-days` set, a re-read can bring back rows pruned earlier until the next pruning ingest; default retention is forever, and Claude deletes transcripts after 30 days, so the window is small.

Finding from the dry run → owner decision 2026-10-31: `claude-desktop` is interactive; classification changed to *agent = sidechain or entrypoint starting with `sdk`*. Sent back to the implementer.

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

**BEFORE** (Supervisor, 2026-10-31T14:49:29+07:00, main de84e68, before any implementation commit):
```
$ grep -rn "entrypoint" src
(no output — no entrypoint column)
$ ls src/easy_personal/core/migrations | grep m000
m0001_baseline.py … m0006_connectors.py
$ grep -n "typed\|\"agent\"" src/easy_personal/core/metrics.py
(no output — usage_summary has no typed/agent split)
```

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
