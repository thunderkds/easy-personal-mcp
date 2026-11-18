# TASK_GUIDE — T111: Reply to idle sessions 1 — `hook ask` on `Stop`, the hold, `/api/reply*`, `pending_reply`
**Date**: 2026-11-18
**Complexity Level**: C3
**Risk Level**: High (a hook at the end of every turn of every bridge-enabled terminal session; writes the owner's user settings file)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `docs/ddr/0006-write-sessions-cli-parity.md`, `docs/ddr/0001-*` (why `~/.claude.json` is never edited)
2. `PRD.md` FR-121, FR-122, FR-113, FR-114, FR-115, FR-116, FR-105, question 42
3. `tasks/evidence-T110/SPIKE.md` (all: the Stop input keys, the block output, `stop_hook_active`, `isMeta` transcript record, timeout and Esc behaviour)
4. `rust/crates/app/src/answer_bridge.rs` (all: socket, `hook ask`, install/remove/status, the hold and release, key per prompt), `rust/crates/app/src/live_feed.rs` (`pending_answer`), `rust/crates/core/src/live.rs` (your-turn state, the transcript parser), `rust/crates/app/src/api.rs` (`/api/answer*`, `/api/hooks`), `rust/parity/leak_rust.py` (`BRIDGE_PATTERNS`, `API_CALLS`)
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`.

## Requirement
Owner 2026-11-18: "if the agent is idle, I can input the request to the live session also, can we do that" → "plan it and update [admin]"; hold rule chosen by the owner: **hybrid** (PRD question 42, FR-121). This task is the server half; T112 is the Live view.

## Dependencies & Reachability
**Depends on**: T110 (spike GO). Builds on T106 (merged).
**Entry point**: `easy-personal-rs hook ask` (Stop input), `easy-personal-rs hooks install|status`, `/api/reply`, `/api/reply/release`, `/api/reply/watch`, `/api/reply/mark`, `pending_reply` in `GET /api/live`

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `hooks install` adds a third entry `hooks.Stop` (no matcher, `<abs binary> hook ask`, timeout 660) next to the T106 two, with the same add-only, backup, re-read-before-rename rules; an install that has only the T106 two reads `on (no reply)` in `status` and `GET /api/hooks`, and install adds the third; `remove` drops all three when they are ours; a hand-edited Stop entry → `changed by hand` and kept | FR-113, FR-121 |
| 2 | `hook ask` with a Stop input exits 0, empty stdout, without contacting the app beyond one socket check, when `stop_hook_active` is true, when there is no socket, no Live stream, the session is not in the live list after the 1 s grace, or the session is an app session / `entrypoint` starting `sdk`; timed test: median and p90 < 50 ms with no app (20 runs, as T106 AC 4) | FR-121 |
| 3 | With the app and a Live stream: the session gets `pending_reply {id, until, marked, last}` in the `/api/live` snapshot (`last` cleaned as FR-114 card text, cut at 4,000 chars); with no watch and no mark the hold ends at 90 s with exit 0, empty stdout (test with a shortened clock) | FR-121 |
| 4 | `POST /api/reply/watch {session_id}` keeps the hold at least 60 s past the call; with calls every 20 s the hold lasts until 10 min after the hold began at most; without a mark the watch alone also stops at 10 min | FR-121 |
| 5 | `POST /api/reply/mark {session_id, on}` (in memory) makes the session's next hold last up to 10 min without watch calls; unmark returns to the 90 s rule; marks are dropped at app exit; a mark on a session not in the live list is a 404 | FR-121 |
| 6 | `POST /api/reply {id, text, confirmed: true}`: `hook ask` prints exactly `{"decision":"block","reason":"<text>"}` (text as typed, line breaks kept, leading `/` kept); empty or > 8,000 chars → 400; unknown or ended id → 409; a second reply → 409; `POST /api/reply/release {id}` → exit 0, empty stdout | FR-121 |
| 7 | The hold ends with exit 0 and empty stdout 2 s after the last Live stream closes, at app exit, and when the hook process is killed (Esc in the terminal: the app drops the item, no stale `pending_reply`) | FR-121 |
| 8 | Two sessions holding at once never cross (keyed by session id + random id); a Stop of a session that already has a waiting `pending_answer` (FR-114) is not held | FR-121, FR-114 |
| 9 | The transcript parser skips `isMeta` user records (`Stop hook feedback:`): a your-turn session that got a reply is counted with the owner's prompts unchanged (golden from the T110 transcript shape, text replaced) | FR-121, FR-105 |
| 10 | Leak gate: reply text and `last` reach `GET /api/live` (and `hook ask` stdout for the reply) only; never the store, logs, `/api/alerts`, tray, popups, MCP, CLI; new routes registered in `API_CALLS` / guard calls; NFR-008 guard + `confirmed: true` on every write route (403/400 tests) | FR-115, FR-116 |
| 11 | README: the bridge section says what the Stop entry does, the hold rule (90 s / looking / marked 10 min), that Esc in the terminal ends a hold, and that sessions started before the install are not covered | FR-121 |
| 12 | `cargo fmt --check`, clippy `-D warnings` (with and without `window`), `cargo test --workspace`, `python3 parity/leak_rust.py` 0 hits | all |

## Approach
Extend `answer_bridge.rs` (one bridge, one socket): a Stop request kind next to the question/permission kinds, a hold with a deadline the routes can extend (watch, mark) and end (reply, release, stream gone, app exit). Reuse the T106 key, socket and cleaning code. `pending_reply` is added to the snapshot the same way `pending_answer` is. Keep `stop_hook_active` the first check in `hook ask`.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/answer_bridge.rs` | Stop kind, hold rule, install third entry, status `on (no reply)` |
| `rust/crates/app/src/live_feed.rs` | `pending_reply` in the snapshot |
| `rust/crates/app/src/api.rs` (+ routes file if split) | `/api/reply`, `/release`, `/watch`, `/mark` |
| `rust/crates/core/src/live.rs` | skip `isMeta` user records |
| `rust/parity/leak_rust.py` | reply strings and routes |
| `rust/README.md` | bridge section |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file | DDR-0001; tests use temp dirs |
| project `.claude/settings.json` of any repo | kernel-owned |
| `web/**` | T112 |
| `src/easy_personal/**` | Python app |

## Test plan
Unit and route tests per AC with a shortened hold clock; the T106 test helpers for a fake app socket and live list; leak gate.

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -5 && python3 parity/leak_rust.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (a real terminal session on a bridge-enabled account gets a reply from the app; owner)
- [ ] `docs/` updated (README)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
