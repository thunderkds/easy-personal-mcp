**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/backend.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/backend.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

The owner wants warnings labelled improve/aware/warning that cite the rule and measured value, and an improvement plan ordered by priority with a fix per item (PRD US-003, US-004, FR-006, FR-007, FR-026).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> A declared-rule engine with thresholds in config, shipping `cache_hit_low`, `session_huge_no_compact`, `tokens_jump`, `project_dominates`, `records_unreadable`; `warnings` and `improvement_plan` expose the hits.

**Out of scope** (what this task explicitly does NOT do):

* `tool_errors_high` (kit_trace task)
* `context_index_stale` (context task)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-006: declared rules, cite id + value, thresholds in config
* FR-007: plan grouped by priority with fix line
* FR-026: v1 rule pack
* NFR-004: deterministic

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-35 — `usage_breakdown` + `session_outliers`

**Entry point**: `warnings` and `improvement_plan` (MCP tools); `easy-personal warnings` / `easy-personal plan` (CLI)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Each of the 5 rules fires on a fixture built to cross its default threshold and not on one just below it | FR-026 |
| 2 | Every hit returns rule id, label, measured value, threshold and fix line | FR-006 |
| 3 | Thresholds are read from config; changing one changes firing (test) | FR-006 config |
| 4 | `improvement_plan` groups unmet rules by priority with their fix lines; same input → same output | FR-007, NFR-004 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fixture just above / just below each threshold | fires / does not fire | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_rules.py tests/surfaces -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-37.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-37.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: `easy-verifier-mcp/src/easy_verifier/core/score.py` (declared RatingRule pattern) — conventions only — never import or vendor sibling code (PROJECT_SPEC Critical Constraints)

**Vital slice**: rule declaration format + evaluator + the 5 rules
**Cut list**: No per-project rule overrides; No rule history over time

Rules as declarative data (id, metric fn name, comparator, threshold, label, priority, fix) evaluated over core metrics; no rule code reads raw events directly.

---

## Edge Case Checklist

- [ ] Too little data for a rule (e.g. < 7 days) → rule abstains, not passes
- [ ] `/compact` detection from metadata only (record type), never text
- [ ] Ties in project_dominates
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/{rules,plan}.py` | Engine + plan |
| `src/easy_personal/data/rules.yaml` | Rule pack defaults |
| `src/easy_personal/surfaces/{mcp,cli}.py` | Tools/commands |
| `tests/leak/` | Register new tools |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Threshold boundary tests per rule; config override test; determinism test; leak suite extended.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (if Medium/High risk)
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app
- [ ] `memory/MEMORY.md` updated (if new patterns or feedback learned)
- [ ] Supervisor notified: task ready for Stage 4 review