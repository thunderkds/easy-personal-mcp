**Date**: 2026-10-21
**Complexity Level**: C3
**Risk Level**: High
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

The owner wants Claude Code usage (sessions, tokens, cache) kept beyond the 30-day cleanup and summarised; only metadata may ever be stored (PRD US-001, US-002, FR-001–FR-003, FR-009, FR-027).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Running `easy-personal ingest` incrementally copies metadata from `~/.claude/projects/**/*.jsonl` into `~/.easy-personal/store.sqlite`, each session assigned to a project, and `usage_summary` (MCP + CLI) returns sessions, tokens in/out and cache-hit ratio for a period.

**Out of scope** (what this task explicitly does NOT do):

* Breakdowns, outliers, cost (next task)
* Rules
* Kit sources
* Scheduling (separate task)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-001: incremental, idempotent ingest
* FR-002: metadata only
* FR-003: abstain on unknown shapes
* FR-009: project assignment at ingest
* FR-027: machine_id on every row
* NFR-002, NFR-003, NFR-004

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-21)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (`grill-with-docs` terminology run 2026-10-21)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-32 — Spike; HUN-33 — Skeleton

**Entry point**: `usage_summary` (MCP tool) and `easy-personal ingest` / `easy-personal usage` (CLI)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Ingest of the fixtures stores only FR-002 fields; no column can hold message or tool content | metadata only |
| 2 | Re-running ingest adds nothing; appending lines to a fixture ingests only the new complete lines | incremental + idempotent |
| 3 | A truncated last line is not consumed; the offset never points mid-line | edge: live file |
| 4 | Unknown record types are counted as abstained per version and returned by `usage_summary` | FR-003 |
| 5 | Sessions in a subfolder / worktree / non-git folder / deleted folder are assigned per FR-009 and stored | FR-009 |
| 6 | Every row carries `machine_id`; times stored in UTC | FR-027, FR-004 |
| 7 | `usage_summary` MCP and `easy-personal usage` CLI return identical numbers for the same period | FR-008 one core |
| 8 | Source files are opened read-only; their mtime/size are unchanged after ingest | NFR-003 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fixtures from the spike | Expected session/token/cache totals (oracle written by Supervisor/QA before code) | automated |
| 2 | Fixture with planted secret in message text | Secret absent from every store row (`sqlite3 .dump` grep) | automated |
| 3 | Second ingest run | 0 new rows | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/sources/test_claude_code.py tests/core/test_store.py tests/surfaces/test_usage_summary.py -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-34.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-34.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: `easy-context-mcp/src/easy_context_mcp/store.py` (SQLite WAL setup) and `discovery.py` (git root detection) — conventions only — never import or vendor sibling code (PROJECT_SPEC Critical Constraints)

**Vital slice**: one source → one store table → one tool, end to end
**Cut list**: No per-model/day breakdowns; No cost; No retention pruning

Stream files line by line; keep `(path, inode, offset)` per file; parse JSON per complete line; map by declared per-version shape table, else abstain. Resolve project with `git rev-parse --show-toplevel` / `--git-common-dir` at ingest and cache per cwd; store full path, expose folder name. Store schema has explicit columns only (no JSON blob of the raw record).

---

## Edge Case Checklist

- [ ] **Dedupe tokens on `message.id`** (HUN-32 spike): one API message is split over several `assistant` records, one per content block, each repeating the same `usage`. 93 records carried 40 distinct ids; summing per record over-counts tokens ~2.3×. Count usage once per `message.id`.
- [ ] **Version is per record, not per file**: every real file mixed versions. ~28% of lines are metadata types with no `version` and no `cwd` (`agent-name`, `custom-title`, `last-prompt`, `atis-latch`, `queue-operation`, `cost-state`, `file-history-*`, `artifact-*`, `frame-link`) — classify them as known metadata, do NOT abstain on them.
- [ ] API-error `assistant` records (`isApiErrorMessage: true`, zero usage) appear from 2.1.302 — count the session, add 0 tokens.
- [ ] `cost-state.totalCostUSD` is Claude Code's own cost figure — do not read it (owner decision pending; FR-005 cost comes only from the price table).
- [ ] Fixtures: `tests/fixtures/claude_code/{2.1.293,2.1.295,2.1.302,unversioned}/` from HUN-32 (see `docs/spikes/transcript-shapes.md`); `isSidechain`, nested `subagents/` files and truncated last lines were NOT observed live but are in the fixtures as assumptions.
- [ ] Fix carried from HUN-33 review (P2): `core/config.py` `data_dir()` must not chmod a pre-existing directory it did not create (e.g. `EASY_PERSONAL_HOME=$HOME`); add a test.
- [ ] Partial last line
- [ ] File deleted by cleanup → rows kept, offset retired
- [ ] File replaced (same path, new inode) → restart offset
- [ ] Sessions spanning midnight
- [ ] cwd no longer exists at ingest → folder itself, marked missing on disk
- [ ] Huge files → bounded memory
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/sources/claude_code.py` | Versioned parser |
| `src/easy_personal/core/store.py` | Schema, offsets, WAL |
| `src/easy_personal/core/discovery.py` | Project assignment |
| `src/easy_personal/core/metrics.py` | usage_summary |
| `src/easy_personal/surfaces/{mcp,cli}.py` | Tool + commands |
| `tests/...` | Tests above |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Unit tests per parser shape from fixtures; store idempotency and offset tests; project assignment with temp git repos and worktrees; MCP/CLI parity test; planted-secret dump grep.

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