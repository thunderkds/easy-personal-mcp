**Date**: 2026-11-04
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Abstained, Rule; Critical Constraints: read-only, metadata only)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-003 and FR-026, and `src/easy_personal/core/migrations/m0008_tool_errors.py` (shape to copy)
7. This task changes stored rows: run `Skill({ skill: "migration-safety" })` before writing the migration

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-04: the `warnings` tool shows `records_unreadable` ("20% of records from Claude Code 2.1.312 could not be read"). Supervisor investigation (read-only, owner store `~/.easy-personal/store.sqlite`):

* Every abstained row in the last 7 days has `record_type` `api-request` (27,360 rows all time) or `api-request-shape` (987).
* HUN-175 (`be8abc2`, 2026-11-01) added both types to `METADATA_TYPES` in `sources/claude_code.py`. Rows stored before `2026-10-31T14:50Z` keep `abstained = 1`; every row after is `abstained = 0`.
* Migration 8 re-read every transcript, but `store.save_file`'s `ON CONFLICT ... DO UPDATE` only changes `entrypoint` and `tool_error_count`, so `abstained` never clears.
* Metadata rows carry no tokens, so token totals are correct; the warning is a false alarm. Owner approved fix option A ("migration 9") on 2026-11-04.

**Restated intent**:

> Add store migration 9 (`core/migrations/m0009_metadata_abstained.py`): `UPDATE events SET abstained = 0 WHERE source = 'claude_code' AND abstained = 1 AND record_type IN (<frozen copy of the metadata type list of sources/claude_code.py METADATA_TYPES as of 2026-11-04>)`. A metadata parse keeps only `record_type`, `version`, `ts_utc`, `session_id`, which `_abstain` already stored, so the cleared row equals what a fresh parse would write. The migration reads no file and adds or deletes no row. After it, `records_unreadable` no longer fires on stale metadata rows.

**Out of scope**:

* Changing `store.save_file` so a re-read clears `abstained` in general (owner chose option A)
* Rows abstained for any other reason (bad shape of `user`/`assistant`/`attachment`/`system`, unknown type) — they stay abstained
* Any change to `core/rules.py`, `rules.yaml`, thresholds, surfaces or `web/`
* Other sources (`kit_trace`, `git_local`, ...)

**Requirement Refs**: FR-003, FR-026

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner-approved option A, 2026-11-04)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `core/store.migrate` (`MIGRATIONS`, `m0009_metadata_abstained.apply`) → read by `core/metrics.record_readability` → rule `records_unreadable` in `core/rules.evaluate` (MCP `warnings`, CLI `easy-personal warnings`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A v8 store opened by the new code becomes v9 and `<store>.bak-v8` exists | FR-003 |
| 2 | A `claude_code` row with `abstained = 1` and `record_type` in the frozen metadata list (`api-request`, `api-request-shape`, and each other listed type) becomes `abstained = 0`; its `record_type`, `version`, `ts_utc`, `session_id` and all other columns are unchanged | FR-003 |
| 3 | Rows that must stay abstained stay: `claude_code` abstained rows of type `user`/`assistant`/`attachment`/`system`, of an unknown type, or with NULL `record_type`; and an abstained row of another source with a metadata type name | FR-003 |
| 4 | The event row count is the same before and after; running `apply` twice gives the same result | FR-003 |
| 5 | On a v9 store holding stale metadata rows from before the migration, `rules.evaluate` no longer reports `records_unreadable` from those rows (fixture: 30% stale `api-request` rows → hit before migration, no hit after) | FR-026 |
| 6 | Real run on a **copy** of the owner store: paste `SELECT record_type, SUM(abstained), COUNT(*) FROM events WHERE record_type IN ('api-request','api-request-shape') GROUP BY 1`, the total event count before and after, and `easy-personal warnings` output against the copy | FR-003, FR-026 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_migrations.py tests/core/test_rules.py -q && uv run pytest -q
```

## Approach

**Vital slice**: one migration module with a frozen tuple of type names and one `UPDATE`; tests in `tests/core/test_migrations.py` (and one rules test).
**Cut list**: no general "re-read clears abstained" in `save_file`; no offset reset (no file read is needed); no new CLI command.

Keep the type list as a literal inside the migration (a migration is frozen in time; do not import `METADATA_TYPES`). Write the module docstring like m0008's: what, why, idempotence, undo (`<store>.bak-v8` is the restore path). Run the real migration on a **copy** of `~/.easy-personal/store.sqlite` (use the store override the tests use), never on the live store.

---

## Edge Case Checklist

- [ ] Store already at v9 → no-op
- [ ] Old binary against a v9 store → existing version guard refuses (check an existing test covers it)
- [ ] `record_type` NULL on an abstained row → untouched
- [ ] Same type name under another `source` → untouched
- [ ] Ingest lock held by the timer during the real run → wait or retry, never break the lock
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log (no new read path; leak suite stays green)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/migrations/m0009_metadata_abstained.py` (new) | the `UPDATE` |
| `tests/core/test_migrations.py` | AC 1–4 |
| `tests/core/test_rules.py` (or the existing rules test file) | AC 5 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/store.py`, `src/easy_personal/sources/claude_code.py` | Option A only |
| `src/easy_personal/core/rules.py`, `src/easy_personal/data/rules.yaml` | Out of scope |
| `src/easy_personal/surfaces/`, `web/` | No reader change |
| `~/.easy-personal/store.sqlite` (live) | Real run on a copy only |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

v8 fixture store with abstained metadata rows, abstained versioned rows, unknown-type and NULL-type abstained rows, and one other-source row; migrate to v9; check flags, columns, row count, idempotence, backup file; rules fixture with 30% stale `api-request` rows before and after; real run on a store copy.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review