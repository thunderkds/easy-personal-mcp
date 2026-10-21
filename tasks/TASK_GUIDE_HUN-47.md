**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
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

Agents may see the catalog, which connectors are configured (yes/no) and usage-based suggestions, but can never create/modify a connector or read a secret (PRD FR-021, FR-022, US-011).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Three read-only MCP tools; suggestions come from counters over tool names, Bash first words and WebFetch/MCP target domains.

**Out of scope** (what this task explicitly does NOT do):

* Any MCP write tool

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-021
* FR-022

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-45 — Connector tracer; HUN-34 — Tracer bullet

**Entry point**: `connector_catalog`, `connector_status`, `connector_suggestions` (MCP tools)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Tool-surface test: no MCP tool can write config/keychain or return a secret | FR-021 |
| 2 | `connector_status` returns configured yes/no only | FR-021 |
| 3 | Suggestions derive only from the three metadata counters; a canary in message text never influences or appears | FR-022, NFR-002 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fixture sessions using `gh` and WebFetch to [slack.com](<http://slack.com>) | suggests Slack | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/connectors/test_mcp_tools.py tests/leak -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-47.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-47.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: Usage MCP tools from earlier tasks

**Vital slice**: status + suggestions
**Cut list**: No ranking beyond counts

Extend the claude_code source to count Bash first word and WebFetch/MCP domains at ingest (metadata only); map domains/commands to catalog providers.

---

## Edge Case Checklist

- [ ] Domains extracted without storing full URLs
- [ ] Bash first word only (no args)
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/connectors/suggestions.py` | Counters → suggestions |
| `src/easy_personal/sources/claude_code.py` | Counters |
| `src/easy_personal/surfaces/mcp.py` | Tools |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Tool-surface test; suggestion fixtures; leak suite.

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