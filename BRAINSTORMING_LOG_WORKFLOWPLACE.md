# BRAINSTORMING_LOG.md — T113: workflow placement moves into the live service

**Date**: 2026-11-18 · **Tier**: Standard · **Risk**: Medium (live snapshot contract + live engine + Workflow page)

## The Problem Space
The `// kit › Workflow` page (T108, FR-117/FR-118) shows most agents on **Stage 4 · Review**. Verified causes:
1. `live.rs:1503 set_stage` sets a **session** stage from the last mapped Skill call only; nothing clears or advances it.
2. `WorkflowView.jsx:87-110 workingAgents`: on easy-kit **every agent of a session sits on the session's stage**; only non-kit flows use the agent's own calls.
3. Stage 4 skills (`code-review`, `security-review`, `html-report`) are the last mapped skills of most tasks. Stage 5 needs `verify`/`ship`/`delivery-report` as Skill calls, but verify is usually written by `.claude/hooks/stop_auto_verify.py`, so Stage 5 rarely registers.
4. Placement is worked out in the browser from the feed rows the page happens to hold (`eventsOf`), so two tabs, or a tab opened late, can disagree.

Owner direction (2026-11-18): the **background live service** should own this and push it on live updates; the page only draws it.

## Questions for the User
1. Which path (A / B / C below)? ← asked now
2. (Later, only if A or C) Should a hook-written verify move a session to Stage 5? It needs a transcript signal we have not confirmed exists — on the cut list until proven.

## Alternative Paths

### Option A — Server places every (session, agent) — Recommended
`live.rs` keeps, per stream (main thread or sub-agent), its own kit stage (Skill calls inside that stream) and its newest/first call times. A new `workflow::place(flow, agent, stream_stage, session_stage, last_tool, last_cmd)` in `workflow.rs` picks the step from the same table the page draws (`STAGE_SKILLS` + `signals`). The snapshot gains `agents: [{agent, first_ts, last_ts, steps: {"easy-kit": "3", "plain": "change"}}]` per session; a step change bumps `state_version`. `WorkflowView.jsx` drops `matches`/`placeSession`/`workingAgents` logic and keeps only the 5-minute "working" filter (time-based, the server does not re-send on time passing).
- Pros: one source of truth; all tabs agree; uses the full stream history, not the page's buffer; sub-agents placed by their own type (`backend-developer` → 3, `qa-expert` → 4/5).
- Cons: biggest diff (~250 Rust + ~-100 JS); snapshot grows by one small array per session.

### Option B — Fix it in the page only
Keep placement in `WorkflowView.jsx`; on easy-kit let a sub-agent use its own type signal before the session stage.
- Pros: ~20 lines, no contract change.
- Cons: **not what the owner asked**; still per-tab, still buffer-dependent.

### Option C — Server sends a per-stream stage only
`live.rs` tracks the stage per stream and sends `agents: [{agent, stage, last_ts}]`; the page still maps stage → step and still matches plain-flow signals itself.
- Pros: about half of A's code; fixes the easy-kit pile-up.
- Cons: placement still split between server and page; plain flow unchanged.

## Adversarial Review
| Option | Why this might fail |
|---|---|
| A | A sub-agent type in two steps (`qa-expert` in 4 and 5) needs a tie rule; unknown types (`Explore`, `general-purpose`) must fall back to the session stage, else they vanish. A missed `state_version` bump = stale page. Warm replay must rebuild per-stream stages identically. |
| B | Leaves the owner's actual ask undone; the disagreement between tabs stays. |
| C | Two places own placement rules again — the drift T108 AC 2 (one table) was written to prevent. |

## 50% Rule Check
Option C is the 50%-less-code version of A. A is still recommended, because C keeps the split ownership the owner wants removed. Inside A, the code is cut by: no per-tool history (keep only the newest mapped tool/command per stream), no Stage 5 hook detection (cut list).

## Recommended Path
**Option A.** Tie rule: an agent type listed on several steps goes to the one that holds the session's stage, else the first. No signal → the session's stage (today's behaviour), so nothing disappears.

## Surgical Scope
**Touch**: `rust/crates/core/src/live.rs` (Stream fields, set_stage per stream, snapshot `agents`), `rust/crates/core/src/workflow.rs` (`place`, tests), `web/src/WorkflowView.jsx` (render the server's steps), their tests, `PRD.md` (FR-118 amendment).
**Must not touch**: `/api/kit/workflow` body (flows unchanged), `LiveView.jsx` stage chip (session `stage` stays as is), reply/answer bridge code (T106–T112), Python `src/easy_personal/core/live.py` (no workflow page there).

## Edge Case Checklist for TASK_GUIDE
- Sub-agent with no `agentType` yet (`meta_since` wait) → "sub-agent", placed by session stage.
- Skill call inside a sub-agent stream sets that stream's stage, not the whole session's.
- `qa-expert` tie rule; unknown agent types fall back.
- Out-of-order timestamps (`ts >= stage_ts` rule kept per stream).
- Warm replay / restart gives the same placement.
- Every placement change bumps `state_version` exactly once.
- No prompt text or labels beyond what the snapshot already sends (NFR-002 leak check).

## Next Actions
1. Owner picks a path.
2. Stage 2: `tasks/TASK_GUIDE_T113.md` + KANBAN row (T113).
3. Stage 3: backend-developer (live.rs/workflow.rs), then frontend-developer (WorkflowView.jsx).

## User Selection
Option A — server places every (session, agent). Owner, 2026-11-18.

## Cut list
- Stage 5 from a hook-written verify (needs a confirmed transcript signal).
- Owner-defined flows (T109) — `projects` is `{}` today.
