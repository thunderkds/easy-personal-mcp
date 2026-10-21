**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
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

The owner wants usage broken down by day, model, project and tool, with tokens as the main unit and $ only as a labelled API-equivalent cost from a dated price table (PRD US-001, FR-004, FR-005).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Breakdowns and outlier sessions are available through MCP and CLI; days follow the OS local timezone; cost is labelled "API-equivalent cost" and abstains for unknown models.

**Out of scope** (what this task explicitly does NOT do):

* Rules/warnings
* Dashboard

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-004: breakdowns, tokens primary, local day, folder names
* FR-005: dated price table, labelled, abstain
* FR-008: MCP usage tools

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-34 — Tracer bullet

**Entry point**: `usage_breakdown` and `session_outliers` (MCP tools); `easy-personal usage --by` (CLI)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `usage_breakdown(by=day|model|project|tool, period)` returns tokens per group; totals equal `usage_summary` | FR-004 |
| 2 | Days are computed in the OS local timezone (test with TZ set, including a DST change) | FR-004 local day |
| 3 | Every cost value is labelled `api_equivalent_cost` with the price-table date; a model missing from the table yields cost `abstained` for that model only | FR-005 |
| 4 | Project appears as folder name only in every response | FR-004 folder names |
| 5 | `session_outliers` lists sessions with ≥ 90 tool calls or very long contexts, with the measured value | US-001 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fixtures + TZ=Europe/Berlin across DST | Expected per-day split | automated |
| 2 | Model absent from price table | Its cost `abstained`, others priced | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core/test_metrics.py tests/core/test_prices.py tests/surfaces -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-35.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-35.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: The `usage_summary` path built in the dependency task (same core/surface split)

**Vital slice**: by-day and by-model breakdowns with correct local-day bucketing
**Cut list**: No currency other than USD; No per-branch breakdown

Price table as a dated data file in the package; metrics in core; surfaces only format. Bucket by local day using zoneinfo from the OS.

---

## Edge Case Checklist

- [ ] Carried from HUN-34: `project_path` is stored at ingest; the *missing on disk* mark is computed when shown (check `Path(project_path).exists()` at query time), not stored.
- [ ] DST transitions
- [ ] Sessions spanning midnight
- [ ] Price table older than N days → still used, date shown
- [ ] Empty period → empty groups, not an error
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/{metrics,prices}.py` | Breakdowns, cost |
| `src/easy_personal/data/prices.yaml` | Dated price table |
| `src/easy_personal/surfaces/{mcp,cli}.py` | Tools/commands |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Metric unit tests with fixed TZ; price abstain test; MCP/CLI parity test.

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