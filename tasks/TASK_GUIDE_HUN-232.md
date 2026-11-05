**Date**: 2026-11-05
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/backend.md`
5. C1: apply the C1 process from the Complexity matrix in `agents/backend.md`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05 (DB review, HUN-230 item 2), approved plan:

1. `store.connect()` always runs `INSERT OR IGNORE INTO meta (key, value) VALUES ('machine_id', ?)` in a transaction. Even when no row changes, this takes the SQLite write lock, so every MCP tool call and `/api/*` read (each opens a new connection via `store.opened()`) competes with the live ingest writer (`core/live.py`) and may wait up to `busy_timeout` (5 s).
2. The owner's real store (v10) still has index `events_ts ON events (ts_utc)`, which `m0002_message_indexes` drops (superseded by `events_ts_message`). It costs \~14 MB and slows every insert. Likely cause: an older pre-migration install re-ran the full schema; the cause is not in scope.

**Restated intent**:

> Opening an existing store performs no write, so reads never wait on the ingest writer; and a new migration 11 drops `events_ts` again on any store that still has it.

**Out of scope**:

* Finding which old install recreated `events_ts`.
* Pooling or reusing connections across requests.
* Any other index or schema change; daily rollup table.
* Backup cleanup (HUN-231).

**Requirement Refs**:

* NFR-009: versioned migrations, each tested from the previous released schema; usage queries covered by indexes
* NFR-013: `/api/*` reads return in < 2 s

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner "yes" 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered

---

## Dependencies & Reachability

**Depends on**: HUN-231 — backup cleanup must be merged first, so this task's migration 11 leaves one \~400 MB backup on the owner's machine instead of adding a seventh

**Entry point**: `store.connect`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | With another connection holding `BEGIN IMMEDIATE` on an up-to-date store, `store.connect()` returns in < 1 s (today it blocks \~5 s and raises `database is locked`) | no write lock on open |
| 2 | A new store still gets exactly one `machine_id`, and it stays the same across reopens (existing `test_machine_id_is_stable_across_reopen` keeps passing) | no regression |
| 3 | A v10 store that has `events_ts`: after `connect()` it is at v11, `events_ts` is gone, `events_ts_message` and `events_account_ts` remain, every table's row count is unchanged | drop stale index |
| 4 | A v10 store without `events_ts` migrates to v11 with no error (`DROP INDEX IF EXISTS`); a second run changes nothing | idempotent |
| 5 | `test_message_plan_uses_the_ts_index_not_a_scan` and the 1M-event 30-day timing test still pass | NFR-009 indexes |
| 6 | `migration-safety` check run at Stage 3 and Stage 4 | DB change rule |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | latest store, second connection in `BEGIN IMMEDIATE` | `store.connect()` returns < 1 s, `machine_id()` readable | automated test |
| 2 | v10 store + `CREATE INDEX events_ts ON events (ts_utc)` | v11, no `events_ts`, row counts equal, `bak-v10` exists | automated test |
| 3 | owner's real store after install (Stage 5, read-only check) | `sqlite3 -readonly ~/.easy-personal/store.sqlite "PRAGMA user_version; SELECT name FROM sqlite_master WHERE name='events_ts'"` prints `11` and nothing else | app run |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_migrations.py tests/core/test_store.py -q
```

---

## Approach

**Pattern reference**: `src/easy_personal/core/migrations/m0002_message_indexes.py` (the drop) and `m0010_investigate_reports.py` (docstring shape: what, why, idempotent, undo = `<store>.bak-v10`); tests: `test_migration_10_*` in `tests/core/test_migrations.py`.

**Vital slice**: a `SELECT` before the `INSERT` in `connect()`, and a one-statement migration.
**Cut list**: connection reuse; finding the old install.

* `connect()`: `SELECT 1 FROM meta WHERE key = 'machine_id'`; only when absent run the existing `with conn: INSERT OR IGNORE ...` (keep `OR IGNORE`: two processes may create a new store at once).
* `m0011_drop_events_ts.py`: `conn.execute("DROP INDEX IF EXISTS events_ts")`. Docstring states why (see Requirement 2) and the undo.

---

## Edge Case Checklist

- [ ] Python's sqlite3 implicit transactions: a plain `SELECT` must not open a write transaction (default `isolation_level` only begins on DML)
- [ ] Two processes creating a new store at once still end with one `machine_id`
- [ ] `migrate()` itself still takes `BEGIN IMMEDIATE` when a migration is pending — AC 1 is about an up-to-date store only

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/store.py` | `connect()` reads before it writes `machine_id` |
| `src/easy_personal/core/migrations/m0011_drop_events_ts.py` | new migration |
| `tests/core/test_migrations.py` | AC 3–4 |
| `tests/core/test_store.py` | AC 1 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/migrations/m0001_baseline.py` … `m0010_*` | released migrations are frozen |
| `~/.easy-personal/*` | tests use `tmp_path` only |

---

## Test Plan

Unit tests as in Success Criteria 1–2; full suite `uv run pytest -q` green. Stage 5: Success Criterion 3 on the owner's store, read-only, after install.

---

## Completion Checklist

- [ ] Implementation done
- [ ] `Skill({ skill: "migration-safety" })` run (Stage 3 and Stage 4)
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into the TASK_REVIEW Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run
- [ ] Supervisor notified: task ready for Stage 4 review