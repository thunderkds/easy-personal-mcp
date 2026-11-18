# TASK_GUIDE — T113: Workflow placement moves into the live service — each agent's step comes in the live snapshot
**Date**: 2026-11-18
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
**Assigned agents**: backend-developer (step A: live engine + flow table), then frontend-developer (step B: the page draws the server's steps)
**Agent guides**: `agents/backend.md`, `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. Read `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` FR-082, FR-083, FR-117, FR-118
2. Read `BRAINSTORMING_LOG_WORKFLOWPLACE.md` (Option A chosen by the owner)
3. Read the memory slice in your spawn prompt, this file completely, and your agent guide
4. Step A: `rust/crates/core/src/workflow.rs` (`STAGE_SKILLS`, `EASY_KIT`, `PLAIN`, `stage_of`, tests), `rust/crates/core/src/live.rs` (`Stream` ~853, `Session` ~880, snapshot ~1085, ingest loop ~1395, `set_stage` ~1503)
5. Step B: `web/src/WorkflowView.jsx` (`placeSession`, `workingAgents`, `eventsOf`), `web/src/WorkflowView.test.jsx`, `web/src/LiveView.jsx` (`fromSnapshot`, `SAME_KEYS`)
- Never run the real `claude`. Never write under `~/.easy-personal`, `~/.claude*`, `~/.local`, `~/.config`. Never run an `#[ignore]` test.

## Requirement
**Original request** (owner, 2026-11-18): "for the workflow sections, does the app show correctly, cause I saw most of the agents are in stage 4", then "should assign this tasks to the brackground service for the live update"; picked Option A ("server places agents"); then "[admin] let do your jobs".

**Restated intent**: The Workflow page shows most agents on Stage 4 because (1) a session's stage is the last mapped Skill call and never moves on, (2) on easy-kit every agent of a session sits on that session stage, and (3) each tab places agents from the feed rows it happens to hold. The live service (`live.rs`) becomes the one place that decides the step of each (session, agent), for every built-in flow, and sends it in the `/api/live` snapshot; the page only draws it.

## Dependencies & Reachability
**Depends on**: none (T108 on `main`)
**Entry points**: `GET /api/live` snapshot; the `// kit` › **Workflow** tab

## Acceptance Criteria
1. Each snapshot session carries `agents: [{agent, first_ts, last_ts, steps: {<flow id>: <step id | null>}}]`, one row per agent the session has: the sub-agent type, `"sub-agent"` when a sidechain has no type, or `"main session"`. `steps` has a key for every flow in `workflow::json()`.
2. A Skill call inside a sub-agent stream sets **that agent's** stage, not the session's other agents'. The session `stage` / `stage_skill` fields (FR-083, the Live stage chip) keep today's values byte-for-byte.
3. easy-kit step of an agent, in order: its own stage → the step holding it; else its sub-agent type in one step's `signals.agents` → that step (a type on several steps, e.g. `qa-expert` on 4 and 5, → the one holding the session's stage, else the first); else the session stage's step; else `null`. A stage no step holds → that stage id (the page's extra node, as today).
4. Plain (and any non-kit built-in) step of an agent: the step its newest matching call points at (tool name, Bash first word, sub-agent type, Skill), else the session's step on that flow, else `null` — today's `workingAgents` rule, now in Rust, in `workflow.rs`.
5. Any change to an agent's step, `first_ts` at that step, or a new agent bumps `state_version` once; warm replay / restart yields the same `agents`.
6. `WorkflowView.jsx` draws from `session.agents` only: no `matches`, no `eventsOf`, no flow-signal logic left in the page. It keeps the 5-minute working filter (`last_ts`) and the "step not known yet" line. A snapshot from an older server (no `agents`) shows nothing placed, without an error.
7. Nothing new leaves the process beyond agent types, step ids and times (NFR-002): `scripts/leak_rust.py` 0 hits.
8. Test fixture: a session whose main thread ran `code-review` and whose `backend-developer` sub-agent is editing files shows the main session on Stage 4 and the backend-developer on Stage 3.

## UI / Design Acceptance Criteria
- No visual change: the same nodes, highlight, panel rows (project · agent · time). ux-evaluate: a before/after screenshot of the Workflow tab at 375 and 1280 only.

## Evaluation & Acceptance
### Verification Command
```bash
cd rust && cargo test -p easy-personal-core workflow && cargo test -p easy-personal-core live && cargo test -p easy-personal-rs && cd ../web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```
### Evidence (filled by reviewer at Stage 4/5)
> In `tasks/TASK_REVIEW_T113.md`.

## Demonstration
BEFORE: a session that ran `code-review` puts all its working agents, including a `backend-developer` editing files, on **Stage 4 · Review**. AFTER: the snapshot says `backend-developer → "3"`, `main session → "4"`, and the page draws them on Stage 3 and Stage 4.

## Approach
Option A of `BRAINSTORMING_LOG_WORKFLOWPLACE.md`.
- **Step A (backend)**: `Stream` gains its own `stage` / `stage_ts`, `first_ts`/`last_ts` and the newest mapped tool/command/label (not a history). `set_stage` also updates the stream it came from. New pure fn `workflow::place(...)` next to the table, unit-tested per AC 3/4. Snapshot builds `agents` from the session's streams grouped by agent name. Bump `state_version` where AC 5 says.
- **Step B (frontend)**: replace `placeSession`/`workingAgents` internals with reads of `session.agents[i].steps[flow.id]`; keep exported names if tests import them; carry `agents` through `fromSnapshot` and `SAME_KEYS` in `LiveView.jsx`.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/core/src/workflow.rs` | `place()` + tests |
| `rust/crates/core/src/live.rs` | per-stream stage/times/signal; snapshot `agents`; version bumps; tests |
| `web/src/WorkflowView.jsx` | draw the server's steps |
| `web/src/WorkflowView.test.jsx` | fixtures with `agents` |
| `web/src/LiveView.jsx` | carry `agents` from the snapshot |
| `PRD.md` | FR-118 placement sentence: the live service decides, the page draws |

## Files must NOT touch
| File | Reason |
|------|--------|
| `rust/crates/app/src/api.rs` `/api/kit/workflow` body | flows unchanged |
| `rust/crates/app/src/claude_cli.rs` | T114 (another live session) is changing it |
| `src/easy_personal/core/live.py` | no workflow page on the Python side |
| reply / answer bridge code (T106–T112) | unrelated, just merged |
| `.claude/hooks/**` | Stage 5 from hook verify is on the cut list |

## Edge Case Checklist
- Sub-agent before its `agentType` arrives (`meta_since` wait) → "sub-agent", then re-grouped under its type without a duplicate row.
- Out-of-order timestamps: the `ts >= stage_ts` rule, per stream.
- Unknown types (`Explore`, `general-purpose`) on easy-kit → session stage.
- Session with no stage and no signals → `null` steps → "step not known yet".
- Old snapshot without `agents` → empty placement, no throw.

## Cut list
- Stage 5 from a hook-written verify (needs a confirmed transcript signal) — follow-up task if wanted.
- Owner-defined flows (T109): `projects` is `{}` today; `steps` covers built-in flows only.

## Questions Log
- 2026-11-18 Owner chose Option A (server places every agent) over C (stage only) and B (page only).

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `docs/` updated
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
