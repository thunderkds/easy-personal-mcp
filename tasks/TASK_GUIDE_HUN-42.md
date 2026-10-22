**Date**: 2026-10-21
**Complexity Level**: C1
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

Verifier task/job scores over time per project, read only from the score JSON easy-verifier writes next to each HTML report; HTML-only reports are "no data" (PRD FR-011; XR-1).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Read score JSON files from `<repo>/reports/` and `~/.easy-verifier-reports/<repo>/`, store score, scope and time per project, and add them to `kit_summary`.

**Out of scope** (what this task explicitly does NOT do):

* Parsing HTML
* Calling easy-verifier
* Changing easy-verifier (XR-1 lives on its board)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-011: JSON only, HTML-only = no data
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

**Depends on**: HUN-39 — `kit_trace` source (never `summary`) + `tool_errors_high` rule + `kit_summary` (kernel part). External XR-1 = easy-verifier-mcp T120 — Done 2026-10-21; shape pasted under `## Approach`

**Entry point**: `kit_summary` verifier entry

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Scores from fixture JSON stored per project/time | FR-011 |
| 2 | HTML report without JSON → counted as no data | FR-011 |
| 3 | Unknown JSON schema version → abstain | FR-003 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fixture JSON (shape agreed in XR-1) | score stored | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/sources/test_verifier.py -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-42.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-42.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: kit_trace source

**Vital slice**: JSON read + kit part
**Cut list**: No per-dimension breakdown unless the JSON makes it free

XR-1 is done: easy-verifier-mcp T120 (Done 2026-10-21). Agreed shape, `schema_version` 1 (from `easy-verifier-mcp/src/easy_verifier/core/report.py` `_score_summary`; real file `../easy-verifier-mcp/reports/evidence-report-project-20261021T124041-825600Z.json`):

```json
{
  "schema_version": 1,
  "scope": "project",
  "generated_at": "2026-10-21T12:40:41Z",
  "overall": {"value": 48, "abstained": false, "reason_code": null},
  "dimensions": [
    {"dimension": "api", "rating": 0, "abstained": false, "reason_code": null},
    {"dimension": "architecture", "rating": 100, "abstained": false, "reason_code": null}
  ]
}
```

* File name `evidence-report-<scope>-<UTC>.json`, beside the HTML of the same base name. `overall.value` is `null` (never 0) when abstained; `dimensions` may be empty.
* Locations per project (folder name = repo directory name): `<project>/reports/` and `~/.easy-verifier-reports/<project folder name>/` (the shared container dir, `EASY_VERIFIER_REPORTS_DIR`). Match the folder name exactly: dirs like `<project>-HUN-42` are task-worktree reviews, not the project — ignore them.
* Per project keep the newest file by `generated_at` (tie: file name). Store project, scope, generated_at, overall value/abstained/reason_code; dimensions only if cheap (cut list allows skipping).
* Reject anything else: unknown `schema_version`, missing keys, wrong types, non-JSON, files over a small size cap → skip that file (count it as skipped), never crash ingest. Read with no symlink following outside the dir. Never read or parse the HTML.
* `kit_summary` part `verifier`: `{"status":"no data"}` when no file, else `{scope, generated_at, overall: {value, abstained, reason_code}}` (+ `dimensions` if stored).
* Existing HTML-only reports (before T120) have no JSON: they give "no data" — no back-fill.
* Note: the running easy-verifier container predates T120, so real JSON files appear only after it is updated; tests use fixtures in exactly this shape.

---

## Edge Case Checklist

- [ ] Both report locations have the same report
- [ ] Repo name collisions in `~/.easy-verifier-reports/`
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/sources/verifier.py` | Source |
| `src/easy_personal/core/kit.py` | part |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Fixture-based tests.

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