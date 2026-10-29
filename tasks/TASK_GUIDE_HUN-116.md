**Date**: 2026-10-29
**Complexity Level**: C1
**Risk Level**: Low
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

Owner issue 2026-10-29 #4: "for the agent tab by account, should show each account like the current status line". Owner decision: disk data + plan limits. The data comes from `/api/accounts` `status_line` (task "Account status-line data in /api/accounts").

**Restated intent**:

> On the Agents page, each account row becomes a status card that reads like the owner's status line: name + status chip; a line with 👤 email · plan · organization; a line with last model and "last active 2 h ago"; tokens today and last 7 days; and, when a limits snapshot exists, two bars "5h N% · resets in 1h 20m" and "7d N% · resets in 3d 4h" coloured green < 70 %, amber 70–89 %, red ≥ 90 % (same thresholds as the status line). A stale snapshot says "as of <time>, reset since". No snapshot → a muted hint "Plan limits: add the status-line snippet (README)". The account actions (confirm / rename / ignore) and the workspace picker stay below.

**Out of scope**: backend changes; live refresh; the Usage page.

**Requirement Refs**: owner issue #4.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's issue and decision (Supervisor, 2026-10-29)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: the status-line data task (API fields) and the re-choose accounts task (both edit `AgentsView.jsx`; branch from main after both merge). **Entry point**: dashboard → Agents.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | Card shows email, plan, organization when present; omits the line when all three are null | status line: email |
| 2 | Card shows last model and relative last-active time ("never" when null) | status line: model |
| 3 | Card shows tokens today and last 7 days with `fmt()` | usage per account |
| 4 | Limits bars use `Progress` from `Widgets.jsx` with tone by threshold (<70 green, 70–89 amber, ≥90 red) and text "5h 42% · resets in 1h 20m"; each bar has an `aria-label` with its numbers | plan limits |
| 5 | `stale: true` → text "as of <local time>, reset since"; `limits: null` → muted README hint | snapshot states |
| 6 | Works in By account and By provider | both groupings |
| 7 | Existing Agents tests pass | no regression |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | fixture 5h 92 % | red bar, "92%" | Vitest |
| 2 | fixture 7d 75 % | amber bar | Vitest |
| 3 | `limits: null` | README hint | Vitest |
| 4 | `last_active_utc` 2 h ago (fake timers) | "last active 2 h ago" | Vitest |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-116.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-116.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Agents with 3 accounts (limits ok / high / none) | headless Chrome + LLM vision review | reads like the status line: one glance gives who, what model, how much used |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | grep hex/rgb outside `tokens.css` | none |
| Contrast | computed | ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 px | screenshot | one card per row, bars full width |
| 1280 px | screenshot | cards in a grid, 2–3 per row |

---

## Approach

**Pattern reference**: `web/src/Widgets.jsx` `Progress`; `web/src/format.js`.
**Vital slice**: a `StatusCard` component inside `AgentsView.jsx`.
**Cut list**: context % bar (not stored); icons beyond the 👤 marker.

---

## Edge Case Checklist

- [ ] `used_percentage` null in one window → that bar hidden
- [ ] reset already passed but `stale` false (clock skew) → "resets soon"
- [ ] very long email → wraps, no overflow

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/AgentsView.jsx` + `.test.jsx` | status card |
| `web/src/format.js` | relative time helper if missing |
| `web/src/app.css` | card grid |
| `web/src/test/fixtures.js` | `status_line` fixtures |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` (except rebuilt static) | backend done in the data task |
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