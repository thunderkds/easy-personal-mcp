**Date**: 2026-10-26
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *Connector id / alias*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely, and `BRAINSTORMING_LOG_WORKSPACES.md` (verified facts W1–W10, edge cases)
4. Read `agents/frontend.md`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. **C2/C3 or multi-file tasks only**: read `memory/codebase-map.md`

---

## Requirement (Pillar 1 — Adapt the requirement)

Workspaces created from an Add button, edited (incl. rename) and deleted (FR-028); connectors per workspace with alias, move, delete, orphan assign (FR-029–FR-031); legacy adoption from the page (FR-034); [claude.ai](<http://claude.ai>) link first (FR-020); secrets never echoed (NFR-002). Approved sketch in `BRAINSTORMING_LOG_WORKSPACES.md`.

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> Replace the Connectors view with a Workspaces view: cards (color, email, repo count, connectors by alias, used-by accounts, orphan card with Assign…), + Add workspace, workspace form (fields, repos, connectors: add from catalog form, alias, Move to…, Delete), every write shows the plan then Confirm; banner offering legacy adoption when the API reports legacy connectors.

**Out of scope** (what this task explicitly does NOT do):

* Agents page (HUN-68)
* Usage filters (HUN-70)

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-028
* FR-029
* FR-030
* FR-031
* FR-034
* FR-020
* NFR-002
* NFR-008
* US-014

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (Supervisor, from the owner-approved breakdown 2026-10-26)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary (requirement grill 2026-10-26)
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

> An agent must NOT start implementing until this gate is checked. If anything here is unclear,
> STOP and ask the Supervisor (Karpathy: Think Before Coding).

---

## Dependencies & Reachability

**Depends on**: HUN-64 — Workspaces; HUN-66 — Move/alias/rename/delete; HUN-67 — Legacy adoption

**Entry point**: Dashboard nav → Workspaces (replaces Connectors)

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Cards per workspace + orphan card, matching the approved sketch | FR-028 |
| 2 | Add/edit form: name, color, email, description, repos (+/✕); rename allowed except orphan | FR-028 |
| 3 | Add connector: catalog-driven form (secret fields are password inputs, [claude.ai](<http://claude.ai>) link first), into this workspace | FR-029, FR-020 |
| 4 | Alias edit, Move to…, Delete per connector, each showing the plan then Confirm | FR-031, FR-030 |
| 5 | Delete workspace explains its connectors go to orphan, then plan + Confirm | FR-030 |
| 6 | Legacy banner appears when the API lists legacy connectors; Adopt shows plan + Confirm | FR-034 |
| 7 | Typed secret values are cleared after any final answer and never rendered back | NFR-002 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fixture API with 2 workspaces + orphan(2) | cards as sketch | Vitest + easy-ui |
| 2 | Move company-slack to side | plan shown; confirm call sent | Vitest |
| 3 | Planted secret in fixture form | not in DOM after submit | Vitest |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest tests/surfaces tests/leak -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-69.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-69.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

> UI evidence is captured with the easy-ui container during development (PRD Open Question 9): QA drives the running dashboard (fixture data) with easy-ui and attaches the session JSON + screenshots.

### 1. Visual Regression

| Screen / Component | Verification method | Expected result |
| -- | -- | -- |
| Workspaces page cards incl. orphan | easy-ui screenshot + LLM vision review against this guide's AC and the approved sketch in `BRAINSTORMING_LOG_WORKSPACES.md` | matches AC; no broken layout |
| Workspace form (add/edit) | easy-ui screenshot + LLM vision review against this guide's AC and the approved sketch in `BRAINSTORMING_LOG_WORKSPACES.md` | matches AC; no broken layout |
| Connector move/alias/delete with plan confirmation | easy-ui screenshot + LLM vision review against this guide's AC and the approved sketch in `BRAINSTORMING_LOG_WORKSPACES.md` | matches AC; no broken layout |
| Legacy connectors banner | easy-ui screenshot + LLM vision review against this guide's AC and the approved sketch in `BRAINSTORMING_LOG_WORKSPACES.md` | matches AC; no broken layout |

### 2. Design-System Compliance

| Criterion | Verification method | Expected result |
| -- | -- | -- |
| Colors match design tokens | CSS audit of `web/src/tokens.css` usage | all colors come from the tokens file |
| Typography matches spec | computed style via easy-ui | one font family, sizes from tokens |
| Spacing / layout matches spec | easy-ui screenshot + LLM vision | consistent spacing scale |

### 3. Layout / Responsiveness

| Viewport | Verification method | Expected result |
| -- | -- | -- |
| Mobile (320–480px) | easy-ui screenshot | single column, no horizontal scroll |
| Tablet (768px) | easy-ui screenshot | readable, no overlap |
| Desktop (1024px+) | easy-ui screenshot | multi-panel layout |

---

## Approach

**Pattern reference**: `web/src/ConnectorsView.jsx` (Provider form, plan → confirm)

**Vital slice**: cards + form + connector add/move with plan
**Cut list**: workspace icon picker beyond color + short icon text

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Move the provider form logic from `ConnectorsView.jsx` into the workspace form; delete `ConnectorsView.jsx` and its test once replaced (tests move to the new view).

---

## Edge Case Checklist

- [ ] Workspace name invalid → inline error from API message
- [ ] Move refused (provider exists) → API message shown, nothing changed
- [ ] Empty state: no workspaces → only orphan + Add button
- [ ] Long alias / long repo paths wrap without horizontal scroll
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/WorkspacesView.jsx` (new) + test | view |
| `web/src/ConnectorsView.jsx` + test | removed/merged |
| `web/src/App.jsx` | nav |
| `web/src/test/fixtures.js` | fixtures |

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

Vitest component tests; secret-not-rendered test; easy-ui session at 3 viewports.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Security review: `Skill({ skill: "security-review" })` run (Medium/High risk)
- [ ] Lint passes (`uv run ruff check .` + `npm --prefix web run lint` if configured)
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md`'s Evidence table (Hard-Stop Gate 5)
- [ ] `Skill({ skill: "verify" })` run — feature confirmed working in running app (fixtures only)
- [ ] `memory/MEMORY.md` updated (if new patterns or feedback learned)
- [ ] Supervisor notified: task ready for Stage 4 review