# TASK_GUIDE — T116: Live like Claude 2 — session process files, Stop a terminal session, the queue
**Date**: 2026-11-18
**Complexity Level**: C3
**Risk Level**: High (the app signals a process it did not start; `hook ask` on every Stop)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` question 45, US-043, FR-124, FR-125, FR-121, FR-113, FR-105, NFR-002 (fifth exception), NFR-008
2. `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVENATIVE.md` (Edge Case Checklist: all of it applies)
3. `rust/crates/app/src/answer_bridge.rs`, `hooks_cmd.rs` (Stop handling of T111), `live_feed.rs` (`pending_reply`, `reply_bridge`), `rust/crates/core/src/accounts.rs` (config dirs), `rust/crates/app/src/app_session.rs` (process group stop, for contrast)
4. A real `~/.claude*/sessions/<pid>.json` may be **listed and its JSON keys read** to write a fixture (values replaced); never copy one into the repo, never open `*.key`
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-18: "should check the live, cause i need the work like claude native, where user can input, choose the option, trace the logs, and manage the sections, [admin], supervisor". Owner answers the same day: full transcript (shown, never stored); queue until the turn end for a busy terminal session; one place for all sessions; stop a terminal session; past sessions + resume; filters and names. This task is the server half of *stop* and *input while busy*.

## Dependencies & Reachability
**Depends on**: none (builds on T111, merged)
**Entry point**: `/api/live` snapshot (`proc`, `queued`), `POST /api/live/sessions/{id}/stop`, `POST /api/reply/queue`, `easy-personal-rs hook ask` (Stop input)

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Reader of `<config>/sessions/*.json` for each Claude account (every field optional, files > 64 KB or not JSON skipped, the hostile-file open rules); only `pid, sessionId, procStart, status, name, kind, entrypoint` kept in memory; refreshed at most every 2 s on the existing live loop (NFR-012: no new thread); never opens another file of the folder | FR-124 |
| 2 | Snapshot: a live session with a process file gets `proc: {busy, name}` in the page's `/api/live` snapshot only (never `LiveTail::snapshot()`, DDR-0005) | FR-124 |
| 3 | `POST /api/live/sessions/{id}/stop {confirmed: true}` → SIGTERM only when `/proc/<pid>` is owned by our uid, its exe file name or argv[0] base name is `claude` (or a `node`/`bun` running a `claude` script path), and its start time (field 22 of `/proc/<pid>/stat` → wall clock) matches `procStart` ±2 s; else 409 one line; no process file → 404; an app session → 409 (use FR-110); test with a spawned dummy process named `claude` in a temp dir | FR-124 |
| 4 | `POST /api/reply/queue {session_id, text, confirmed: true}` for a listed terminal session of a `reply_bridge` account → 200; `text: null` cancels; 1..8,000 chars else 400; not bridge-enabled or not listed → 409/404; one text per session (replace) | FR-125 |
| 5 | `hook ask` on a Stop with `stop_hook_active` false and a queued text for that session prints exactly `{"decision":"block","reason":"<text>"}` at once, without a Live stream and without a hold; the queue entry is dropped only after the socket write succeeded (Delivery + ack as T111); with no queue the T111 behaviour is unchanged (all T111 tests pass) | FR-125, FR-121 |
| 6 | Snapshot `queued: {text}` on the session; queue dropped at app exit and when the session leaves the live list | FR-125 |
| 7 | Leak gate: queued text and process names reach only the page snapshot and the hook stdout; routes in `API_CALLS`; NFR-008 + `confirmed` 403/400 tests | FR-124, FR-125, NFR-002 |
| 8 | README Live section: Stop of a terminal session (SIGTERM, checks), the queue (needs the bridge's Stop entry, dropped at exit) | FR-124, FR-125 |
| 9 | `cargo fmt --check`, clippy `-D warnings` (with and without `window`), `cargo test --workspace`, leak 0 hits | all |

## Approach
New `rust/crates/app/src/session_procs.rs` (reader + stop checks). Extend the T111 bridge state with `queued: HashMap<session_id, String>`; the Stop path checks it before the hold rule.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/session_procs.rs` | new |
| `rust/crates/app/src/answer_bridge.rs` | queue + Stop delivery |
| `rust/crates/app/src/live_feed.rs` | `proc`, `queued` in the page snapshot |
| `rust/crates/app/src/api.rs` | routes |
| `rust/parity/leak_rust.py` | routes, markers |
| `rust/README.md` | Live section |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |
| store schema (`rust/crates/core/src/schema_v11.sql`) | no schema change in this plan |
| `web/**` | T118/T119 |
| `src/easy_personal/**` | Python app |

## Test plan
Unit tests with fixture process files in a temp config dir; a spawned dummy `claude` process (a copy of `sleep` named `claude` in a temp dir) for Stop; T111 hook tests with a queue.

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -5 && python3 parity/leak_rust.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: queue a message to a busy terminal session and see it arrive at its turn end; stop a scratch terminal session)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
