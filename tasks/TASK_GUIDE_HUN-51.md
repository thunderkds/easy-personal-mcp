**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/frontend.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/frontend.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

Warnings with labels and measured values, the improvement plan, and per-project kit results with explicit "no data" (PRD US-003, US-004, US-007, FR-023).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Two views reading the read API (extended by this task's backend counterpart in the same API module): warnings + plan, and kit panel per project (kernel, verifier, context freshness, git) plus easy-ui overall.

**Out of scope** (what this task explicitly does NOT do):

* Connector forms

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-023
* FR-006
* FR-007
* FR-015

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-50 — Dashboard usage view; HUN-37 — Rules engine + 5 usage rules + `warnings` + `improvement_plan`; HUN-39 — `kit_trace` source (never `summary`) + `tool_errors_high` rule + `kit_summary` (kernel part)

**Entry point**: Dashboard routes `/warnings` and `/kit`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Each warning shows label, rule id, measured value, threshold, fix line | FR-006 |
| 2 | Plan grouped by priority | FR-007 |
| 3 | Kit panel shows "no data" explicitly, never zero; stale index highlighted | FR-015, FR-025 |
| 4 | Read endpoints for warnings/plan/kit added behind the same guard and leak suite | NFR-008, NFR-002 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Seeded store with hits and no-data parts | rendered as specified | component test + easy-ui session |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && uv run pytest tests/surfaces/test_api.py tests/leak -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-51.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-51.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

> UI evidence is captured with the easy-ui container during development (PRD Open Question 9): QA drives the running dashboard with easy-ui and attaches the session JSON + screenshots.

### 1. Visual Regression

| Screen / Component | Verification method | Expected result |
| -- | -- | -- |
| Warnings & plan view; Kit panel | easy-ui screenshot + LLM vision review against this guide's AC | matches AC; no broken layout |

### 2. Design-System Compliance

| Criterion | Verification method | Expected result |
| -- | -- | -- |
| Colors match design tokens | CSS audit of `web/src` tokens file | all colors come from the tokens file |
| Typography matches spec | computed style via easy-ui | one font family, sizes from tokens |
| Spacing / layout matches spec | easy-ui screenshot + LLM vision | consistent spacing scale |

### 3. Layout / Responsiveness

| Viewport | Verification method | Expected result |
| -- | -- | -- |
| Mobile (320–480px) | easy-ui screenshot | single column, no horizontal scroll |
| Tablet (768px) | easy-ui screenshot | single or two columns, readable charts |
| Desktop (1024px+) | easy-ui screenshot | multi-panel layout |

---

## Approach

**Pattern reference**: Usage view task

**Vital slice**: warnings list + kit table
**Cut list**: No drill-down per kit task id

Small read endpoints added to the existing API module (same guard); views reuse the usage view's components.

---

## Edge Case Checklist

- [ ] Project missing on disk
- [ ] Many projects → sortable table
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/**` | Views |
| `src/easy_personal/surfaces/api.py` | Read endpoints |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Vitest + API tests + easy-ui session.

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