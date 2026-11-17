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

**Restated intent** (redo; `PRD.md` FR-117, FR-118):
> 1. The route answers **flows**, not one stage list: `{"flows": [easy-kit, plain], "projects": {}}`; each flow `{id, name, steps, gates}`, each step `{id, name, parts, agents, signals}`; `signals` = `{stages, skills, agents, tools, commands}`.
> 2. **easy-kit** = the kit's latest pipeline (`docs/claude-md/pipeline-stages.md`), six steps: Stage 0 Define (parts Requirements · Grilling · Direction; signals.stages ["0.5"]), Stage 1 Set up (Environment · Agent team; ["1","1.5"]), Stage 2 Plan, Stage 3 Execute, Stage 4 Review, Stage 5 Integrate; agents and gates as before. FR-083's ids and `stage_of` output stay exactly the same (Python parity); the skills move into `signals.skills`.
> 3. **plain** = a session with no kit: Understand → Plan → Change → Check → Ship, signals from tool names, sub-agent types and Bash first words (FR-117 lists them).
> 4. The view: a flow picker, the chosen flow as **one horizontal line** of steps (name, parts, agents; **no skills shown**), gates as markers between steps; the line scrolls inside its own strip on narrow screens. Each live session is put on one flow (repo assignment from `projects` → else easy-kit when `stage` is known → else plain) and on a step (easy-kit: its `stage`; others: its newest matching feed event). Board counts on the easy-kit line only.
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
| 3 | Step A: plain has steps understand, plan, change, check, ship with the FR-117 signals; no signal value fails the FR-081 label pattern; no value sits in two steps of one flow | intent 3 |
| 4 | Step B: the `// kit` group keeps Kit unchanged and has **Workflow**; the view shows a flow picker (one button per flow, `aria-pressed`), easy-kit chosen first; the steps render in one row (a test asserts the row container and step order); no skill name from the route is rendered (test: a fixture skill text is absent) | intent 4 |
| 5 | Step B: a live session with `stage: "1.5"` shows on easy-kit's Stage 1 step; `stage: null` with a newest feed event `tool: "Edit"` shows on plain's Change step; `label: "cargo"` (a Bash event) on plain's Check; no matching event → "step not known yet"; a session whose project is in `projects` goes to that flow; a sub-agent type seen in the last 5 minutes shows on every step of the chosen flow that lists it | intent 4 |
| 6 | Step B: easy-kit's line shows the board counts of `/api/board` in the page's scope; other flows show none; no board → one muted line; route failing → an error line | intent 4 |
| 7 | Gates render between steps as markers with their text (not hover-only); a flow with no gates shows none | intent 4 |
| 8 | No message text, tool input or path reaches the view (FR-081 labels, project names, ids only); leak gate 0 hits | NFR-002 |
| 9 | `ux-evaluate` at 375 px and 1280 px: the line is one row at 1280 px; at 375 px the strip scrolls inside itself and the page has no horizontal scroll | — |
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

- 2026-11-17 Owner redo: flows, horizontal line, no skills, latest kit names; own flows file + editor → T109 (PRD decision 44)

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `docs/` updated
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
