# TASK_GUIDE — T124: Live like Claude 2a — sub-agent logs (server)
**Date**: 2026-11-19
**Complexity Level**: C2
**Risk Level**: High (widens what the transcript route reads: sub-agent files, NFR-002 fifth exception)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` question 46, US-044, FR-129, FR-123, FR-082, NFR-002 (fifth exception, 2026-11-19 sentence), NFR-008
2. `docs/ddr/0008-live-subagent-logs-and-steering.md`, `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVE2.md` (Edge Case Checklist: all of it applies)
3. `rust/crates/core/src/transcript.rs` (`ops` drops `isSidechain` at ~226; `read` opens via `live::open_regular`), `rust/crates/app/src/transcript_route.rs` (id check, live/past resolution, root check), `rust/crates/core/src/live.rs` (`session_origin` ~1037, `meta_agent_type` ~622, open rules at the top), `rust/parity/leak_rust.py`
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-19: "yes, I want from the live, I can do all the things, prompt, choose the option, start new session, trace the agents logs include sub-agents, ..., let investigate and update all" [admin]. This task is the server half of *trace the agents logs include sub-agents*.

## Dependencies & Reachability
**Depends on**: none (builds on T115, merged)
**Entry point**: `GET /api/live/sessions/{id}/agents` and `GET /api/live/sessions/{id}/transcript?agent=<id>`

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `?agent=<id>` reads `<main file stem>/subagents/agent-<id>.jsonl` of that session (main file found as today: live tail or store), with the same answer shape, caps (16,000 chars a block, 500 entries, 2 MB) and `after`/`before` paging; its `isSidechain` records are kept; the main file still drops them (T115 goldens unchanged) | FR-129 |
| 2 | Agent id must match `^[A-Za-z0-9]{1,64}$`; a symlinked, non-regular, gone or outside-root file, a `..`/`/` id, or an id with no file → 404 with one line, never a 500 (hostile-file tests as T115) | FR-129 |
| 3 | `GET .../agents` lists the session's `subagents/agent-*.jsonl` (newest 200 by mtime, `total`): `{id, type, parent_call, depth, finished, size, modified}`; meta read for `agentType` (FR-081 pattern, else "sub-agent"), `toolUseId` (`^[A-Za-z0-9_]{1,128}$`, else null), `spawnDepth` (int), `finished` (bool) only — a test proves `description` is never in the answer | FR-129, FR-082 |
| 4 | Works for a past session (store path) and a live one; a session with no `subagents/` folder → `{agents: [], total: 0}` | FR-129 |
| 5 | Golden: a sub-agent fixture (records with `isSidechain: true`, `agentId`, tool_use/tool_result, a nested `Agent` call) gives the expected entries | FR-129 |
| 6 | Leak gate: planted markers in a sub-agent file and its meta `description` reach only the transcript answer (description: nowhere); both routes in `API_CALLS`; NFR-008 guard tests (403 without token, bad Host/Origin) | NFR-002, NFR-008 |
| 7 | Nothing cached; the live tail's state is not changed (NFR-012) | NFR-012 |
| 8 | `cargo fmt --check`, clippy `-D warnings` (with and without `window`), `cargo test --workspace`, `python3 parity/leak_rust.py` 0 hits; `rust/README.md` Live section names the two routes | all |

## Approach
Add a `sidechain: bool` (keep sidechain records) to `transcript::read`/`parse`; in `transcript_route.rs` resolve the main file as today, then join only `subagents/agent-<id>.jsonl` under its stem folder and re-run the root check on the canonical parent. Listing: read the folder with `read_dir`, no recursion, meta parsed with a size cap (64 KB) and only the four fields.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/core/src/transcript.rs` | sidechain flag + golden |
| `rust/crates/app/src/transcript_route.rs` | `agent=` param, listing handler |
| `rust/crates/app/src/api.rs` | route |
| `rust/parity/leak_rust.py` | markers, routes |
| `rust/README.md` | Live section |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |
| store schema (`rust/crates/core/src/schema_v11.sql`) | no schema change in this plan |
| `src/easy_personal/**` | Python app |
| `web/**` | T125 |
| `rust/crates/core/src/live.rs` state | NFR-012; read-only helpers only |

## Test plan
Parser goldens, route tests with temp folders (live + past + hostile), leak gate.

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -5 && python3 parity/leak_rust.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: open a session that ran sub-agents and read a sub-agent's log)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
