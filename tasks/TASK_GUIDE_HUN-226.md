**Date**: 2026-11-05
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` in full
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and **no** `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-080; `web/src/KitView.jsx` (`PARTS`, `PART_ORDER`, `projectHealth`, `Part`, `PartDetail` and every place that loops over parts), `web/src/KitView.test.jsx`, `web/src/SavedReports.jsx` (`Side`, `Now`)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-05: "I removed the context out of family"; chose "Hide when not in use". The server task (HUN-225, in parallel) sends each Kit project's `context` part as `{"status": "not in use"}` when easy-context had no call in the last 7 days, and a saved report rule's `now` as `{"state": "not_in_use", "reason": "not in use"}`.

**Restated intent**:

> On the Kit page, a part whose `status` is `"not in use"` is not shown at all: no Code index row, column, legend entry or detail line, and it never counts in `projectHealth` (no "code index is out of date" reason, and it is not "data" for the Healthy verdict). When every project's `context` part is `"not in use"`, the Code index column/legend disappears from the page. Other parts are unchanged. In Saved reports, a `now` with `state: "not_in_use"` shows its `reason` text ("not in use"), never a number or `0`; check the existing `Side` / `Now` code already does this and add a test.

**Out of scope**:

* Any server change (HUN-225)
* Other Kit parts, the Kit findings block

**Requirement Refs**: FR-080 (UI part)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner chose "Hide when not in use", 2026-11-05)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-225 for the real run only (fixtures carry the shape)

**Entry point**: dashboard sidebar → Kit; Warnings & plan → Saved reports

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Projects whose `context` is `{"status":"not in use"}` show no Code index part anywhere on the Kit page (row, detail, legend/column) | FR-080 |
| 2 | `projectHealth` ignores a "not in use" part: a project with only `context` "not in use" + no other data → "No kit data"; with healthy kernel → "Healthy" | FR-080 |
| 3 | A project with `context` in use (status ok, stale) still shows the part and the "code index is out of date" reason (existing tests green) | FR-080 |
| 4 | Saved report `now.state == "not_in_use"` → "not in use" text | FR-080 |
| 5 | Real run on a store copy after HUN-225 merges (or with its branch's server): Kit page screenshot 1280×720 with no Code index | FR-080 |

---

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Kit, 1280×720 | headless screenshot BEFORE/AFTER | no Code index part |
| Kit, 375×812 | headless screenshot | no horizontal scroll |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | code read | no new colours |

### 3. Responsiveness

| Width | Expected |
| -- | -- |
| 375 px | layout unchanged apart from the missing part |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run src/KitView.test.jsx src/WarningsPlanView.test.jsx && npx vitest run && npm run build
```

## Approach

**Vital slice**: one `inUse(part)` helper in `KitView.jsx` used wherever parts are listed or counted; tests.
**Cut list**: no server change, no new setting.

---

## Edge Case Checklist

- [ ] Mixed: some projects "not in use", others in use (scoped) → only those projects hide the part
- [ ] `context` key missing entirely → treated as today ("no data")
- [ ] Planted secret/text fixtures never appear on the page

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/KitView.jsx` | skip "not in use" parts |
| `web/src/KitView.test.jsx`, `web/src/WarningsPlanView.test.jsx` | AC 1–4 |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | server is HUN-225 |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Kit fixtures with "not in use" context parts (all / some projects); health verdicts; saved report `not_in_use`; screenshots on a store copy.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] Real run done and pasted (command + output)
- [ ] Supervisor notified: task ready for Stage 4 review