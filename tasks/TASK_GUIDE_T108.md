# TASK_GUIDE — T108: Kit workflow — the kit's Stage 0–5 pipeline, the agents at each stage, and where live sessions and board tasks are now
**Date**: 2026-11-17
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
**Assigned agents**: backend-developer (step A: the route), then frontend-developer (step B: the view)
**Agent guides**: `agents/backend.md`, `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. Read `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` FR-083, FR-084, FR-117, FR-118
2. Read the memory slice in your spawn prompt
3. Read this file completely, and your agent guide
4. Step A: `rust/crates/core/src/live.rs` (`stage_of`, line ~83), `rust/crates/app/src/api.rs` (the `/api/kit*` routes, ~225), `rust/crates/app/src/api_tests.rs`
5. Step B: `web/src/App.jsx` (`GROUPS`, the `// kit` group), `web/src/KitView.jsx`, `web/src/LiveView.jsx` (`LiveContext`, the session `stage` / `stage_skill`, the event `agent` and `label`), `web/src/BoardView.jsx`, `web/src/api.js`, `web/src/test/fixtures.js`, `web/src/app.css`
- Never run the real `claude`. Never write under `~/.easy-personal`, `~/.claude*`, `~/.local`, `~/.config`. Never run an `#[ignore]` test.

---

## Requirement

**Original request** (owner, 2026-11-17): "compose the plan to update the Kit section to the feature that can show the workflow of the kit, the position of the agents in the strict workflow", then "go, [admin]" on the plan.

**Restated intent**:
> 1. The Kit section gets a **Workflow** view: the kit's fixed pipeline, Stage 0 to Stage 5, one lane per stage, each with its skills, the agents that work there, and the checks between stages (no TASK_GUIDE → no Stage 3; review before verify; verify before merge).
> 2. **Live layer**: each live session sits on the lane of its current kit stage (FR-083); a sub-agent seen working in the last few minutes sits on the lane of its type. Read from the page's one `/api/live` stream; nothing new stored or read.
> 3. **Board layer**: per lane, the count of board tasks of the page's scope by board status (Todo → waiting for Stage 3; In Progress → Stage 3; Ready for Review → Stage 4–5; Done last), from `GET /api/board`.
> 4. The map comes from one place: a fixed table in the Rust core served by `GET /api/kit/workflow`. The live stage map (`stage_of`) and the route read the same table, so the page never holds its own copy.

**Out of scope (cut list)**: a workflow read from each project's own `CLAUDE.md` or `AGENTS.md`; the agent named on each board row (the board route is a Python parity golden, `rust/PROJECT_SPEC.md:62`); a history of stage changes (stages are never stored, FR-083); links from a stage to a skill's text; the Python app; any MCP tool or CLI command.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-17, `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `PRD.md` FR-117, FR-118; FR-083

---

## Dependencies & Reachability
**Depends on**: none (FR-083 stage and FR-082 sub-agent types are on `main`)
**Entry points**: `GET /api/kit/workflow`; the `// kit` › **Workflow** tab

## Dependency rule
No new crate, no new npm package.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Step A: `GET /api/kit/workflow` answers 200 with `{"stages": [...], "gates": [...]}`; each stage has `id` (exactly the ids of `stage_of`: "0.5", "1", "1.5", "2", "3", "4", "5"; 0.5 is named as Stage 0 · Define), `name`, `skills`, `agents`; behind the NFR-008 guard like the other `/api/kit*` routes (no token → 401/403 as they answer) | intent 1, 4 |
| 2 | Step A: one table in `live.rs` (or a new `workflow.rs` in core) feeds both `stage_of` and the route; a test proves every skill of `stage_of` is in the route's stage of the same id and the reverse; all `golden-live*` tests still pass unchanged | intent 4 |
| 3 | Step B: the `// kit` group has a second tab **Workflow**; the Kit tab is unchanged (its tests unchanged and passing) | intent 1 |
| 4 | Step B: the view draws one lane per stage from the route, with skills, agents and the gates between lanes; with the route failing it shows an error line, not a blank page | intent 1 |
| 5 | Step B: a live session with `stage: "4"`, `stage_skill: "code-review"` shows on the Stage 4 lane (project name and skill); a session with `stage: null` shows under "stage not known yet"; a sub-agent event (`agent: "backend-developer"`) from the last 5 minutes shows on each lane that lists that agent | intent 2 |
| 6 | Step B: board counts per lane match `/api/board` for the page's workspace/account scope; no board → the counts are hidden with one muted line | intent 3 |
| 7 | No message text, tool input or path reaches the view (only the FR-081 labels, project names and ids); the leak gate stays at 0 hits | NFR-002 |
| 8 | `ux-evaluate` at 375 px and 1280 px: lanes stack on narrow screens, no horizontal page scroll | — |
| 9 | `cd rust && cargo test --workspace`, clippy (with and without `--features window`) `-D warnings`, `cargo fmt --check`, leak gate 0 hits; `cd web && npm run lint && npm test && npm run build`; `python3 scripts/check_static.py` ok | all |

## UI / Design Acceptance Criteria
- Uses the page's existing tokens and classes (`app.css`: chips, `count`, `muted`, cards); no new colour values
- Keyboard: every lane and chip is reachable or plain text; no hover-only information
- Screens: 375 px and 1280 px, light and dark if the page has both

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs workflow && cargo test -p easy-personal-core live && cd ../web && npm run lint && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

### Evidence (filled by reviewer at Stage 4/5)
> In `tasks/TASK_REVIEW_T108.md`.

## Demonstration
BEFORE: `GET /api/kit/workflow` answers 404 on a real `serve`; the `// kit` group has only the Kit tab. AFTER: the route answers the map, and the Workflow tab shows the lanes with a live session on its stage.

## Approach
Path B of `BRAINSTORMING_LOG_KITWORKFLOW.md` (both Debates agreed).
- **Step A (backend)**: in `rust/crates/core/src/live.rs` (or a new `workflow.rs` re-exported by `lib.rs`), one `pub const` table: per stage `id`, `name`, `skills`, `agents`; plus the `gates`. `stage_of` becomes a lookup in that table (same output for every skill: the `golden-live*` tests prove it). Agents per stage come from the kit's pipeline (`AGENTS.md` `## Stage 0–5 Agentic Pipeline`, the base team): 0.5 Supervisor; 1 Supervisor, common-infrastructure; 1.5 Supervisor (craft-agent roles); 2 Supervisor; 3 common-infrastructure, backend-developer, frontend-developer; 4 Supervisor, qa-expert; 5 qa-expert, Supervisor. Gates: after 2 before 3 "No TASK_GUIDE, no Stage 3"; after 3 before 4 "Ready for Review starts the reviews"; after 4 before 5 "Reviews before verify"; after 5 "Verify passes before merge". `GET /api/kit/workflow` in `api.rs` returns the table through the same guarded `serve` helper as `kit_reports` (no store read is needed; if `serve` needs a conn, ignore it). Do not change the Python `live.py`: its map is unchanged, so parity holds.
- **Step B (frontend)**: `web/src/WorkflowView.jsx` + a `{ id: 'workflow', label: 'Workflow', View: WorkflowView, scoped: true }` tab in the `// kit` group of `App.jsx`. Read `/api/kit/workflow` and `scopedPath('/api/board', scope)` with `useLoad`; read live sessions and events from `useContext(LiveContext)` (null outside a provider: then no live layer). Lanes as a list of cards (`FeedCard`-like classes from `app.css`), stacked on narrow screens. Existing tokens only; no owner design was supplied (decision 2 of the review file).

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/core/src/live.rs` (or new `rust/crates/core/src/workflow.rs` + `lib.rs`) | the one stage table; `stage_of` reads it |
| `rust/crates/app/src/api.rs`, `rust/crates/app/src/api_tests.rs` | `GET /api/kit/workflow` + tests |
| `web/src/App.jsx` | the Workflow tab in the `// kit` group |
| `web/src/WorkflowView.jsx`, `web/src/WorkflowView.test.jsx` (new) | the view + tests |
| `web/src/app.css` | lane layout only |
| `web/src/test/fixtures.js` | a workflow fixture |
| `rust/README.md` or `README.md` | one line for the new tab, if the README lists the views |

## Files must NOT touch
| File | Reason |
|------|--------|
| `src/easy_personal/**`, `tests/**`, `pyproject.toml` | the Python app (`rust/PROJECT_SPEC.md` Critical Constraints) |
| `rust/crates/core/src/board.rs`, `rust/parity/**` | the board route is a parity golden (cut list) |
| `web/src/KitView.jsx`, `web/src/LiveView.jsx` | read them; reuse exports only (an extra `export` is allowed) |

## Edge Case Checklist
- A session on a skill not in the map keeps its last stage (FR-083) — the view only shows what the stream sends
- Two sessions of the same project on different lanes: both shown
- Many sessions on one lane: at most 5 chips, then "+N more"
- The stream disconnected: the map and board layer still show; the live layer says so in one muted line
- A stage id the page does not know: shown as its own lane at the end, never dropped

## Questions Log
- 2026-11-17 DEBATE where the stage/skill/agent map comes from → one table in the Rust core + `GET /api/kit/workflow` (Advocate 1: B; Advocate 2: B) ⚠ decided for you — please review
- 2026-11-17 DEBATE where the view goes and with what design → a second `// kit` tab Workflow from existing `app.css` tokens (Advocate 1: A; Advocate 2: A) ⚠ decided for you — please review

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `docs/` updated
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
