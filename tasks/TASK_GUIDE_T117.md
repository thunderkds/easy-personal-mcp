# TASK_GUIDE — T117: Live like Claude 3 — past sessions and own names
**Date**: 2026-11-18
**Complexity Level**: C2
**Risk Level**: Medium (read routes over the store; one owner-only file)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` question 45, US-043, FR-126, FR-127, FR-112, FR-119 (owner-only file pattern), NFR-002 (fifth exception), NFR-008
2. `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVENATIVE.md` (Edge Case Checklist: all of it applies)
3. `rust/crates/core/src/sessions.rs`, `store.rs`, `schema_v11.sql` (`events`), `rust/crates/app/src/` flows file code of T109 (`flows.json` write: temp + rename, owner-only)
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-18: "should check the live, cause i need the work like claude native, where user can input, choose the option, trace the logs, and manage the sections, [admin], supervisor". Owner answers the same day: full transcript (shown, never stored); queue until the turn end for a busy terminal session; one place for all sessions; stop a terminal session; past sessions + resume; filters and names. This task is the server half of *past sessions*, *filters* and *names*.

## Dependencies & Reachability
**Depends on**: none
**Entry point**: `GET /api/sessions/past`, `PUT /api/sessions/{id}/name`

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `GET /api/sessions/past?project=&branch=&from=&to=&kind=&account=&before=` lists sessions from `events` (no text): `{id, project, branch, first, last, kind (terminal/app/headless from entrypoint), account, model, name}` newest `last` first, 50 a page, `before` = cursor; one SQL query with an index check (EXPLAIN in a test: no full scan on a 100k-row fixture beyond the ts index) | FR-126 |
| 2 | Bad filter values → 400 one line; dates ISO only | FR-126 |
| 3 | `PUT /api/sessions/{id}/name {name, confirmed: true}`: 1..80 chars after FR-081 cleaning, `null` removes; `session_names.json` owner-only (0600), temp + rename, ≤ 2,000 names (oldest dropped); a missing or invalid file = no names, never a 500 | FR-127 |
| 4 | `name` appears in the past list and in the page's `/api/live` snapshot (`name` beside `proc.name`) | FR-127 |
| 5 | NFR-008 guard + `confirmed` tests; routes in `API_CALLS`; leak gate: names never reach logs, MCP, CLI, tray, popups | FR-126, FR-127 |
| 6 | `cargo fmt --check`, clippy (both), `cargo test --workspace`, leak 0 hits | all |

## Approach
A query function in `core/src/sessions.rs`; a small names module beside the flows file code.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/core/src/sessions.rs` | past sessions query |
| `rust/crates/app/src/session_names.rs` | new |
| `rust/crates/app/src/api.rs` | routes |
| `rust/crates/app/src/live_feed.rs` | `name` in the page snapshot |
| `rust/parity/leak_rust.py` | routes |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |
| store schema (`rust/crates/core/src/schema_v11.sql`) | no schema change in this plan |
| `web/**` | T118/T119 |
| `src/easy_personal/**` | Python app |

## Test plan
Query tests on a temp store; names file tests in a temp home.

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -5 && python3 parity/leak_rust.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: name a session and see it in Live)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
