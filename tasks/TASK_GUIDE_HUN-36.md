**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P0
**Assigned agent**: QA-Automation-Agent
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/qa.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/qa.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

No secret, message text or tool output may appear in any tool response, report, log or store row; fixtures with planted secrets prove it (PRD NFR-002, FR-002).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> A reusable leak suite: canary strings planted in every fixture field that carries content, and checks that scan the store dump, every MCP/CLI output and captured logs for them.

**Out of scope** (what this task explicitly does NOT do):

* Fixing leaks found in other code (report them to the Supervisor)
* Dashboard/API checks (added by the dashboard tasks via the suite's helper)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* NFR-002: no secret/text in any output
* FR-002: metadata only

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

**Entry point**: Standalone — N/A: a test suite (`tests/leak/`) run by `uv run pytest`; extended by later source/surface tasks

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Canary helper plants unique tokens (fake API key, email, message text, file content) into fixtures | planted secrets |
| 2 | Suite runs ingest + every current MCP tool + CLI command and asserts no canary in outputs, logs or `sqlite3 .dump` | all surfaces |
| 3 | Adding a new MCP tool without registering it in the suite fails a meta-test | suite stays complete |
| 4 | Suite is documented so later tasks extend it (sources, API) | extensible |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Current tools | 0 canary hits | automated |
| 2 | A deliberately leaking test tool | Suite fails | automated (self-test) |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/leak -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-36.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-36.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: None — no comparable prior art in this repo; follow `agents/qa.md` oracle rules

**Vital slice**: canary planting + store dump scan + tool-output scan
**Cut list**: No fuzzing; No scan of OS-level logs (only logs the package writes)

Session-scoped fixture that builds a temp `~/.easy-personal/` and fixture source tree with canaries, runs ingest, enumerates registered MCP tools via the server's tool list, calls each with default args, captures stdout/stderr/logging.

---

## Edge Case Checklist

- [ ] Canary split across JSON escapes (\\u-escapes)
- [ ] Base64 or URL-encoded forms of the canary
- [ ] Error messages that echo input
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `tests/leak/` | Suite + helpers |
| `tests/fixtures/leak/` | Canary fixtures |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

The suite itself plus a self-test with a deliberately leaking tool.

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