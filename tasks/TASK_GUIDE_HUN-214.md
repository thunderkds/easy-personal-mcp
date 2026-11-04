**Date**: 2026-11-04
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Rule, Abstained, Owner project, Scratch; Critical Constraints: deterministic, metadata only)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-006, FR-007, FR-026, FR-057, FR-069, FR-073, NFR-004; then `src/easy_personal/core/rules.py`, `src/easy_personal/core/plan.py`, `src/easy_personal/data/rules.yaml`, `src/easy_personal/core/metrics.py` (token_total, usage_breakdown, owners grouping) and the existing rules/plan tests

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-04, about the Warnings & plan page: "can not get the idea here" and "no solution related, so it make the users confuse too much in the case we just showing this". Owner said "yah, should" to: each warning shows the cause from the owner's own data and a fix that matches that cause.

Supervisor probe of the owner store (read-only, 2026-11-04) for the one hit, `tokens_jump` (+200.5%): previous 7 days 1,249.0M tokens / 100 sessions, last 7 days 3,756.5M / 395 sessions, so tokens per session fell (12.5M → 9.5M); 97% of tokens are cache reads; biggest rises `personal-agentic-claude` +409M (32 → 74 sessions), `easy-verifier-mcp` +400M (2 → 11), `idea-radar` +376M (new, 42 sessions). Today the fix line says only "Open Usage and find which project or session drove the rise", which the owner found useless.

**Restated intent**:

> In `core/rules.evaluate`, each hit gets `causes` (≤ 3 items: `name`, `kind` (`project` | `session` | `tool` | `record_type`), numbers, a short fixed `text`) and a `diagnosis` id, worked out from the same window and scope (workspace/account) as the rule, with FR-057/FR-069 project grouping. A fixed per-rule table in the code maps each diagnosis to its fix lines (`steps`), replacing the generic steps for that hit. Per-rule causes and diagnoses exactly as FR-073 lists them. Where a rule counts tokens, the hit also carries `cache_read_share`. `plan.improvement_plan` items carry the same fields. Measured values, thresholds, labels, priorities and which rules fire are unchanged. All text is fixed in code (NFR-004: no LLM).

**Out of scope**:

* The dashboard page (separate task, FR-074) and the MCP prompt (separate task, FR-075)
* New rules, new thresholds, changes to `rules.yaml` keys other than adding fix lines per diagnosis if you keep the table there (prefer a table in `rules.py` beside the metric functions; say which in the report)
* Links/deep links into the Usage page

**Requirement Refs**: FR-073 (FR-006, FR-007, FR-026 kept)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner "yah, should", 2026-11-04)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `rules.evaluate` → MCP `warnings` / `improvement_plan` (`surfaces/mcp.py`), CLI `easy-personal warnings` / `plan`, `GET /api/warnings` / `GET /api/plan` (`surfaces/api.py`)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `tokens_jump` fixture: 4× sessions, tokens per session down → diagnosis `more_sessions`; causes = 3 projects with the largest rise, a new project marked `new: true`; steps say the rise is more sessions, not bigger ones, and name the new project | FR-073 |
| 2 | `tokens_jump` fixture: same session count, tokens per session +80% → diagnosis `bigger_sessions`; causes = the 3 largest sessions (id, project, tokens) | FR-073 |
| 3 | `cache_hit_low`, `session_huge_no_compact`, `project_dominates`, `tool_errors_high`, `records_unreadable`, `context_index_stale`: each fixture hit has ≤ 3 causes of the FR-073 kind and a diagnosis with its fixed steps | FR-073 |
| 4 | Scope: with `workspace=` / `account=` the causes come only from that scope; Scratch folders appear as one `Scratch` project (FR-069) | FR-073 |
| 5 | Measured values, thresholds and the set of fired rules are identical to before on the existing rules fixtures (golden tests unchanged or updated only by the new fields) | FR-073 |
| 6 | `improvement_plan` items carry the same `causes`, `diagnosis`, `steps` as the matching warning | FR-073, FR-007 |
| 7 | No message or tool content in any cause (leak suite green; causes hold names, ids, numbers only) | FR-002 |
| 8 | Real run on a **copy** of the owner store: paste `easy-personal warnings` JSON for the `tokens_jump` hit; it should match the Supervisor probe above (≈ 100 → 395 sessions, `more_sessions`) | FR-073 |
| 9 | `warnings` stays within NFR-013 (< 2 s median of 3 on the store copy); paste timings before/after | NFR-013 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
uv run pytest tests -q -k "rules or plan or warnings" && uv run pytest -q
```

## Approach

**Vital slice**: a `_causes_<metric>(conn, since, until, measured, **scope) -> (diagnosis, causes)` function per metric beside the metric functions, a `FIXES = {(rule_id, diagnosis): [steps]}` table, wired into `evaluate`; tests per rule.
**Cut list**: no new SQL index unless AC 9 fails; no Usage deep links; no LLM; no change to which rules fire.

Reuse `metrics` helpers (token totals by project, sessions) and the FR-057/FR-069 grouping; do not write new grouping code.

---

## Edge Case Checklist

- [ ] A rule that abstains → no causes, unchanged abstain reason
- [ ] Fewer than 3 candidates → fewer causes, never padding
- [ ] Previous window has a project the current one lacks → never a negative "rise" cause at the top
- [ ] Unpriced models → causes use tokens, never $0 cost
- [ ] Planted secret/text fixtures never appear in any store row, MCP/CLI/API response, page or log (leak suite green)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/rules.py` | causes + diagnosis + fix table |
| `src/easy_personal/core/plan.py` | carry the fields |
| `tests/core/test_rules.py`, plan tests, surface golden files if they pin the JSON | AC 1–7 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `web/` | page is another task |
| `src/easy_personal/core/store.py`, migrations | read-only feature |
| `~/.easy-personal/store.sqlite` (live) | real run on a copy only (`sqlite3 ~/.easy-personal/store.sqlite ".backup <copy>/store.sqlite"`) |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Fixture stores per rule (two 7-day windows), assert diagnosis, causes and steps; scope tests; golden JSON for surfaces updated; leak suite; timing on a store copy.

---

## UI / Design Evidence

☐ N/A — backend only.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review