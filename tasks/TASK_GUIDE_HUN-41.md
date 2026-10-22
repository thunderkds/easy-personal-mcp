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

Show easy-context index freshness per project by calling the easy-context service on 127.0.0.1 with read-only tools; never trigger indexing (PRD FR-012, FR-025; DDR-0002).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> During ingest, call `list_indexes`/`get_stats` on `http://127.0.0.1:8767/mcp`, record `last_index_at` per project, mark stale indexes and fire `context_index_stale`; when the service is down, report "no data".

**Out of scope** (what this task explicitly does NOT do):

* Calling `index_path` or any non-read tool
* Reading easy-context's SQLite files

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-012: read-only MCP client, no data on failure
* FR-025: stale rule
* NFR-001: 127.0.0.1 only
* DDR-0002

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-39 — `kit_trace` source (never `summary`) + `tool_errors_high` rule + `kit_summary` (kernel part); HUN-40 — `git_local` source (merged 2026-10-22): latest local commit time via `core.discovery.run_git(project, "log", "-1", "--format=%ct")`

**Entry point**: `kit_summary` context entry and rule `context_index_stale`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Client can only send tool names in the allow-list (`list_indexes`, `get_stats`); test proves `index_path` is refused locally | DDR-0002 |
| 2 | Only `127.0.0.1` URLs accepted; any other host refused | NFR-001 |
| 3 | Service down / timeout → ingest completes, context part = "no data" | FR-012 |
| 4 | Index older than the project's latest local commit → `context_index_stale` fires with fix "call `index_path`" | FR-025 |
| 5 | Spike answered and recorded: one `list_indexes` call vs. one call per repo header | DDR-0002 follow-up |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fake MCP server fixture returning stats | freshness stored | automated |
| 2 | No server | no data, ingest exit 0 | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/sources/test_context.py -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-41.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-41.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: `easy-context-mcp/README.md` (header `X-Easy-Context-Root`, endpoint) for the contract — conventions only — never import or vendor sibling code (PROJECT_SPEC Critical Constraints)

**Vital slice**: allow-listed client + freshness + rule
**Cut list**: No retries beyond one; No caching across ingests

Use the MCP SDK streamable-HTTP client with a short timeout; wrap in a tiny adapter that holds the allow-list. Compare `last_index_at` with git_local's latest commit time.

---

## Edge Case Checklist

- [ ] Service slow → bounded timeout so ingest/dashboard never hang
- [ ] Index for a repo that no longer exists
- [ ] Clock skew between container and host
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/sources/context.py` | Client + source |
| `src/easy_personal/data/rules.yaml` | stale rule |
| `tests/fakes/mcp_server.py` | Fake service |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Tests against a fake local MCP server; allow-list test; host guard test.

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