# PROJECT_KANBAN.md — Rust port (`rust/`)
**Last updated**: 2026-11-11

> Compact task board for the **Rust port** only, kept in `rust/` (Gate 4: a separate board file; the repo-root `PROJECT_KANBAN.md` is the Python app's Linear cache and is overwritten on every pull, so Rust tasks never go there). Guides live in the repo-root `tasks/` so the spawn hook finds them. Full context: `rust/PROJECT_SPEC.md`.
> Task-to-task preconditions live in each guide's `Depends on:` field.

---

## Board

### Todo

### In Progress
- [ ] **T024** — A Rust step in the `/verify` recipe (`skills/verify/SKILL.md`, additions only; every command run and pasted) | Common-Infrastructure | C1 | Risk: Low | P2 | guide `tasks/TASK_GUIDE_T024.md`; Started: 2026-11-11; Depends on T023

### Ready for Review
- [ ] **T022** — Core `workspaces::list_all` and `unassigned` and their two GET routes (4 goldens; writes stay 405) | Backend-Implementer | C2 | Risk: Medium | P1 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence `tasks/TASK_REVIEW_T022.md`, reports `reports/code-review_main_20261110T231200.html`, `reports/security-review_main_20261110T231300.html`; open: README heading 'no CLI/API yet' is stale; shared spawn-test helpers duplicated in `tests/workspaces.rs`
- [ ] **T021** — Serve the dashboard's static files from `serve --static-dir <DIR>` (same origin, no token for non-`/api/` paths, CSP on all, path-safety table) | Backend-Implementer | C3 | Risk: High | P0 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence `tasks/TASK_REVIEW_T021.md`, reports `reports/code-review_main_20261110T231000.html`, `reports/security-review_main_20261110T231100.html`; differs from Python: dotfiles are 404; not tried against the real built dashboard (not built: needs `npm ci`)
- [ ] **T020** — Serve `/api/usage/sessions`, `/api/usage/outliers` and the MCP `session_outliers` tool (six routes, six tools; 20 HTTP + 3 MCP goldens equal) | Backend-Implementer | C2 | Risk: Medium | P1 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence `tasks/TASK_REVIEW_T020.md`, reports `reports/code-review_main_20261110T221000.html`, `reports/security-review_main_20261110T221100.html`
- [ ] **T019** — Wire `workspace` and `account` into the four HTTP routes and four MCP tools (501 and tool error gone; TZ pinned in the MCP spawn tests) | Backend-Implementer | C3 | Risk: Medium | P0 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence `tasks/TASK_REVIEW_T019.md`, reports `reports/code-review_main_20261110T211200.html`, `reports/security-review_main_20261110T211300.html`; open (owner): Python's MCP `usage_summary`/`usage_breakdown` ignore the scope, ours honour it
- [ ] **T018** — Core `session_outliers` and `session_timeline` with scope and `detail_sessions` | Backend-Implementer | C3 | Risk: Medium | P1 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence `tasks/TASK_REVIEW_T018.md`, reports `reports/code-review_main_20261110T211000.html`, `reports/security-review_main_20261110T211100.html`; 23 goldens equal (5 default + 18 rich); wired in T020
- [ ] **T017** — Scope in the core: `workspace` and `account` for usage summary, breakdown, warnings and plan | Backend-Implementer | C3 | Risk: Medium | P0 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence `tasks/TASK_REVIEW_T017.md`, reports `reports/code-review_main_20261110T203000.html`, `reports/security-review_main_20261110T204000.html`; 41 scoped goldens equal; open: 2 app tests (T015) fail under other TZ, fixed in T019
- [ ] **T016** — Window spike: Tauri v2 under WSLg opens a window, loads a loopback page, token fragment and header work under the dashboard CSP (throwaway, `rust/spikes/t016-window/`) | Common-Infrastructure | C2 | Risk: Low | P1 | PR/branch: none (throwaway spike; evidence in `tasks/TASK_REVIEW_T016.md`, findings `rust/docs/spike-T016.md`); open: a dead `DISPLAY` hangs in GTK init (P1b needs its own timeout); not looked at by eye; the spike's `target/` is 3.7 GB (gitignored)
- [ ] **T015** — P1 wiring: `warnings` and `improvement_plan` tools in the MCP server (hand-written schemas equal to the golden; clock passed to `run_tool`) | Backend-Implementer | C2 | Risk: Medium | P1 | Depends on T013; parallel with T014 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T015.md`; code-review and security-review run 2026-11-10; open: missing store is a tool error (T008 difference); `workspace`/`account` filters are a tool error; not proven against the real Claude Code client
- [ ] **T014** — P1 wiring: `/api/usage/summary`, `/api/usage/breakdown`, `/api/warnings`, `/api/plan` over the guarded HTTP server (501 for `workspace`/`account`, 503 for a missing store) | Backend-Implementer | C3 | Risk: Medium | P0 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T014.md`; code-review and security-review run 2026-11-10; open: missing store is 503 (Python creates an empty store); the `workspace`/`account` filters answer 501; `/api/usage/sessions` and `/outliers` are not ported (404); whole suite re-run with T015: 157 passed, 0 failed
- [ ] **T013** — P1 wiring: `store::open_checked` and `clock::now_micros` in the core; `mcp.rs` uses them | Backend-Implementer | C2 | Risk: Low | P0 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T013.md`; code-review run 2026-11-10 (Low risk); open: P3 `now_micros` returns 0 for a pre-1970 clock
- [ ] **T011** — Generator hardening (QA P2/P3 of T003): assert every clock patch landed; list prompts and resources loudly; add a bad-token, `?token=`, 404 and 405 guard case and record header values; delete only what the script wrote under `--out`; refuse `--out` / `--fixtures` equal to a real home; install from `uv.lock`; drop the CPython error text; remove dead code; `ruff format` | Backend-Implementer | C2 | Risk: Low | P2 | Same file as T007: run after it | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T011.md`; code-review run 2026-11-10 (Low risk); open: owner decides whether to rebuild the venv from `uv.lock` (it could change the goldens); `list_resource_templates` and a `/work/rich` check are not covered
- [ ] **T010** — `plan` / `improvement_plan` in the core (after T006) | Backend-Implementer | C3 | Risk: Medium | P2 | Depends on T006, T012; start after T007 closes, so its richer plan golden exists | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T010.md`; code-review run 2026-11-10 (Low risk: no security review); open: stale comment in `rules.rs` line 9 ("the plan answer" not ported); `workspace` / `account` filters not ported (12 scoped plan goldens uncompared)
- [ ] **T007** — Richer oracle (QA P1s of T003): a second, invented store with >14 days of history, a huge session, tool errors, costed models, an agent-root with two accounts, workspaces, reports and a kit project; goldens for the `workspace` / `account` / report-folder parameters; add a `raw_sha256` per entry (or state that key order is not compared) | Backend-Implementer | C3 | Risk: Medium | P1 | Blocks calling warnings, plan, kit, board or accounts "parity" | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T007.md`; code-review and security-review run 2026-11-10; open: `context_index_stale` never fires in the rich oracle (36 synthetic cases cover it); `project_dominates` fires between Scratch and (unknown); kit, board and accounts goldens hold `<WORK>` tokens
- [ ] **T012** — Remove the ~330 lines `rules.rs` copied from `metrics.rs` (`parse_iso`, `project_map`, `run_git`, `folder`): make them `pub(crate)` and import them; keep the 10 s / 5 s git limits as parameters | Backend-Implementer | C2 | Risk: Low | P2 | Start after T009 lands (both edit `metrics.rs`); also pin T005's `breakdown_by_day_matches_golden` to `TZ=UTC` in a child process (it fails under Pacific/Kiritimati +14) | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T012.md`; code-review run 2026-11-10 (Low risk: no security review); open: optional P3 split of `metrics.rs` (about 1,020 lines) and `rules.rs` (about 1,640)
- [ ] **T009** — Local-timezone parity: Python groups `by=day` and reads offset-less `since`/`until` in the OS local zone (DST-aware); the Rust core reads UTC. Generator emits goldens under TZ=Asia/Bangkok and a DST zone; the Rust core follows the OS zone | Backend-Implementer | C3 | Risk: Medium | P1 | Blocks flipping any usage surface; T003 review closed and T005 fixes landed: guide `tasks/TASK_GUIDE_T009.md`; Started: 2026-11-10 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T009.md`; code-review and security-review run 2026-11-10; open: owner to confirm the `usage_parity.rs` edit; the usage gate from T005 is lifted
- [ ] **T006** — P1 slice: rule pack + `warnings` in the core, parity with goldens (the `plan` answer is T010) | Backend-Implementer | C3 | Risk: Medium | P1 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T006.md`; code-review and security-review run 2026-11-10; open: T012 (de-duplicate the 330 copied lines of `metrics.rs`)
- [ ] **T008** — P1 slice: MCP stdio server (`rmcp` =3.9.0) with `ping`, `usage_summary`, `usage_breakdown` | Backend-Implementer | C3 | Risk: Medium | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T008.md`; code-review and security-review run 2026-11-10; open: owner to confirm missing-store and unknown-tool behaviour differ from Python; T009 before any usage surface is wired
- [ ] **T005** — P1 slice: price parser + `usage_summary` / `usage_breakdown` in the core, parity with goldens | Backend-Implementer | C3 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T005.md`; code-review and security-review run 2026-11-10; open: T009 (local timezone) before any usage surface is wired
- [ ] **T004** — P1 slice: loopback HTTP server, Host/Origin/token guard, `/api/version` | Backend-Implementer | C2 | Risk: High | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T004.md`; code-review and security-review run 2026-11-10
- [ ] **T002** — P0 foundation: Cargo workspace, CLI `--version`, read-only store open + `user_version` guard | Backend-Implementer | C2 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T002.md`; code-review and security-review run 2026-11-10
- [ ] **T001** — Environment and crate spike | Common-Infrastructure | C2 | PR/branch: none (no product code; evidence in `tasks/TASK_REVIEW_T001.md`, `rust/docs/spike-T001.md`)

### Done

- [x] **T023** — The window: `easy-personal-rs app --static-dir DIR` behind a `window` Cargo feature (Tauri v2, off by default), display preflight with a timeout, navigation lock, `--self-test` | Common-Infrastructure | C3 | Risk: High | P0 | committed to `main` 2026-11-11 (c26d7d4, 2dec625); Stage 4 done (code-review 0 P0/P1, security-review clean), owner decisions made 2026-11-11 (default binary for CLI/MCP, MPL-2.0 accepted, BEFORE accepted); guide `tasks/TASK_GUIDE_T023.md`; evidence `tasks/TASK_REVIEW_T023.md`, reports `reports/code-review_main_20261111T104341.html`, `reports/security-review_main_20261111T104848.html`, `reports/delivery-report_main_20261111T035424.html`; `/verify` Python recipe passed but has no Rust step (T024); Depends on T016, T021
- [x] **T003** — Parity oracle: 52 golden entries + 18 skipped, store-v11.sqlite | Backend-Implementer | C2 | PR/branch: committed to `main` 2026-11-11 (8d78a3d, 2247fd4, c26d7d4); review pending; evidence in `tasks/TASK_REVIEW_T003.md`; independent QA read closed 2026-11-10 (8 of 8 criteria pass); follow-ups T007, T011
---

## Blocked

| Task | Reason | Waiting on |
|------|--------|-----------|
| ~~Tauri build check (follow-up of T001)~~ RESOLVED 2026-11-10: apt installed by the owner, scratch `cargo check` of `tauri` 2.12.5 passed in 41 s | Ubuntu window libraries need `sudo apt install` — an agent cannot run sudo. Line: `sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev` (checked for Ubuntu 26.04.1; 197 new packages) | owner runs it; needed before P1b, not before T002 |

---

## Phase Plan (from `BRAINSTORMING_LOG_RUSTPORT.md`)

| Phase | Delivers |
|-------|----------|
| P0 | Foundation + `--version` |
| P1 | Read-only core, `/api` GET, MCP read tools, embedded React bundle |
| P1b | Desktop window |
| P2 | Ingest (sources, store writer) |
| P3 | Write surfaces (workspaces, accounts, connectors, sync, keychain, launcher, scheduler, `save_report`) |
| P4 | macOS |
| P5 | Windows |
| P6 | Python sunset |

## Stage Tracker

| Stage | Status |
|-------|--------|
| 0 Define | ✅ Done (brainstorming log; owner approved 2026-11-10) |
| 1 Set up | 🔄 In Progress (T001) |
| 2 Plan | 🔄 In Progress |
| 3 Execute | 🔄 In Progress (nothing In Progress now; T001, T002, T004–T022 in Ready for Review; T003 and T023 Done) |
