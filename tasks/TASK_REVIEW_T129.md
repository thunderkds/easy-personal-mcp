# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` or `[admin]` tag; with `[admin]`, the `Not done:` line names `merge (admin: after the auto verify pass)`,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.
`Memory:` records the diff-driven memory pass you run before you stop: each entry written, or `nothing new`.
After an auto verify pass the hook, never you, adds `Closed: row moved to Done <date> — …` and moves the row,
or adds `Paused: done not moved — <reason>`. It moves the row only when this block holds `Mode: auto`, a
numbered decision, `Checks:`, `Memory:`, the `Not done:` auto verify slot with its result, and `Not done:`
naming the merge — no `<...>` left (`docs/claude-md/tracks.md` `### Auto close-out`).
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. Fix on the route side: entries carry `kind`; the `/agents` listing keeps `type` (agent type) — FACT: tasks/TASK_GUIDE_T129.md "Owner decision 2026-11-20 (\"go with both\"): fix on the route side (entries carry `kind`"
2. AC 4: no code change; the session is saved under the account's folder and `claude --resume <id>` works; follow-ups recorded (copy-resume-command button with the account; app sessions labelled "headless" in Past) — DEBATED: no code in T129 (Advocate 1: no code, file a copy-resume-command follow-up; Advocate 2: no code, but add a follow-up and an owner smoke step, the picker cause is untested; Advocate 3: no code, smallest bugfix, T130 may replace app sessions) ⚠ decided for you — please review
3. AC 5: the page counts entries it cannot draw and says `N of M entries cannot be shown on this page.` (a mix is the same page/route disagreement the bug was); the contract test asserts exact golden text and tool names — DEBATED: count (Advocate 1: count n of M, one line, also for a mix; Advocate 2: count, and the contract test was vacuous on `log.children.length`; Advocate 3: keep all-or-nothing); settled by tasks/TASK_GUIDE_T129.md AC 6 "Regression test(s) lock the cause" ⚠ decided for you — please review
Checks: tests `cargo test --workspace --no-fail-fast` — on f40eb18 1364 passed + 1 load-timing flake (memgraph_parity two_hundred_thousand_links, passes alone 2/2, file not in the diff); `npm test` — 46 files, 1513/1513 after the Stage 4 round (a0a0209); `cargo test -p easy-personal-core --test transcript_golden` 4/4; reviews code-review — 0 P0/P1, 1 P2 fixed (vacuous contract assertion), cut list recorded; security-review — no finding >= 8 (independent reviewer); real run leak_rust.py — printed `leak check: 0 hits over 42 canaries, 1335 surfaces` (implementer)
Memory: memory/learnings.md — 2026-11-20 — T129: route entries keyed `type` vs page `kind` hid 2 days behind consumer-written fixtures; contract tests read the producer's golden
Not done: merge (admin: owner "merge T129 [admin]"); auto verify: pass (commit 3a117e8, log .claude/verify-runs/T129-3a117e8f83d0.log)
Paused: auto-mode check failed — Risk High: decision 1 (`FACT`) is a choice — its FACT has no Citation (`path:line "quoted text"`) — so it must be `DEBATED` with exactly 3 Advocates named; 2 `DEBATED` line(s) need ≥6 Advocate `Agent` calls (description `Advocate <n>:` and a prompt starting `Using the repo, answer alone:`); the trace has 3 [report 1832e1911231]
Paused: done not moved — report incomplete: auto verify slot

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/transcript_golden.rs` (`every_entry_names_its_kind_in_kind_and_never_in_type`: main and sub-agent golden), `rust/crates/app/src/transcript_route.rs` (`a_live_sessions_sub_agent_is_listed_and_read_with_its_sidechain_records`: `kind` set and `type` absent, main thread and `?agent=`), `web/src/TranscriptContract.test.jsx` (the page over `session.golden.json` and `agent.golden.json`: the cannot-show line is absent, every user/assistant/reply entry shows its exact text, every tool entry its name; Stage 4 round: a mixed list draws its known entries and says `1 of 3 entries cannot be shown on this page.`, an all-unknown list says `2 of 2 ...`). AC 1, 2, 3: the cause was the page drawing nothing for the route's real answer; AC 2 and 3 (Working, send) were never at fault (the screenshot shows the state and the composer). AC 4: no code change, see Smoke suite. |
| Verification command run | ☑ pass | 2026-11-20, worktree `T129`, machine load average ~29 (other agents' builds). `cargo fmt --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: Finished, no warning. `cargo clippy --workspace --all-targets --features window -- -D warnings`: Finished, no warning. `cargo test --workspace --no-fail-fast`: 99 suites, 1361 passed, 4 failed, 29 ignored; the 4 are timing or temp-folder tests unrelated to the transcript (`jobs_parity::a_crontab_of_ten_thousand_lines_with_hostile_lines_is_listed_fast`, `answer_bridge::a_stop_hook_with_no_app_is_silent_exit_zero_and_fast`, `answer_bridge::hook_ask_with_no_app_is_silent_exit_zero_and_fast`, `ingest_coexist::a_killed_rust_ingest_leaves_whole_files_and_a_rust_rerun_equals_a_clean_run`); rerun alone each passed (`jobs_parity` 7 passed, `answer_bridge` 18 passed, `ingest_coexist a_killed` 1 passed). Leak gate `env -u CLAUDE_CONFIG_DIR ./parity/.venv/bin/python parity/leak_rust.py` (main checkout's venv): first run FAILED 2 checks that read an entry's `type` (`T115 transcript entries are user, assistant, tool, reply`, `T124 agent entries are user, tool`), after the fix of `leak_rust.py` lines 1289 and 1320: `leak check: 0 hits over 42 canaries, 1335 surfaces, 2960358 bytes`. `npm test`: first run 2 failed of 1511 under load (concurrent clippy), rerun alone `Test Files 46 passed (46) Tests 1511 passed (1511)`. Stage 4 fix round (web only, Rust unchanged), 2026-11-20: `npm test` `Test Files 46 passed (46)`, `Tests 1513 passed (1513)` (first run, no flake); `npm run build` `✓ built in 365ms`; `python3 scripts/check_static.py` `check_static: ok`. `npm run build`: `✓ built in 388ms`. `python3 scripts/check_static.py`: `check_static: ok`. |
| Negative cases hold | ☑ pass | Red against the prior code (tests written first): `cargo test -p easy-personal-core --test transcript_golden` -> `every_entry_names_its_kind_in_kind_and_never_in_type` panicked `main: {"count":2,"offset":0,"type":"skipped"}`, `1 failed`; `npx vitest run src/TranscriptContract.test.jsx` -> `Tests 3 failed (3)`. Green after the fix: `4 passed` and `Tests 3 passed (3)`. Mutation diff of the fix: `git diff --stat` below in the commit. The AC 5 test feeds `{type:'user'}` (the old shape) and an unknown `kind`: one line, never an empty box. |
| verify | ☑ pass | auto-run by hook, session ec734e2b-97a3-429e-8b23-60ad74d234e1, commit 3a117e8f83d0bd373279a72bff6aa4e7f7d33cf6, log .claude/verify-runs/T129-3a117e8f83d0.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff main...T129`: core transcript.rs + goldens + golden test, transcript_route.rs test, leak_rust.py, Transcript.jsx, TranscriptContract.test.jsx; callers of entry `kind` in web/src (Transcript, SubAgents) checked; the agents listing `type` untouched |
| Full smoke suite still green (no regression) | ☑ pass | Supervisor 2026-11-20: `npm test` 46 files 1513/1513 (a0a0209); `cargo test --workspace --no-fail-fast` 1364 passed + 1 load-timing flake not in the diff (passes alone 2/2) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier refuses repo paths since 2026-11-01 (memory/learnings.md, easy-verifier registered); advisory only |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | DOM verdict, no screenshot: TranscriptContract.test.jsx renders the page over the real Rust goldens and finds every user/assistant text and every tool name (before the fix the log had 0 children); the only new visible element is one muted line, shown only when some entries cannot be drawn |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | no CSS changed; the new line reuses the existing `muted` list item class |
| **UI: Responsiveness at target viewports** | ☑ N/A | no layout or CSS change; the entries now drawn use the existing session-log styles (checked at 375/1280 in T118/T121) |
| Repro loop | ☑ pass | 2026-11-20T03:14:11Z real claude 2.1.329 through the app's own `Sessions::start` (scratchpad `git init` repo, haiku, dontAsk, effort medium, account `CLAUDE_CONFIG_DIR` kept, stdin closed at `shutdown`, no process of mine left): app view `{"state":"idle","session_id":"8715b26e-fb03-493d-8d02-fe372db94d8b","items":[{"kind":"user","text":"Reply with the single word OK."},{"kind":"assistant","text":"OK"}]}`; file `projects/<munged cwd>/8715b26e-….jsonl records=31 entrypoint=sdk-cli`. 03:14:28Z `claude -p --resume 8715b26e-…` answered `Your previous message was: "Reply with the single word OK."` (saved, resumable). 03:14:46Z the route over that file: `GET .../transcript -> 200`, entries `[skipped, user, skipped, assistant, skipped, user, skipped, assistant, skipped]`, every entry `keys=[cut|count, offset, text, type]`, `kind=null`. 03:15:32Z `npx vitest run` of a throwaway test (`<Conversation>` over core's `transcript_golden/session.golden.json`, 14 entries): `log <ol> children: 0 innerHTML: ""`, `Unable to find an element with the text: <typed text>`. Throwaway code removed; patch kept outside the repo. |
| Regression test | ☑ pass | the three files named in the first row; the vitest reads the Rust golden answers, so a change of key on either side fails one of them |
| Smoke suite | ☑ pass | AC 4, no code change: at 2026-11-20T03:14Z a real app session (the app's own `Sessions::start`, claude 2.1.329, scratchpad repo) wrote `projects/<munged cwd>/8715b26e-….jsonl` (31 records, `entrypoint: sdk-cli`) under the account's `CLAUDE_CONFIG_DIR`, and `claude -p --resume 8715b26e-…` answered "Your previous message was: "Reply with the single word OK."". So the session is saved and resumable; the terminal `/resume` picker is Claude Code's own and was not tested. Owner smoke test still open: start a session from Live, see Working and the conversation, send a message. |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (bugfix: the Phase 1 repro loop in the Evidence table, captured 2026-11-20T03:14-03:15Z, before any fix commit): the route answers 200 with `type`-keyed entries; the page draws `<ol role="log">` with 0 children for 14 entries (`innerHTML: ""`), the owner's empty box.

**AFTER**: the same vitest over `session.golden.json` (now written with `kind`): `<Conversation>` draws the typed text, assistant text, reply, tool calls (`Bash`, `Read`) and `2 other records` (`Tests 3 passed (3)`); entries of an unknown shape draw `This page cannot show these entries: their shape is not one it knows.` The route test shows `kind` set and `type` absent on the main thread and on `?agent=`.

**DELTA**: the Live panel now shows the conversation of every session (app, terminal, past, sub-agent log) instead of an empty box, and says so in one line when entries cannot be drawn.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T129.jsonl`, never the
implementing agent alone]
