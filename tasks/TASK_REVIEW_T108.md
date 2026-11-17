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
1. Direction: the map is one table in the Rust core that feeds `stage_of` and a new read-only `GET /api/kit/workflow`; not a copy in React, not parsed from project files — DEBATED: B (Advocate 1: B, one table + route, cites `PRD.md:180` "fixed *stage map* in code"; Advocate 2: B, same, cites `web/src/LiveView.jsx:31` "the page never works one out") ⚠ decided for you — please review
2. Placement and design: a second `// kit` tab **Workflow** built only from existing `app.css` tokens; no owner design was asked for (the owner said "I will check the final later") — DEBATED: A (Advocate 1: A, cites `docs/claude-md/pipeline-stages.md:201` "With no design and no rows, the baseline checks are its design pass"; Advocate 2: A, cites `memory/decisions.md:282` D7 and the owner's newer instruction) ⚠ decided for you — please review
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
Not done: merge (admin: after the auto verify pass); auto verify: not started (redo 2 in progress); companion-setup skipped (auto mode); readiness incomplete: root PROJECT_SPEC.md has no ## Readiness (yours); push to github (yours)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (web) | web: `web/src/WorkflowView.test.jsx` (21 tests: AC3-AC7 and the Edge Case Checklist), `web/src/App.test.jsx` (Workflow tab in the kit group; scoped `/api/board?workspace=work` read), `web/src/test/fixtures.js` (`workflow`, `/api/kit/workflow`, `/api/board`). Step A (backend): `rust/crates/app/src/api_tests.rs` (`kit_workflow_is_the_fixed_pipeline_guarded_read_only_and_reads_no_store`: AC 1, 403 without token, 405 on writes, no store opened) and `rust/crates/core/src/workflow.rs` (`the_route_and_the_live_stage_map_are_the_same_table_both_ways`: AC 2, two-way against FR-083 as written in the PRD; `a_skill_is_on_one_stage_only_and_every_gate_names_a_known_stage`). |
| Verification command run | ☑ pass (web) | web, 2026-11-17: `cd web && npx vitest run` → `Test Files  30 passed (30)` / `Tests  1065 passed (1065)` (baseline 29 files, 1043 tests); `npm run build` → `✓ built in 756ms`; `python3 scripts/check_static.py` → `check_static: ok`. `npm run lint` does not exist (`npm error Missing script: "lint"`: web/package.json has only `build` and `test`), so no lint ran. Step A (backend), 2026-11-17, `cd rust`: `cargo test --workspace` → exit 0, 1109 passed, 0 failed, 28 ignored (the `#[ignore]` ones were not run), including `workflow::tests::*` (2) and `api::tests::kit_workflow_...` (1), and `live_parity` (15, the `golden-live*` goldens, unchanged); `cargo clippy --workspace --all-targets -- -D warnings` → rc 0; `cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings` → rc 0; `cargo fmt --check` → ok; leak gate `parity/leak_rust.py` → `leak check: 0 hits over 42 canaries, 867 surfaces, 2041381 bytes` (`surface api GET /api/kit/workflow: 1393 bytes` scanned; the route is registered in `API_CALLS` of that script, which refuses an unregistered route). |
| Negative cases hold | ☑ pass (web) | web: RED before the code, see Demonstration BEFORE (3 App tests failed). Mutations of `WorkflowView.jsx`, each caught then restored (`git diff --stat -- web/src/WorkflowView.jsx` is empty only because the file is new): window 5 min → 50 min fails "ignores a sub-agent older than 5 minutes"; `MAX_CHIPS` 5 → 50 fails "at most 5 live chips"; session chip without its skill fails 5 tests. Step A (backend): RED first, `GET /api/kit/workflow` → 404 (Demonstration BEFORE, 2026-11-17T11:27:11Z). Mutation: `"ideate"` removed from Stage 0.5 in `workflow.rs` → `the_route_and_the_live_stage_map_are_the_same_table_both_ways` FAILED (`stage 0.5 left: ["brainstorming"] right: ["brainstorming", "ideate"]`), restored. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | `ux-evaluate` run by the frontend agent (a scratch harness builds the real `App` with `fakeFetch` fixtures and a fake `EventSource`, served from a scratch folder, shot with `google-chrome --headless`): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/8c138a75-e773-4538-855b-70cceaaa4742/scratchpad/ux/workflow-375.png`, `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/8c138a75-e773-4538-855b-70cceaaa4742/scratchpad/ux/workflow-1280.png`. Read both: seven lanes in route order, gates between them, "3 waiting for Stage 3" before Stage 3, "In Progress 2" on Stage 3, "Ready for Review 1 · Stage 4–5" on Stage 4, "41 done" after Stage 5, live chips on their lanes with "+3 more", "agent · qa-expert" on Stages 4 and 5, an extra "Stage 6" lane, and "Stage not known yet". Baseline B-a..B-d, B-f..B-i pass or are n/a (the page is read-only: no action, no tap target); B-e fails (see Design-system row). No UX rows in the guide: baseline only. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass, one note | Method: read the diff of `web/src/app.css` (12 rules, layout only: flex, gap, wrap, one 40rem media rule; every value is a `var(--space-*)`, `var(--color-*)`, `var(--fs-*)` or `var(--font-mono)` token already in use; no colour literal added) and `WorkflowView.jsx` (existing `FeedCard`, `.chip`, `.chip-ok`, `.count`, `.muted`, `.error`). Dark is the only theme of the page. Note: B-e (body text at least 16px at 375) fails page-wide, not by this change: the app's own tokens are `--fs-body` 14px, `--fs-note` 12.5px and `--fs-label` 11.5px, and the cut chips use `--fs-label` like the Live and Board chips. Not changed: a new type scale is out of scope (no owner design). |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 px and 1280 px, shots above. At 375 the lanes stack (one column, each lane's rows stacked: label above chips), no horizontal scrollbar and no clipped chip (a 28-character project name wraps inside its chip); at 1280 the label and chips sit side by side inside each lane next to the right rail. Long chips use `overflow-wrap: anywhere`. |

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

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
