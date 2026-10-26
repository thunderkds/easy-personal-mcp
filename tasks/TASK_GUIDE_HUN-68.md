**Date**: 2026-10-26
**Complexity Level**: C2
**Risk Level**: Medium
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

The Agents view groups accounts by provider or by account (FR-032); a confirmed Claude account picks workspaces with plan + confirmation (FR-033); Codex listed only. Approved sketch: `BRAINSTORMING_LOG_WORKSPACES.md` → Approved sketch.

**Restated intent** (Supervisor's interpretation, in the project's domain language):

> New Agents view: toggle by account / by provider; row per account (label, folder, status, workspaces multi-select for confirmed Claude accounts, 'listed only' for Codex); detected folders show Confirm & name / Ignore; changing picks shows the sync plan then Confirm.

**Out of scope** (what this task explicitly does NOT do):

* Workspaces page (HUN-69)
* Any backend change beyond wiring existing HUN-62/HUN-65 routes

**Requirement Refs** (FR/NFR/US IDs from `PRD.md` this task satisfies):

* FR-032
* FR-033
* US-015
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

**Depends on**: HUN-62 — Agent accounts; HUN-65 — Connector id + alias + sync

**Entry point**: Dashboard nav → Agents

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Toggle switches grouping between account and provider | FR-032 |
| 2 | Detected folder row offers Confirm & name (label input) and Ignore | FR-032 |
| 3 | Confirmed Claude row offers a workspaces multi-select excluding orphan; Codex row says 'listed only' with no picker | FR-033 |
| 4 | Saving picks shows the plan lines from the API and writes only after Confirm | FR-033 |
| 5 | Missing folder rows are marked 'missing' | FR-032 |
| 6 | All writes carry the dashboard token (existing `api.js`) | NFR-008 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_<this task ID>.md`, copied from
> `templates/TASK_REVIEW_template.md`. A task is **not done** until every row has evidence.
> Rule: the implementing agent must NOT be the sole author of its own acceptance test — the
> Supervisor writes or signs off on the oracle first, so code and test can't be wrong together.

### Success Criteria (observable, pass/fail)

| \# | Given (input/state) | Expect (output/behavior) | How it's checked |
| -- | -- | -- | -- |
| 1 | Fixture API: 3 Claude + 1 Codex + 1 detected | rows as in the sketch | Vitest + easy-ui session |
| 2 | Pick personal for Claude work | plan shown, then confirm call sent | Vitest |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest tests/surfaces -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-68.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-68.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

> UI evidence is captured with the easy-ui container during development (PRD Open Question 9): QA drives the running dashboard (fixture data) with easy-ui and attaches the session JSON + screenshots.

### 1. Visual Regression

| Screen / Component | Verification method | Expected result |
| -- | -- | -- |
| Agents page (by account) | easy-ui screenshot + LLM vision review against this guide's AC and the approved sketch in `BRAINSTORMING_LOG_WORKSPACES.md` | matches AC; no broken layout |
| Agents page (by provider) | easy-ui screenshot + LLM vision review against this guide's AC and the approved sketch in `BRAINSTORMING_LOG_WORKSPACES.md` | matches AC; no broken layout |
| Pick-workspaces plan confirmation | easy-ui screenshot + LLM vision review against this guide's AC and the approved sketch in `BRAINSTORMING_LOG_WORKSPACES.md` | matches AC; no broken layout |

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

**Pattern reference**: `web/src/ConnectorsView.jsx` plan → confirm flow

**Vital slice**: list + toggle + pick with plan
**Cut list**: drag-and-drop; account rename UI

Shared design for the workspace feature (Supervisor, 2026-10-26; source: `BRAINSTORMING_LOG_WORKSPACES.md`, Option A):

* **Desired state + reconcile.** The store holds desired state (workspaces, connectors, account→workspace picks). Claude configs are changed only by one `plan(account)` / `apply(plan)` pair that goes through `connectors/writer.py` (`claude mcp add-json|remove --scope user`). Every user action = one DB transaction + plan/apply for affected accounts; the plan lines are the confirmation text.
* **Default account** = folder `~/.claude`; its Claude config is `~/.claude.json`, so `claude` runs with `CLAUDE_CONFIG_DIR` **unset**. Every other Claude account runs `claude` with `CLAUDE_CONFIG_DIR=<folder>` (its config is `<folder>/.claude.json`). Brainstorm fact HUN-62.
* **Connector id / alias.** id is fixed: keychain service `easy-personal/<id>`, launcher `easy-personal-launch exec <id>` (launcher unchanged: it already treats its argument as the keychain key). Alias = the Claude Code name; default `<workspace>-<provider>`; `alias_custom` = 1 keeps an owner-typed alias. Legacy connectors: id = alias = old name. New ids: `c` + 8 lowercase hex chars. All ids/aliases match `launch._NAME`.
* **Store rules (NFR-009).** Schema changes only as numbered migrations on `PRAGMA user_version`, one transaction each, tested from the previous schema with data; `foreign_keys=ON` + `busy_timeout` on every connection; business rules as constraints (UNIQUE / partial unique indexes / CHECK / triggers) as well as code. No secret value in any table.
* **Tests** never touch real accounts: temp HOME / `EASY_PERSONAL_AGENT_ROOT`, fake keychain + fake `claude` (`tests/fakes`, `tests/connectors/conftest.py`).

Task design (Supervisor, 2026-10-26):

* Reuse `useLoad`, `getJson`, `sendJson`; tokens from `tokens.css`; add route/tab in `App.jsx`.

---

## Edge Case Checklist

- [ ] No workspaces yet → picker shows 'create a workspace first' link
- [ ] API error on confirm → error shown, picks reverted to server state
- [ ] Double click on Confirm
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log; every new CLI command / API route is registered in `tests/leak/harness.py` (`CLI_CALLS` / `API_CALLS`).

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/AgentsView.jsx` (new) + test | view |
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

Vitest component tests with fixtures; easy-ui session screenshots at 3 viewports.

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