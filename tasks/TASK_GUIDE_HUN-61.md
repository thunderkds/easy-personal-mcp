**Date**: 2026-10-26
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *Connector id / alias*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely, and `BRAINSTORMING_LOG_WORKSPACES.md` (verified facts W1–W10, edge cases)
4. Read `agents/backend.md`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md`

---

## Requirement (Pillar 1 — Adapt the requirement)

The store must scale to every agent account and forever history using database best practice (PRD NFR-009, NFR-005). Today: `PRAGMA user_version` = 0, `foreign_keys` off, no `busy_timeout`, schema via `CREATE TABLE IF NOT EXISTS` + a hand patch (`_rebuild_old_verifier_scores`, the HUN-58 drift class), and `metrics._messages` groups all events before filtering by time (full scan; brainstorm HUN-64, HUN-65).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Introduce an ordered migration runner on `PRAGMA user_version`, make the current schema migration 1 (absorbing the verifier hand patch), turn on FK + busy_timeout, make usage queries filter by time before grouping with supporting indexes, and prove < 500 ms on 1,000,000 events — with every usage number unchanged on the fixtures.

**Out of scope** (what this task explicitly does NOT do):

* Any new table for workspaces/accounts (later tasks add their own migrations)
* A pre-aggregated `messages` table (cut unless the benchmark fails — then STOP and report to the Supervisor)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* NFR-009
* NFR-005
* NFR-004

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-26)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (requirement grill 2026-10-26)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: None — can start immediately

**Entry point**: `core.store.connect()` (every surface) and `core.metrics` usage functions

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `connect()` applies pending migrations in order, each in one transaction, and sets `user_version`; re-running is a no-op | NFR-009 |
| 2 | An existing v0 store (today's schema, with rows, incl. a pre-HUN-42 `verifier_scores`) upgrades with every row kept | NFR-009 |
| 3 | A failing migration rolls back fully and leaves `user_version` unchanged | NFR-009 |
| 4 | Every connection has `foreign_keys=ON` and `busy_timeout` ≥ 5000 ms | NFR-009 |
| 5 | Usage summary / breakdown / outliers / timeline numbers are identical before and after on all existing fixtures | NFR-004 |
| 6 | `EXPLAIN QUERY PLAN` of the usage message query uses an index on `ts_utc` (no full `SCAN events`) | NFR-009 |
| 7 | Seeded 1,000,000-event store: 30-day usage summary returns in < 500 ms | NFR-009 |
| 8 | No test opens the real data dir: an autouse fixture in `tests/conftest.py` points `EASY_PERSONAL_HOME` at a tmp dir for every test, and a guard test fails if any test resolves `config.data_dir()` under the real `~/.easy-personal` (Stage 4 finding 2026-10-26: `tests/leak/test_leak_suite.py::test_leaking_tool_fails_the_suite` opened and migrated the owner's real store) | NFR-003 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | v0 fixture store copied to temp | upgraded to latest; row counts equal; `user_version` = latest | automated |
| 2 | Migration that raises mid-way (test-only) | schema and `user_version` unchanged | automated |
| 3 | 1M synthetic events (perf marker) | summary < 500 ms on this machine | automated (`-m perf`) |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core -q && uv run pytest tests/core -m perf -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-61.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-61.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (backend/QA only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: `core/store.py` `connect()`; HUN-58 fix (`_rebuild_old_verifier_scores`)

**Vital slice**: migration runner + time-first query + benchmark
**Cut list**: messages rollup table; down-migrations (document the restore-from-backup path instead: copy `store.sqlite` before upgrading)

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Migrations live in `src/easy_personal/core/migrations/` as ordered Python modules (`m0001_baseline.py`, …) each exposing `apply(conn)`; runner in `store.connect()` reads `PRAGMA user_version`, runs each pending one inside `BEGIN IMMEDIATE … COMMIT`, then sets `user_version`. Before the first pending migration on an existing non-empty store, copy `store.sqlite` to `store.sqlite.bak-v<N>` (cheap, makes rollback a file copy — migration-safety).
* m0001 = today's SCHEMA (IF NOT EXISTS, so a v0 store passes) + the verifier rebuild logic moved inside it. Delete the ad-hoc call from `connect()`.
* Time-first query: inner `WHERE ts_utc >= :since_pad AND ts_utc < :until_pad` with a 1-day pad, `GROUP BY message_id`, then the existing exact outer filter on `MIN(ts)` — exact while a message's events span < 1 day. Add index `events(ts_utc, message_id)`; drop `events_ts` if it becomes redundant (inside the migration).
* Benchmark: a test helper that bulk-inserts N synthetic events (executemany in one transaction) into a temp store; perf tests carry `@pytest.mark.perf` (register the marker in `pyproject.toml`).

---

## Edge Case Checklist

- [ ] Store file opened by the dashboard while the timer ingest migrates: `busy_timeout` waits, ingest lock still serialises writers
- [ ] Store created by a newer version (`user_version` > latest known) → refuse with a clear message, never downgrade
- [ ] Empty/new store: all migrations run, no `.bak` copy made
- [ ] Period with no `since` (all time): query still correct and indexed where possible
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/store.py` | migration runner, PRAGMAs |
| `src/easy_personal/core/migrations/` (new) | m0001 baseline, m0002 indexes |
| `src/easy_personal/core/metrics.py` | time-first `_messages` and other usage queries |
| `tests/core/` | migration, plan, perf tests |
| `pyproject.toml` | `perf` marker |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| real `~/.claude*/`, `~/.claude.json`, `~/.codex*/` | Never read or written by tests or verify: fixtures + temp HOME only (learnings: verify against fixtures). Product code writes Claude config only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| `~/.claude*/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `src/easy_personal/launch.py` beyond what the task states | NFR-006: stdlib + `keyring` only, < 200 ms |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Migration tests from a v0 fixture store; rollback test; regression equality on all usage functions; EXPLAIN QUERY PLAN assertions; 1M-event perf test.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (Medium/High risk)
- [ ] `Skill({ skill: "migration-safety" })` run (schema change) — Stage 3 and Stage 4
- [ ] Lint passes (`uv run ruff check .`)
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app (fixtures only)
- [ ] `memory/MEMORY.md` updated (if new patterns or feedback learned)
- [ ] Supervisor notified: task ready for Stage 4 review