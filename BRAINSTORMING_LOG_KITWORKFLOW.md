# BRAINSTORMING_LOG — Kit workflow view (T108)
**Date**: 2026-11-17 · **Tier**: Standard · **Mode**: `[admin]` (direction decided by `docs/claude-md/tracks.md` row 4 and a Debate; see `tasks/TASK_REVIEW_T108.md`)

## The Problem Space
The owner wants the Kit section to show **the kit's workflow** (Stage 0–5) and **where the agents sit in it**. Today the Kit page (`web/src/KitView.jsx`) shows the by-workspace block, the per-project cards and the easy-ui checks. Nothing shows the pipeline.

What exists (checked in code):
- A fixed skill → stage map: `rust/crates/core/src/live.rs:83` `stage_of` (FR-083), mirrored in `src/easy_personal/core/live.py` `STAGE_MAP`.
- The live snapshot carries `stage` and `stage_skill` per session (`live.rs:968`). Each feed event carries `agent`, the FR-082 sub-agent type, and `label` (`live.rs:864`). The page holds one stream in `LiveContext` (`web/src/LiveView.jsx:384`).
- `GET /api/board` gives `id`, `title` and `status` per task (`rust/crates/core/src/board.rs:379`). The agent column of a board row is cut off with the title (`before_spaced_bar`). That route is a Python parity golden (`rust/parity/golden/manifest.json`).

## The Alternatives

| Path | What | Pros | Cons |
|---|---|---|---|
| A. Static page map | The stage/skill/agent map hard-coded in a new React view; live and board layers from existing reads | Smallest; no backend | A second copy of `stage_of`: the page and the tail can disagree after a skill is added |
| B. One table in core + `GET /api/kit/workflow` (recommended) | A const table in the Rust core feeds `stage_of` and a new read-only route; the view draws it plus the live and board layers | One source of truth; route is static, no I/O, no new data; no parity golden touched | A small backend step; `stage_of` refactor touches the live tail |
| C. Read each project's own workflow | Parse `CLAUDE.md` / `AGENTS.md` / `agents/` per project to draw that project's stages | Shows custom kits | Free-text parsing, fragile, project-specific; new file reads (NFR-003 review); much more code |

**50% less code**: Path A is that. It is rejected only for the duplicated map; if the Debate prefers it, the view still works.

## Adversarial Review
- **A fails** when a new skill joins `stage_of` and the page is not updated: a session on that skill sits on the wrong lane, silently.
- **B fails** if the refactor changes `stage_of` output: guarded by the `golden-live*` tests and a two-way table test (AC 2).
- **C fails** on the first project whose `CLAUDE.md` is written differently, and it widens what the app reads from repos.

## Silent failures / edge cases
- A session on an unmapped skill keeps its last stage (FR-083). The view must not guess.
- The stream drops: the map and board counts must still render.
- Sub-agent events exist only since the page opened (no history replay of the feed, FR-086). The live agent layer is "seen in the last 5 minutes", not "running now".
- A stage id that the page does not know: shown as an extra lane, never dropped.
- 50 sessions on one lane: cap the chips, show "+N more".

## Surgical Scope
**Touch**: `rust/crates/core/src/live.rs` (or new `workflow.rs`), `rust/crates/app/src/api.rs`, `api_tests.rs`, `web/src/App.jsx`, new `web/src/WorkflowView.jsx` + test, `web/src/app.css`, `web/src/test/fixtures.js`, `PRD.md` (FR-117, FR-118).
**Must not touch**: `src/easy_personal/**`, `board.rs`, `rust/parity/**`, `KitView.jsx` behaviour.

## Recommended Path
**B**, as a second tab **Workflow** in the `// kit` group (the Kit tab stays as it is).

## Next Actions (Stage 2)
1. PRD FR-117 (map + route), FR-118 (live + board layers), Out of Scope line.
2. TASK_GUIDE T108: step A backend, then step B frontend in the same worktree.
3. The edge cases above go into the guide's Edge Case Checklist.
