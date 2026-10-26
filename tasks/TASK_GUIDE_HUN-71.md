**Date**: 2026-10-26
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: QA-Automation-Agent
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *Connector id / alias*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely, and `BRAINSTORMING_LOG_WORKSPACES.md` (verified facts W1–W10, edge cases)
4. Read `agents/qa.md`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md`

---

## Requirement (Pillar 1 — Adapt the requirement)

Independent verification of the workspace feature across accounts: FR-028..FR-036, NFR-002, NFR-003, NFR-005, NFR-009 hold together end to end.

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> A smoke suite on a temp HOME with three fake Claude accounts + one Codex, fake keychain and fake `claude`: detect → confirm → ingest → create workspaces → add connectors → pick → sync → move → delete → adopt legacy → usage filters, asserting Claude config writes per account, no secret in any output, and that nothing outside the temp HOME is touched.

**Out of scope** (what this task explicitly does NOT do):

* New product code (report defects to the Supervisor via the bugfix flow)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-028
* FR-029
* FR-030
* FR-031
* FR-032
* FR-033
* FR-034
* FR-035
* FR-036
* NFR-002
* NFR-003
* NFR-009

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-26)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (requirement grill 2026-10-26)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-66 — Move/alias/rename/delete; HUN-67 — Legacy adoption

**Entry point**: `tests/e2e/test_workspaces_flow.py` (CLI + in-process API)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Full flow passes via CLI and via API with identical end state | FR-028..FR-036 |
| 2 | Fake `claude` call log shows each call with the expected `CLAUDE_CONFIG_DIR` (unset for default) | FR-033 |
| 3 | Planted secrets never appear in store, outputs, plan lines or logs | NFR-002 |
| 4 | A guard asserts no file outside the temp HOME / temp store changed (real HOME untouched) | NFR-003 |
| 5 | Store `user_version` latest, `PRAGMA foreign_key_check` and `integrity_check` clean after the flow | NFR-009 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Temp HOME, 3 Claude + 1 Codex | flow passes | automated |
| 2 | Planted secret values | absent everywhere | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/e2e tests/leak -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-71.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-71.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (backend/QA only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: `tests/leak/harness.py`; `tests/connectors/conftest.py` fakes

**Vital slice**: one end-to-end flow + leak + no-touch guard
**Cut list**: UI e2e (UI evidence is in HUN-68–HUN-70)

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Use `EASY_PERSONAL_AGENT_ROOT`, `EASY_PERSONAL_HOME`, fake keyring and fake claude from existing fixtures; record mtimes of real `~/.claude*` paths before/after as the no-touch guard (read-only stat).

---

## Edge Case Checklist

- [ ] Run the flow twice → second run makes no Claude calls (idempotent)
- [ ] Kill between plan steps → re-sync converges
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `tests/e2e/` (new) | flow test |

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

The suite itself; report defects, don't patch product code.

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