# TASK_REVIEW — T084: Watch follow-up: alert engine's first look never blocks server start on a hung D-Bus

> Sibling of `tasks/TASK_GUIDE_T084.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
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

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/alerts.rs` tests `a_hung_notifier_never_blocks_the_start` (AC 1), `a_waiting_session_in_a_real_live_tail_raises_one_alert` (AC 2, exactly one alert and one popup over several ticks), `stop_before_the_first_tick_shows_nothing`, `paused_before_the_first_tick_lists_the_alert_and_shows_nothing`; `rust/crates/app/src/server.rs` test `start_with_alerts_runs_the_engine_serves_the_list_and_stop_joins_it` (waits for the popup). 23 passed, 0 failed (2026-11-15T10:25Z) |
| Verification command run | ☑ pass | `cargo test -p easy-personal-rs --bin easy-personal-rs alert`: `test result: ok. 23 passed; 0 failed`. `cargo test --workspace --no-fail-fast`: 993 passed, 2 failed, both timing tests in `easy-personal-core` (`jobs_parity::a_crontab_of_ten_thousand_lines_with_hostile_lines_is_listed_fast` 3.2-3.35 s vs 3.0 s limit, `memgraph_parity::two_hundred_thousand_links_are_counted_in_time` 7.1-9.2 s) at load average 33-40; `core` is not in this diff. Leak gate from `rust/`: `leak check: 0 hits over 42 canaries, 818 surfaces, 1804554 bytes`, exit 0. `cargo fmt --check` and `cargo clippy -p easy-personal-rs --all-targets` clean |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☑ pass | Supervisor `/verify` 2026-11-15 ~10:35Z on `9f0b313`, load avg 38: `cargo build` ok; alert tests 23 passed; server tests 16 passed; window build `app --self-test` → `self-test: ok`; tray tests 16 passed; leak gate `0 hits`, 1 failed check = `store-check made no file beside the store (corrupted)` — the known load flake fixed by T085, not this diff; merge-tree clean — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff b4c8469..T084` (alerts.rs, server.rs) reviewed inline by the Supervisor (C1, Low): the first `Sampler` item is a no-op, as `sampler::limits` already does; `sampler.rs` untouched, so machine/limits first-look behaviour unchanged; first alert now ~1 TICK (5 s) after start, as intent 2 allows. 0 P0-P3 |
| Full smoke suite still green (no regression) | ☑ pass | agent workspace `--no-fail-fast`: 993 passed; 2 failed = core timing tests `jobs_parity a_crontab_of_ten_thousand…` and `memgraph_parity two_hundred_thousand…` (untouched code; memgraph fails on main too at load 32) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier MCP not connected this session; advisory only |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-15T09:52:08Z (start of the run), worktree `T084` at b4c8469 plus the new tests only (no implementation change), command `cd rust && timeout 590 cargo test -p easy-personal-rs --bin easy-personal-rs alerts::`. The test `a_hung_notifier_never_blocks_the_start` uses a notifier that blocks until released. On the old code `start_every` runs the first look on the calling thread, so it never returned and the 590 s timeout killed the run (last lines of the output, after the build):

```
running 19 tests
...
test alerts::tests::stop_before_the_first_tick_shows_nothing ... FAILED
test alerts::tests::the_thread_looks_at_the_inputs_and_stops_quickly ... ok
...
test alerts::tests::paused_before_the_first_tick_lists_the_alert_and_shows_nothing ... ok
test alerts::tests::a_waiting_session_in_a_real_live_tail_raises_one_alert ... ok
test alerts::tests::a_hung_notifier_never_blocks_the_start has been running for over 60 seconds
```

(`stop_before_the_first_tick_shows_nothing` fails because the first look already popped at start.)

**AFTER**: 2026-11-15T10:25:04Z, same command with the filter `alert`, after the change (`alerts::start_every` makes the `Sampler::start` first item a no-op, so the first look is the thread's first tick):

```
test alerts::tests::stop_before_the_first_tick_shows_nothing ... ok
test alerts::tests::a_hung_notifier_never_blocks_the_start ... ok
test alerts::tests::the_thread_looks_at_the_inputs_and_stops_quickly ... ok
test alerts::tests::paused_before_the_first_tick_lists_the_alert_and_shows_nothing ... ok
test alerts::tests::a_waiting_session_in_a_real_live_tail_raises_one_alert ... ok
test server::tests::start_with_alerts_runs_the_engine_serves_the_list_and_stop_joins_it ... ok
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 155 filtered out; finished in 5.10s
```

**DELTA**: `serve --alerts`, `app --alerts` and `tray` no longer wait for a D-Bus `Notify` call at start; a hung notification service only delays the engine thread.

**WITNESS**: Supervisor (not the implementing agent) ran `/verify` 2026-11-15 ~10:35Z with active_task = T084.
