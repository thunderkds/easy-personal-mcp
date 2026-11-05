**Date**: 2026-11-05
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` in full (glossary: Rule, Abstained, Saved report, Rule snapshot)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and **no** `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-012, FR-073, FR-078, FR-080; `src/easy_personal/core/rules.py` (`_measure`, `evaluate`, `_context_stale_indexes`, `_causes_context_stale_indexes`), `src/easy_personal/sources/context.py`, `src/easy_personal/core/kit.py` (where `context_part` is used), `src/easy_personal/core/reports.py`, `src/easy_personal/core/metrics.py` (`_scope`, how `tool_names` is queried), and the existing rules / kit / reports tests

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05, pasting the saved report's line "Code index is out of date — then abstained: no easy-context data … → now abstained …": "I removed the context out of family". Owner chose "Hide when not in use". The owner's store's last `mcp__easy-context__*` call is 2026-10-22; no Claude account registers easy-context.

**Restated intent**:

> Add one function, e.g. `context.in_use(conn, since, until, **scope) -> bool`: True when the scope's stored events in \[since, until) have at least one tool name starting `mcp__easy-context__` (window = 7 days ending at `now`). In `rules.evaluate`, when the rule's metric is `context_stale_indexes` and easy-context is not in use, leave the rule out completely: not in `hits`, not in `abstained`, not in `checked` (so `improvement_plan` drops it too). In `reports.get_report`, a rule absent from "now" because it is not in use gets `now = {"state": "not_in_use", "reason": "not in use"}`; and a rule whose `before.state` was `abstained` and which is not in use now is left out of `rules`. In `kit`, each project's `context` part is `{"status": "not in use"}` when easy-context is not in use for that scope (same 7-day window ending now). When a call appears again, all of it returns. Measured values, thresholds and every other rule are unchanged.

**Out of scope**:

* The Kit page / Saved reports page (separate task, FR-080 UI)
* Removing the easy-context source, its ingest, its table or `kit_findings` server list
* Reading any Claude config file

**Requirement Refs**: FR-080 (server part)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner chose "Hide when not in use", 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `rules.evaluate` → MCP/CLI/API `warnings`, `improvement_plan`; `reports.get_report` → `GET /api/reports/{id}`; `kit.kit_summary` → `GET /api/kit`, MCP `kit_summary`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Fixture with no `mcp__easy-context__*` call in 7 days: `context_index_stale` is absent from `hits`, `abstained` and `checked`; `improvement_plan` has no item for it | FR-080 |
| 2 | Fixture with one such call 3 days ago: the rule behaves exactly as before (fires / abstains as today) | FR-080 |
| 3 | A call 8 days ago only → not in use | FR-080 |
| 4 | Scope: a call only in another workspace/account → not in use for this scope | FR-080 |
| 5 | Saved report whose snapshot has `context_index_stale` abstained, not in use now → the rule is not in `rules`; one measured then → `now.state == "not_in_use"` | FR-080 |
| 6 | Kit: every project's `context` part is `{"status": "not in use"}` when not in use; unchanged when in use | FR-080 |
| 7 | All other rules' outputs identical (existing goldens unchanged); leak suite green | FR-073, NFR-002 |
| 8 | Real run on a **copy** of the owner store: paste `easy-personal warnings` (no `context_index_stale` anywhere), `GET /api/reports/1` (rule gone), and one project's kit `context` part | FR-080 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests -q -k "rules or plan or warnings or kit or report or context or leak" && uv run pytest -q
```

## Approach

**Vital slice**: `context.in_use` (one SQL query on `events.tool_names`, scoped like the other metrics), one guard in `evaluate`, two small changes in `reports.get_report` and `kit`.
**Cut list**: no config setting, no Claude config reads, no removal of easy-context code.

---

## Edge Case Checklist

- [ ] `tool_names` storage format: check how it is stored (JSON / comma list) and match the prefix exactly; `mcp__easy-context-old__x` must not count
- [ ] Empty store / no history → rule left out (not in use), not an error
- [ ] Planted secret/text fixtures never appear in any store row, MCP/CLI/API response, page or log (leak suite green)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/sources/context.py` | `in_use` |
| `src/easy_personal/core/rules.py` | guard in `evaluate` |
| `src/easy_personal/core/reports.py` | `not_in_use`, drop abstained-then rule |
| `src/easy_personal/core/kit.py` | context part `not in use` |
| tests for rules, reports, kit, context | AC 1–7 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `web/` | page is another task |
| migrations, `src/easy_personal/data/rules.yaml` | not needed |
| `~/.easy-personal/store.sqlite` (live) | real run on a copy only |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Fixture stores with / without easy-context tool calls inside and outside the window and scope; rules, plan, reports, kit tests; goldens unchanged; leak suite; real run on a store copy.

---

## UI / Design Evidence

☐ N/A — backend only.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review