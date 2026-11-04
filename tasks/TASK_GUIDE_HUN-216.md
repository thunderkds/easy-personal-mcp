**Date**: 2026-11-04
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Rule, Abstained; Critical Constraints: metadata only)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-006, FR-007, FR-073, FR-074, FR-075; `tasks/TASK_GUIDE_HUN-214.md` (the `causes` / `diagnosis` JSON shape); `web/src/WarningsPlanView.jsx` and its test; `web/src/tokens.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-04: "for the warning and plan, can not get the idea here" and "no solution related, so it make the users confuse too much". The page today shows warnings and the plan as two lists of the same items, with generic fix lines.

**Restated intent**:

> `WarningsPlanView` shows one list: the warning hits ordered by plan priority (1 first), each once. Each item: title, label, summary, a "Why this happened" block listing its `causes` (name + numbers + text; a `new` project marked; `cache_read_share` shown when present), and the fix `steps` chosen by its `diagnosis`. Abstained rules are listed after, with their reason. At the end: "Ask Claude to investigate: run `/mcp__easy-personal__investigate` in Claude Code." with a copy button for the command. Workspace/account scope works as today. With no hits: "Nothing to fix this week." plus the abstained list and the hint.

**Out of scope**:

* Any server change (HUN-214 provides the fields; HUN-215 provides the prompt)
* Links into the Usage page; charts
* CLI/MCP output

**Requirement Refs**: FR-074

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner "yah, should", 2026-11-04)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-214

**Entry point**: `WarningsPlanView` in `web/src/WarningsPlanView.jsx`, reached from the `Warnings & plan` nav item in `web/src/App.jsx`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | With 2 hits (priorities 3 and 1) the page shows 2 items, priority 1 first, and no item twice | FR-074 |
| 2 | Each item shows title, label, summary, a "Why this happened" block with every cause (name + numbers), a `new` mark where set, and the diagnosis steps | FR-074 |
| 3 | `cache_read_share` shows as "N% of these tokens are cache reads" only when present | FR-074 |
| 4 | Abstained rules list after the hits with their reason | FR-074 |
| 5 | No hits → "Nothing to fix this week." plus abstained list and hint | FR-074 |
| 6 | The hint shows `/mcp__easy-personal__investigate` and a copy button that writes exactly that text to the clipboard (mocked) with a toast (FR-065) | FR-074, FR-075 |
| 7 | Workspace/account scope still passed to the request (existing tests kept) | FR-074 |
| 8 | Real run on a store copy: screenshot at 1280×720 and 375×812 with the owner's `tokens_jump` hit | FR-074 |

---

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Warnings & plan, 1 hit + abstained, 1280×720 | headless screenshot BEFORE/AFTER | one list; causes block; hint at end |
| Same, 375×812 | headless screenshot | readable; no page-wide horizontal scroll |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | no hard-coded colours |
| Accessibility | vitest | items are a list; causes a nested list; copy button labelled |

### 3. Responsiveness

| Width | Expected |
| -- | -- |
| 1280 px | items full width of the main column |
| 375 px | numbers wrap, no horizontal scroll |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run src/WarningsPlanView.test.jsx && npx vitest run && npm run build
```

## Approach

**Vital slice**: rewrite `WarningsPlanView.jsx` around one list from `/api/plan` (or `/api/warnings` sorted by `priority`, whichever already carries the fields after HUN-214 — read its merged JSON); reuse the existing toast host for the copy button.
**Cut list**: no new route; no Usage links; no collapse/expand.

Start only after HUN-214 is merged into `main`; merge `main` into your branch first.

---

## Edge Case Checklist

- [ ] A hit with no causes → no "Why this happened" block, steps still shown
- [ ] Clipboard API missing → button shows the text selected / a fallback toast, no crash
- [ ] Load error → existing error state kept
- [ ] Planted secret/text fixtures never appear on the page

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/WarningsPlanView.jsx` | one list, causes, steps, hint |
| `web/src/WarningsPlanView.test.jsx` | AC 1–7 |
| `web/src/app.css` | rules for the new blocks |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | server is HUN-214 / HUN-215 |
| Other views in `web/src/` | out of scope |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Mock `/api/*` responses shaped like HUN-214's JSON; assert ordering, causes, steps, abstained, empty state, hint + copy; screenshots from a real run on a store copy.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] All three UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] Real run done and screenshots attached
- [ ] Supervisor notified: task ready for Stage 4 review