# TASK_REVIEW — T086: Cutover follow-up: `store-check` on a big store and its hint text; CUTOVER.md steps 0.2 / 0.4 / the week

> Sibling of `tasks/TASK_GUIDE_T086.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/store_health_parity.rs` `the_hint_names_the_rust_binary`; `rust/crates/app/tests/cutover_doc.rs` `the_steps_that_expect_a_verdict_run_store_check_full`, `step_0_2_does_not_ask_to_close_every_session`; BEFORE failed on the old source, AFTER pass |
| Verification command run | ☑ pass | Supervisor: `cutover_doc` 7 passed, `store_check` 12 passed 1 ignored, `store_health_parity` 11 passed |
| Negative cases hold | ☑ pass | golden `budget_zero.txt` keeps Python's bytes; the parity test maps the Rust-only hint back (intended difference recorded in rust/README.md) |
| verify | ☑ pass | Supervisor `/verify` 2026-11-15 ~10:20Z on `bfee4d6`, load avg 31: `cargo build` ok; the 3 test files pass; `store-check` on a temp copy of `store-v11.sqlite` → `verdict: healthy`, exit 0; code check: `exit_code()` returns 3 for not fully checked in every build, so the doc's 'exit code is 3' is right; leak gate `0 hits`, 1 failed check = `store-check made no file beside the store (text past the cut)` — the known load flake T085 fixes, not this diff; `memgraph_parity two_hundred_thousand_links_are_counted_in_time` fails on main too at load 32 (9.1 s) — pre-existing timing flake; merge-tree clean — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff b4c8469..T086` (6 files) reviewed inline by the Supervisor (C1, Low): 0 code P0-P2. P1 PROCESS: the agent says it did not read PROJECT_SPEC.md, rust/PROJECT_SPEC.md, agents/backend.md or PRD FR-091/NFR-013 (startup reads) — recorded, no code impact. P3: step 0.2's new `systemctl --user is-active` line was not run (never run cutover steps); the owner already ran the cutover |
| Full smoke suite still green (no regression) | ☑ pass | agent workspace `--no-fail-fast`: only the timing tests `live_parity pairing_100k_calls_is_fast` (passes alone) and `memgraph_parity two_hundred_thousand…` (fails on main too under load) failed |
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

**BEFORE** (captured 2026-11-15, before any implementation commit; new tests written first, main b4c8469 code and doc unchanged):

`2026-11-15T09:51:37Z` `cargo test -p easy-personal-core --test store_health_parity the_hint`
```
test the_hint_names_the_rust_binary ... FAILED
thread 'the_hint_names_the_rust_binary' panicked at crates/core/tests/store_health_parity.rs:130:5:
verdict: not fully checked (integrity_check, foreign_key_check, sources not finished in time); run `easy-personal store-check --full` for the integrity verdict
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out
```

`2026-11-15T09:5xZ` `cargo test -p easy-personal-rs --test cutover_doc`
```
test step_0_2_does_not_ask_to_close_every_session ... FAILED
test the_steps_that_expect_a_verdict_run_store_check_full ... FAILED
Step 0.4: `rust/target/debug/easy-personal-rs store-check` lacks --full
Step 0.2 still asks the owner to close every session
test result: FAILED. 5 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

Correction (honest note): the first capture above failed at line 130 because my test asserted the wrong prefix (`not fully checked (` instead of `verdict: not fully checked (`). I fixed the test line, put the old `store_health.rs` back for one run, and captured again on the same old source:

`2026-11-15T09:59:15Z` `cargo test -p easy-personal-core --test store_health_parity the_hint`
```
thread 'the_hint_names_the_rust_binary' panicked at crates/core/tests/store_health_parity.rs:131:5:
verdict: not fully checked (integrity_check, foreign_key_check, sources not finished in time); run `easy-personal store-check --full` for the integrity verdict
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out
```

Prior doc content (verbatim, `rust/docs/CUTOVER.md` at b4c8469):
- Step 0.2: "Also close every Claude Code session that uses the easy-personal MCP server. Each open session runs its own Python `easy-personal mcp` process, and it keeps the store open. Close the Python dashboard too." / "You should see: `pgrep` prints nothing. If it prints a line, close that process and run it again."
- Step 0.4: command `rust/target/debug/easy-personal-rs store-check`; "You should see: the last line is `verdict: healthy`, and the exit code is 0 (`echo $?` prints `0`)."
- The week: "`easy-personal-rs store-check` ends with `verdict: healthy`."

**AFTER** (2026-11-15, same tests, new code):
```
cargo test -p easy-personal-core --test store_health_parity   -> 11 passed (incl. the_hint_names_the_rust_binary, every_golden_case_equals_python)
cargo test -p easy-personal-rs --test cutover_doc             -> 7 passed (incl. the_steps_that_expect_a_verdict_run_store_check_full, step_0_2_does_not_ask_to_close_every_session)
cargo test -p easy-personal-rs --test store_check             -> 12 passed, 1 ignored
```
New text: hint `run `easy-personal-rs store-check --full` for the integrity verdict`. CUTOVER.md Step 0.2 "Stop the ingest timer" (no session close; says why the timer stops), Step 0.4 runs `store-check --full` and explains `not fully checked` (exit 3), The week uses `store-check --full`.

**DELTA**: the owner can follow CUTOVER.md on a large store and get `verdict: healthy`, keep Claude Code sessions open during Step 0, and the hint names the right binary.

**WITNESS**: Supervisor (not the implementing agent) ran `/verify` 2026-11-15 ~10:20Z with active_task = T086.
