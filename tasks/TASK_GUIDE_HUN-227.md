**Date**: 2026-11-05
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (Critical Constraints: metadata only, deterministic; glossary)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` NFR-013; `BRAINSTORMING_LOG_PERF.md` (measurements, Debate, recommended path); then `src/easy_personal/core/metrics.py` (`_scope`, `_messages` and every caller), `src/easy_personal/core/owners.py` (`_stored_branches`, `owner_map`, `project_map`, `_cached`), `src/easy_personal/core/store.py` (`connect`, `opened`), `src/easy_personal/surfaces/api.py` (`_query`), `src/easy_personal/core/workspaces.py` (`project_paths`), `src/easy_personal/core/accounts.py` (`resolve`)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05: "the data loading take so long, investigate the root cause and propose the fix", then "yes, investigate and fix them \[auto\]". Root cause measured on a store copy (see `BRAINSTORMING_LOG_PERF.md`): the Usage page's \~14 parallel reads take 4–5 s each (page 5.1–6.0 s) because the same costly work is redone per read and Python threads hardly overlap: `metrics._messages` \~0.31 s × \~7 per page, `owners._stored_branches` \~0.26 s in every `project_map` / `owner_map` caller. NFR-013: every `/api/*` read a view makes on open < 2 s (median of 3).

**Restated intent**:

> Add one small core helper (e.g. `src/easy_personal/core/memo.py`) that shares the result of a costly store computation between calls while the store has not changed. Key = the function + every argument that changes the result (for `_messages`: the store path, `since_utc`, `until_utc`, `workspace`, `account`; for `_stored_branches`: the store path) + the store's **data version**. The data version comes from `PRAGMA data_version` read on **one long-lived watcher connection per store path**, owned by the helper (thread-safe; opened lazily; never used for anything else). `PRAGMA data_version` changes on every commit by any other connection in any process (insert, upsert, update, delete, workspace / account edits), so a result is never served after a change. **Single-flight**: when several threads ask for the same key at once, one computes and the others wait for that result. Cached values are immutable (tuples / frozensets / read-only mappings) or copied on the way out, so a caller cannot change the shared value. Entries of an older data version are dropped; the cache holds at most a small fixed number of entries. Wrap `metrics._messages` and `owners._stored_branches` with it (scope resolution inside `_messages` must stay inside the memoized call, or be part of the key). If the watcher connection cannot be opened or read, compute without caching (never an error). Every API, MCP and CLI output stays byte-identical.

**Out of scope**:

* A whole-response cache in `api.py` (Debate: option B rejected)
* A combined endpoint, a new index, a schema change, a write-side version counter
* The page (other task)
* Changing any metric's maths or output

**Requirement Refs**: NFR-013 (server side)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (auto (Supervisor), 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None (runs in parallel with the page task; API shapes unchanged)

**Entry point**: `metrics._messages` (used by `usage_breakdown`, `session_outliers`, …) and `owners._stored_branches` (used by `owner_map`, `project_map`) → `GET /api/usage/*`, `/api/warnings`, `/api/plan`, `/api/kit`, `/api/workspaces/*`, and the MCP / CLI tools that call the same core

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Two calls of `_messages` with the same arguments and no store change compute once (counter / spy); different arguments compute separately | NFR-013 |
| 2 | 8 threads asking for the same key at once → the computation runs once; all 8 get equal results | NFR-013 |
| 3 | Never stale: after each of (a) a new event insert, (b) an upsert of an existing event row (`ON CONFLICT … DO UPDATE`), (c) `UPDATE events`, (d) a retention delete, (e) a workspace repo add / remove, (f) an account workspace change — made through a **different connection**, and (g) one made from a **separate process** — the next call returns the new result | NFR-013 |
| 4 | A caller that mutates what it got does not change what the next caller gets | NFR-013 |
| 5 | The cache never holds more than its bound; entries of an older data version are dropped | NFR-013 |
| 6 | Watcher connection failure (e.g. patched to raise) → results still correct, computed without cache, no exception | NFR-013 |
| 7 | All existing tests and goldens pass unchanged (API, MCP, CLI outputs byte-identical); leak suite green | NFR-002, NFR-013 |
| 8 | Real run on a **copy** of the owner store: the Supervisor's page-load script (`/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/perf/pageload.py par`, run with `EASY_PERSONAL_HOME=<copy dir>`; copy it, drop its prototype `memo` import, do not edit it in place) BEFORE on `main` and AFTER on the branch: paste page wall time and each read's time, median of 3. Target: page wall ≤ 2.7 s cold; report any read still > 2 s honestly with its profile (do not widen scope to fix it) | NFR-013 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests -q -k "memo or metrics or owners or usage or api or leak" && uv run pytest -q
```

## Approach

**Vital slice**: one helper (\~50 lines: watcher connection per path + lock, `data_version` read, single-flight dict of per-key locks / futures, bounded store), two decorations, tests per AC.
**Cut list**: no TTL, no config switch, no stats endpoint, no caching of other functions (measure first; list candidates in the review file instead).

Store copy: `sqlite3 ~/.easy-personal/store.sqlite ".backup <dir>/store.sqlite"`, then `EASY_PERSONAL_HOME=<dir>`. The store path the memo keys on must be the resolved path the connection opened, so two stores never share entries (tests use temp stores).

---

## Edge Case Checklist

- [ ] Writes made through the **same** connection that reads are invisible to `data_version` on that connection — the watcher is a separate connection, so test that a write through the request's own connection also invalidates
- [ ] Tests create many temp stores: watcher connections must not leak file handles across tests (close / weak registry, or a test fixture that resets the helper)
- [ ] A computation that raises: waiting threads get the exception (or recompute); nothing broken is cached
- [ ] `workspace` / `account` unknown → `ValueError` / `AccountError` still raised exactly as before
- [ ] Planted secret/text fixtures never appear in any store row, MCP/CLI/API response, page or log (leak suite green)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/memo.py` | new helper |
| `src/easy_personal/core/metrics.py` | wrap `_messages` |
| `src/easy_personal/core/owners.py` | wrap `_stored_branches` |
| `tests/core/test_memo.py` | AC 1–6 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `web/` | page is the other task |
| migrations, `src/easy_personal/data/rules.yaml` | not needed |
| `~/.easy-personal/store.sqlite` (live) | real run on a copy only |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Temp stores; spy on the wrapped function; threads for single-flight; writes through other connections and a subprocess for invalidation; mutation test; bound test; failure fallback; full suite and leak suite; real page-load timing on a store copy before / after.

---

## UI / Design Evidence

☐ N/A — backend only.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review