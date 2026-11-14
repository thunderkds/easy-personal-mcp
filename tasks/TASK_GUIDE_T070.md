# TASK_GUIDE — T070: Per-screen polish against the Graphite Mint artboards (Usage, Warnings & plan, Kit, Workspaces, Agents)
**Date**: 2026-11-14
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/frontend.md`
5. Read `rust/docs/UI_REDESIGN_PLAN.md` (rows G3, G4) and `docs/design/graphite-mint/README.md`
6. Read the artboards `Main`, `Warnings`, `Kit`, `Workspaces`, `Agents` in `docs/design/graphite-mint/`
7. Read `tasks/TASK_GUIDE_HUN-126.md` (the restyle that already landed) and the matching `web/src/*View.jsx`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder

---

## Requirement

**Original request** (owner, 2026-11-14): UI update from the Graphite Mint prototype; plan accepted ("yes, just following the guide").

**Restated intent**:
> First take a 1440 px screenshot of each of the five views on fixture data and list every visible difference from its artboard in a table in `tasks/TASK_REVIEW_T070.md` (view, difference, fix / keep, why). Then close each difference that needs **only markup or CSS** and data the page already receives. Known candidates:
> - Workspaces: banner for connectors set up before workspaces, with **Adopt** and the "This will change:" preview (only if the existing plan/apply calls give the preview; else list it as "needs server").
> - Agents: `missing` account card ("The folder is gone. Its stored usage stays in reports.").
> - Kit: note "\"no data\" means the kit part left nothing to read for that project. It is not zero."
> - Warnings: "Not measured is not the same as passed."
> - Problem red: artboards use `#E8968F` for problem states, `tokens.css` has `--color-red: #ef7b7b`. Change the token value only if every artboard red is `#E8968F` and contrast on `--color-bg` stays ≥ 4.5:1; record the check.
>
> A difference that needs a new or changed API is **not built**: list it as "needs server" for the owner.

**Out of scope**: top bar (T068), First run (T069), Live/Board/Scheduled/Memory views, any API change, behaviour changes.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (plan accepted 2026-11-14)
- [x] Every Acceptance Criterion traces to the Requirement
- [ ] Requirement Refs: dashboard FRs in `PRD.md` (cite those touched)

---

## Dependencies & Reachability
**Depends on**: T068 merged (shared top bar; screenshots must show the final shell)
**Entry point**: `web/src/UsageView.jsx`, `WarningsPlanView.jsx`, `KitView.jsx`, `WorkspacesView.jsx`, `AgentsView.jsx`

## Dependency rule
No new npm package.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `tasks/TASK_REVIEW_T070.md` has the difference table for all five views, each row fix / keep / needs server with a reason | intent |
| 2 | Each fixed row that adds text or an element has a Vitest assertion | fixes |
| 3 | Red token decision recorded with the contrast numbers | red |
| 4 | Existing tests and `npm run build` pass | all |
| 5 | UI evidence rows filled with BEFORE/AFTER screenshots of each view | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd web && npm test 2>&1 | tail -5 && npm run build 2>&1 | tail -3
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Web suite and build pass | ☐ | |
| code-review | ☐ | |
| UI: visual regression | ☐ | |
| UI: design-system compliance | ☐ | |
| UI: responsiveness | ☐ | |

## Demonstration
BEFORE/AFTER screenshot pair per view at 1440 px.

## UI / Design Acceptance Criteria

### 1. Visual Regression
| Screen / Component | Verification method | Expected result |
|---|---|---|
| Usage, Warnings & plan, Kit, Workspaces, Agents at 1440 px | easy-ui MCP or headless `google-chrome --screenshot`, LLM vision vs each artboard | every "fix" row closed; "keep"/"needs server" rows listed |

### 2. Design-System Compliance
| Criterion | Verification method | Expected result |
|---|---|---|
| Colours | grep changed CSS for raw hex | tokens only |
| Typography / spacing | computed style on changed elements | as the artboard |

### 3. Layout / Responsiveness
| Viewport | Verification method | Expected result |
|---|---|---|
| 375 / 1024 / 1440 px | screenshot of each changed view | no horizontal scroll; rail rules as HUN-126 |

---

## Approach
Audit first, then one commit per view. Do not change working behaviour to match sample data.

## Edge Case Checklist
- Sample numbers in artboards are not targets; only layout, text and style are
- Empty states (no workspaces, no accounts) still render

## Files to Change (Predicted)
The five view files + tests, `web/src/app.css`, maybe `web/src/tokens.css` (red only), new `tasks/TASK_REVIEW_T070.md`.

## Files Must NOT Touch
`rust/**`, `src/easy_personal/**`, `App.jsx` top bar, `FirstRun.jsx`.

## Test Plan
Vitest for added elements; screenshots.

## Completion Checklist
- [ ] AC 1-5 with pasted output
- [ ] Commits on branch `T070` only; final report with `Status:`
