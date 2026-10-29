**Date**: 2026-10-29
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *API-equivalent cost*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Verify on fixtures only: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_AGENT_ROOT`; never open the real `~/.easy-personal/` or `~/.claude*/`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner issue 2026-10-29 #3: "from the dashboard, don't see the way to choose the workspace and account". Owner decision: **one global selector** in the sidebar; Usage, Warnings and Kit all filter by it. Today only the usage endpoints accept `workspace` / `account`; `/api/warnings`, `/api/plan` and `/api/kit` ignore them.

**Restated intent**:

> `rules.evaluate`, `plan.improvement_plan` and `kit.kit_summary` accept optional `workspace` and `account` and pass them to every metric they read through `metrics._scope`. `/api/warnings`, `/api/plan`, `/api/kit` read them from the query string like the usage routes. The MCP tools `warnings`, `improvement_plan`, `kit_summary` and the CLI commands gain the same optional arguments, matching the usage tools. No argument = today's output, byte for byte.

**Out of scope**: the selector UI (follow-up task); kit parts that are not per-account (git, context, verifier) — they filter by workspace (project list) only, and by account through the project list the account has used.

**Requirement Refs**: owner issue #3; FR-035, FR-036.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's decision (Supervisor, 2026-10-29)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: none. **Entry point**: `GET /api/warnings|plan|kit?workspace=&account=`; MCP + CLI.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Without `workspace`/`account`, the three outputs equal today's (golden test on the existing fixture store) | no regression |
| 2 | With `account`, usage rules measure only that account's events | account filter |
| 3 | With `workspace`, usage rules measure only the workspace's projects; `kit_summary` lists only those projects | workspace filter |
| 4 | With `account`, `kit_summary` lists only projects that account has events for | account filter |
| 5 | Unknown workspace → 404-style error JSON like the usage routes; unknown account → same as usage routes | consistent errors |
| 6 | MCP tools and CLI accept the same optional args; new flags registered in `tests/leak/harness.py` (`MCP_CALLS`, `CLI_CALLS`) | leak suite rule |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | 2 accounts, only B has low cache ratio | `warnings?account=B` fires `cache_hit_low`; `account=A` does not | pytest |
| 2 | workspace with 1 of 3 projects | kit lists 1 project | pytest |
| 3 | no args | identical JSON to before | pytest golden |

### Verification Command (exact, runnable)

```bash
uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-117.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-117.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

Pure backend task.

### 1. Visual Regression

☐ N/A — no UI change

### 2. Design-System Compliance

☐ N/A — no UI change

### 3. Layout / Responsiveness

☐ N/A — no UI change

---

## Approach

**Pattern reference**: `src/easy_personal/core/metrics.py` `_scope`; `surfaces/api.py` `_scope(q)`; usage MCP tools in `surfaces/mcp.py`.
**Vital slice**: thread `workspace`, `account` through `_measure` → metric functions.
**Cut list**: per-account thresholds; filtering git/verifier data itself.

---

## Edge Case Checklist

- [ ] `tokens_jump` (week over week) uses the same scope for both weeks
- [ ] `project_dominates` with a 1-project workspace → abstains (fewer than two projects)
- [ ] `tool_errors_high` (kit trace has no account) → filtered by workspace projects only; with only `account`, by that account's projects

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/rules.py`, `plan.py`, `kit.py` | scope args |
| `src/easy_personal/surfaces/api.py`, `mcp.py`, `cli.py` | pass args |
| `tests/`, `tests/leak/harness.py` | tests + registration |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/data/rules.yaml` | rule texts change in the warnings task |
| `web/` | follow-up task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest for AC 1–6; leak suite.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows N/A (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review