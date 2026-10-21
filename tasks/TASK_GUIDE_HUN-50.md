**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
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

The owner reviews the week at a glance: usage charts and the session timeline (per session start→end, model, token use over time, gaps), with a notice that cloud sessions and other machines are not included (PRD US-012, FR-023, FR-004, FR-005).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> The Usage view shows totals, breakdowns by day/model/project, outlier sessions and the session timeline, tokens first and cost labelled API-equivalent.

**Out of scope** (what this task explicitly does NOT do):

* Warnings/plan/kit panel
* Connector forms

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-023
* FR-004
* FR-005

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-49 — Dashboard server

**Entry point**: Dashboard route `/` (Usage view)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | View renders totals and breakdowns from the API; numbers equal API values | FR-023 |
| 2 | Session timeline shows start→end, model, token use over time and gaps per session | FR-023 |
| 3 | Cost always labelled "API-equivalent cost"; abstained cost shown as such | FR-005 |
| 4 | Notice: cloud sessions and other machines are not included | FR-023 |
| 5 | No request leaves 127.0.0.1 (easy-ui network log) | NFR-001 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Seeded store | matching numbers on screen | component test + easy-ui session |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-50.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-50.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

> UI evidence is captured with the easy-ui container during development (PRD Open Question 9): QA drives the running dashboard with easy-ui and attaches the session JSON + screenshots.

### 1. Visual Regression

| Screen / Component | Verification method | Expected result |
| -- | -- | -- |
| Usage view (totals, breakdown charts, session timeline, notice) | easy-ui screenshot + LLM vision review against this guide's AC | matches AC; no broken layout |

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

**Pattern reference**: The `web/` placeholder from the toolchain task

**Vital slice**: totals + by-day chart + timeline
**Cut list**: No date-range picker beyond preset periods

Chart library chosen in this task (bundled, no CDN); data via the guarded API client holding the token.

---

## Edge Case Checklist

- [ ] Empty store → clear empty state
- [ ] Very long sessions on the timeline
- [ ] Abstained values rendered distinctly, never as zero
- [ ] Carried from HUN-35 review (P2): `usage_breakdown(by="project")` groups by full path but shows only the folder name, so two projects with the same folder name give two groups with the same `key`. Do not index groups by `key` alone; render both rows.
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/**` | Usage view |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Vitest component tests; easy-ui session for UI evidence.

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