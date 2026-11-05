**Date**: 2026-11-05
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Saved report, Rule snapshot, Before / now)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-076, FR-078, FR-079; `src/easy_personal/core/reports.py`, `src/easy_personal/core/rules.py` (`evaluate`, `load_rules`, `Rule`), `tests/core/test_reports.py`, `tests/surfaces/test_api_reports.py`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05, after seeing the Saved reports section: "the report content is full, but the showup information is not good, can we follow the same format with our warning and plan". Owner chose "Warning cards" (FR-079). The page needs every field the Warnings & plan card uses, and the snapshot today keeps only `rule_id`, `summary`, `diagnosis`, `causes` of a fired rule.

**Restated intent**:

> In `reports.save`, store each fired rule's full `evaluate` hit (every key the hit has: `rule_id`, `label`, `priority`, `measured`, `threshold`, `window_days`, `fix`, `title`, `why`, `steps`, `summary`, optional `detail`, `causes`, `diagnosis`, optional `cache_read_share`) in `snapshot["fired"]`. In `get_report`, a fired rule's `before` carries all those keys plus `state` (keep the existing `state`, `measured`, `threshold`, `fired` fields). For a snapshot saved before this change (a fired entry without `label`), fill `label`, `priority`, `why`, `fix`, `window_days` from the current rule config for that `rule_id`, and leave `steps` out. If the rule is no longer in the config, leave those fields out. `now` is unchanged. No migration: the column is JSON.

**Out of scope**:

* The page (separate task, FR-079 UI)
* Any change to `evaluate`, rule maths, `list_reports`, limits or the secret check

**Requirement Refs**: FR-079 (server part)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner chose "Warning cards", 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `reports.save` (MCP `save_report`) and `reports.get_report` (`GET /api/reports/{id}`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | A new save stores the full fired hit; `GET /api/reports/{id}` `before` of that rule has `label`, `priority`, `why`, `steps`, `summary`, `causes`, `diagnosis` equal to the `warnings` output at save time | FR-079 |
| 2 | An old-shape snapshot (fired entry with only `rule_id`, `summary`, `diagnosis`, `causes`) returns `before` with `label`, `priority`, `why`, `fix` from the current config and no `steps` | FR-079 |
| 3 | Old-shape snapshot whose rule is gone from the config: no crash; `before` keeps its stored fields | FR-079 |
| 4 | Existing HUN-221 tests stay green; leak suite green | FR-076, NFR-002 |
| 5 | Real run on a **copy** of the owner store (it holds report 1, old shape): paste `GET /api/reports/1` showing the filled fields | FR-079 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests -q -k "report or leak" && uv run pytest -q
```

## Approach

**Vital slice**: one line change in `save` (store the whole hit), a small `_fill_from_config(entry)` in `get_report`, tests.
**Cut list**: no migration, no backfill of stored rows, no API for rule units.

---

## Edge Case Checklist

- [ ] `hit["detail"]` / `cache_read_share` absent → not added
- [ ] Old snapshot + rule removed from config → stored fields only
- [ ] Planted secret/text fixtures never appear in any store row, MCP/CLI/API response, page or log (leak suite green)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/reports.py` | full hit in snapshot; fill old shape |
| `tests/core/test_reports.py`, `tests/surfaces/test_api_reports.py` | AC 1–3 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `web/` | page is another task |
| `src/easy_personal/core/rules.py`, migrations | not needed |
| `~/.easy-personal/store.sqlite` (live) | real run on a copy only |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Unit tests on `save` / `get_report` with a fixture store; an old-shape snapshot row inserted by hand; API test; real run on a store copy.

---

## UI / Design Evidence

☐ N/A — backend only.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review