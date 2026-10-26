**Date**: 2026-10-26
**Complexity Level**: C1
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

On first run with workspaces, pre-workspace connectors must be placed in `orphan` with their current name as both id and alias (keychain untouched), and — after one confirmation — removed from the Claude config they were registered in (FR-034).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Detect catalog-id connectors that exist in the keychain and/or the default account's Claude config but not in the `connectors` table; show the plan; on confirmation insert them into orphan and `claude mcp remove` them from the default account.

**Out of scope** (what this task explicitly does NOT do):

* Assigning them to a workspace (HUN-66 move does that)
* Dashboard banner (HUN-69)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-034
* FR-031
* DDR-0001

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

**Entry point**: `easy-personal connector adopt-legacy [--yes]`; `GET /api/connectors/legacy`, `POST /api/connectors/legacy` (confirm); `doctor` mentions pending legacy connectors

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Legacy = catalog id with keychain item `easy-personal/<id>` or `claude mcp get <id>` success in the default account, and no `connectors` row with that id | FR-034 |
| 2 | Adoption inserts `connectors(id=<name>, alias=<name>, workspace=orphan)`; keychain item untouched | FR-034 |
| 3 | Only after confirmation: `claude mcp remove <name> --scope user` in the default account; failure leaves the row in orphan and reports the step as pending | FR-034 |
| 4 | Running it again finds nothing (idempotent) | FR-034 |
| 5 | Later assigning the adopted connector to a workspace works through HUN-66 move with keychain still keyed by the old name | FR-031 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fake keychain has `easy-personal/slack`; fake claude has `slack` in default account | plan shows remove slack from Claude default; after yes: orphan row, keychain still there, claude entry gone | automated |
| 2 | Run twice | second run: nothing to adopt | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/connectors tests/surfaces tests/leak -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-67.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-67.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (backend/QA only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: HUN-65 sync; HUN-45 `doctor`

**Vital slice**: CLI adopt-legacy
**Cut list**: auto-adoption without the command (the dashboard banner in HUN-69 calls the same API)

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Reuse `writer._entry_exists` / `_keychain_item` for detection; reuse sync steps for the remove.

---

## Edge Case Checklist

- [ ] Keychain item but no Claude entry → adopt, no Claude call
- [ ] Claude entry but no keychain item → adopt (it will fail at launch until re-added) and say so
- [ ] Keychain backend missing → refuse with the existing guidance
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/connectors/sync.py` or `legacy.py` (new) | detection + adoption |
| `src/easy_personal/surfaces/cli.py`, `api.py` | command + routes |
| `tests/connectors/` | tests |
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

Fake keychain + fake claude scenarios; idempotence; leak registration.

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