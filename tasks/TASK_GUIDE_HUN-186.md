**Date**: 2026-11-02
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Gone project; Critical Constraints: read-only sources, metadata only)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` US-025, FR-037, FR-041, FR-054, NFR-010, NFR-013 and Open Question 20

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-11-02: "I check the board, but the component UX is not good at all, could you test all and update" → owner chose three parallel tasks (this one = speed + gone-project filter).

Measured by the Supervisor 2026-11-02 on the owner's real store (330 MB, 160 stored projects, only 27 folders exist, 96 under `/tmp`), machine load average \~60: `GET /api/kit` 81 s, `/api/usage/sessions` 72 s and 7.6 MB, `/api/warnings` 21–25 s, `/api/plan` 22 s, `/api/memory-graph` (list) 3–20 s, `/api/board` 1–3 s, `/api/usage/summary` 3–4 s, `/api/usage/outliers` 3.7 s. The right rail shows "Next actions: Loading" for \~20 s on every view.

**Restated intent**:

> Make every `/api/*` read a dashboard view makes on open return in < 2 s on the owner's machine at normal load (median of 3), and make `/api/usage/sessions` for the last 7 days send < 1 MB, without changing any number the views show. Also drop gone projects (folder not on disk) from the `/api/board`, `/api/kit` and `/api/memory-graph` project lists, and add a top-level `gone` count to each of those three responses.

**Out of scope**:

* Any `web/` change (two parallel frontend tasks)
* Changing what MCP tools or CLI commands return (only the three dashboard routes drop gone projects; `kit_summary` MCP keeps `missing_on_disk` rows)
* Changing usage totals, warnings or the plan: a gone project's events still count
* A background cache that outlives a request, unless measurement shows the < 2 s target cannot be met any other way (then say so in the review and keep it in-process, invalidated by ingest)

**Requirement Refs**: US-025, FR-054, NFR-013

### Response contract (the frontend tasks build on this)

* `/api/board`, `/api/kit`, `/api/memory-graph` (list form, no `project=`): add top-level `gone` (int, number of stored projects hidden because their folder does not exist; 0 when none). Every other field unchanged. Gone projects are not in `projects`.
* `/api/memory-graph` list: a Claude-only key (Claude folder name that maps to no project) is not a gone project; keep it.
* `/api/usage/sessions`: same fields per session as today; reduce size by sending less redundant data per point (e.g. fewer timeline points per session or no repeated per-point constants) — any field removal must be checked against `web/src/Timeline.jsx` and `web/src/UsageView.jsx` and must not change what they draw; if a field the page reads must change, STOP and report (the frontend tasks do not own `Timeline.jsx`).

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, owner chose "All three tasks in parallel", 2026-11-02)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: none

**Entry point**: `easy-personal dashboard` → `src/easy_personal/surfaces/api.py` routes `/api/kit`, `/api/usage/sessions`, `/api/warnings`, `/api/plan`, `/api/memory-graph`, `/api/board`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Profile first: paste a per-route breakdown (where the time goes) for kit, usage/sessions, warnings, plan and memory-graph list on a copy of the owner's store, BEFORE any fix | NFR-013 |
| 2 | Every route in AC 1 plus board, usage/summary, usage/outliers returns in < 2 s median of 3 on the owner's store copy (state the load average at the time) | NFR-013 |
| 3 | `/api/usage/sessions` (default 7 days) response < 1 MB on the owner's store copy; Timeline/Usage page tests still pass unchanged | NFR-013 |
| 4 | Golden equality: for a fixture store, every changed route returns the same JSON as before the change (except the new `gone` field and any documented sessions size change) — test compares old vs new output | NFR-013 |
| 5 | A stored project whose folder does not exist is absent from `/api/board`, `/api/kit` and `/api/memory-graph` lists and counted in `gone`; MCP `kit_summary` output is unchanged | FR-054 |
| 6 | Usage summary, warnings and plan still count events of a gone project (test with a fixture whose project folder is deleted) | FR-054 |
| 7 | Any new index or schema change goes through a migration with `migration-safety` (reversible, tested on a copy of the owner's store, time measured) | NFR-013 |
| 8 | Leak suite passes; no new route | NFR-002 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest -q && uv run ruff check src tests
```

---

## Approach

**Vital slice**: profile → fix the biggest cost per route (likely: per-project SQL over 160 projects in kit, `SELECT DISTINCT project_path` scans, per-point session timeline size, rules re-reading the same data for warnings and plan) → gone filter → measure.
**Cut list**: no cross-request cache unless measured necessary; no change to MCP/CLI outputs; no frontend change.

Work on a COPY of the owner's store (`cp ~/.easy-personal/store.sqlite <scratch>/`; point `EASY_PERSONAL_HOME` at the copy's folder); never write to the real store. Delete the copy when done.

---

## Edge Case Checklist

- [ ] Project folder deleted between two requests → counted in `gone` next request, no error
- [ ] Workspace / account scope still applies after the gone filter (`gone` counts only in-scope projects)
- [ ] Worktree paths still skipped in board as today
- [ ] Empty store → `gone: 0`, empty lists
- [ ] A migration adding an index on a 330 MB store: time measured, `bak` file written as other migrations do
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/kit.py`, `core/board.py`, `core/memgraph.py`, `core/metrics.py`, `core/rules.py`, `core/plan.py` | speed + gone filter |
| `src/easy_personal/surfaces/api.py` | only if a route handler needs the `gone` count wiring (no new route) |
| `src/easy_personal/core/migrations/` | only if an index is needed |
| `tests/core/*`, `tests/surfaces/*` | golden-equality + gone + timing-budget tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `web/**` | parallel frontend tasks |
| `src/easy_personal/surfaces/mcp*.py`, CLI output shape | MCP/CLI unchanged |
| `~/.easy-personal/**` (the real store) | work on a copy |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Fixture-store tests for `gone` on the three routes and for unchanged usage/warnings/plan counts; golden equality old vs new JSON; timing measured on the owner's store copy with `curl -w %{time_total}` (median of 3) before and after, pasted with load average.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review