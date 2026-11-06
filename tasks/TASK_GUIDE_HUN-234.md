**Date**: 2026-11-06
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Live tail, Attention state; Critical Constraints: metadata only and its 2026-11-06 Live exception)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` FR-072, FR-081..FR-084, NFR-015; `docs/ddr/0004-live-call-labels-show-never-store.md`; `tasks/TASK_GUIDE_HUN-233.md` for the exact `label` / `agent` / `stage` / `stage_skill` shapes; then `web/src/LiveView.jsx`, `web/src/LiveView.test.jsx`, the `.live-*` rules in `web/src/app.css`, `web/src/tokens.css`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-06: "from the tools logs for the live, it just show the simple, don't know what is agent doing … call which tools, which steps of the kits agent are in will be better". Owner choice: **row chip + panel feed**.

**Restated intent**:

> Each session row shows a stage chip "Stage <stage> · <stage_skill>" from the snapshot, and no chip while `stage` is null. In the session panel feed, each call row shows who made it ("main", or the event's `agent` sub-agent type), the tool name, and its `label` beside it; MCP labels as "server · tool". Events without a label look as today. The page only renders what the tail sends; it works out nothing. Update the metadata-only footnote to say tool names, skill / sub-agent / MCP / command names, counts and times are shown, never message or tool content.

Wire shapes (HUN-233, merged 2026-11-06; `surfaces/api.py` `_EVENT_KEYS` / `_SESSION_KEYS` already carry them — no server change): a `live` event has `label` = a string (`"tdd"`, `"uv"`, `"qa-expert"`), an object `{"server": "easy-verifier", "tool": "score"}` for MCP, or `null`; and `agent` = `null` (main thread) or a sub-agent type string (`"backend-developer"`, or `"sub-agent"` while the type is unknown). A snapshot session has `stage` (`"0.5"`…`"5"` or `null`) and `stage_skill` (bare skill name or `null`).

Owner, 2026-11-06 (after HUN-233 merged): "seem like it can not show the subagent also" — the page ignores `agent` today and only shows " agent" for `is_sidechain`.

**Out of scope**:

* Any server change (HUN-233)
* Stage history, filters, colours per stage beyond one chip style
* Other views

**Requirement Refs**: FR-084 (FR-072, NFR-015 kept)

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (owner choices recorded above)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: HUN-233 (backend, FR-081..FR-083) (event `label`, `agent`; snapshot `stage`, `stage_skill`)

**Entry point**: `LiveView` default export in `web/src/LiveView.jsx`, reached from the `Live` tab in `web/src/App.jsx`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Snapshot session with `stage "4"`, `stage_skill "code-review"` → its row shows "Stage 4 · code-review"; `stage null` → no chip | FR-084 |
| 2 | A new snapshot (state change) with a new stage updates the chip; ordinary `live` events do not re-render untouched rows (existing memo test still passes; `stage` added to the row's compare keys) | FR-084, NFR-015 |
| 3 | Panel feed: event `{tool: "Skill", label: "tdd", agent: null}` → "main · Skill · tdd"; `{tool: "Bash", label: "pytest", agent: "backend-developer"}` → shows "backend-developer" and "pytest"; MCP `{server, tool}` label → "easy-verifier · score"; no label → as today | FR-084 |
| 4 | Text from any other event field never appears (fixture with an extra planted field) | FR-084, FR-045 |
| 5 | Footnote text updated; existing LiveView tests still pass (adapted, not deleted) | FR-084 |
| 6 | Real run with HUN-233 merged: `easy-personal dashboard` with a session in a kit stage; screenshots of rows (chip) and the open panel at 1280×720 and 375×812 | FR-084 |

---

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Live rows with a stage chip, 1280×720 | headless screenshot BEFORE/AFTER | chip beside the id/badges, row height unchanged or +1 line at most |
| Open panel feed with labels, 1280×720 and 375×812 | headless screenshot | agent · tool · label readable; long labels wrap, no horizontal scroll |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Tokens | `web/src/tokens.css` variables only | chip uses existing tag/label tokens; no hard-coded colours |
| Accessibility | vitest | chip text is real text (not only colour); feed stays one `aria-live` region |

### 3. Responsiveness

| Width | Expected |
| -- | -- |
| ≥ 768 px | chip inline in the row header |
| 375 px | chip wraps under the project name; no page-wide horizontal scroll |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run src/LiveView.test.jsx && npx vitest run && npm run build
```

## Approach

**Vital slice**: `fromSnapshot` keeps `stage`/`stage_skill` (add to `SAME_KEYS`); `feedItems` keeps `label`/`agent`; `SessionRow` renders the chip; `Feed` renders agent + label; one `.live-stage` CSS rule; footnote text.
**Cut list**: no new component file; no per-stage colours.

---

## Edge Case Checklist

- [ ] `agent` null but `is_sidechain` true (a sidechain line in a main transcript file, HUN-233 code-review P3) → show "sub-agent", not "main"
- [ ] Label object vs string handled; unexpected shape → no label shown
- [ ] Session row from a `live` event before any snapshot → no chip
- [ ] Server restart snapshot clears feed as today
- [ ] Very long sub-agent type / label → wraps (`overflow-wrap: anywhere`)

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/LiveView.jsx` | chip, feed labels, footnote |
| `web/src/LiveView.test.jsx` | AC 1–5 |
| `web/src/app.css` | `.live-stage` |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/` | Server is HUN-233 |
| Other views in `web/src/` | Out of scope |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Extend `LiveView.test.jsx` (FakeEventSource): chip from snapshot, null stage, chip update on new snapshot, memo keys, feed rows with label/agent/MCP shapes, planted extra field not shown, footnote. Screenshots at two widths from a real run after the HUN-233 merge.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] All three UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] Real run done and screenshots attached
- [ ] Supervisor notified: task ready for Stage 4 review