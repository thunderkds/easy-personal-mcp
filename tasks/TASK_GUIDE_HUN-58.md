**Date**: 2026-10-22
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches what the slice does not cover
3. Read this file completely
4. Read `agents/backend.md`
5. First action: `Skill({ skill: "diagnose" })`. Apply the C1 process from the Complexity matrix.

---

## Bug Fix Task Guide

### Mental Model (confirmed by user, 2026-10-22)

* Observed: `easy-personal ingest` aborts with `sqlite3.OperationalError: table verifier_scores has 6 columns but 7 values were supplied` (`src/easy_personal/sources/verifier.py:130`). No ingest result is printed; the whole run fails.
* Expected (testable):
  * (a) On a store whose `verifier_scores` table has the pre-`db12d8c` 6-column shape, `ingest` exits 0 and the table ends up with the current 7-column shape (incl. `status`) without any manual step.
  * (b) Any exception inside the verifier step does not fail `ingest`: exit 0, the other sources' results are kept, and `result["verifier"]` carries a failure status (no traceback) — as FR-011 / the `ingest` docstring ("Never raises") promise.
  * (c) A regression test covers (a) and (b).
* Likely divergence point: `src/easy_personal/core/store.py` `SCHEMA` uses `CREATE TABLE IF NOT EXISTS verifier_scores`, which never alters an existing table; `connect()` has no migration step. `verifier.ingest` does not catch the `sqlite3` error.
* Recent context: commit `db12d8c` (HUN-42) changed `verifier_scores` in place (added `status TEXT NOT NULL`, relaxed `scope`/`generated_at_utc` to nullable) after `3802a3c` had already created it on users' stores. `git log -p` of `store.py` shows no other table changed shape after its first commit. `verifier_scores` is a rebuildable cache: `ingest` runs `DELETE FROM verifier_scores` then re-inserts every row on each run, so dropping/recreating it loses no data.

### Intake

* Trigger: a store created by a build before `db12d8c` (e.g. owner's `~/.easy-personal/store.sqlite`), then `easy-personal ingest` with the current build. Repro (deterministic, empty projects dir is enough):

  ```
  H=$(mktemp -d); P=$(mktemp -d)
  sqlite3 $H/store.sqlite "CREATE TABLE verifier_scores (project_path TEXT PRIMARY KEY, scope TEXT NOT NULL, generated_at_utc TEXT NOT NULL, overall_value REAL, overall_abstained INTEGER NOT NULL, reason_code TEXT);"
  EASY_PERSONAL_HOME=$H EASY_PERSONAL_CLAUDE_PROJECTS=$P uv run easy-personal ingest
  ```
* Severity: P1 (every ingest fails for upgraded installs; manual-SQL workaround exists)
* Affected area: `src/easy_personal/core/store.py` (`SCHEMA`, `connect`), `src/easy_personal/sources/verifier.py` (`ingest`), possibly `src/easy_personal/surfaces/cli.py` `_ingest` (only if the catch belongs there — prefer inside `verifier.ingest` to match its docstring).

### Out of scope

* Version bump, tags, release planning (version stays `0.1.0`; owner reinstalls from main).
* A general migration framework / `PRAGMA user_version` system for all tables (only `verifier_scores` drifted). Note it as a post-mortem follow-up if you think it is needed.
* Wrapping `context`/`ui` sources the same way unless the diagnosis shows they share the defect.

### Complexity & Risk

* Complexity: C1 (2 source files + tests)
* Risk: Medium (schema change on users' local store → `migration-safety` required)

### Diagnosis Gates (Pillar 1 — must pass before any fix)

- [x] Phase 1 feedback loop built and running
- [x] Bug reproduces deterministically on the loop
- [x] 3–5 ranked falsifiable hypotheses listed (consistent with confirmed mental model)
- [x] Correct hypothesis identified via Phase 4 instrumentation

### Attempts Log (filled live during diagnosis — required if >1 hypothesis tested)

| \# | Hypothesis | Predicted signal | Actual result | Verdict |
| -- | -- | -- | -- | -- |
| 1 | `CREATE TABLE IF NOT EXISTS` keeps the 6-col table; INSERT supplies 7 values | repro fails at verifier.py:130 with 6 vs 7 | Repro fails exactly so; after fix the same loop exits 0 | confirmed |
| 2 | `verifier.ingest` has no catch | any sqlite error escapes `cli._ingest` | traceback escaped; test with patched `_projects` now gets status `failed` | confirmed |
| 3 | Another table drifted too | another INSERT fails | `git log -p` shows none changed; ingest passed all earlier sources | disproven |

**Stuck checkpoint** (if 2 consecutive hypotheses disproven):

- [ ] 3 options presented (next hypothesis / widen scope / abandon+escalate)
- [ ] Chosen option: \__\_
- [ ] User's explicit go-ahead: \__\_

### Fix Gates (Pillar 2)

- [x] Regression test written before the fix (or no-seam documented)
- [x] Fix applied; regression test passes
- [x] Phase 1 loop no longer reproduces the bug
- [x] Fix matches the "correct behaviour" definition from the mental model
- [x] migration-safety (Skill tool unavailable to this agent; checklist applied by hand): findings below
  * Reversible/safe: `verifier_scores` is a rebuildable cache (ingest does DELETE + full re-insert), so DROP loses no data.
  * Idempotent: runs only when the table exists AND lacks `status`; after one rebuild it is a no-op. Runs in its own transaction before `SCHEMA`.
  * Other tables untouched; no version bump; no manual SQL step. Tests: tests/sources/test_verifier.py (4 new).

### Cleanup Checklist (Pillar 3)

- [x] All \[DEBUG-...\] instrumentation removed (grep verified)
- [x] Throwaway prototypes deleted
- [x] Correct hypothesis stated in commit message
- [x] Post-mortem: an in-place CREATE TABLE edit to an already-shipped schema had no migration path, and the 'never raises' promise had no test with a broken table. A `PRAGMA user_version` migration list would prevent the next drift; recommended follow-up if a non-cache table ever changes shape (not built here).

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-58.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-58.md` (review sections are never taken from the board).

