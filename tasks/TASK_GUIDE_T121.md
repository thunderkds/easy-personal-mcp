# TASK_GUIDE — T121: UX audit and fix pass, whole app
**Date**: 2026-11-19
**Complexity Level**: C3
**Risk Level**: Medium (page only; app-wide visual and live-update blast radius)
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `agents/frontend.md`, `skills/ux-evaluate/SKILL.md`
2. `web/src/tokens.css`, `web/src/app.css`, `web/src/appsession.css`, `web/src/App.jsx`
3. `tasks/TASK_REVIEW_T119.md` and `tasks/TASK_REVIEW_T120.md` (their ux-evaluate notes; the T120 shoot scripts and fixture mock are in the Supervisor scratchpad `ux-t120/` and may be reused)
- Tests and screenshots use fixture data and a mock or temp-store server only; never the real `~/.claude*` or `~/.easy-personal`. Kill only processes you started. Port 8765 is taken by an unrelated process.

## Requirement
Owner 2026-11-19, after installing the T119 + T120 package: "the ux/ui of the application is not good, let check them all again, the size is small, the component is drop when change status, and lot of small bugs, let verify and fix".
Supervisor defaults (told to the owner): body text goes to **16 px** (ux-evaluate's B-e bar); "drop when change status" = an element that disappears, unmounts, jumps, loses focus/scroll or closes when a session or item changes state (working → idle → needs-you → ended), hunted on every view that updates live.

## Dependencies & Reachability
**Depends on**: T119, T120 (merged)
**Entry point**: Standalone — N/A (every view)

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | **Audit first**: before any fix, `TASK_REVIEW_T121.md` holds a findings table covering every view (Usage, Live list + panel + Past, App sessions / New session, Watch, Workspaces, Board, Jobs, Agents, Kit, Workflow, Memory, Warnings/Plan, Settings, rail, toasts, dialogs) at 375 and 1280: id, view, width, what is wrong, evidence (screenshot path or measured value), severity (P1 broken / P2 clearly wrong / P3 polish) | owner |
| 2 | **Size**: `--fs-body` 16 px, the other `--fs-*` tokens rescaled from the T120 ratios keeping order; nothing smaller than 12 px anywhere (labels included); ux-evaluate B-e passes at 375 | owner |
| 3 | **No drop on status change**: for every live-updating view, a test drives state changes through the mocked stream/poll (working → idle → needs-you → ended, item added/removed, list re-sorted) and asserts the open panel / card / focused control / scroll position / typed text stay put (same DOM node where it matters) and no row jumps under the pointer; every drop found in the audit is fixed and covered | owner |
| 4 | Every P1 and P2 finding fixed (or written up in TASK_REVIEW with a reason it needs the owner); P3s fixed when small and surgical | owner |
| 5 | ux-evaluate re-run after the fixes on every view at 375 and 1280: B-a..B-i pass or have a written reason; no horizontal page scroll; touch targets ≥ 44 px at 375 | ux |
| 6 | `cd web && npx vitest run && npm run build`; `python3 scripts/check_static.py` ok | all |

## Approach
Options weighed by the Supervisor (Medium risk, compact brainstorm):

| Option | Invasiveness | Regression risk | Pick |
|---|---|---|---|
| A. Audit all views with real screenshots + live-change tests, then fix finding by finding, tokens first | Medium (many small CSS/JSX edits) | each fix is local and test-backed | **Yes** |
| B. Redesign the layout system | High | breaks everything at once; not asked | No |
| C. Fix only the size and the one drop | Low | leaves the "lot of small bugs" | No |

Fix order: tokens (size) → drops on status change (keys, conditional unmounts, re-sorts, focus) → layout bugs → polish. Keep component behaviour; match existing style; tokens only.

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/tokens.css` | size scale |
| `web/src/app.css`, `web/src/appsession.css` | layout fixes |
| `web/src/*View.jsx`, `SessionPanel.jsx`, `LiveFilters.jsx`, `PastSessions.jsx`, cards | stable keys, no unmount on state change, focus |
| `web/src/*.test.jsx` | live-change tests |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/**`, `src/easy_personal/**` (except the build output) | server code — if a bug needs the server, write it up |

## Test plan
Audit screenshots + measured metrics; vitest live-change tests; ux-evaluate before/after.

## Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Audit table written (before fixes)
- [ ] Implementation done
- [ ] Tests pass
- [ ] ux-evaluate after-run
- [ ] Smoke test confirmed (owner: reinstall the .deb, use Live while sessions change state)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
