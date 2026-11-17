# TASK_GUIDE — T109: Own flows — `flows.json` in the app's folder, routes to write it, and the flow editor on the Workflow tab
**Date**: 2026-11-17
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
**Assigned agents**: backend-developer (step A: file + routes), then frontend-developer (step B: the editor)
**Agent guides**: `agents/backend.md`, `agents/frontend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. Read `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` FR-117..FR-120, FR-103 (the owner-only setting file), NFR-003, NFR-008
2. Read `tasks/TASK_GUIDE_T108.md` (the flow shape and the view this task extends)
3. Step A: `rust/crates/core/src/workflow.rs`, `rust/crates/app/src/api.rs`, `rust/crates/app/src/writes.rs`, `rust/crates/app/src/agent_run.rs` (~302: new owner-only file renamed over the old), `rust/crates/core/src/store.rs` (~175: the data folder), `rust/crates/app/src/api_tests.rs`
4. Step B: `web/src/WorkflowView.jsx`, `web/src/SettingsView.jsx` (form and write patterns), `web/src/api.js`, `web/src/test/fixtures.js`
- Never run the real `claude`. Never write under `~/.easy-personal`, `~/.claude*`, `~/.local`, `~/.config`: tests set `EASY_PERSONAL_HOME` to a temp dir. Never run an `#[ignore]` test.

## Requirement
Owner 2026-11-17: "how can we adapt any kit, flow, and maybe just the pure propmt, not kit, ... let cover them all", then on file / file in each repo / in-app editor: "we should add to the app also" (`PRD.md` decision 44).

**Restated intent**:
> 1. The owner keeps their own flows (another kit, or their own process) in one owner-only file `flows.json` in the app's data folder, with a repo → flow map. `GET /api/kit/workflow` adds them after the two built-ins and fills `projects`.
> 2. The app can write that file: create/replace a flow, delete a flow, set the repo map; each write validated, atomic, owner-only.
> 3. The Workflow tab gets an editor: New flow, Edit / Delete on own flows, Copy to my flows on a built-in, repos picked from the stored repos.
> 4. Nothing is read from a repo (NFR-003).

**Out of scope (cut list)**: a flow read from a repo file; importing/exporting flows; sharing flows between machines (the owner can copy the file); flow history or undo past the one delete confirm; the Python app; MCP or CLI.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: owner, 2026-11-17 ("we should add to the app also"); auto (Supervisor) for the details, `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `PRD.md` FR-119, FR-120

## Dependencies & Reachability
**Depends on**: T108 (the flow shape and the view; branch `T109` is cut from `T108`)
**Entry points**: `PUT /api/kit/flows/{id}`, `DELETE /api/kit/flows/{id}`, `PUT /api/kit/flow-projects`; `// kit` › Workflow › **New flow**

## Dependency rule
No new crate, no new npm package.

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Step A: with `flows.json` in `EASY_PERSONAL_HOME` holding a valid flow and `{"projects": {"repo-a": "<id>"}}`, `GET /api/kit/workflow` answers the built-ins then that flow, and `projects` = that map; no file → built-ins and `{}`; a file that is not JSON, too big (> 256 KiB), or fails validation → built-ins, `{}` and `"own_error": "<one line>"`, status 200 | FR-119 |
| 2 | Step A: validation per FR-119: ids and signal values match the FR-081 pattern; names and parts match it but may also hold spaces and `·` (≤ 64 chars); ≤ 20 flows, ≤ 12 steps, ≤ 40 signals a step, ≤ 8 agents a step, ≤ 12 gates; ids `easy-kit`, `plain` refused; a gate's `after`/`before` must name steps of that flow (`before` may be null); a `projects` value must name an existing flow (own or built-in); unknown keys refused | FR-119 |
| 3 | Step A: `PUT /api/kit/flows/{id}` (body = one flow, `id` in the body must equal the path) creates or replaces; `DELETE /api/kit/flows/{id}` removes it and its `projects` entries (404 when absent); `PUT /api/kit/flow-projects` (body = the map) replaces the map; each answers 200 with the new `GET` body, 400 `{"error": "<one line>"}` on a bad body, 403 with no token; a write against an invalid existing file answers 409 and does not overwrite it | FR-119, NFR-008 |
| 4 | Step A: the write is a new file mode 0600 in the same folder, then `rename` over `flows.json` (test: mode 0600, no temp file left, old content intact when validation fails); two writes in a row both land (no lost update within one process: a mutex around read-modify-write) | FR-119 |
| 5 | Step B: **New flow** opens a form: name, steps (add, remove, move left/right; name, parts, agents, and signals as comma-separated `stages`/`skills`/`agents`/`tools`/`commands`), gates (after, before, text), repos (check boxes from the stored repos the page already loads; no free path); Save sends the PUT and the new flow is picked; a 400 shows its one line and keeps the form | FR-120 |
| 6 | Step B: own flows show **Edit** and **Delete** (delete asks once); built-ins show **Copy to my flows** (opens the form filled, id/name made unique, no Edit/Delete); `own_error` shows as one warning line above the picker and the editor is disabled with that reason | FR-120 |
| 7 | Step B: the placement rule of T108 uses `projects` from the route (a session of `repo-a` lands on its flow) — test | FR-118, FR-119 |
| 8 | No repo file is read; no text other than what the owner typed, project names, FR-081 labels and ids is shown; leak gate 0 hits; the token never goes in a URL | NFR-002, NFR-003 |
| 9 | `ux-evaluate` at 375 px and 1280 px on the editor: fields stack on narrow screens, every control keyboard-reachable, labels on every input | — |
| 10 | `cd rust && cargo test --workspace`, clippy (with and without `--features window`) `-D warnings`, `cargo fmt --check`, leak gate 0 hits; `cd web && npx vitest run && npm run build`; `python3 scripts/check_static.py` ok | all |

## UI / Design Acceptance Criteria
- Existing tokens and classes only (`app.css`, the Settings form styles); no new colour values
- Keyboard: every control reachable; no hover-only information
- Screens: 375 px and 1280 px

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs workflow && cargo test -p easy-personal-rs flows && cargo test -p easy-personal-core workflow && cd ../web && npx vitest run 2>&1 | tail -5 && npm run build && cd .. && python3 scripts/check_static.py
```

### Evidence (filled by reviewer at Stage 4/5)
> In `tasks/TASK_REVIEW_T109.md`.

## Demonstration
BEFORE: `PUT /api/kit/flows/my-flow` answers 405/404 on a real `serve` (temp `EASY_PERSONAL_HOME`); the Workflow tab has no New flow. AFTER: the PUT answers the new body, `flows.json` exists with mode 0600, and the tab picks the new flow.

## Approach
- **Step A (backend)**: core `workflow.rs`: `pub fn own_flows(home: &Path) -> (Vec<Value>, Map, Option<String>)` reading `flows.json` (size cap, then `serde_json`, then `validate`), `pub fn validate(file: &Value) -> Result<(), String>`, and `json_with(own)`. App: three routes beside `kit_workflow`, reusing the guarded write helpers of `writes.rs` and the data-folder lookup the store uses (`EASY_PERSONAL_HOME`, else `~/.easy-personal`); write = `OpenOptions` mode 0600 `create_new` temp in the same folder → `write_all` → `sync_all` → `rename`; a `static Mutex<()>` around read-modify-write.
- **Step B (frontend)**: `web/src/FlowEditor.jsx` (+ test), opened from `WorkflowView.jsx`; writes through the page's existing `api.js` write helper (token header, never URL). Repo list from the route the Workspaces/Settings page already uses for stored repos.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/core/src/workflow.rs` | own flows: read, validate, merge |
| `rust/crates/app/src/api.rs`, `rust/crates/app/src/api_tests.rs` | three write routes, GET with own flows + tests |
| `rust/parity/leak_rust.py` | register the new routes (the script refuses unregistered routes; T108 decision 8) |
| `web/src/FlowEditor.jsx`, `web/src/FlowEditor.test.jsx` (new) | the editor |
| `web/src/WorkflowView.jsx`, `web/src/WorkflowView.test.jsx` | buttons, own_error line, projects |
| `web/src/test/fixtures.js`, `web/src/app.css` | fixture; form layout only |
| `README.md`, `rust/README.md` | one line each: own flows file and editor |

## Files must NOT touch
| File | Reason |
|------|--------|
| `src/easy_personal/**`, `tests/**`, `pyproject.toml` | the Python app |
| `rust/crates/core/src/board.rs`, `rust/crates/core/src/live.rs` | not needed |

## Edge Case Checklist
- `flows.json` is a symlink: refuse to read or write through it (`symlink_metadata`), `own_error`
- The data folder does not exist yet: the first write creates it (0700), as the store does
- Two flows with the same id in the file: invalid
- A repo assigned to a flow then the flow deleted: its `projects` entry goes too
- An own flow with no gates and one step: valid
- Very long comma lists in the form: the server limits answer 400 with the reason

## Questions Log
- 2026-11-17 where own flows live → one file in the app's folder **and** an in-app editor (owner: "we should add to the app also")

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes (n/a: web/ has no linter)
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `docs/` updated
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
