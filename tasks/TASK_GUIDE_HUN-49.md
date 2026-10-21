**Date**: 2026-10-21
**Complexity Level**: C2
**Risk Level**: High
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

An on-demand local dashboard on 127.0.0.1 that only this browser session can drive, with no remote assets, over the same core numbers (PRD FR-023, NFR-001, NFR-002, NFR-008; BRAINSTORMING_LOG B2).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> `easy-personal dashboard` runs ingest, starts Starlette on `127.0.0.1` at a free port with a per-launch token, serves the built SPA and a read-only JSON API for usage, and opens the browser; Ctrl-C stops it.

**Out of scope** (what this task explicitly does NOT do):

* Connector write endpoints (forms task)
* UI views

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-023
* NFR-001
* NFR-002
* NFR-008

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-35 — `usage_breakdown` + `session_outliers`; HUN-48 — `web/` toolchain

**Entry point**: `easy-personal dashboard` (CLI) and `GET /api/usage/summary`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Binds `127.0.0.1` only; free port chosen; URL printed | NFR-001 |
| 2 | Every API request without the token, or with a `Host`/`Origin` other than the dashboard's own, gets 403 (incl. DNS-rebinding Host) | NFR-008 |
| 3 | CSP `default-src 'self'` on every page response | NFR-001 |
| 4 | API numbers equal MCP numbers for the same period (parity test) | FR-008 |
| 5 | Token never appears in logs (access log off or redacted) | NFR-002 |
| 6 | Leak suite extended to every API route | NFR-002 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Request with Host: evil.example | 403 | automated |
| 2 | Request from another origin | 403 | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/surfaces/test_api.py tests/leak -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-49.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-49.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: None in repo; Starlette comes with the MCP SDK (BRAINSTORMING_LOG F10)

**Vital slice**: guarded server + one read endpoint
**Cut list**: No HTTPS (loopback only)

Starlette app with middleware for token + Host/Origin; token passed to the SPA via the opened URL fragment, then held in memory and sent as a header.

---

## Edge Case Checklist

- [ ] Port in use
- [ ] Browser not available (headless) → print URL
- [ ] Ingest lock held by the timer → wait with message
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/surfaces/api.py` | Server + API |
| `src/easy_personal/surfaces/cli.py` | dashboard command |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Starlette TestClient tests for guard, CSP, parity; leak suite.

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