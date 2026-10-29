**Date**: 2026-10-29
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *API-equivalent cost*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Verify on fixtures only: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_AGENT_ROOT`; never open the real `~/.easy-personal/` or `~/.claude*/`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner issue 2026-10-29 #3: "from the dashboard, don't see the way to choose the workspace and account". Today the two selects exist only on the Usage tab, small, portaled into the top-right of the top bar; on other tabs there is nothing. Owner decision: **one global selector** in the sidebar, kept across tabs; Usage, Warnings and Kit filter by it.

**Restated intent**:

> The sidebar shows a "Scope" block under the brand: a labelled Workspace select (All + non-orphan workspaces) and a labelled Account select (All + confirmed Claude accounts). `App` owns the selection, keeps it when the owner switches tabs, and remembers it in `localStorage` (try/catch; works without it). Usage, Warnings & plan and Kit receive `{workspace, account}` and send them to their API calls. Each of these views shows one line "Showing: <workspace> · <account>". Usage's own Filters component and the top-bar portal are removed. On mobile the block sits at the top of the opened sidebar menu and the top bar shows the current scope as text.

**Out of scope**: backend changes (done in the scope task); Workspaces/Agents views ignore the scope.

**Requirement Refs**: owner issue #3; FR-035, FR-036.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's decision (Supervisor, 2026-10-29)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: the warnings/plan/kit scope task (API args) and the re-choose accounts task (touches `App.jsx` / `UsageView.jsx`). **Entry point**: dashboard sidebar.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Sidebar has `<section aria-label="Scope">` with two labelled selects visible on every tab at ≥ 64rem | visible selector |
| 2 | Choosing a workspace/account on any tab, then switching tabs, keeps the choice | kept across tabs |
| 3 | Usage, Warnings & plan, Kit requests include `workspace=` / `account=` when set, and refetch on change | filters apply |
| 4 | Each of the 3 views shows "Showing: …" with the chosen names (All when empty) | clear scope |
| 5 | Selection persists in `localStorage`; a stored workspace/account that no longer exists falls back to All | remember |
| 6 | Below 64rem: the scope block is in the opened menu; the top bar shows the scope text | mobile |
| 7 | The old Usage `Filters` + portal are gone; existing Usage tests updated only for that move | no duplicate |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | pick workspace "personal" on Usage, click Kit | Kit request has `workspace=personal`; select still "personal" | Vitest (`App.test.jsx`) |
| 2 | stored account id 99 not in list | select shows All | Vitest |
| 3 | `localStorage` throws | page works, no error | Vitest |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-118.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-118.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Sidebar on Usage, Warnings, Kit | headless Chrome + LLM vision | the owner sees where to choose workspace and account at first glance |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | grep | none outside `tokens.css` |
| Contrast | computed | ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 px | screenshot (menu open + closed) | scope in menu; scope text in top bar |
| 1280 px | screenshot | scope block in sidebar |

---

## Approach

**Pattern reference**: `web/src/UsageView.jsx` `loadOptions()` + `Filters`; `web/src/App.jsx` shell.
**Vital slice**: move `loadOptions` + `Filters` to `App`, pass `scope` props.
**Cut list**: URL query state (the fragment holds the token); a scope on Workspaces/Agents.

---

## Edge Case Checklist

- [ ] options request fails → selects hidden, views load unscoped (as today)
- [ ] orphan workspace never listed
- [ ] scope change while a view is loading → stale answer ignored (existing `current` flag pattern)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/App.jsx` + `.test.jsx` | scope state + sidebar block |
| `web/src/UsageView.jsx` + `.test.jsx` | props instead of own filters |
| `web/src/WarningsPlanView.jsx`, `KitView.jsx` + tests | scope props in requests |
| `web/src/app.css` | sidebar block |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` (except rebuilt static) | backend in the scope task |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

Vitest for AC 1–7; screenshots at 375/768/1280.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review