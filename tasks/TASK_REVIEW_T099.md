# TASK_REVIEW — T099: terminal `account ignore` of the agent run account also turns the agent run off

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. The id passed to `forget` is the row's `id` from the body `accounts::ignore` returned, as a number — RULE: tasks/TASK_GUIDE_T099.md:53 "The id passed to `forget` comes from the row the ignore returned (the numeric id, as the route uses it)"
2. AC 2 tests are by id (plain `1`, padded `01`) and by folder path, no label test — FACT: rust/crates/core/src/accounts.rs:421 "`_find`: the account id for an id (plain digits) or a folder path; 404 `no such account`." (guide AC 2 corrected at Stage 4)
3. The call is `Runner::system()` then `forget`, not `agent_routes::account_gone` — RULE: tasks/TASK_GUIDE_T099.md:74 "call the same helper the route uses (`agent_routes::account_gone(&dir, id)`, or `Runner::forget` with the agent run dir)"
4. `forget` runs after `write(...)` returned, so after the ignore committed; a failed ignore returns before it — RULE: tasks/TASK_GUIDE_T099.md:74 "In `run_account`, `AccountAction::Ignore`: after `write(...)` returns the body"
5. AC 4's test is an integration test over the real binary with its own shell-script fake `claude` (`rust/crates/app/tests/account_ignore_agent_run.rs`), not `agent_run::tests::Fake`; guide AC 4 reworded to match — DEBATED: keep the integration test (Advocate 1: keep it, the new code reads every path from the process environment and T091's `tests/agent_run_routes.rs:230` tests ignore-stops-run the same way; Advocate 2: keep it, an in-crate test would need `set_var`, unsafe in edition 2024 and racing other test threads, or would skip the T099 wiring) ⚠ decided for you — please review
6. The Verification Command's `agent` filter matches 1 of the 8 new tests; the other 7 ran by file name and in the full workspace run — FACT: rust/crates/app/tests/account_ignore_agent_run.rs:143 "fn ignoring_the_agent_run_account_removes_the_flag_and_the_setting_is_off() {" is the one name holding `agent`
Checks: tests `cd rust && cargo test --workspace --no-fail-fast` — 95 targets, 1065 passed, 1 target failed: `connector_cli_parity` (the installed `.deb` puts `easy-personal-launch` on `PATH`; fixed by T097, not on this branch); clippy, fmt, leak gate pasted in Evidence; reviews code-review (Supervisor inline, 2026-11-17, `T091...T099`) — 0 P0, 0 P1 code, 0 P2; P1 process: the agent's report has no `Startup reads:` line (the spawn prompt had no `**Startup reads**` block either: Supervisor's miss); P3 x2: the guide's AC 2 said `label` (guide fixed: id or folder path), lane `EXCEEDS (lines 354 > 250)` from the 246-line test file (C1 kept: one code call); security-review (`security-review` skill, scoped by a sub-task to `git diff T091...T099 -- rust/`, 2026-11-17) — no finding; `flag()` reads `symlink_metadata` so a symlinked flag is left alone; the removed path is fixed (`Runner::system().dir` + `FLAG`); the only effect is removing the owner-only flag file when it holds the ignored row's numeric id, after the ignore committed; no new output, no input echoed; real run: the tests run the real binary over temp folders with a fake `claude` (auto-checked, not /verify)
Memory: nothing new
Not done: auto verify (the Supervisor starts it), merge; `html-report` pages and the companion score not made
Paused: auto-mode check failed — Risk Medium: decision 1 (`FACT`) is a choice — its FACT has no Citation (`path:line "quoted text"`) — so it must be `DEBATED` with exactly 2 Advocates named; Risk Medium: decision 2 (`SAFER OPTION`) is a choice — it is flagged ⚠ — so it must be `DEBATED` with exactly 2 Advocates named; Risk Medium: decision 3 (`SAFER OPTION`) is a choice — it is flagged ⚠ — so it must be `DEBATED` with exactly 2 Advocates named; Risk Medium: decision 4 (`SAFER OPTION`) is a choice — it is flagged ⚠ — so it must be `DEBATED` with exactly 2 Advocates named; Risk Medium: decision 5 (`SAFER OPTION`) is a choice — it is flagged ⚠ — so it must be `DEBATED` with exactly 2 Advocates named; Risk Medium: decision 6 (`SAFER OPTION`) is a choice — it is flagged ⚠ — so it must be `DEBATED` with exactly 2 Advocates named [report 5cdf97af8654]


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/account_ignore_agent_run.rs` (8 tests over the real binary, temp `HOME` / `EASY_PERSONAL_HOME` / `EASY_PERSONAL_AGENT_ROOT`, a shell-script fake `claude` first on `PATH`): AC 1 `ignoring_the_agent_run_account_removes_the_flag_and_the_setting_is_off`; AC 2 `the_account_may_be_named_by_a_padded_id`, `the_account_may_be_named_by_its_folder`; AC 3 `another_accounts_flag_stays`, `the_default_accounts_empty_flag_stays`, `no_flag_stays_no_flag`, `a_failed_ignore_leaves_the_flag`; AC 4 `a_run_already_going_stops_at_its_next_poll_after_the_terminal_ignore` (`agent-run now` in one process, `account ignore 1` in another: the first prints `investigate: stopped…`, exits non-zero, `claude` and its child are gone, the flag is gone) |
| Verification command run | ☑ pass | 2026-11-17 03:38 UTC, `cd rust && cargo test -p easy-personal-rs agent`: unit `test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 190 filtered out`, `tests/account_ignore_agent_run.rs` `1 passed; 7 filtered out` (the filter matches one name; the file alone, 03:37 UTC: `test result: ok. 8 passed; 0 failed`), `tests/agent_run_routes.rs` `2 passed`, every other target `ok`. `cargo clippy --workspace --all-targets --features window -- -D warnings` (after the commit, 2 m 50 s): `Finished \`dev\` profile … target(s) in 2m 50s`, no warning. `cargo fmt --check` exit 0 (before and after the comment edit). Leak gate `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py` from `rust/`, binary rebuilt at the final code commit `40d87cd` (03:46 UTC): `leak check: 0 hits over 42 canaries, 866 surfaces, 2023148 bytes`, no failed check line |
| Negative cases hold | ☑ pass | Another account's flag (`2`), the default account's empty flag, no flag, and an unknown account `99` (exit non-zero, flag `1` stays, status `agent run: on`): the four tests above, all green |
| verify | ☐ pass / ☐ fail / ☐ N/A | not run: the Supervisor starts the auto verify run |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Stage 4 2026-11-17 read `cli_cmds.rs` `run_account`, `Runner::system`/`forget`, `accounts::ignore`/`find`, the new test file. Blast radius for the reviewer: one call in `rust/crates/app/src/cli_cmds.rs` `AccountAction::Ignore` (one dependent, `run_account`); the poll loop in `agent_run.rs` and the route in `writes.rs` are untouched |
| Full smoke suite still green (no regression) | ☑ pass (one known environment failure) | 03:41-03:46 UTC, `cd rust && cargo test --workspace --no-fail-fast`: 95 targets, 1065 passed, 1 failed target: `-p easy-personal-rs --test connector_cli_parity` (`every_connector_and_doctor_step_equals_the_python_cli`: the installed `.deb` puts `easy-personal-launch` on `PATH`; T091's and T090's review files record the same; fixed by T097, not on this branch). App unit tests `test result: ok. 218 passed; 0 failed`. The two known load flakes (`memgraph_parity two_hundred_thousand…`, the leak gate's store check) did not fail |
| **Companion score (easy-verifier)** | ☐ N/A | not run (Stage 4/5) |
| **UI: Visual regression (diff or verdict pasted)** | ☐ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☐ N/A | no UI |

---

## Demonstration

**BEFORE** (backend-developer, 2026-11-17 03:36 UTC, branch `T099`; the new test file existed and no implementation commit did; `cli_cmds.rs` unchanged; a throw-away home per test, the flag names account 1 `work`, then `account ignore 1`):
```
$ date -u; cd rust && cargo test -p easy-personal-rs --test account_ignore_agent_run
Tue Nov 17 03:36:10 AM UTC 2026
running 8 tests
test the_default_accounts_empty_flag_stays ... ok
test no_flag_stays_no_flag ... ok
test the_account_may_be_named_by_its_folder ... FAILED
test the_account_may_be_named_by_a_padded_id ... FAILED
test another_accounts_flag_stays ... ok
test a_failed_ignore_leaves_the_flag ... ok
test ignoring_the_agent_run_account_removes_the_flag_and_the_setting_is_off ... FAILED
test a_run_already_going_stops_at_its_next_poll_after_the_terminal_ignore ... FAILED

---- ignoring_the_agent_run_account_removes_the_flag_and_the_setting_is_off stdout ----
thread '…' panicked at crates/app/tests/account_ignore_agent_run.rs:149:5:
assertion `left == right` failed: the flag is gone
  left: Some("1\n")
 right: None
---- the_account_may_be_named_by_a_padded_id stdout ----
assertion `left == right` failed
  left: Some("1\n")
 right: None
---- a_run_already_going_stops_at_its_next_poll_after_the_terminal_ignore stdout ----
the run was not stopped

test result: FAILED. 4 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.06s
```
(the line numbers 149, 159, 170 are those of the file at that commit; the 4 passing tests are the negative cases)

**AFTER** (same command, branch `T099` at `40d87cd`, 2026-11-17 03:37 UTC):
```
$ date -u; cd rust && cargo test -p easy-personal-rs --test account_ignore_agent_run
Tue Nov 17 03:37:20 AM UTC 2026
running 8 tests
test the_default_accounts_empty_flag_stays ... ok
test no_flag_stays_no_flag ... ok
test a_failed_ignore_leaves_the_flag ... ok
test the_account_may_be_named_by_a_padded_id ... ok
test the_account_may_be_named_by_its_folder ... ok
test another_accounts_flag_stays ... ok
test ignoring_the_agent_run_account_removes_the_flag_and_the_setting_is_off ... ok
test a_run_already_going_stops_at_its_next_poll_after_the_terminal_ignore ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

**DELTA**: `easy-personal-rs account ignore <account>` of the agent run account now turns the agent run off (the flag is removed, `agent-run status` says off, a run going stops at its next poll), as the dashboard's ignore route already did.

**WITNESS**: backend-developer, 2026-11-17 03:36-03:46 UTC, in `easy-personal-mcp-wt/T099`; the Supervisor's `memory/event-trace/T099.jsonl` is the witness record, not read here.
