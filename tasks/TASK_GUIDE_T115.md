# TASK_GUIDE — T115: Live like Claude 1 — the transcript route
**Date**: 2026-11-18
**Complexity Level**: C3
**Risk Level**: High (fifth exception of NFR-002: tool input and output reach the page)
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` question 45, US-043, FR-123, FR-115, FR-121 (`isMeta`), FR-002, NFR-002 (fifth exception), NFR-008
2. `docs/ddr/0007-live-transcript-show-never-store.md`, `BRAINSTORMING_LOG_LIVENATIVE.md` (Edge Case Checklist: all of it applies)
3. `rust/crates/core/src/live.rs` (hostile-file open rules at the top, `session_origin` ~line 1037, `clean_block`, `call_label`), `rust/crates/app/src/api.rs` (guard, route layout), `rust/crates/app/src/live_feed.rs`, `rust/parity/leak_rust.py`
4. `tasks/evidence-T110/SPIKE.md` (the `isMeta` record shape)
- Tests write only to temp config dirs and a temp `EASY_PERSONAL_HOME`; never the real `~/.claude*` or `~/.easy-personal`. Never open a `*.key` file or a Claude Code messaging socket. Kill only processes your test started.

## Requirement
Owner 2026-11-18: "should check the live, cause i need the work like claude native, where user can input, choose the option, trace the logs, and manage the sections, [admin], supervisor". Owner answers the same day: full transcript (shown, never stored); queue until the turn end for a busy terminal session; one place for all sessions; stop a terminal session; past sessions + resume; filters and names. This task is the server half of *trace the logs*.

## Dependencies & Reachability
**Depends on**: none (builds on T105/T111, merged)
**Entry point**: `GET /api/live/sessions/{id}/transcript?after=<offset>`

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | The route reads the main transcript of a live session (path from `LiveTail::session_origin`, a T105 resume included) or of a stored past session (newest `source_path` of that session id in `events`, main thread only) and answers `{entries, next, complete}`; an id the tail and store do not know → 404 | FR-123 |
| 2 | Entries in file order: `user` (typed text; string or text blocks), `reply` (`isMeta` *Stop hook feedback* record, text after the label), `assistant` (text blocks only; thinking blocks are not shown), `tool` `{id, name, label, input, result, is_error}` (the `tool_result` matched to its `tool_use` by id, even when it comes in a later record; `input` rendered as FR-115 renders a permission prompt's input), `skipped` (one entry with a count per run of other or unknown records) | FR-123 |
| 3 | Each text block cut at 16,000 chars with `cut: true`; one answer ≤ 500 entries and ≤ 2 MB; `after=<offset>` returns entries after that byte offset (the poll), `before=<offset>` the 500 before it (*Load earlier*); `next` is the offset to poll from | FR-123 |
| 4 | Hidden and control characters shown as visible escapes (the FR-115 rule); no redaction | FR-123, FR-115 |
| 5 | A gone, unreadable, symlinked or oversized-line file → 404 with one line, never a 500 (golden tests with the hostile-file cases of `live.rs`) | FR-123 |
| 6 | Goldens: a transcript fixture with every record kind of claude 2.1.327 (text replaced) — user, assistant text + tool_use, tool_result (string and block forms, `is_error`), isMeta, summary, system, unknown type | FR-123 |
| 7 | Leak gate: planted markers in user text, assistant text, tool input and tool output reach only this route's body; never the store, logs, `/api/live`, `/api/alerts`, tray, popups, MCP, CLI; route in `API_CALLS`; NFR-008 guard tests (403 without token, bad Host/Origin) | FR-123, NFR-002 |
| 8 | Nothing cached: two calls read the file twice (test with a changed file) | FR-123 |
| 9 | `cargo fmt --check`, clippy `-D warnings` (with and without `window`), `cargo test --workspace`, `python3 parity/leak_rust.py` 0 hits | all |

## Approach
New `rust/crates/core/src/transcript.rs`: a pure parser `parse(bytes, from, to) -> Page` over lines, reusing `live.rs` cleaning and label code; the app route opens the file with the live tail's open rules and calls it. No change to the live tail's own state.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/core/src/transcript.rs` | new: parser + goldens |
| `rust/crates/core/src/live.rs` | expose open helper / cleaning if private |
| `rust/crates/core/src/store.rs` | past session's transcript path by id (read only) |
| `rust/crates/app/src/api.rs` | route |
| `rust/parity/leak_rust.py` | markers, route |
| `rust/README.md` | Live section: transcript |

## Files must NOT touch
| File | Reason |
|------|--------|
| `~/.claude.json`, any real `~/.claude*` file, any `*.key`, Claude Code's messaging socket | DDR-0001, DDR-0007 |
| store schema (`rust/crates/core/src/schema_v11.sql`) | no schema change in this plan |
| `web/**` | T118/T119 |
| `src/easy_personal/**` | Python app |

## Test plan
Golden parser tests, route tests with temp files, leak gate.

## Verification Command
```bash
cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -5 && python3 parity/leak_rust.py
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes
- [ ] Tests pass
- [ ] Smoke test confirmed (owner: open a running terminal session in Live and see its conversation)
- [ ] `docs/` updated (README where the guide says)
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
