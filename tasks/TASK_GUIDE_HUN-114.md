**Date**: 2026-10-29
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P0
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

Owner issues 2026-10-29:

* #5 "from the agent account, when user pick ignored, noway to rechoose and config again, the same with provider"
* #2 "for the usage, last 7days do not get any information"

Root cause (Supervisor, owner's store 2026-10-29): accounts 2,3,4,6,7 (`~/.claude-personal`, `~/.claude-wife`, `~/.claude-work`, `~/.claude-hung2`, a backup folder) are `ignored`; only `~/.claude` (no sessions after 2026-10-20) is confirmed. Ingest reads confirmed accounts only, so the newest event is 2026-10-20 and the last 7 days are empty. `accounts.confirm()` already accepts an ignored account, but `AgentsView` `AccountRow` shows the Name/Confirm form only when `status === 'detected'`, so there is no way back. The same `AccountRow` is used by "By provider".

Owner decision 2026-10-29: do not touch the owner's store now; the owner re-confirms in the UI after this ships.

**Restated intent**:

> On the Agents page (By account and By provider), an ignored account shows a "Use this account" action that opens the Name + Confirm form. A confirmed non-default account can be renamed and can be ignored again. The default account still cannot be ignored. On the Usage page, when ignored accounts exist, a note says how many accounts are ignored and links to the Agents page, so an empty period explains itself.

**Out of scope**: auto-confirming accounts; running ingest from the dashboard; the status card (separate task).

**Requirement Refs**: owner issues #2, #5; FR-036 (agent accounts).

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's issues (Supervisor, 2026-10-29)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: none. **Entry point**: `easy-personal dashboard` → Agents; Usage.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | An `ignored` account row shows a "Use this account" button; clicking it shows the Name field + "Confirm & name"; confirming calls `POST /api/accounts/{id}/confirm` and the row becomes confirmed | re-choose ignored |
| 2 | A confirmed account row shows "Rename" (same confirm call with the new label) and, unless it is the default account, "Ignore" (`POST /api/accounts/{id}/ignore`) | config again |
| 3 | AC 1–2 work the same in the "By provider" grouping | "same with provider" |
| 4 | `/api/accounts` marks the default account (`is_default: true`) so the UI hides its Ignore button; the API still refuses ignoring it (existing test) | default stays |
| 5 | After a confirm the row tells the owner that usage appears after the next ingest, with the exact command `easy-personal ingest` | explain gap |
| 6 | Usage page: when 1+ Claude accounts are `ignored`, a note reads "N agent accounts are ignored, so their usage is not counted." with a button that opens the Agents view | 7-day empty explained |
| 7 | Server error text from a refused action shows in a `role="alert"` on that row | errors visible |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | fixture account status `ignored` | "Use this account" → form → confirm request with label | Vitest |
| 2 | confirmed non-default | Rename + Ignore buttons; Ignore sends POST | Vitest |
| 3 | default account | no Ignore button | Vitest |
| 4 | by=provider | same buttons | Vitest |
| 5 | 2 ignored Claude accounts | Usage note "2 agent accounts are ignored…" | Vitest |
| 6 | ignored → confirm via API | status confirmed, label set | pytest |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-114.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-114.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Agents (ignored, confirmed, default rows) | headless Chrome on fixtures | actions visible and readable |
| Usage with ignored note | headless Chrome | note above the period buttons |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | grep hex/rgb outside `tokens.css` | none |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 / 768 / 1280 px | screenshot | buttons wrap, no horizontal scroll |

---

## Approach

**Pattern reference**: `web/src/AgentsView.jsx` `Detected`; `src/easy_personal/core/accounts.py` `confirm()` / `ignore()` / `_row()`.
**Vital slice**: reuse `Detected` as a name form for ignored and confirmed rows; `is_default` in `_row`.
**Cut list**: undo; bulk confirm; dashboard ingest button.

---

## Edge Case Checklist

- [ ] `missing` folder (status shown as missing) → no confirm form
- [ ] Codex account: confirm/ignore allowed, no workspace picker (as today)
- [ ] Rename to an empty label → button disabled
- [ ] Planted secret fixtures never appear (no new data source)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/AgentsView.jsx` + `.test.jsx` | actions per status |
| `web/src/UsageView.jsx` + `.test.jsx` | ignored note |
| `web/src/App.jsx` | let a view switch to Agents (callback prop) |
| `src/easy_personal/core/accounts.py` + tests | `is_default` |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| real `~/.claude*/`, `~/.easy-personal/` | owner decision: no store change now |
| ingest code (`sources/`) | out of scope |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |

---

## Test Plan

Vitest for AC 1–7; pytest for `is_default` and ignored→confirmed; screenshots.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review