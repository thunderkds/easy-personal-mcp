# TASK_GUIDE — T125: Live like Claude 2b — sub-agent logs in the panel
**Date**: 2026-11-19
**Complexity Level**: C2
**Risk Level**: Medium (page only; uses T124's guarded routes)
**Priority**: P1
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` question 46, US-044, FR-130, FR-129, FR-128, FR-123, NFR-002
2. `docs/ddr/0008-live-subagent-logs-and-steering.md`, `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVE2.md` (Edge Case Checklist: all of it applies)
3. `web/src/SessionPanel.jsx`, `Transcript.jsx` (`useTranscript`, `Tool`), `LiveView.jsx`, `PastSessions.jsx`, `api.js`, `web/src/test/fixtures.js`, `tokens.css`; T124's route shapes in `tasks/TASK_GUIDE_T124.md` (mock them)
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-19: "yes, I want from the live, I can do all the things, prompt, choose the option, start new session, trace the agents logs include sub-agents, ..., let investigate and update all" [admin]. This task is the page half of *trace the agents logs include sub-agents*.

## Dependencies & Reachability
**Depends on**: T124 (route shapes; build against mocks, merge after T124)
**Entry point**: the Live panel (`web/src/SessionPanel.jsx`)

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | An `Agent`/`Task` tool entry whose id equals a listed `parent_call` shows **Open log**; clicking opens that sub-agent's conversation (`?agent=<id>`) in the panel with the same entry view, 2 s refresh while open, Load earlier | FR-130 |
| 2 | A **Sub-agents** list in the panel: type, running/finished, nested under the parent sub-agent by `parent_call` (any depth); unknown parent → top level; the count shown; refreshed while the panel is open (≤ every 5 s, only while open) | FR-130 |
| 3 | A breadcrumb `main › <type> › <type>` shows where you are; each crumb leads back; switching session resets to main | FR-130 |
| 4 | Works in Past sessions (read-only) the same way | FR-130 |
| 5 | All text is React text (no HTML); 375 px: breadcrumb and list wrap, no page-wide scroll | FR-130 |
| 6 | vitest covers: open log, nesting, breadcrumb back, unknown parent, empty list, 404 shows one line; `npm run build` and `scripts/check_static.py` pass | all |

## UX Acceptance Criteria

| # | Criterion | Verdict |
|---|---|---|
| U1 | A sub-agent's log is reachable from its Agent call in ≤ 1 click | |
| U2 | The breadcrumb always shows which log is open | |
| U3 | At 375 px nothing overflows the page width | |

## Approach
Keep one `Transcript` component; give `useTranscript` an optional `agent` id that goes into the query. Fetch `/agents` with the panel's existing poll timing; build the tree client-side from `parent_call`.

## Files to change (predicted)
| File | Change |
|------|--------|
| `web/src/SessionPanel.jsx` | sub-agents list, breadcrumb, selected agent |
| `web/src/Transcript.jsx` | `agent` param, Open log on Agent entries |
| `web/src/api.js` | two calls |
| `web/src/PastSessions.jsx` | same view for past |
| tests | new specs |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |
| store schema (`rust/crates/core/src/schema_v11.sql`) | no schema change in this plan |
| `src/easy_personal/**` | Python app |
| `rust/**` | T124 |

## Test plan
vitest with mocked routes; `ux-evaluate` at 375/1280.

## Verification Command
```bash
cd web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: open a session's sub-agent, then its nested sub-agent, and back to main)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
