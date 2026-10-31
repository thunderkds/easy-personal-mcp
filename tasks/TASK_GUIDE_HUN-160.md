**Date**: 2026-10-31
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P0
**Assigned agent**: Frontend-Implementer (owns the view) — the API route is small enough to stay in the same task; Backend-Implementer only if the Supervisor splits the spawn
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: Work view terms, Critical Constraints incl. the 2026-10-31 named exception)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `memory/codebase-map.md`, `PRD.md` US-021, FR-042, and `BRAINSTORMING_LOG_WORKVIEW.md` (sketches, Edge Case Checklist, Surgical Scope)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-31: "wanna build the personal like that [https://github.com/blogminhquy/javis-os](<https://github.com/blogminhquy/javis-os>) ... core vision still go with ours, but the idea is good to following — the data showing, the UX, the board". Grill + brainstorm decisions: `memory/decisions.md` 2026-10-31 "Work view inspired by Javis OS, read-only"; `BRAINSTORMING_LOG_WORKVIEW.md` Option A (owner-selected; sketches there are the layout reference).

**Restated intent**:

> Regroup the dashboard rail into `// work` (empty slot for Board, Scheduled, Memory — filled by later tasks), `// usage` (Usage, Warnings & plan), `// kit` (Kit), `// connect` (Workspaces, Agents) and `// system` (First run). Add a First-run checklist view backed by `GET /api/first-run` that reports done / not done for: first ingest (any event stored), ingest timer installed (`easy-personal-ingest.timer` present under the systemd user unit dir), agent accounts confirmed (≥ 1 confirmed account), first workspace (≥ 1 non-`orphan` workspace). Each not-done step shows the existing CLI command or links to the existing view/button. The rail shows a `N left` badge on First run until all 4 are done.

**Out of scope**: new write actions (the checklist never installs, ingests or creates); the Work views themselves; renaming existing views.

**Requirement Refs**: PRD US-021, FR-042.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — Supervisor, 2026-10-31, owner approved PRD answers, Option A and this 5-task breakdown
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: None — can start immediately. **Entry point**: `GROUPS` in `web/src/App.jsx`; route `/api/first-run`.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Rail shows the 5 groups in this order with the listed views; `// work` renders even when it has no views yet (hidden header) so later tasks only append tabs | FR-042 |
| 2 | `GET /api/first-run` returns `{steps: [{id, done, hint}]}` for `first_ingest`, `ingest_timer`, `accounts_confirmed`, `first_workspace`; read-only (GET only), same token/Host rules as other routes | FR-042, NFR-008 |
| 3 | Not-done steps show the exact existing command (`easy-personal ingest`, `easy-personal schedule install`, …) or a button that switches to the Agents / Workspaces view; no new POST route | FR-042 |
| 4 | Badge `N left` on First run; hidden when N = 0 | US-021 |
| 5 | Existing view tests and `shell.test.js` still pass (tabs, scope selects, family footer) | no regression |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | fresh `EASY_PERSONAL_HOME` | all 4 steps not done, badge `4 left` | pytest + vitest |
| 2 | fixture with events, a confirmed account, workspace `acme`, timer unit file in temp `XDG_CONFIG_HOME` | all done, no badge | pytest |
| 3 | POST to `/api/first-run` | 405 | pytest |

### Verification Command (exact, runnable)

```bash
uv run pytest -q tests/surfaces && npm --prefix web ci && npm --prefix web test -- --run && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-160.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-160.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Rail + First run at 1280×720 | easy-ui MCP (`http://localhost:8765/mcp`) screenshot on fixture data vs the sketch in `BRAINSTORMING_LOG_WORKVIEW.md` | 5 groups, badge, checklist cards in the dashboard card style |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | `web/src/neon.test.jsx`-style test | only `var(--…)` from `web/src/tokens.css`, no raw literals |
| Fonts / labels | easy-ui computed style | Plex Sans body, Plex Mono labels, same card style as Kit/Usage views |
| Accessibility | vitest + easy-ui `ui_check` | landmarks, `aria-label` on SVG, keyboard focus visible, contrast ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 | easy-ui screenshot (fixed viewport, see learnings "easy-ui + Graphite Mint") | matches sketch |
| narrow (rail collapsed) | easy-ui screenshot | no horizontal page scroll; wide content scrolls inside its box |

---

## Approach

**Pattern reference**: existing `GROUPS` array in `web/src/App.jsx` and the `_query` route helper in `src/easy_personal/surfaces/api.py`. Put the step logic in a new `core/first_run.py` (reuse `schedule.unit_dir()`, `accounts.list_accounts`, `workspaces.list_all`) so MCP/CLI could reuse it later; no MCP tool now.
**Vital slice**: regrouped rail + 4-step checklist + badge.
**Cut list**: a setup wizard; step progress persistence; per-account checklist.

---

## Edge Case Checklist

- [ ] no systemd on the machine → `ingest_timer` shows the cron snippet hint instead, still not-done
- [ ] detected-but-unconfirmed accounts only → `accounts_confirmed` not done
- [ ] only the `orphan` workspace exists → `first_workspace` not done
- [ ] active tab id stored in URL fragment? (no — fragment holds the token): keep in-page state only

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/App.jsx` | new GROUPS, First run tab, badge |
| `web/src/FirstRun.jsx (new) + test` | checklist view |
| `src/easy_personal/core/first_run.py (new)` | step checks |
| `src/easy_personal/surfaces/api.py` | `/api/first-run` route |
| `tests/surfaces/…, web/src/App.test.jsx, web/src/shell.test.js` | tests |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/metrics.py` | usage split is another task |
| `src/easy_personal/surfaces/mcp.py` | no MCP change in this task |
| `src/easy_personal/connectors/**`, `src/easy_personal/launch.py` | connectors and keychain are out of scope |
| any repo's `PROJECT_KANBAN.md` / `memory/**`, `~/.claude*/**` | read-only sources (NFR-003) |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest for the route and each step with temp homes; vitest for rail order, badge, hints; easy-ui screenshots for the UI rows.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled or N/A with reason (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run
- [ ] Supervisor notified: task ready for Stage 4 review