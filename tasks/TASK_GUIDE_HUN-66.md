**Date**: 2026-10-26
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P1
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

Deleting a workspace moves its connectors to `orphan` (id, alias, keychain kept) and removes them from every account; connectors are deleted one at a time only (FR-030). Moving a connector, renaming its workspace or changing its alias never touches the keychain — only Claude entries change (remove old alias, add new alias, same id) in every affected account; a move into a workspace that has that provider is refused; orphan is active in no account (FR-031).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Add move / alias / rename / delete-workspace operations as DB updates followed by the HUN-65 reconcile for every affected account, with default aliases following the workspace name and custom aliases kept.

**Out of scope** (what this task explicitly does NOT do):

* Legacy adoption (HUN-67)
* Dashboard (HUN-69)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-030
* FR-031
* FR-029
* US-014
* NFR-002

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-26)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (requirement grill 2026-10-26)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-65 — Connector id + alias + sync

**Entry point**: `connector move <id> <workspace>`, `connector alias <id> <alias|--default>`, `workspace edit --name`, `workspace delete`; matching API routes

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Move: refused if target (non-orphan) has the provider; otherwise workspace_id changes, default alias becomes `<target>-<provider>`, plan = remove old alias / add new alias in every affected account | FR-031 |
| 2 | Alias change: custom alias set (`alias_custom`=1) or reset to default; same plan shape | FR-031, FR-029 |
| 3 | Workspace rename: every default-alias connector of it gets the new alias; custom aliases unchanged | FR-031 |
| 4 | Workspace delete: connectors → orphan with id/alias kept; picks of that workspace removed; plan removes their aliases from every account; repo rows deleted | FR-030 |
| 5 | Assign out of orphan = move; orphan may hold several connectors of one provider and duplicate aliases are allowed only inside orphan | FR-030, FR-031 |
| 6 | Keychain is never read, written or deleted by any of these operations (fake keychain call log empty) | FR-031, NFR-002 |
| 7 | All operations show the plan and need confirmation; a failure mid-plan is repaired by re-running sync | FR-033 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | company-slack picked by 2 accounts; move to side (picked by 1 of them) | remove company-slack from both; add side-slack to the one picking side; keychain untouched | automated |
| 2 | Delete company with 2 connectors | both in orphan, removed from all accounts, workspace gone | automated |
| 3 | Move orphan slack into company that already has slack | refused, nothing changed | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/connectors tests/core tests/surfaces tests/leak -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-66.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-66.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (backend/QA only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: HUN-65 `sync.plan/apply`

**Vital slice**: move + delete→orphan
**Cut list**: bulk move UI semantics (one connector per call)

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Each operation = one DB transaction changing desired state, then `plan()` for the accounts whose desired or registered sets changed, then confirmation, then `apply()`. When not confirmed, roll back the DB transaction (dry-run: compute plan inside a transaction and ROLLBACK).
* Alias uniqueness check happens before commit; conflicts return a clear message naming the clashing connector.

---

## Edge Case Checklist

- [ ] Rename makes a default alias exceed 64 chars → refused before any change
- [ ] Custom alias equals another connector's default alias after a rename → refused, nothing changed
- [ ] Delete a workspace that no account picks → no Claude calls at all
- [ ] Move while an account folder is missing → DB change kept, that account's steps reported pending
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/connectors/sync.py` | operations |
| `src/easy_personal/core/workspaces.py` | rename/delete hooks |
| `src/easy_personal/surfaces/cli.py`, `api.py` | commands + routes |
| `tests/connectors/` | operation tests |
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

Operation tests with fake keychain call log + fake claude; dry-run rollback test; conflict tests.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (Medium/High risk)
- [ ] Lint passes (`uv run ruff check .`)
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app (fixtures only)
- [ ] `memory/MEMORY.md` updated (if new patterns or feedback learned)
- [ ] Supervisor notified: task ready for Stage 4 review