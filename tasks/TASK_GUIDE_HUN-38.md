**Date**: 2026-10-21
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Common-Infrastructure-Agent
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Note the **Complexity Level** above and apply the matching process (brainstorm / decompose / verify depth) from the Complexity matrix in your role guide (`agents/common-infrastructure.md`)
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md` for directory layout, entry points, and blast-radius hotspots — skip if the task is C0/C1 and touches a single known file

---

## Requirement (Pillar 1 — Adapt the requirement)

The 30-day transcript cleanup must lose nothing; ingest must be schedulable on Linux; history is kept forever unless `retention_days` prunes (PRD FR-024, FR-027, US-002).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> One command installs a systemd user timer that runs `easy-personal ingest` on schedule; concurrent ingests never corrupt the store; an optional `retention_days` prunes old rows.

**Out of scope** (what this task explicitly does NOT do):

* cron support beyond printing a snippet
* macOS/Windows

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-024: schedulable ingest on Linux
* FR-027: keep forever, retention_days prunes

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

**Entry point**: `easy-personal schedule install` / `easy-personal schedule remove` (CLI)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | `schedule install` writes a user unit + timer under `~/.config/systemd/user/` and enables it; `schedule remove` reverses it | FR-024 |
| 2 | Two concurrent ingests: the second waits or exits with a message; store integrity check passes | single writer |
| 3 | With `retention_days` unset nothing is pruned; set to N prunes rows older than N days only | FR-027 |
| 4 | Without systemd user session, the command prints a cron snippet and exits non-zero without writing | fallback |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Two ingests started together | `PRAGMA integrity_check` = ok, no duplicate rows | automated |
| 2 | `retention_days=30` | rows older than 30 days gone, newer kept | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/ops -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-38.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-38.md` (review sections are never taken from the board).

## Approach

**Pattern reference**: `easy-context-mcp/src/easy_context_mcp/jobs.py` (single-writer lock) — conventions only — never import or vendor sibling code (PROJECT_SPEC Critical Constraints)

**Vital slice**: file lock + timer install
**Cut list**: No notification on failed scheduled runs

flock on a lock file in `~/.easy-personal/`; WAL already on. Unit files rendered from a template with the absolute path of the installed `easy-personal`.

---

## Edge Case Checklist

- [ ] Laptop asleep at timer time → `Persistent=true`
- [ ] Installed path changes after reinstall → `schedule install` is idempotent and rewrites
- [ ] Lock file left by a crashed run
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/lock.py` | Single writer |
| `src/easy_personal/surfaces/cli.py` | schedule commands |
| `src/easy_personal/data/systemd/*` | Unit templates |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `~/.claude/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `easy-ui-mcp/reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `~/.claude.json` (direct edit) | Only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

Concurrency test with two processes; unit rendering test with a temp XDG dir; retention test.

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