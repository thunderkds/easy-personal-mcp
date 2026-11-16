# TASK_GUIDE — T094: Waiting sessions 1 — the *your turn* state and the question text on `GET /api/live`
**Date**: 2026-11-16
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` (glossary: Attention state, Your turn, Question text, Typed work / Agent work, Entrypoint) and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_WAITING.md` (Option A, Edge Case Checklist), `docs/ddr/0005-live-question-text-show-never-store.md`, `docs/ddr/0004-live-call-labels-show-never-store.md`, and the `PRD.md` rows FR-047, FR-049, FR-105, FR-106, NFR-002, NFR-012
6. Read `rust/crates/core/src/live.rs` (`WAITING_TOOLS`, `call_label`, `struct Call`, `fn attention`, `attention_json`, `snapshot`), `rust/crates/core/src/ingest/claude_code.rs` (`parse_record`), `rust/crates/app/src/live_feed.rs` (`SESSION_KEYS`, `snapshot_data`, `LiveStream::step`), `rust/crates/core/tests/live_parity.rs`, `rust/crates/app/tests/live.rs`, `rust/parity/leak_rust.py`, `rust/parity/README.md`
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders). Never run a `#[ignore]` keychain test.

---

## Requirement

**Original request** (owner, 2026-11-16): "from the tray icon, or notification, with the HITL let show the notif for users to click on. For the dashboard let show the question, or stop point to the live section" · "go [admin], with question text".

**Restated intent**:
> 1. **Your turn** (FR-105). The live tail gives a session the attention state `your_turn` while the newest record of its main thread that is an `assistant` or a `user` record is an `assistant` record whose `message.stop_reason` is the string `end_turn`, and the main thread has no open call. Only a session of *typed work* gives it: a session whose `entrypoint` starts with `sdk`, or whose entrypoint is not known, never does. A subagent stream never does. Any later `assistant` or `user` record of the main thread ends it. The state is worked out where the other states are (no new loop, thread or route) and holds until the session leaves the live list. Order in a session: `failing`, `waiting`, `your_turn`, `needs_attention`. Its JSON: `{"state": "your_turn", "tool": null, "count": 1, "is_sidechain": false, "since": <timestamp of that assistant record>}`. A session with this state stays in the live list as a session with any state does.
> 2. **Question text** (FR-106, DDR-0005). For an open `AskUserQuestion` call the live tail reads `input.questions[].question` and nothing else of the input: at most 4 questions, each cut to 300 characters (cut on a character boundary), control and hidden characters removed (the FR-092 list: C0, U+007F, C1, U+2028/U+2029, U+200B-U+200F, U+202A-U+202E, U+2066-U+2069, U+FEFF), a question that the core's secret check (`ingest::secret::is_secret`) matches dropped whole, an empty result dropped. The text is held in memory on the open call and dropped when its result arrives.
> 3. **Only the stream sends it.** `LiveTail::snapshot()`, the JSON that `tray.rs` and `alerts.rs` read, never holds the text. The stream's snapshot (`live_feed.rs`) adds it to the `waiting` state of that session as the key `questions`: a list of strings, absent when there is none. For several open `AskUserQuestion` calls in one stream: the questions of the oldest open call. Nothing else is added to the stream.
> 4. **Leak proofs.** A planted marker question reaches `GET /api/live` and nothing else: not the store, not a log line, not `GET /api/alerts`, not a tray row, not a popup text, not an MCP tool answer, not a CLI command. A planted `header`, option `label`, option `description`, and an `ExitPlanMode` `plan` never reach `GET /api/live`. A planted secret-shaped question reaches nothing.
> 5. **The Python app does not change**, and no parity golden changes. `rust/parity/README.md` names the two differences (the state, the text).

**Out of scope**: the page (T095); the tray rows, the alert for *your turn*, the popup action and the click that selects a session (T096); `header`, option labels, the plan text; a permission prompt; hooks; a setting for any limit; the store schema; Python.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16, `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `PRD.md` US-040, FR-105, FR-106, FR-047, NFR-002, NFR-012; `docs/ddr/0005-live-question-text-show-never-store.md`

---

## Dependencies & Reachability
**Depends on**: None (T049, T056, T074, T076 are merged)
**Entry point**: `GET /api/live` (`rust/crates/app/src/live_feed.rs` `snapshot_data`)

## Dependency rule
No new crate. No change to `Cargo.lock`.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Engine test: a typed-work main thread whose newest `assistant`/`user` record is `assistant` with `stop_reason: "end_turn"` has `your_turn` in `snapshot()`, with the JSON of intent 1; a following `user` record or `assistant` record removes it on the next poll | intent 1 |
| 2 | Negative cases, each its own test or table row: `entrypoint` `sdk-cli`, `sdk-ts`; no entrypoint; a subagent stream's `end_turn`; `stop_reason` `tool_use`, `stop_sequence`, `max_tokens`, `null`, a number, an object; an open call on the main thread. None gives `your_turn` | intent 1 |
| 3 | State order test: a session that is `failing`, `waiting` and `your_turn`-eligible lists its states in the order `failing`, `waiting`, `your_turn`, `needs_attention` | intent 1 |
| 4 | Warm start (FR-086 replay) of a transcript that ended its turn before the engine started: `your_turn` shows; a session idle past the one-hour rule is gone as before | intent 1 |
| 5 | Question text: an open `AskUserQuestion` with 2 questions gives `"questions": [q1, q2]` in the stream snapshot's `waiting` state, and no `questions` key in `LiveTail::snapshot()`; after the call's result arrives, the next stream snapshot has no `questions` key | intents 2, 3 |
| 6 | Limits, table-driven: 5 questions → 4; a 10,000-character question → 300 characters, valid UTF-8 with a multi-byte character at the cut; each character class of the FR-092 list removed; a secret-shaped question dropped while its neighbour stays; `questions` not a list, an item not an object, `question` not a string, an empty string → no key or the item left out; nothing panics | intent 2 |
| 7 | Never read: planted `header`, option `label`, option `description`, `ExitPlanMode` `plan`, `multiSelect` appear in no stream message and in no engine state (assert on the serialized bytes) | intent 4 |
| 8 | Leak gate: `rust/parity/leak_rust.py` gains planted transcripts with a marker question, a secret-shaped question, a `header`, an option label and a plan. It proves the marker question is in `GET /api/live` (positive control) and in no other surface it already checks (routes incl. `/api/alerts`, CLI commands, MCP tools, stdout/stderr, the store file bytes); the other planted strings are nowhere. Exit 0 | intent 4 |
| 9 | `alerts.rs` and `tray.rs`: with a waiting session that has a planted question, the alert text, `/api/alerts` JSON and the tray rows hold none of it (extend the existing `no_secret_or_message_text_reaches_the_popup_or_the_list` style test; no behaviour change in those files beyond what the test needs) | intents 3, 4 |
| 10 | Parity: `cargo test -p easy-personal-core --test live_parity` and every other parity test pass with no golden file changed (`git status --short rust/parity/golden*` prints nothing); `rust/parity/README.md` names the two differences from Python | intent 5 |
| 11 | `cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` pass; `cargo build --features window` builds | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test --workspace 2>&1 | tail -6 && cargo build -q && env -u CLAUDE_CONFIG_DIR /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/rust/parity/.venv/bin/python parity/leak_rust.py; echo exit=$?
```
(Run it from your worktree; the Python interpreter path is the main checkout's venv, read-only use.)

### Evidence (filled by reviewer at Stage 4/5)
> Moved to `tasks/TASK_REVIEW_T094.md`.

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T094.md`. BEFORE (capture before your first implementation commit): a temp `projects/` tree with one typed-work transcript that ends with `stop_reason: "end_turn"` and one with an open `AskUserQuestion` holding a marker question; `serve` on it; the first `snapshot` of `GET /api/live` shows no `your_turn` and no `questions`. AFTER: the same run shows both.

## UI / Design Acceptance Criteria
N/A — no page change (T095 changes the page).

---

## Approach

**Pattern reference**: `rust/crates/core/src/live.rs` `call_label` and DDR-0004 — one named place that reads an input field, a short allow-list, a drop on doubt; `rust/crates/app/src/alerts.rs` `clean` / `safe` for the character and secret checks.

**Vital slice**: the two engine changes and the stream key, with the leak proofs. No setting, no new route, no new message kind.

Test first (`tdd`): write the engine tests of AC 1-7 and see them fail for the right reason, then the code.

Sketch (change it if you find a simpler one that meets the ACs):
- `parse_record` (or the live item built from it) carries `stop_reason` only as "is `end_turn`" (a bool); do not keep the string. Check how the entrypoint already reaches the live session; if the live tail does not hold it yet, carry it the same way the stored event does.
- Keep the question strings on `Call` (an `Option<Vec<String>>`, built once when the call opens, by one new function next to `call_label` that is the only reader of `questions`).
- Give the text to the stream through a second read on `LiveTail` (for example a map `session id → questions` beside `snapshot()`), which `live_feed.rs` merges into its own snapshot after the `SESSION_KEYS` pick. `snapshot()` itself stays byte-identical for the same input, so the parity tests and the tray keep their contract.
- `your_turn` belongs in `fn attention` beside `waiting`.

## Edge Case Checklist
- A headless session (`entrypoint` starts with `sdk`) and a session with no known entrypoint never get *your turn*
- A subagent stream's `end_turn` never gives it; an `end_turn` followed by any `assistant` or `user` record of the main thread ends it
- A message split over several `assistant` records: only the newest one decides
- A warm start shows the state; the bounded replay (64 MB) may not see an older `end_turn`: then no state, never a guess
- `stop_reason` on a record that is not `assistant` is ignored
- The stream's change detection (`LiveStream::take`) must send a new snapshot when only `questions` changed, and must not send one when nothing changed
- A question text is never written with `{:?}` or `eprintln!`; a panic message never holds it
- 4 × 300 characters is the bound per call; with many sessions the memory stays bounded by the open-call bound that exists

## Files to Change (Predicted)
`rust/crates/core/src/live.rs`, `rust/crates/core/src/ingest/claude_code.rs` (only if `stop_reason` must be parsed there), `rust/crates/app/src/live_feed.rs`, `rust/crates/app/src/alerts.rs` and `rust/crates/app/src/tray.rs` (tests only), `rust/crates/core/tests/*live*`, `rust/crates/app/tests/live.rs`, `rust/parity/leak_rust.py`, `rust/parity/README.md`, `tasks/TASK_REVIEW_T094.md`

## Files Must NOT Touch
`src/easy_personal/**`, `tests/**`, `web/**`, `rust/parity/golden*/**`, `rust/parity/gen_*.py`, `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/crates/app/src/window.rs`, `rust/crates/app/src/keychain.rs`, `PRD.md`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `memory/**`, `.claude/**`, `docs/**`

## Test Plan
AC 1-7, 9: unit and integration tests in the two crates, output pasted. AC 8: the leak gate's output with its count of checks before and after. AC 10: `git status --short rust/parity` and the parity test output. AC 11: the four commands' last lines.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T094.md` (Evidence and Demonstration)
- [ ] Commits on branch `T094` only; final report with `Status:`

## Questions Log
- 2026-11-16 DEBATE Should the question text be a key of the shared `LiveTail::snapshot()` or be given to the stream alone by a separate read? → a separate read that only `live_feed.rs` uses; `snapshot()` never holds the text (Advocate 1: separate read, stream only; Advocate 2: separate read, stream only; Advocate 3: separate read, stream only) ⚠ decided for you — please review
- 2026-11-16 DEBATE How wide is the question-text exception (which fields are read, where may the text go)? → `questions[].question` only, in memory, on `GET /api/live` and the Live view only (Advocate 1: question text only, Live view only; Advocate 2: question text only, Live view only; Advocate 3: question text only, Live view only, 4 × 300 characters) ⚠ decided for you — please review
