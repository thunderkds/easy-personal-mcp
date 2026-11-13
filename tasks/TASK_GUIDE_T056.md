# TASK_GUIDE — T056: Rust live feed, part 1: polling thread, `GET /api/live` (SSE), `Last-Event-ID`, token in the query for the live route
**Date**: 2026-11-13
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the entries "Rust live feed: Path 1" and "Rust P3 planned" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read the Rust you build on: `rust/README.md` section "Live tail engine (T049)" (the engine API and the "decided for you" list), `rust/crates/core/src/live.rs` (public API only: `LiveTail`, `Root`, `account_roots`), `rust/crates/app/src/{server,api,static_files,main,window}.rs`, `rust/crates/app/tests/` (how a server test starts `serve` and calls it)
6. Read the Python you copy, **read-only**: `src/easy_personal/surfaces/api.py` (`Guard`, `_query_token`, `live_stream`, `_sse`, `_snapshot_data`, `_EMPTY_SNAPSHOT`, `_resume_id`, `_SESSION_KEYS`, `_EVENT_KEYS`, `PING_S`, `POLL_S`), `src/easy_personal/core/live.py` (`start`, `stop`, `_run`, `POLL_SECONDS`), `src/easy_personal/surfaces/cli.py` lines near 392 (how the dashboard starts the tail)
7. Read `web/src/LiveView.jsx` (how the page consumes the stream) so the Rust stream feeds it unchanged
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use temp folders, the committed fixture store `rust/parity/store-rich/` in a temp copy, and fixture transcripts you create in temp folders

---

## Requirement

**Original request** (owner, 2026-11-13): "the live from the window application does not show anything" then "run 56 and 57 now". Direction locked 2026-11-12: Path 1 (the feed runs inside the `serve` / `app` process only while it is open), publish-only tail (`memory/decisions.md`).

**Restated intent**:
> `easy-personal-rs serve` and `easy-personal-rs app` answer `GET /api/live` as Python does: a Server-Sent Events stream with a `snapshot` message and then one `live` message per new event, fed by the T049 engine polled by a thread that runs only while the server runs. The dashboard's Live tab shows the steps Claude is running. The tail never writes the store.

**Out of scope**: the kit-trace feed and any change to the `app` command beyond starting the tail (T057), the `config.yaml` thresholds, an always-on collector, any write to the store, any change to the web page, Python files.

**Requirement Refs**: FR-043..FR-049, FR-071, FR-081..FR-083, FR-086 (Python live tail, `PRD.md`); RS-001, RS-003 and NFR-001/NFR-002 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T049 (engine, on `main`)
**Entry point**: `GET /api/live` (`rust/crates/app/src/api.rs`, `server.rs`); the thread starts in the `serve` command and the `app` command (`main.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Poll thread.** A `LiveFeed` (app crate) owns one `LiveTail` behind a lock and one thread that calls `poll_store` every 0.1 s with the clock from `clock.rs`. `serve` and `app` start it before they accept requests and stop it on shutdown (Ctrl-C, window close) within 10 s, joining the thread. No thread, no store open and no file read happens in a build or command that does not serve (`store-info`, `mcp`, `ingest`). The thread survives a poll that fails (store missing, store not v11, unreadable root): the step is skipped, nothing is printed that names a path | FR-043 |
| 2 | **Stream.** `GET /api/live` answers `200`, `content-type: text/event-stream`, `cache-control: no-store` (and `x-accel-buffering: no`; Python sends these, corrected 2026-11-13 after Stage 4), and the same message order as Python's `live_stream`: first a `snapshot` (unless `Last-Event-ID` names an id the buffer still reaches), then `live` events with `id:` lines, a fresh `snapshot` when the attention state changes (events up to its `last_id` first), and a `: ping` comment every 15 s. The JSON keys are Python's `_SESSION_KEYS`, `_EVENT_KEYS` and snapshot keys and nothing else (no cache counts, no text). With no tail data the snapshot is empty (`sessions: []`, `last_id` 0) | FR-044, FR-045 |
| 3 | **Resume.** `Last-Event-ID` as a non-negative integer resumes after that id; an id above the tail's last id, an id older than the buffer, absent or not a number starts with a snapshot, exactly as Python (`_resume_id`, `rolled_past`) | FR-046 |
| 4 | **Guard.** `GET /api/live` needs the token like every `/api/` route, and **only this route, only for `GET`**, also accepts it as the `token` query parameter (constant-time compare). Host and Origin checks stay. A token in the query on any other route, or on `HEAD`/`POST` to the live route, is refused `403`. The token never appears in any output, log or error text; a server access log is not added | NFR-001, DDR-0004 |
| 5 | **Shutdown and disconnect.** An open stream ends when the client disconnects and when the server stops, so Ctrl-C and closing the window never wait on a stream. Ten streams open at once all get events; a stream that never reads does not block the poll thread or the other streams (bounded per-stream buffering, the stream ends rather than grows) | FR-047 |
| 6 | **Publish-only.** The tail never creates or takes `ingest.lock`, never writes the store (bytes and mtime of `store.sqlite` unchanged after a run; test it with a fixture store and the scheduled ingest running at the same time), and every event carries names, ids, numbers and timestamps only (DDR-0004) | decisions 2026-11-12 |
| 7 | **Oracle.** A fixture transcript tree in a temp folder is fed to the Rust server and to the Python app's own `live_stream` (a generator `rust/parity/gen_live_stream.py` records the Python stream for each case into `rust/parity/golden-live-stream/`, made once, read-only afterwards). The test compares the message sequence (event names, `id`, data JSON) for: a new session, a tool call and its result, an error streak, a sub-agent file, a resume with a good id, a resume with a stale id, and an empty tail. The Python-only differences are listed in "decided for you" | FR-044..FR-049 |
| 8 | **Leak and regression.** `parity/leak_rust.py` registers `/api/live` (a bounded read of the stream: first `snapshot`, then close) and scans the stream for the canaries: 0 hits. Three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml` and `Cargo.lock` unchanged unless the stream needs a dependency: then stop and report which one and why before adding it. Existing tests and goldens still pass | RS-003, NFR-002 |
| 9 | **Docs.** `rust/README.md`: the route, the SSE message shapes, the token-in-query rule, the thread's lifetime, the "decided for you" list, and the corrected "not served yet" lists (`/api/live` is now served; the kit-trace feed and the `app` wiring stay in T057). `rust/PROJECT_*.md` and `memory/**` are not edited (Supervisor) | — |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T056.md`.
> **Locked oracle**: `golden-live-stream/` made once by Python, plus the existing `golden-live/` for the engine.

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | each oracle case | the Rust message sequence equals Python's | the parity test |
| 2 | `serve --static-dir` over a fixture transcript tree, then a new transcript line is appended | the line arrives as a `live` event within 2 s | the HTTP test |
| 3 | no token / bad Host / `?token=` on another route / `POST /api/live` | 403 / 403 / 403 / 403 or 405 as the other routes | server test |
| 4 | Ctrl-C (a `SIGINT` to the child) with two streams open | the process exits within 10 s, code 0 | process test |
| 5 | `git status -- src tests web` and `Cargo.lock` | no change | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

**Reviewer note**: High risk (the first long-lived stream and the first token in a URL). Stage 4 includes the Supervisor's line-by-line read of the guard change and the thread's shutdown, and an independent security review.

---

## Approach

**Pattern reference**: how `server.rs` runs the guard and routes, and how the T049 tests build fixture transcript trees.
**Vital slice**: (1) the `LiveFeed` thread and a unit test with a fake clock; (2) the stream function that turns `events_since` and `snapshot` into SSE bytes, tested without a socket; (3) the route and the guard carve-out; (4) shutdown; (5) the oracle; (6) leak registry and docs.
**Cut list**: no compression, no `Range`, no new endpoint besides `/api/live`, no tail state saved to disk, no store write, no config file.
**Order**: red test first for each step.

---

## Edge Case Checklist

- [ ] The tail thread starts when the store does not exist (fresh machine): empty snapshot, keep-alives, and it picks data up when the store appears
- [ ] A transcript file that shrinks, is deleted or is a symlink out of the root: the engine's rules (T049) apply; the thread never panics (a panic in a poll step is caught, the step is skipped, the thread continues; test it with an injected failure)
- [ ] `Last-Event-ID` with a huge number, a negative number, spaces, `0`, a repeated header (refuse as the other routes do for repeated headers)
- [ ] A client that reads slowly (the stream's buffer is bounded; the stream ends, the thread is not blocked)
- [ ] 1,000 events in one poll step: the stream sends all in order, ids strictly increasing, none twice
- [ ] The window opens the page while the stream is open: one origin, same guard
- [ ] Shutdown while a poll step is reading a 32 MiB line: stop within 10 s (the engine's bounds apply; if a step can run longer, say so)
- [ ] The token in the query is not echoed in any error, any `Location`, any log line, and not stored in `Referer`-visible places (the stream sends no redirect)
- [ ] Time derived from "now" uses the injected clock; three `TZ` values
- [ ] `leak_rust.py` and `tests/mcp.rs` lists updated by a few lines only

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/live_feed.rs` (new) | `LiveFeed`: the thread, the lock, start and stop |
| `rust/crates/app/src/{server,api,main}.rs` | the route, the guard carve-out (GET and `/api/live` only), starting and stopping the feed in `serve` and `app` |
| `rust/crates/app/tests/live.rs` (new), `rust/crates/app/src/api_tests.rs` (extend) | HTTP, resume, guard, shutdown, many streams |
| `rust/parity/gen_live_stream.py` and `rust/parity/golden-live-stream/` (new) | the Python stream oracle |
| `rust/parity/leak_rust.py` | register `/api/live`: a few lines |
| `rust/README.md` | section for the feed, routes, "decided for you", corrected "not served yet" lists |
| `tasks/TASK_REVIEW_T056.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/crates/core/src/live.rs` | T049's engine; if it needs a change (an extra accessor), make the smallest one, say so in the report, and keep its 15 parity tests green |
| existing `rust/parity/golden*/**`, `gen_golden.py`, `store-rich/**`, `fixtures-rich/**` | the oracle (if a golden looks wrong, stop and report) |
| `rust/Cargo.toml`, `rust/Cargo.lock` | no new dependency without a stop-and-report |
| T057's files and scope: the kit-trace feed, `app` wiring beyond starting the feed | another task owns them |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Unit tests (clock, stream bytes), then HTTP tests, then the oracle, then the leak gate and three `TZ` runs (`unset`, `Pacific/Kiritimati`, `Pacific/Pago_Pago`: `cargo test --workspace --no-fail-fast`).

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T056.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review
