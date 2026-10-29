**Date**: 2026-10-29
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P0
**Assigned agent**: Backend-Implementer
**Agent guide**: `agents/backend.md`

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

Owner issue 2026-10-29 #1: "the workspace loading long time, it's not good".

Root cause (Supervisor, measured on the owner's machine 2026-10-29 with `TestClient`): `/api/connectors/legacy` takes **3.51 s**; every other call of the Workspaces page takes under 0.1 s. `legacy.detect()` runs `claude mcp get <id>` (\~0.7 s each) once per catalog id (5 ids), one after another. `WorkspacesView` waits for all 4 requests in one `Promise.all` before it renders anything.

**Restated intent**:

> The Workspaces page renders its workspaces and connectors as soon as the fast calls answer. The legacy (pre-workspace connector) banner loads on its own and appears when ready. The legacy scan runs its `claude mcp get` checks in parallel, so it takes about one call's time, not five.

**Out of scope**: caching `claude mcp get` across requests; changing what counts as legacy; any change to adoption writes.

**Requirement Refs**: owner issue #1; FR-034 (legacy adoption) behaviour unchanged.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's issue (Supervisor, 2026-10-29)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: none. **Entry point**: `easy-personal dashboard` → Workspaces.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | `WorkspacesView` renders workspaces and connectors while `/api/connectors/legacy` is still pending | page not blocked |
| 2 | When the legacy answer arrives, the legacy banner shows as today; a failed legacy call shows no banner and no page error (same as today's `optional`) | banner unchanged |
| 3 | `legacy.detect()` runs the per-id keychain + `claude mcp get` checks concurrently (thread pool, max 8 workers); result order stays catalog order | parallel scan |
| 4 | With a fake `claude` that sleeps 0.5 s per call and 5 catalog ids, `legacy.detect()` finishes in under 1.5 s | measurable speed |
| 5 | All existing legacy/adoption tests and Vitest tests pass | no regression |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | legacy fetch never resolves | workspace list visible | Vitest |
| 2 | legacy resolves with 1 item later | banner appears | Vitest |
| 3 | fake slow `claude` (0.5 s), 5 ids | `detect()` < 1.5 s, same items as sequential | pytest |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-113.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-113.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Workspaces (banner pending, then shown) | headless Chrome screenshot on fixtures | list shows first; banner later, same style as today |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| No new colors | grep for hex/rgb outside `tokens.css` | none |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 / 1280 px | screenshot | no layout shift that hides content when the banner appears |

---

## Approach

**Pattern reference**: `web/src/WorkspacesView.jsx` `optional()` loader; `src/easy_personal/connectors/legacy.py` `detect()`.
**Vital slice**: split the legacy request out of the page's `Promise.all`; `concurrent.futures.ThreadPoolExecutor` in `detect()`.
**Cut list**: server-side cache; a loading spinner on the banner.

---

## Edge Case Checklist

- [ ] `claude` not on PATH → no subprocess, same result as today
- [ ] keychain error (`LaunchError`) inside a worker still propagates → 503 as today
- [ ] legacy answer arrives after the user navigated away → no React state update on an unmounted view

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `src/easy_personal/connectors/legacy.py` | parallel checks |
| `tests/` (legacy tests) | timing test with fake slow `claude` |
| `web/src/WorkspacesView.jsx` + `.test.jsx` | load legacy separately |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/connectors/writer.py` write paths | read path only |
| real `~/.claude*/`, `~/.easy-personal/` | fixtures only |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

pytest timing test (fake `claude` script on PATH); Vitest pending-promise test; screenshot.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review