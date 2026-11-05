**Date**: 2026-11-05
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/backend.md`
5. C2: apply the C2 process from the Complexity matrix in `agents/backend.md`
6. Read `memory/codebase-map.md` for layout

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05 (DB review, from HUN-230 item 2 "check db effectively"): the store folder holds \~1.7 GB of migration backups next to a 403 MB store (`store.sqlite.bak-v0/-v2/-v6/-v7/-v8/-v9`, plus `bak-v7-shm`/`-wal`). `store.migrate()` writes a full copy before every migration run and nothing deletes them, so each new migration adds \~400 MB. Owner decision (2026-11-05, "yes"): after a successful migration keep only the newest backup.

**Restated intent**:

> After `store.migrate()` commits every pending migration, it deletes all older `<store>.bak-v<N>` files (and their `-wal`/`-shm` side files), keeping only the backup this run just wrote. Rollback per `RUNBOOK.md` keeps working, because it only ever uses the newest backup.

**Out of scope**:

* `store.sqlite.pre-v*` copies (made by hand per `RUNBOOK.md`): never touched.
* Cleaning up when no migration runs (an up-to-date store keeps whatever backups it has until its next migration).
* Any config knob for how many backups to keep (owner chose 1).
* Daily rollup table / retention changes (deferred; not requested).
* The `connect()` write-lock fix and dropping `events_ts` (separate task).

**Requirement Refs**:

* NFR-009: store migrations are ordered, versioned, one transaction each (this task changes only what happens after they commit)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner "yes" 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `store.migrate`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A store at version N < latest with existing `store.sqlite.bak-v0`, `bak-v3`, `bak-v3-wal`, `bak-v3-shm` beside it: after `store.connect()` the folder holds exactly one backup, `store.sqlite.bak-v<N>`, with N's rows and `user_version` N | keep only the newest |
| 2 | `store.sqlite.pre-v0.4.0` and any other file not matching `^<store name>\.bak-v\d+(-wal\|-shm)?$` (e.g. `store.sqlite.bak-vX`, `other.sqlite.bak-v1`) is left untouched | pre-v\* never touched |
| 3 | A migration that raises: every old backup is still there, plus the new one (nothing deleted) | delete only after success |
| 4 | An up-to-date store (version == latest) with old backups beside it: `connect()` deletes nothing | out of scope: no cleanup without a migration |
| 5 | A delete that fails (e.g. file removed concurrently, or permission error) never makes `connect()` fail; the store opens normally | safety |
| 6 | A new store (no tables, so no backup written) deletes nothing | only when this run wrote a backup |
| 7 | `RUNBOOK.md` states once that only the newest `bak-v<N>` is kept after a migration, and the rollback steps that cite older `bak-vN` files say they may no longer exist | rollback docs stay true |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | v(latest-1) store + `bak-v0`, `bak-v3`, `bak-v3-wal`, `bak-v3-shm`, `pre-v0.4.0` | after connect: `bak-v<latest-1>` + `pre-v0.4.0` only | automated test |
| 2 | same, with a fake migration that raises | old backups + new backup all present; `user_version` unchanged | automated test |
| 3 | latest store + `bak-v0` | `bak-v0` still present | automated test |
| 4 | `os.remove` patched to raise `PermissionError` | connect succeeds, store at latest | automated test |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_migrations.py tests/core/test_store.py -q
```

---

## Approach

**Pattern reference**: `src/easy_personal/core/store.py` `migrate()` — backup step and the per-migration transaction loop; `tests/core/test_migrations.py` `test_v0_store_upgrades_keeping_every_row_and_leaves_a_backup` / `test_failing_migration_rolls_back_fully_and_keeps_user_version` for test shape (pass a custom `migrations=` list for the failing case).

**Vital slice**: one helper called at the end of `migrate()` after the loop finishes without an exception, only when a backup was written in this run.
**Cut list**: keep-count setting; cleanup on runs with no migration; a CLI command to clean backups.

In `migrate()`, remember the backup path written (`<path>.bak-v<version>`). After the `for` loop completes with no exception, list `path.parent`, match names with `re.fullmatch(re.escape(path.name) + r"\.bak-v(\d+)(-wal|-shm)?", name)`, and delete every match except the kept backup file itself. Wrap each delete in `contextlib.suppress(OSError)`. Do not delete anything in the `except`/failure path. Keep the docstring of `migrate()` accurate (it names the restore path).

---

## Edge Case Checklist

- [ ] A stale `bak-v<N>` with the same N as this run is overwritten by the backup (sqlite `backup()` into an existing file) — the kept file must hold the new copy, not the stale one
- [ ] Backups with a higher number than the current version (store restored from an older backup) are deleted too
- [ ] The kept backup's own `-wal`/`-shm` (if present from an earlier open) are deleted — they belong to an older file content
- [ ] Path with regex characters in the folder or file name (`re.escape`)
- [ ] Concurrent process already migrated meanwhile (the loop applies nothing) — a backup was still written, so cleanup still runs; harmless

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/store.py` | cleanup helper + call at the end of `migrate()`; docstring |
| `tests/core/test_migrations.py` | tests for AC 1–6 |
| `RUNBOOK.md` | AC 7 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/migrations/*` | no schema change in this task |
| `~/.easy-personal/*` (the owner's real store) | tests use `tmp_path` only; `tests/test_real_home_guard.py` enforces it |

---

## Test Plan

Unit tests in `tests/core/test_migrations.py` with `tmp_path` stores, as in Success Criteria 1–4 plus AC 6 (new store). Full suite must stay green: `uv run pytest -q`.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (Medium risk)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into the TASK_REVIEW Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run
- [ ] Supervisor notified: task ready for Stage 4 review