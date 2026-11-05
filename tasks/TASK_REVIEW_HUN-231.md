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
4. accepted the implementer's extra step: delete the same-numbered `bak-v<N>-wal`/`-shm` before writing the new backup — FACT: tasks/TASK_GUIDE_HUN-231.md:106 "The kept backup's own `-wal`/`-shm` (if present from an earlier open) are deleted"
5. README.md:318–337 still tells users to restore from `.bak-v7`; left as a P2 suggestion, not fixed — RULE: skills/code-review/SKILL.md:142 "Do not apply P2/P3 fixes automatically"
Checks: tests `uv run pytest tests/core/test_migrations.py tests/core/test_store.py -q` — 37 passed, 1 deselected (Supervisor re-run); full suite `uv run pytest -q` — 1544 passed, 2 skipped, 1 xfailed (implementer); reviews code-review — 0 P0, 0 P1, 1 P2 (README `.bak-v7`), lane `C2, 3 files, 169 lines, FITS`; security-review — the built-in skill failed (`fatal: ambiguous argument 'origin/HEAD...'`, no origin remote), so the Supervisor did a manual pass: deletes confined to the 0700 store folder by an anchored name pattern, a symlink named like a backup loses only the link (target kept), a folder named like a backup is skipped — no findings; real run: tmp store at latest-1 with bak-v0/bak-v3(+wal/shm)/pre-v0.4.0 → after `store.connect()` `['store.sqlite', 'store.sqlite.bak-v9', 'store.sqlite.pre-v0.4.0']` (auto-checked, not /verify)
Not done: merge (yours); auto verify: running (T182)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | tests/core/test_migrations.py: AC1 `test_connect_keeps_only_the_newest_backup_and_leaves_other_files`, `test_stale_backup_of_the_same_number_is_replaced_and_its_side_files_and_higher_numbers_go`; AC2 same + `test_backup_names_with_regex_characters_in_the_store_name`; AC3 `test_failing_migration_deletes_no_old_backup`; AC4 `test_up_to_date_store_deletes_nothing`; AC5 `test_a_failing_delete_never_fails_connect`; AC6 `test_new_store_deletes_nothing`; AC7 RUNBOOK.md (doc, no test). `37 passed, 1 deselected in 0.81s` |
| Verification command run | ☑ pass | `uv run pytest tests/core/test_migrations.py tests/core/test_store.py -q` → `37 passed, 1 deselected in 0.81s` (Supervisor, 2026-11-05) |
| Negative cases hold | ☑ pass | failed migration deletes nothing; up-to-date and new stores delete nothing; a failing delete never fails `connect()`; `pre-v*`, `bak-vX`, `other.sqlite.bak-v1` untouched; symlink target kept, folder skipped (Supervisor probe) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed `store.migrate()` + new helper, its callers (`store.connect`), the new tests, RUNBOOK.md; README.md read for backup mentions only |
| Full smoke suite still green (no regression) | ☑ pass | `uv run pytest -q` → `1544 passed, 2 skipped, 6 deselected, 1 xfailed, 1 warning in 207.43s` (implementer run) |
| **Companion score (easy-verifier)** | ☑ N/A | advisory: `score` ran with scope `changes`, ref `main...HEAD`, and returned overall 68, change_risk 100. Both branches got the same overall, so it measures the repo rather than this diff, and no report file was written (no `report:` path).
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | pure backend task, no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | pure backend task, no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | pure backend task, no UI |

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

**WITNESS**: Supervisor re-ran the verification command in this worktree on 2026-11-05 (see Checks); BEFORE/AFTER captured by the implementer.
