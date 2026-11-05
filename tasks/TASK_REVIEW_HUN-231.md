# TASK_REVIEW — HUN-231: Migration backups: keep only the newest store.sqlite.bak-v<N> after a successful migration

> Sibling of `tasks/TASK_GUIDE_HUN-231.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
Owner instruction (not a decision): run HUN-231 and HUN-232 in parallel, auto mode ("yes, do both parallel [auto]", 2026-11-05).
1. Requirement Fidelity Gate stays signed by the owner ("yes", 2026-11-05) — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` row 13
2. spawned on claude · C2 → claude-sonnet-5-5 — FACT: PROJECT_SPEC.md:165 "| claude | `claude` | `claude-haiku-5-5` | `claude-sonnet-5-5` | `claude-sonnet-5-5` |"
3. HUN-231 merges before HUN-232 — FACT: tasks/TASK_GUIDE_HUN-232.md:54 "**Depends on**: HUN-231 — backup cleanup must be merged first"
Checks: pending
Not done: merge (yours); auto verify: running (T182)

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

**BEFORE**: captured 2026-11-05T15:26:25Z (`date -u`) on branch HUN-231 before any implementation commit. Script (scratchpad `demo231.py`): builds a tmp store, sets `user_version` to latest-1 (9), adds `bak-v0`, `bak-v3`, `bak-v3-wal`, `bak-v3-shm`, `pre-v0.4.0`, opens it with `store.connect()`, lists the folder.
```
before connect: ['store.sqlite', 'store.sqlite.bak-v0', 'store.sqlite.bak-v3', 'store.sqlite.bak-v3-shm', 'store.sqlite.bak-v3-wal', 'store.sqlite.pre-v0.4.0']
after  connect: ['store.sqlite', 'store.sqlite.bak-v0', 'store.sqlite.bak-v3', 'store.sqlite.bak-v3-shm', 'store.sqlite.bak-v3-wal', 'store.sqlite.bak-v9', 'store.sqlite.pre-v0.4.0']
```
All old backups stay; each migration run adds one more.

**AFTER**: same script, 2026-11-05T15:31:04Z (`date -u`), after the change:
```
before connect: ['store.sqlite', 'store.sqlite.bak-v0', 'store.sqlite.bak-v3', 'store.sqlite.bak-v3-shm', 'store.sqlite.bak-v3-wal', 'store.sqlite.pre-v0.4.0']
after  connect: ['store.sqlite', 'store.sqlite.bak-v9', 'store.sqlite.pre-v0.4.0']
```

**DELTA**: After a migration the store folder keeps one `bak-v<N>` (the one just written) instead of growing by about one full store copy per migration.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
