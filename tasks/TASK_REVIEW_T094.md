# TASK_REVIEW — T094: Waiting sessions 1 — the *your turn* state and the question text on `GET /api/live`

> Sibling of `tasks/TASK_GUIDE_T094.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Stage 0 questions answered from the request and the repo: the request is the owner's three messages of 2026-11-16 — FACT: PRD.md:313 "Decided by the owner 2026-11-16 (\"go [admin], with question text\")"
2. Width of the question-text exception: only `questions[].question`, in memory, on `GET /api/live` and the Live view only — DEBATED: question text only, Live view only (Advocate 1: question text only, Live view only; Advocate 2: question text only, Live view only; Advocate 3: question text only, Live view only, 4 × 300 characters) ⚠ decided for you — please review
3. The text is given to the stream by a separate read; `LiveTail::snapshot()` never holds it — DEBATED: separate read, stream only (Advocate 1: separate read, stream only; Advocate 2: separate read, stream only; Advocate 3: separate read, stream only) ⚠ decided for you — please review
4. *Your turn* is given only to a session of typed work — FACT: PROJECT_SPEC.md:74 "*Agent work* = a sidechain record, or any record of a session whose **entrypoint** starts with `sdk` (headless)."
5. *Your turn* adds no timer of its own: it ends with the session's place in the live list — RULE: PRD.md § Non-Functional Requirements (NFR-012: "Live attention states add no new polling loop, thread or SSE route")
6. A DDR (0005), not an ADR — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 3)
7. Base team, own task breakdown (T094, T095, T096), Requirement Fidelity Gate signed `auto (Supervisor)` — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (rows 11-13)
Checks: <filled at Stage 4/5>
Memory: <filled before the run ends>
Not done: merge (admin: after the auto verify pass); auto verify: <not started>
Paused: auto-mode check failed — no `Memory:` line in the report — run the memory pass before you stop and record it there (each entry written, or `Memory: nothing new`); no test run in memory/event-trace/T094.jsonl — write /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID with the task ID (minimal-track step 1): `mkdir -p /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d && printf '%s\n%s\n' "T094" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID`, then rerun the task's tests so the trace records them under the task, then stop; 2 `DEBATED` line(s) need ≥6 Advocate `Agent` calls (description `Advocate <n>:` and a prompt starting `Using the repo, answer alone:`); the trace has 5; extended track, but the trace has no `code-review` Skill call (Stage 4 runs in auto mode); Risk High, but the trace has no `security-review` Skill call [report 23df33cea2c8]



## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/live_waiting.rs` (AC 1-7 + parser depth: `live_waiting: test result: ok. 10 passed; 0 failed`), `rust/crates/app/src/live_feed.rs` (`the_stream_adds_the_question_text_to_the_waiting_state_only ... ok`, `a_change_of_the_question_text_alone_sends_a_snapshot ... ok`), `rust/crates/app/tests/live.rs` (`your_turn_and_the_question_text_arrive_on_the_live_route ... ok`; `app live.rs: test result: ok. 9 passed`), AC 9: `tray::tests::no_question_text_reaches_a_row_from_a_real_live_tail ... ok`, `alerts::tests::a_waiting_session_in_a_real_live_tail_raises_one_alert ... ok` (planted real question + positive control); AC 8: `rust/parity/leak_rust.py` (`question_scan`, `question_line`). Run 2026-11-16T09:06Z at `38bf4b1`. |
| Verification command run | ☐ pass / ☑ fail (1 environmental test, see Notes) | 2026-11-16T08:55Z, worktree. `cargo test --workspace`: stops at `connector_cli_parity`: `every_connector_and_doctor_step_equals_the_python_cli` FAILED, `5 problems` all in `sync_no_launcher` / `doctor_no_launcher` (`exit 0, Python 1`; `ok launcher: /usr/bin/easy-personal-launch`): the oracle expects no launcher on PATH, and this machine has `/usr/bin/easy-personal-launch` installed (`present: yes`). T094 touches no connector code. Leak gate: `check alerts listed, one per planted session  ok (5 alerts: {'session_waiting': 4, 'session_failing': 1})` / `check question marker in GET /api/live (positive control)  ok (1 of 1 live answers)` / `check T094 question strings scanned       ok (7 strings, 832 surfaces)` / `leak check: 0 hits over 42 canaries, 831 surfaces, 2028224 bytes` / `exit=0`; checks 100 before (committed script: `leak check: 0 hits over 42 canaries, 828 surfaces, 1797673 bytes exit=0`) -> 102 after. Full `--no-fail-fast` run at `38bf4b1`: `passed 1020, failed 1, ignored 28` (the same test). AC 11: `cargo fmt --check` exit=0; `cargo clippy --workspace --all-targets -- -D warnings` exit=0; same with `--features window` exit=0; `cargo build --features window` exit=0. AC 10: `git status --short parity/golden*` printed nothing; `live_parity: test result: ok. 15 passed`; `the_stream_equals_python_live_stream_for_every_oracle_case ... ok`; `rust/parity/README.md` names the two differences. |
| Negative cases hold | ☑ pass | Red runs before the code (stubs): `test result: FAILED. 1 passed; 8 failed` (e.g. `left: ["failing", "waiting", "needs_attention"]` / `right: ["failing", "waiting", "your_turn", "needs_attention"]`; `left: "None"` / `right: "Some([(0, [\"Which colour?\", \"Ship it now?\"])])"`); stream: `left: Null` / `right: Array [String("Which colour?"), String("Ship it now?")]`. AC 2 negative table: 15 rows, all hold. Leak-gate mutant (both `is_secret` checks removed from `live::clean_question`, `diff` non-empty: `234,236d233 <     if is_secret(raw) {` ...): `FAILED: HIT T094 question_secret as raw in [alerts api GET /api/live (first message)]: ?","[question_secret]"],"` / `exit=1`; restored and rebuilt. |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☑ fail (environmental) | `passed 1020, failed 1, ignored 28`: the one failure is `connector_cli_parity` `*_no_launcher` (machine has `/usr/bin/easy-personal-launch`); no `#[ignore]` test was run |
| **Companion score (easy-verifier)** | ☑ N/A | not run by the implementer |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no page change (T095 changes the page) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no page change |
| **UI: Responsiveness at target viewports** | ☑ N/A | no page change |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (branch `T094` at `4142ce8`, no implementation commit yet; binary built from it with `cargo build`):
```
$ python3 <scratchpad>/demo_t094.py rust <scratchpad>    # temp HOME: store-v11 copy + one confirmed account; serve --no-ingest
$ date -u: 2026-11-16T08:41:45+00:00
first snapshot of GET /api/live, sessions:
{"session_id": "demo-ask-0002", "attention": [{"count": 1, "is_sidechain": false, "since": "2026-11-16T08:41:14.000000Z", "state": "waiting", "tool": "AskUserQuestion"}]}
{"session_id": "demo-turn-0001", "attention": []}
your_turn present: False | questions key present: False
```
(`turn.jsonl`: a `cli` session whose last record is an `assistant` record with `stop_reason: "end_turn"`; `ask.jsonl`: an open `AskUserQuestion` whose one question is `DEMO-MARKER which colour?`, with a `header` and an option.)

**AFTER** (same script, binary built at `38bf4b1`):
```
$ date -u: 2026-11-16T09:10:47+00:00
first snapshot of GET /api/live, sessions:
{"session_id": "demo-ask-0002", "attention": [{"count": 1, "is_sidechain": false, "questions": ["DEMO-MARKER which colour?"], "since": "2026-11-16T09:10:16.000000Z", "state": "waiting", "tool": "AskUserQuestion"}]}
{"session_id": "demo-turn-0001", "attention": [{"count": 1, "is_sidechain": false, "since": "2026-11-16T09:10:16.000000Z", "state": "your_turn", "tool": null}]}
your_turn present: True | questions key present: True
```
(The planted `header` `HDR` and option `Red` / `warm` are not in the answer.)

**DELTA**: The Live view stream now marks a typed session that ended its turn as `your_turn`, and carries the question of a waiting `AskUserQuestion` call (`questions`) on `GET /api/live` only.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
