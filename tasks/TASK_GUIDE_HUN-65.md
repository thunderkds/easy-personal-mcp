**Date**: 2026-10-26
**Complexity Level**: C3
**Risk Level**: High
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

Every connector has a fixed id (keychain + launcher key) and an alias (Claude Code name, default `<workspace>-<provider>`, unique among non-orphan connectors), belongs to exactly one workspace (one per provider, except orphan) (FR-029). A confirmed Claude account picks zero or more workspaces (never orphan); their connectors are written/removed only via `claude mcp add-json|remove --scope user` with `CLAUDE_CONFIG_DIR` for that account; Codex listed only; every write shows its plan and needs confirmation (FR-033).

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Add `connectors`, `account_workspaces` and `account_registrations` tables; `connector add --workspace` stores the secret under the new id; `account use <account> <workspace…>` records picks; one reconcile `plan(account)`/`apply` adds/removes Claude entries (alias → `exec <id>`) in the right config, after confirmation, via the existing writer.

**Out of scope** (what this task explicitly does NOT do):

* Move / alias change / workspace rename / delete→orphan (HUN-66)
* Legacy `slack`/`gmail` adoption (HUN-67)
* Dashboard pages (HUN-68/HUN-69) — `ConnectorsView` add may break until HUN-69; its Vitest mocks fetch so tests still pass

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-029
* FR-033
* FR-019
* FR-021
* DDR-0001
* NFR-002
* NFR-008
* NFR-009
* US-014
* US-015

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-26)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (requirement grill 2026-10-26)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-62 — Agent accounts; HUN-64 — Workspaces

**Entry point**: `easy-personal connector add <provider> --workspace W [--alias A]`, `connector remove <id>`, `account use <account> <workspace…>`, `connector sync [--account A] [--yes]`, `doctor`; `POST /api/connectors` (now with `workspace`), `PUT /api/accounts/{id}/workspaces`, `GET/POST /api/sync`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Schema: `connectors(id TEXT PK, alias TEXT NOT NULL, alias_custom INTEGER, provider TEXT, workspace_id FK)`; partial unique indexes `(workspace_id, provider)` and `(alias)` where workspace is not orphan; `account_workspaces(account_id FK, workspace_id FK, PK)` refusing orphan and non-claude/unconfirmed accounts (trigger); `account_registrations(account_id, alias, connector_id)` = what was written | FR-029, FR-033, NFR-009 |
| 2 | `connector add` creates id `c` + 8 hex, stores secrets in keychain service `easy-personal/<id>`; no Claude write happens until a sync | FR-029, FR-017 |
| 3 | `plan(account)` = add (desired − registered) and remove (registered − desired); each line names account, alias and `exec <id>`; never touches a name not in `account_registrations` / connectors | FR-033 |
| 4 | Default account runs `claude` with `CLAUDE_CONFIG_DIR` unset; other accounts with it set to the folder (asserted on the fake `claude` env) | FR-033 |
| 5 | `apply` is idempotent and resumable: a failure mid-plan leaves registrations matching what actually succeeded; re-run completes | FR-033 |
| 6 | Every write path shows the plan and needs confirmation (`--yes` / API `confirm: true`) | FR-033 |
| 7 | Codex accounts can not pick workspaces (refused) | FR-033 |
| 8 | No secret value in any table, plan line, CLI/API output or log | NFR-002 |
| 9 | MCP stays read-only (no new MCP write tools) | FR-021 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Workspace company (slack), personal (gmail); accounts default→personal, work→company,personal | plan: default add personal-gmail; work add company-slack + personal-gmail; fake claude receives the right CLAUDE_CONFIG_DIR per call | automated |
| 2 | Fake claude fails on 2nd add | 1st registered, 2nd not; re-run adds only the 2nd | automated |
| 3 | Unpick personal from work, sync | remove personal-gmail from work only | automated |

### Verification Command (exact, runnable)

```bash
uv run pytest tests/connectors tests/core tests/surfaces tests/leak -q && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-65.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-65.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

N/A — no UI in this task (backend/QA only). All three UI Evidence rows are ☐ N/A.

---

## Approach

**Pattern reference**: `connectors/writer.py` add/remove/\_claude; HUN-52 API plan/confirm (`lines` = plan)

**Vital slice**: add connector in workspace + pick + sync for two accounts
**Cut list**: parallel apply across accounts; auto-sync without confirmation

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Refactor `writer._claude(*args, env=None)` to pass an env dict (copy of os.environ with `CLAUDE_CONFIG_DIR` set or removed). Keep `add_json(id, launcher)` → `{"command": launcher, "args": ["exec", <id>]}`; the Claude name is the alias.
* New `connectors/sync.py`: `plan(conn, account) -> list[Step]`, `apply(conn, steps, out) -> int`. Each step updates `account_registrations` right after its `claude` call succeeds (one small transaction per step).
* Drift check: for each registered alias, `claude mcp get <alias>` must succeed and point at the launcher; report drift lines in `doctor` and in the plan (re-add), never remove unknown entries.
* `connector add` keeps today's refusal if the keychain item exists. `connector remove <id>` = plan removal from all accounts + keychain delete (confirmation).

---

## Edge Case Checklist

- [ ] Alias already used by a hand-made MCP entry in an account (claude mcp get succeeds but not in registrations) → step refused with a message; nothing overwritten
- [ ] Account folder missing → its plan is skipped with a note
- [ ] `claude` CLI missing → print the snippet per step (existing fallback), registrations unchanged
- [ ] Two workspaces picked by one account both have Slack → aliases differ (`company-slack`, `side-slack`), both added
- [ ] Alias > 64 chars or not matching `launch._NAME` → refused at creation
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/connectors/writer.py` | env-aware `_claude`, add/remove by alias + id |
| `src/easy_personal/connectors/sync.py` (new) | plan/apply |
| `src/easy_personal/core/migrations/` | connectors, picks, registrations |
| `src/easy_personal/surfaces/cli.py`, `api.py` | commands + routes |
| `tests/connectors/` | fake claude records env per call |
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

Plan/apply unit tests on fake claude (env assertions), failure-resume test, constraint tests via direct SQL, API guard tests, leak suite.

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