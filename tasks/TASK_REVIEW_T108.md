# TASK_REVIEW — T108: Kit workflow

> Sibling of `tasks/TASK_GUIDE_T108.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Direction: the map is one table in the Rust core that feeds `stage_of` and a new read-only `GET /api/kit/workflow`; not a copy in React, not parsed from project files — DEBATED: B (re-run 2026-11-18 at Stage 4; Advocate 1: B, high, cites PRD.md:214 "Two flows are built in, from one table in the Rust core that also feeds the FR-083 stage map"; Advocate 2: B, high (asked for the simplest option), same quote, and FR-119 "The app reads nothing from a repo for this (NFR-003)" rules out C) ⚠ decided for you — please review
2. Placement and design: a second `// kit` tab **Workflow** built only from existing tokens — DEBATED: A (re-run 2026-11-18 at Stage 4; Advocate 1: A, high, cites PRD.md:215 "The `// kit` group gets a second tab, **Workflow**"; Advocate 2: C inside LiveView (asked for the least new page code), medium on code size, low that it should win over A, cites web/src/LiveView.jsx:293; settled by the written rule FR-118 (PRD.md:215)) ⚠ decided for you — please review
3. One task (route + view in one worktree, two agents in turn) instead of the two tasks in the chat plan — RULE: `CLAUDE.md` § Karpathy Engineering Principles (Simplicity First: the route is a constant)
4. The agent of each board row is cut (the board route is not changed) — FACT: rust/PROJECT_SPEC.md:62 "| JSON parity with Python | High |"
5. The live layer reads the existing stream only (no backend change for it) — FACT: rust/crates/core/src/live.rs:960 "\"attention\": attention, \"stage\": s.stage, \"stage_skill\": s.stage_skill,"
6. Redo: FR-083 ids stay as they are inside `signals.stages` (Stage 0 holds "0.5", Stage 1 holds "1" and "1.5"), so `stage_of` and the Python app are unchanged — FACT: rust/crates/core/src/workflow.rs:12 "(\"0.5\", &[\"brainstorming\", \"ideate\"]),"
7. A Rust-board task edits `web/` (the page is shared) — FACT: tasks/TASK_GUIDE_T104.md:16 "3. Read `web/src/LiveView.jsx`, the Settings view and the Workspaces page from T092"
8. `rust/parity/leak_rust.py` was edited outside the guide's files (the new route registered in `API_CALLS`), because the leak gate refuses a route it does not know — FACT: rust/parity/leak_rust.py:708 "f\"unregistered /api route(s) in the Rust server: {sorted(rust_routes - set(API_CALLS))}\""
9. Redo scope: two built-in flows on one horizontal line, no skills shown, the kit's latest stage names; own flows and the editor split to T109 — FACT: PRD.md:334 "| 44 | Decided by the owner 2026-11-17"
10. AC 3 narrowed: an agent may sit on several steps (common-infrastructure on Stages 1 and 3, qa-expert on 4 and 5, as the guide's own agent list says); stages, skills, tools and commands stay unique per flow — FACT: tasks/TASK_GUIDE_T108.md:56 "no `stages`, `skills`, `tools` or `commands` value sits in two steps of one flow (an agent may work in several steps)"
11. The 0.625rem chip literal is out: every size comes from the tokens — FACT: web/src/tokens.css:1 "every color, font, size and space in the page comes from here."
12. Redo 2 (owner): the flow only — nodes highlighted where an agent works, a hover/focus/tap panel with project, agent and time taken; the right rail stays (settles the B/C Debate) — FACT: PRD.md:215 "you can show the flow only, with the node that got the agent are working, let highlight this node"
Checks: tests `cd rust && cargo test --workspace` — green (redo step A; `live_parity` 15/15); `cd web && npx vitest run` — 30 files, 1083 passed; `npm run build` ok; `python3 scripts/check_static.py` — ok; clippy (± window) and fmt ok; leak gate 0 hits over 42 canaries; reviews code-review (independent reviewer) — 0 P0/P1, 2 P2 fixed (README, evidence), P3 fixed; security-review — no findings; real run `easy-personal-rs serve --no-ingest` then `GET /api/kit/workflow` — printed `easy-kit ['Stage 0 · Define', … 'Stage 5 · Integrate']` and `plain ['Understand', 'Plan', 'Change', 'Check', 'Ship']` (auto-checked, not /verify)
Memory: owner-workflow-view-shape.md (auto memory) — workflow views are one horizontal line, no skill detail; own flows via file + in-app editor
Not done: merge (admin: after the auto verify pass); auto verify: ☑ pass (session 0c617b12-483c-4f75-9324-226d4dbcd68a, commit 3b83af8a0291e88641d4b77e6200e39023734003, log .claude/verify-runs/T108-3b83af8a0291.log); companion-setup skipped (auto mode); readiness incomplete: root PROJECT_SPEC.md has no ## Readiness (yours); push to github (yours)
Paused: done not moved — report incomplete: auto verify slot

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (web) | web: `web/src/WorkflowView.test.jsx` (21 tests: AC3-AC7 and the Edge Case Checklist), `web/src/App.test.jsx` (Workflow tab in the kit group; scoped `/api/board?workspace=work` read), `web/src/test/fixtures.js` (`workflow`, `/api/kit/workflow`, `/api/board`). Step A (backend): `rust/crates/app/src/api_tests.rs` (`kit_workflow_is_the_fixed_pipeline_guarded_read_only_and_reads_no_store`: AC 1, 403 without token, 405 on writes, no store opened) and `rust/crates/core/src/workflow.rs` (`the_route_and_the_live_stage_map_are_the_same_table_both_ways`: AC 2, two-way against FR-083 as written in the PRD; `a_skill_is_on_one_stage_only_and_every_gate_names_a_known_stage`). |
| Verification command run | ☑ pass (web) | web, 2026-11-17: `cd web && npx vitest run` → `Test Files  30 passed (30)` / `Tests  1065 passed (1065)` (baseline 29 files, 1043 tests); `npm run build` → `✓ built in 756ms`; `python3 scripts/check_static.py` → `check_static: ok`. `npm run lint` does not exist (`npm error Missing script: "lint"`: web/package.json has only `build` and `test`), so no lint ran. Step A (backend), 2026-11-17, `cd rust`: `cargo test --workspace` → exit 0, 1109 passed, 0 failed, 28 ignored (the `#[ignore]` ones were not run), including `workflow::tests::*` (2) and `api::tests::kit_workflow_...` (1), and `live_parity` (15, the `golden-live*` goldens, unchanged); `cargo clippy --workspace --all-targets -- -D warnings` → rc 0; `cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings` → rc 0; `cargo fmt --check` → ok; leak gate `parity/leak_rust.py` → `leak check: 0 hits over 42 canaries, 867 surfaces, 2041381 bytes` (`surface api GET /api/kit/workflow: 1393 bytes` scanned; the route is registered in `API_CALLS` of that script, which refuses an unregistered route). |
| Negative cases hold | ☑ pass (web) | web: RED before the code, see Demonstration BEFORE (3 App tests failed). Mutations of `WorkflowView.jsx`, each caught then restored (`git diff --stat -- web/src/WorkflowView.jsx` is empty only because the file is new): window 5 min → 50 min fails "ignores a sub-agent older than 5 minutes"; `MAX_CHIPS` 5 → 50 fails "at most 5 live chips"; session chip without its skill fails 5 tests. Step A (backend): RED first, `GET /api/kit/workflow` → 404 (Demonstration BEFORE, 2026-11-17T11:27:11Z). Mutation: `"ideate"` removed from Stage 0.5 in `workflow.rs` → `the_route_and_the_live_stage_map_are_the_same_table_both_ways` FAILED (`stage 0.5 left: ["brainstorming"] right: ["brainstorming", "ideate"]`), restored. |
| verify | ☑ pass | auto-run by hook, session 0c617b12-483c-4f75-9324-226d4dbcd68a, commit 3b83af8a0291e88641d4b77e6200e39023734003, log .claude/verify-runs/T108-3b83af8a0291.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Redo 3 Stage 4 (2026-11-18): independent reviewer on `git diff 6509f9d 96fd572` (`web/src/WorkflowView.jsx`, `WorkflowView.test.jsx`, `app.css`) and their caller `App.jsx`; earlier rounds (route, placement) were reviewed before. Findings: 0 P0, 0 P1, 3 P2 (row-to-row edges not port to port; panel under the whole graph / off-screen at 375; gates after all steps in Tab order), 5 P3; all fixed in `b29f133` except two cuts (gatesAfter refactor, ResizeObserver test). Security (Risk Medium): added code scanned for HTML injection, eval, storage, fetch, process/file use: no hit outside tests; the route is a guarded read-only constant (403/405 tests). |
| Full smoke suite still green (no regression) | ☑ pass | 2026-11-18 at `b29f133`: `cd web && npx vitest run` → `Test Files  30 passed (30)` / `Tests  1099 passed (1099)`; `cargo test -p easy-personal-rs workflow` → `ok. 1 passed`; `cargo test -p easy-personal-core live` → ok; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` ok; `npm run build` ok; `python3 scripts/check_static.py` → `check_static: ok` |
| **Companion score (easy-verifier)** | ☑ N/A | not run: the score tool's numbers were low-signal on this repo (memory handoff 2026-11-14: overall 41, change_risk abstained); Stage 4 used an independent reviewer and real-app screenshots instead |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | Redo 3, 2026-11-18, real app (`easy-personal-rs serve` from the T108 build on the owner's live data, headless Chrome over CDP, Workflow tab clicked): BEFORE fixes `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f4cd3c0-cdf6-469c-a97f-525e670398f9/scratchpad/ux/workflow-*.png` (straight diagonal edges between rows, pills cut to "No TAS…", panel off-screen at 375 with the page 720 px wide); AFTER `b29f133` `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f4cd3c0-cdf6-469c-a97f-525e670398f9/scratchpad/ux2/workflow-1280.png`, `workflow-1280-panel.png`, `workflow-375.png`, `workflow-375-panel.png`: dotted canvas, Live sessions start card, step cards with mono subtitle and square ports, curved edges port to port that never cross a card, gate pills readable ("No TASK_GUIDE, no Stage 3", "Verify passes before merge"), panel right under its node, compared with `docs/design/workflow/owner-reference-2026-11-17.png`. ux-evaluate: pass. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | Reviewer: every `var(--…)` in the diff exists in `web/src/tokens.css`, no colour literal added, accent is the page's (not the reference's orange); shots show the page's accent green for working nodes and edges, muted dashed idle edges |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280 (shots above): document `scrollWidth` = `clientWidth` with and without the panel open (375/375, 1280/1280); at 375 one column of cards with S-curved edges, the canvas scrolls inside itself; at 1280 two columns beside the right rail |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (step A, backend; `date -u` 2026-11-17T11:27:11Z, worktree `T108` at `701fab8`, no implementation commit yet; the route test was written first and run against the code as it was):
```
$ cd rust && cargo test -p easy-personal-rs kit_workflow
running 1 test
test api::tests::kit_workflow_is_the_fixed_pipeline_guarded_read_only_and_reads_no_store ... FAILED
thread '...kit_workflow_is_the_fixed_pipeline_guarded_read_only_and_reads_no_store' panicked at crates/app/src/api_tests.rs:1248:5:
assertion `left == right` failed: null
  left: 404
 right: 200
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 258 filtered out; finished in 0.01s
```
(`GET /api/kit/workflow` with a good Host and token answers 404 today.)

**BEFORE** (web: step B, frontend; `date -u` 2026-11-17T11:31:48Z, worktree `T108`, no implementation commit yet; the App tests were changed first to expect the Workflow tab and run against the page as it was):
```
$ cd web && npx vitest run src/App.test.jsx
 ❯ src/App.test.jsx (72 tests | 3 failed)
   × lists the 7 views in the sidebar and marks only the active one
   × puts each view in its group
   × opens the Workflow tab from the kit group and reads the map and the scoped board (T108, FR-118)
 FAIL  ... > puts each view in its group
AssertionError: expected [ 'Kit' ] to deeply equal [ 'Kit', 'Workflow' ]
 FAIL  ... > opens the Workflow tab from the kit group ...
TestingLibraryElementError: Unable to find an accessible element with the role "button" and name "Workflow"
```
(The `// kit` group has only the Kit tab today. Baseline before the change: 29 files, 1043 tests passed, 2026-11-17T11:29:28Z.)

**Redo BEFORE** (step A, backend redo; `date -u` 2026-11-17T15:04:12Z, worktree `T108` at `c424b63`, no redo implementation commit yet; the five flow tests in `workflow.rs` were written first and run against the old `{"stages", "gates"}` code):
```
$ cd rust && cargo test -p easy-personal-core workflow 2>&1 | tail -3
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 65 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p easy-personal-core --lib`
```

**Redo BEFORE (web)** (step B, frontend redo; `date -u` 2026-11-17T15:10:53Z, worktree `T108`, no redo implementation commit yet; the flows fixture and the new `WorkflowView.test.jsx` were written first and run against the old vertical-lane view):
```
$ cd web && npx vitest run src/WorkflowView.test.jsx 2>&1 | tail -4
 Test Files  1 failed (1)
      Tests  36 failed | 2 passed (38)
```
(The old view reads `map.stages`, the new route sends `flows`; `placeSession` is not exported yet.)

**Redo 2 BEFORE (web)** (step B, frontend redo 2; `date -u` 2026-11-17T15:44:33Z, worktree `T108` at `63571cf`, no redo 2 implementation commit yet; the new `WorkflowView.test.jsx` (nodes only, `workingAgents`, `takenText`, the panel) and the App test (no board read) were written first and run against the redo-1 view):
```
$ cd web && npx vitest run src/WorkflowView.test.jsx src/App.test.jsx 2>&1 | grep -aE "Test Files|Tests  "
 Test Files  2 failed (2)
      Tests  31 failed | 84 passed (115)
```
(`workingAgents` and `takenText` are not exported yet; the line still draws parts, agent lists and chips and the page still reads `/api/board`.)

**Redo 3 BEFORE (web)** (step B, frontend redo 3; `date -u` 2026-11-17T16:19:23Z, worktree `T108` at `6509f9d`, no redo 3 implementation commit yet; the new "the drawing (owner reference)" tests (start card count, edge active/idle classes, edge count, pill) were written first and run against the redo-2 line of buttons):
```
$ cd web && npx vitest run src/WorkflowView.test.jsx src/App.test.jsx 2>&1 | grep -aE "Test Files|Tests  |FAIL"
 FAIL  src/WorkflowView.test.jsx > the drawing (owner reference) > draws the start card with the count of live sessions placed on the flow, or "no live stream"
 FAIL  src/WorkflowView.test.jsx > the drawing (owner reference) > says "no live stream" on the start card outside a stream provider
 FAIL  src/WorkflowView.test.jsx > the drawing (owner reference) > gives the edge into a working node the active class and the edge into an idle node the idle class
 FAIL  src/WorkflowView.test.jsx > the drawing (owner reference) > draws one edge between neighbours, hidden from assistive tech, and the start edge active once any agent works on the flow
 Test Files  1 failed | 1 passed (2)
      Tests  4 failed | 116 passed (120)
```
(There is no start card and no SVG edge yet.)

**AFTER** (redo 3 + Stage 4 fixes, `b29f133`, 2026-11-18):
```
$ cd web && npx vitest run 2>&1 | grep -E "Test Files|Tests "
 Test Files  30 passed (30)
      Tests  1099 passed (1099)
```
Real app at 375 and 1280: the flow drawn as a node graph with curved port-to-port edges, gate pills and a hover panel under its node (screenshots in the UI rows).

**DELTA**: the owner can open Kit → Workflow and see the Easy Kit (or plain) flow drawn like their reference picture, with the steps where agents work now highlighted and who is there on hover.

**WITNESS**: the Supervisor, 2026-11-18, independent of the implementing agent: re-ran the suite and shot the real app (`ux2/` screenshots); auto verify by the hook follows.
