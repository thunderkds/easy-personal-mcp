# TASK_GUIDE — DRAFT: read-only store health check on the CLI and the dashboard (FR-091)

> **Draft.** The board is unreachable and `LINEAR_API_KEY` is not set, so no task ID exists yet.
> A new task's ID comes from the board and a create cannot be queued
> (`scripts/board_sync.py:374-376`). When the board is reachable, run
> `python3 scripts/board_sync.py create --guide tasks/DRAFT_store_health.md`; it assigns the ID and
> writes `tasks/TASK_GUIDE_<ID>.md`. Then delete this draft and put the new row on the board.
> Split out of HUN-230 note 2, "check db effectively", at Stage 2 on 2026-11-08.

**Date**: 2026-11-08
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints: metadata only, read-only sources, no traffic leaves the machine)
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-091, US-037, NFR-002, NFR-008, NFR-009, NFR-013, open questions 3 and 33; `src/easy_personal/core/store.py` (`connect`, the migration runner, `PRAGMA user_version`); `src/easy_personal/core/migrations/` (how the newest migration number is held in code); `src/easy_personal/surfaces/api.py` (the `Guard` middleware, `_query`, how a read route is wired); `src/easy_personal/surfaces/cli.py` (how a subcommand and its `--json` flag are wired, and the `doctor` command as the closest existing shape); `src/easy_personal/core/memo.py` (do not break the `PRAGMA data_version` memo); `tests/leak/harness.py` (`API_CALLS`, `CLI_CALLS`)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-08, note 2 of HUN-230: "check db effectively".

**Restated intent**:

> One read-only health check of the local store, reachable two ways: a CLI command and a dashboard
> read route. It answers the question "is my store healthy, or is the dashboard empty because
> something is wrong?" without the owner opening SQLite by hand.
>
> **(a)** `easy-personal store-check [--json]`. Human-readable lines by default, one JSON object with
> `--json`.
>
> **(b)** `GET /api/store/health`, under the same `Guard` (Host, Origin, header token) as every other
> `/api/*` route. `GET` only; any other method 405 as the other routes. No change to the `Guard`.
>
> **Both report the same fields**:
> - whether the store file exists; its size in bytes; the size of its `-wal` file
> - `PRAGMA user_version`, beside the newest migration number the code holds, and a `migration_pending`
>   boolean
> - the verdict of `PRAGMA integrity_check` and of `PRAGMA foreign_key_check` — the verdict only
>   (`ok`, or the count of problems), never a row's content
> - the row count of every table in the schema
> - per source: the last ingest time (UTC) and how many rows the parser abstained on
> - the `bak-v<N>` backup files present, by name only
>
> **Hard rules**: nothing is written — no migration runs, no `VACUUM`, no `REINDEX`, no backup, no
> delete, no store row touched. Open the store read-only. No row content, message text, secret or
> absolute path appears in the answer, in a log or in an error body (NFR-002): the dashboard names the
> store file, never its absolute path. A source that has never run reports "no data" rather than
> failing (PRD question 3). The check answers in < 2 s on the owner's store (NFR-013). Do not disturb
> `core/memo.py`'s `PRAGMA data_version` memo.

**Out of scope**: browsing or editing rows; a repair, `VACUUM`, `REINDEX`, backup or restore; an
export; showing the store's absolute path; a schedule or a timer; an MCP tool; a warning rule; a new
dashboard view (the answer goes in an existing panel, or is CLI-only if no panel fits without a UI
task).

**Requirement Refs**: FR-091, US-037, NFR-002, NFR-008, NFR-009, NFR-013

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request — signed `auto (Supervisor)`, 2026-11-08, not by the user. The owner's words are three words long; PRD question 33 records the reading and is open for owner review.
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `easy-personal store-check` in `src/easy_personal/surfaces/cli.py`, and
`GET /api/store/health` in `src/easy_personal/surfaces/api.py`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | On a fixture store, `store-check --json` returns every field named in the Requirement, with the right types; the human form prints one line per field group and exits 0 | FR-091 |
| 2 | `user_version` equals the newest migration number the code holds and `migration_pending` is false on a migrated store; on a store one migration behind, `migration_pending` is true and the exit code is 1 | FR-091, NFR-009 |
| 3 | `integrity_check` and `foreign_key_check` report `ok` on a good store; on a deliberately corrupted fixture both report a problem count and the exit code is 1, and **no row content appears in the output** | FR-091, NFR-002 |
| 4 | Row counts match a direct `SELECT COUNT(*)` for every table; a table added by a later migration is counted without a code change (the table list comes from the schema, not a hard-coded list) | FR-091 |
| 5 | A source that has never ingested reports "no data" for its last ingest time, and the command still exits 0 | FR-091, PRD question 3 |
| 6 | Backups: with `bak-v3` and `bak-v4` on disk both are listed by name; no absolute path appears; with none, the list is empty | FR-091 |
| 7 | No write: the store file hash, its mtime and the `-wal` file are unchanged after both (a) and (b) run; the store is opened read-only (a write attempt inside the check would raise) | FR-091 |
| 8 | Route: 200 with the token; 401/403 as the other routes without it, with a wrong Host or a foreign Origin; `POST` → 405; the token is not accepted from a `token` query parameter | FR-091, NFR-008 |
| 9 | Leak: a planted secret in a store row, in a file name and in a source path never appears in the CLI output, the route answer, any error body or the log; both surfaces are registered in `tests/leak/harness.py` (`API_CALLS` and `CLI_CALLS`) | NFR-002 |
| 10 | The answer arrives in < 2 s on a fixture store scaled to the owner's size; `core/memo.py`'s memo still works after the check (a `memo.reset()` test, per the recorded gotcha) | NFR-013 |
| 11 | Existing tests pass; `uv run pytest -q` passes. Real run: temp `EASY_PERSONAL_HOME` with a small store; `easy-personal store-check` and `store-check --json`; `easy-personal dashboard` plus `curl` with `X-Dashboard-Token` against `/api/store/health`, and one no-token try; paste the real output | — |

---

## UI / Design Acceptance Criteria

N/A — no page change. If a panel is wanted later it is a separate Frontend task. UI Evidence rows ☐ N/A.

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<ID>.md`.

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core tests/surfaces tests/leak -q && uv run pytest -q
```

## Approach

**Vital slice**: one `core/store_health.py` function that returns a plain dict, called by both the CLI
command and the route; the table list read from the schema; a read-only connection. **Cut list**: no
repair, no `VACUUM`, no backup, no export, no panel, no MCP tool, no warning rule, no history of past
checks.

---

## Edge Case Checklist

- [ ] Store file missing entirely → a clear "no store" answer, exit 1, no traceback
- [ ] Store file present but zero bytes, or not a database → a clear answer, no traceback
- [ ] A `-wal` file with no store file, and a store with no `-wal`
- [ ] Store locked by a running ingest → `busy_timeout` applies, the check still answers
- [ ] A store one migration behind, and a store from a newer code version (`user_version` ahead) → say so, never migrate
- [ ] A corrupted page (truncate a fixture) → problem count only, no row content
- [ ] A table with 0 rows, and a table added by a later migration
- [ ] Planted secrets (NFR-002) never in logs, output or error bodies

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/store_health.py` (new) | the one health function |
| `src/easy_personal/surfaces/cli.py` | the `store-check` subcommand and `--json` |
| `src/easy_personal/surfaces/api.py` | the `GET /api/store/health` route |
| `tests/core/test_store_health.py` (new) | AC 1–7, 10 |
| `tests/surfaces/test_api_store_health.py` (new) | AC 8 |
| `tests/leak/harness.py`, `tests/leak/` | AC 9 |
| `README.md`, `site/` | only if a drift test requires the new command or route to be listed |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/migrations/` | The check never migrates |
| `src/easy_personal/core/store.py` | Read it; the check lives in its own module |
| `Guard` class in `api.py` | The token rule stays: header only |
| `src/easy_personal/sources/` | No ingest change |
| `web/` | No page change in this task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table
- [ ] UI Evidence rows ☐ N/A
- [ ] Supervisor notified: task ready for Stage 4 review

## Questions Log

* 2026-11-08 Supervisor reading of the owner's three words "check db effectively" → a read-only store health check, not a database browser and not a repair tool (PRD question 33, open for owner review) ⚠ decided for you — please review
