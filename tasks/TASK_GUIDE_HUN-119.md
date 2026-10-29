**Date**: 2026-10-29
**Complexity Level**: C2
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

Owner issue 2026-10-29 #6: "for the kit tab, the table is technically, I can not get any information from that".

Today (`web/src/KitView.jsx`): one table, a row per project, columns Kernel / Verifier / Context / Git with raw text like "13,976 tool calls, 0 errors (0.0%); 174 kit tasks", "no data", "87,662 commits" (all-time, back to 2013). Nothing says what a part means, whether it is good or bad, or what is recent.

**Restated intent**:

> Kit becomes a list of project cards, sorted by most recent activity. Each card has: the project name; one health chip — "Healthy", "Needs attention" (tool error rate > 10 %, or context index stale) or "No kit data"; a plain summary sentence, e.g. "Last active today. 13,976 agent tool calls, none failed. 39 commits today, 342 in the last 30 days."; a 30-day commits sparkline; and one short line per part that has data, each with a plain name and meaning: "Agent kit (tool calls the kit traced)", "Code index (easy-context) — indexed 2 days ago, fresh", "Verifier score — 82/100". Parts with "no data" are not listed one by one; one muted line says which tools are not set up. A short intro line says what this page is for. A "Show details" toggle keeps today's table for the technical view.

**Out of scope**: backend changes; new kit sources.

**Requirement Refs**: owner issue #6; FR for kit panel (HUN-51) data unchanged.

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's issue (Supervisor, 2026-10-29)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: the global selector task (both edit `KitView.jsx`). **Entry point**: dashboard → Kit.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | One card per project, sorted by newest of kernel `last_utc` / latest git day, desc; projects with no data at all go last | readable list |
| 2 | Health chip rules: "Needs attention" if kernel `error_rate` > 0.1 or context `stale` true; "Healthy" if any part has data and none needs attention; else "No kit data". The chip names the reason when attention is needed | good or bad |
| 3 | Summary sentence uses last-active (relative), tool calls + failures, commits today and last 30 days (from `commits_by_day`, local day) | recent info |
| 4 | 30-day commits sparkline via `Sparkline` with `aria-label` naming the total and peak | trend |
| 5 | Each part with data has a plain label + one-line meaning; "no data" parts collapse into one line "Not set up here: Code index, Verifier" | no jargon |
| 6 | "Show details" toggle reveals today's table unchanged | technical view kept |
| 7 | Page intro: "How the agent kit, code index and verifier are doing in each project." | purpose |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | kernel error_rate 0.2 | chip "Needs attention: 20% of tool calls failed" | Vitest |
| 2 | commits_by_day 10 today, 5 twenty days ago, 100 a year ago | "10 commits today, 15 in the last 30 days" | Vitest (fake timers) |
| 3 | context + verifier no data | "Not set up here: Code index, Verifier" | Vitest |
| 4 | toggle details | table visible | Vitest |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-119.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-119.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Kit with 3 fixture projects (healthy, attention, no data) | headless Chrome + LLM vision | a non-technical reader can say which project needs attention and why |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | grep | none outside `tokens.css` |
| Contrast | computed | ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 px | screenshot | 1 card per row |
| 1280 px | screenshot | 2–3 cards per row |

---

## Approach

**Pattern reference**: `web/src/Widgets.jsx` (`Sparkline`); card style from HUN-83.
**Vital slice**: `ProjectCard` + pure helper `projectHealth(project, now)` tested alone.
**Cut list**: per-project drill-down page; charts beyond the sparkline.

---

## Edge Case Checklist

- [ ] `missing_on_disk` → "(folder missing)" on the card
- [ ] `error_rate` null → no failure text
- [ ] unknown extra part (e.g. `ui`) → listed with its key as label, generic text (today's `Generic`)
- [ ] 0 projects → today's empty message

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/KitView.jsx` + `.test.jsx` | cards + details toggle |
| `web/src/app.css` | card grid |
| `web/src/test/fixtures.js` | kit fixtures |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` (except rebuilt static) | no API change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

Vitest for AC 1–7 and helpers; screenshots at 375/768/1280.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review