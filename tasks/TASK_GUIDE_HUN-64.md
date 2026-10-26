**Date**: 2026-10-26
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *Connector id / alias*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely, and `BRAINSTORMING_LOG_WORKSPACES.md` (verified facts W1–W10, edge cases)
4. Read `agents/backend.md`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md`

---

## Requirement (Pillar 1 — Adapt the requirement)

Workspaces must be created, listed, edited (including rename) and deleted from the dashboard and the CLI; fields name (slug, unique, `orphan` reserved), color/icon, description, main email (information only), repo paths (a repo may belong to several workspaces) (FR-028). Usage filters to events whose project path equals or sits under a workspace repo path (FR-035).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Add `workspaces` + `workspace_repos` tables with the built-in `orphan` row, CLI + guarded API CRUD, and a `workspace` filter on usage functions/CLI/API that resolves repo paths to stored project paths first.

**Out of scope** (what this task explicitly does NOT do):

* Connectors inside workspaces and delete→orphan move (HUN-65/HUN-66) — in this task delete simply removes the workspace and its repo rows
* Dashboard page (HUN-69)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-028
* FR-035
* US-014
* US-016
* NFR-009
* NFR-008

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-26)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (requirement grill 2026-10-26)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-61 — Store foundation

**Entry point**: `easy-personal workspace add|list|edit|delete`; `GET/POST /api/workspaces`, `PATCH/DELETE /api/workspaces/{id}`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Name slug `[a-z0-9][a-z0-9-]{0,39}`, unique, `orphan` reserved; enforced by CHECK/UNIQUE in the schema and by code | FR-028, NFR-009 |
| 2 | `orphan` exists after migration, cannot be renamed, edited or deleted | FR-028 |
| 3 | Rename changes the name only; the workspace id is fixed | FR-028 |
| 4 | Repo paths are normalised (`~` expanded, resolved, no trailing slash); a repo path may be in several workspaces | FR-028 |
| 5 | Usage functions + CLI `--workspace` + API `workspace=` count only events whose `project_path` equals or is under a workspace repo (resolved like `discovery.resolve_project`); implemented as `project_path IN (...)` over distinct stored paths, never `LIKE` on events | FR-035, NFR-009 |
| 6 | Workspace + account filters combine (AND) | FR-035, FR-036 |
| 7 | Write routes need token + Host/Origin + JSON | NFR-008 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Create `company` with repos `~/work/a` (subfolder `~/work/a/pkg` in events) and `~/work/b` | usage --workspace company counts both, excludes others | automated |
| 2 | `workspace add orphan` / rename orphan | refused | automated |
| 3 | 1M-event store, workspace filter | < 500 ms | automated (`-m perf`) |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/core tests/surfaces tests/leak -q && uv run pytest -m perf -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-64.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-64.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (backend/QA only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: HUN-62 account commands; `core/discovery.resolve_project`

**Vital slice**: CRUD via CLI + usage filter
**Cut list**: workspace icons beyond a color + emoji/short-name field; per-workspace dashboards

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Migration: `workspaces(id INTEGER PK, name TEXT UNIQUE NOT NULL CHECK(...), color TEXT, icon TEXT, description TEXT, email TEXT, is_orphan INTEGER NOT NULL DEFAULT 0)` + unique partial index allowing one `is_orphan=1`; triggers refuse UPDATE/DELETE of the orphan row. `workspace_repos(workspace_id INTEGER REFERENCES workspaces(id) ON DELETE CASCADE, repo_path TEXT, PRIMARY KEY(workspace_id, repo_path))`.
* Filter: SELECT DISTINCT project_path FROM events (small set) → keep those equal to or under any repo path (Python, path-aware, not string-prefix: `/a/b` must not match `/a/bc`) → `IN (...)` in the inner query.
* Email is stored and shown only; never used to send or match anything.

---

## Edge Case Checklist

- [ ] Repo path that does not exist (yet) → kept, matches nothing
- [ ] `/a/b` must not match `/a/bc`
- [ ] Workspace with zero repos → filter returns empty usage, not all usage
- [ ] Rename to an existing name → refused, nothing changed
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/core/workspaces.py` (new) | CRUD + filter resolution |
| `src/easy_personal/core/migrations/` | workspace tables |
| `src/easy_personal/core/metrics.py` | workspace filter |
| `src/easy_personal/surfaces/cli.py`, `api.py` | commands + routes |
| `tests/leak/harness.py` | register |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| real `~/.claude*/`, `~/.claude.json`, `~/.codex*/` | Never read or written by tests or verify: fixtures + temp HOME only (learnings: verify against fixtures). Product code writes Claude config only via `claude mcp add-json|remove --scope user` (DDR-0001) |
| `~/.claude*/projects/`, any `<repo>/memory/event-trace/`, `<repo>/reports/`, `~/.easy-verifier-reports/`, `~/.easy-context/` | Read-only sources (NFR-003) |
| `src/easy_personal/launch.py` beyond what the task states | NFR-006: stdlib + `keyring` only, < 200 ms |
| Sibling repos (`../easy-*`) | Conventions-only references |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files, not product code |

---

## Test Plan

CRUD + constraint tests (also direct SQL to prove constraints, not only code); filter correctness incl. prefix trap; perf test; API guard; leak registration.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (Medium/High risk)
- [ ] `Skill({ skill: "migration-safety" })` run (schema change) — Stage 3 and Stage 4
- [ ] Lint passes (`uv run ruff check .`)
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app (fixtures only)
- [ ] `memory/MEMORY.md` updated (if new patterns or feedback learned)
- [ ] Supervisor notified: task ready for Stage 4 review