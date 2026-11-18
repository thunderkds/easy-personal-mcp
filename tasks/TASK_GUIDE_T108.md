# TASK_GUIDE — T108: Kit workflow — flows drawn as one horizontal line (the kit's latest Stage 0–5 and a plain-prompt flow), the agents at each step, and where live sessions and board tasks are now
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

**Owner redo (2026-11-17, after checking the first build)**: "the workflow should be line in the horizontal, no need to add skill, refer to this for the latest kit, but how can we adapt any kit, flow, and maybe just the pure propmt, not kit, ... let cover them all", then "we should add to the app also" (own flows: file **and** in-app editor; that part is T109).

**Owner redo 2 (2026-11-17)**: "you can show the flow only, with the node that got the agent are working, let highlight this node, when hover into the node, will show the agents are working on that, the agents should show the project, name of agents, and the time that agent have took." This replaces intent 4 below and settles the 1280 px Debate (nodes alone fit; the rail stays).

**Owner redo 3 (2026-11-17)**: "you can draw like this" with a picture, saved as `docs/design/workflow/owner-reference-2026-11-17.png`: a node graph on a dotted canvas, cards with icon/title/mono subtitle and square ports, pill decision nodes, curved edges, the active path solid in the accent colour, idle paths dashed and muted. Applies to intent 4 (redo 2 behaviour kept: highlight, hover/focus/tap panel with project · agent · time).

**Restated intent** (redo; `PRD.md` FR-117, FR-118):
> 1. The route answers **flows**, not one stage list: `{"flows": [easy-kit, plain], "projects": {}}`; each flow `{id, name, steps, gates}`, each step `{id, name, parts, agents, signals}`; `signals` = `{stages, skills, agents, tools, commands}`.
> 2. **easy-kit** = the kit's latest pipeline (`docs/claude-md/pipeline-stages.md`), six steps: Stage 0 Define (parts Requirements · Grilling · Direction; signals.stages ["0.5"]), Stage 1 Set up (Environment · Agent team; ["1","1.5"]), Stage 2 Plan, Stage 3 Execute, Stage 4 Review, Stage 5 Integrate; agents and gates as before. FR-083's ids and `stage_of` output stay exactly the same (Python parity); the skills move into `signals.skills`.
> 3. **plain** = a session with no kit: Understand → Plan → Change → Check → Ship, signals from tool names, sub-agent types and Bash first words (FR-117 lists them).
> 4. The view (redo 2): a flow picker, then **the flow only**: one horizontal line of nodes (step names), arrows between them, gates as small markers on the arrows. A node with an agent working now (a call in the last 5 minutes) is **highlighted**. Hover, keyboard focus or tap on a node opens a panel: one row per working agent — project, agent (sub-agent type or "main session"), time taken (first call of that agent at that node seen by the page → its newest call). Each session goes on one flow (repo assignment → easy-kit when `stage` is known → plain); on easy-kit the session and its sub-agents sit on its stage's step; on other flows each agent sits on the step its own newest call matches, else its session's step. No parts, agent lists, skills or board counts on the line.
### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-17, `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `PRD.md` FR-117, FR-118 (redo 2026-11-17, decision 44); FR-083

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
| 1 | Step A: `GET /api/kit/workflow` answers 200 `{"flows": [...], "projects": {}}`; flows in order `easy-kit`, `plain`; every flow `{id, name, steps, gates}`, every step `{id, name, parts, agents, signals}` with `signals` holding all five keys `stages`, `skills`, `agents`, `tools`, `commands` (arrays, may be empty); NFR-008 guard (403 with no token), 405 on POST/PUT/PATCH/DELETE, no store opened | intent 1 |
| 2 | Step A: easy-kit has six steps ids "0".."5" with the names and parts of intent 2; the union of its `signals.stages` is exactly the FR-083 ids; a test proves `stage_of(skill) == Some(id)` ⇔ `skill` is in `signals.skills` of the step whose `signals.stages` holds `id` (both ways); all `golden-live*` tests pass unchanged | intent 2 |
| 3 | Step A: plain has steps understand, plan, change, check, ship with the FR-117 signals; no signal value fails the FR-081 label pattern; no `stages`, `skills`, `tools` or `commands` value sits in two steps of one flow (an agent may work in several steps) | intent 3 |
| 4 | Step B: the `// kit` group keeps Kit unchanged and has **Workflow**; a flow picker (one button per flow, `aria-pressed`, easy-kit first); the chosen flow renders as one row of nodes in step order showing only step names (test: no parts, no agent list, no skill, no board count in the line) | intent 4 |
| 5 | Step B: placement — a session with `stage: "1.5"` and a sub-agent `backend-developer` of that session both sit on easy-kit Stage 1; `stage: null` + newest `Edit` → plain Change; a plain sub-agent `Explore` whose newest call is `Grep` sits on Understand while its session's main thread sits on Change; a project in `projects` goes to that flow; nothing matching → "step not known yet" under the line | intent 4 |
| 6 | Step B: a node with a working agent (a call ≤ 5 minutes old) has a highlight class and an accessible state (e.g. `aria-label` "… · 2 working"); a node with none has neither; an agent whose newest call is older than 5 minutes is not working | intent 4 |
| 7 | Step B: hover, focus (node is a button) and click/tap open the node's panel; Escape or blur closes it; one row per working agent with project, agent ("main session" for the main thread) and time taken formatted like the page's other durations (e.g. "45s", "12m", "1h 5m"); gates on the arrows are focusable markers whose text shows the same way; route failing → error line; no stream → one muted line | intent 4 |
| 8 | No message text, tool input or path reaches the view (FR-081 labels, project names, ids only); leak gate 0 hits | NFR-002 |
| 9 | `ux-evaluate` at 375 px and 1280 px, compared with `docs/design/workflow/owner-reference-2026-11-17.png` (dotted canvas, cards with ports, gate pills, curved edges, active = accent solid, idle = muted dashed); the rail stays; the canvas scrolls inside itself when narrower than the graph; at 375 px the strip may scroll inside itself, the page never; the panel stays inside the viewport | — |
| 10 | `cd rust && cargo test --workspace`, clippy (with and without `--features window`) `-D warnings`, `cargo fmt --check`, leak gate 0 hits; `cd web && npx vitest run && npm run build`; `python3 scripts/check_static.py` ok | all |

## UI / Design Acceptance Criteria
- Uses the page's existing tokens and classes (`app.css`: chips, `count`, `muted`, cards); no new colour values
- Keyboard: every lane and chip is reachable or plain text; no hover-only information
- Screens: 375 px and 1280 px, light and dark if the page has both

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs workflow && cargo test -p easy-personal-core live && cd ../web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

### Evidence (filled by reviewer at Stage 4/5)
> In `tasks/TASK_REVIEW_T108.md`.

## Demonstration
BEFORE (redo): the first T108 build answers `{"stages": [...7], "gates": [...]}` and draws vertical lanes that list skills. AFTER: the route answers two flows, and the Workflow tab draws the chosen flow as one horizontal line, no skills, with live sessions on their step on easy-kit or plain.

## Approach
Redo on the `T108` branch (not merged; worktree `easy-personal-mcp-wt/T108`).
- **Step A (backend)**: `rust/crates/core/src/workflow.rs`: replace `STAGES` with the two built-in flows (consts). `stage_of` stays a lookup: the skill → the FR-083 id held by the step's `signals.stages` (for easy-kit Stage 0 that is "0.5"; Stage 1 holds both "1" (git-guardrails-claude-code, map-codebase) and "1.5" (craft-agent), so keep a per-skill FR-083 id: e.g. a step signal list of `(skill, id)` pairs internally, serialised as `signals.skills` + `signals.stages`). Output of `stage_of` byte-for-byte as now. `json()` answers `{"flows": [...], "projects": {}}` (`projects` filled by T109). Update `api_tests.rs`. Do not touch `live.py`, `board.rs`, `rust/parity/**` beyond what already exists.
- **Step B (frontend)**: rewrite `web/src/WorkflowView.jsx` for the flows shape: picker, horizontal line (`display: flex`, `overflow-x: auto` on the strip only, steps `flex: 0 0 auto`), connectors and gate markers; placement rules of AC 5 (one exported pure function `placeSession(session, events, flows, projects)` so tests hit it directly); keep the 5-chip cap and "+N more"; keep the muted lines. Existing tokens only.
## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/core/src/workflow.rs` | the two built-in flows; `stage_of` reads them |
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
- Many sessions on one step: at most 5 chips, then "+N more"
- A Bash event with no label (label dropped by FR-081): matches no `commands` signal
- A sub-agent event (`is_sidechain`) on plain: its tool still places the parent session
- A session's project assigned to a flow id that is not in `flows`: falls back to the auto rule
- The stream disconnected: the map and board layer still show; the live layer says so in one muted line
- A stage id the page does not know: shown as an extra step at the end of easy-kit, never dropped

## Questions Log
- 2026-11-17 DEBATE where the stage/skill/agent map comes from → one table in the Rust core + `GET /api/kit/workflow` (Advocate 1: B; Advocate 2: B) ⚠ decided for you — please review
- 2026-11-17 DEBATE where the view goes and with what design → a second `// kit` tab Workflow from existing `app.css` tokens (Advocate 1: A; Advocate 2: A) ⚠ decided for you — please review

- 2026-11-17 DEBATE how the six-step line fits at 1280 px with token-sized chips → paused for the owner (Advocate 1: B hide the rail on Workflow; Advocate 2: C scroll inside the strip); settled by the owner the same day: show the flow only, nodes with a hover/focus panel (redo 2), the rail stays
- 2026-11-17 Owner redo: flows, horizontal line, no skills, latest kit names; own flows file + editor → T109 (PRD decision 44)

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `docs/` updated
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
