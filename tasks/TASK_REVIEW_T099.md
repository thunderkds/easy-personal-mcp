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
1. The id passed to `forget` is the row's `id` from the body `accounts::ignore` returned, as a number; the body always carries it, so no stop was needed — FACT: `rust/crates/core/src/accounts.rs:414` "\"id\": account.id, \"provider\": account.provider," (the guide's "if the body does not carry the id, stop" did not apply)
2. AC 2 says `account ignore <label>`, but `account ignore` takes an id or a folder path, never a label, so the two tests are by id (plain `1`, and a padded `01`, which is not the flag's text) and by folder path (never equal to an id); no label test — SAFER OPTION: the command cannot take a label (`rust/crates/core/src/accounts.rs:421` "`_find`: the account id for an id (plain digits) or a folder path; 404 `no such account`."), and adding label lookup to `ignore` would be a new feature ⚠ decided for you — please review
3. The call is `Runner::system()` then `forget(&id.to_string())`, not `agent_routes::account_gone`: `account_gone` needs the data folder, which the CLI would have to derive from the store path in three lines that `Runner::system()` already holds; the guide names both; an error from either (no store path, no program path, flag I/O) is ignored, as the route ignores it — SAFER OPTION: one existing constructor, no new function or visibility change ⚠ decided for you — please review
4. `forget` runs after `write(...)` returned, so after the ignore's transaction committed (the route calls it inside its closure, also after the commit); a failed ignore returns through `?` before the call, so the flag stays — SAFER OPTION: the flag is never removed for an ignore that did not happen ⚠ decided for you — please review
5. The tests are an integration test file over the real binary (`rust/crates/app/tests/account_ignore_agent_run.rs`) with a shell-script fake `claude` like the one of `agent_run_routes.rs`, not `agent_run::tests::Fake` (AC 4 names it): `Fake` is `cfg(test)` and `pub(crate)` inside the binary crate, and the ignore must come from a second process with its own environment; the in-process route would need `set_var` in a threaded test run — SAFER OPTION: no environment mutation in a shared test process; the fake never runs the real `claude` ⚠ decided for you — please review
6. The Verification Command's filter `agent` matches 1 of the 8 new tests by name (`ignoring_the_agent_run_account_…`); the other 7 ran through the file name and the full workspace run — SAFER OPTION: not renaming tests to fit a filter; the whole file is in the Evidence row ⚠ decided for you — please review
Checks: tests `cd rust && cargo test --workspace --no-fail-fast` — 95 targets, 1065 passed, 1 target failed: `connector_cli_parity` (the installed `.deb` puts `easy-personal-launch` on `PATH`; fixed by T097, not on this branch); clippy, fmt, leak gate pasted in Evidence; reviews not run (Stage 4 is the Supervisor's); real run: the tests run the real binary over temp folders with a fake `claude` (auto-checked, not /verify)
Memory: nothing new
Not done: Stage 4 (code review; security review at Risk Medium), merge (yours), auto verify (the Supervisor starts it)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/account_ignore_agent_run.rs` (8 tests over the real binary, temp `HOME` / `EASY_PERSONAL_HOME` / `EASY_PERSONAL_AGENT_ROOT`, a shell-script fake `claude` first on `PATH`): AC 1 `ignoring_the_agent_run_account_removes_the_flag_and_the_setting_is_off`; AC 2 `the_account_may_be_named_by_a_padded_id`, `the_account_may_be_named_by_its_folder`; AC 3 `another_accounts_flag_stays`, `the_default_accounts_empty_flag_stays`, `no_flag_stays_no_flag`, `a_failed_ignore_leaves_the_flag`; AC 4 `a_run_already_going_stops_at_its_next_poll_after_the_terminal_ignore` (`agent-run now` in one process, `account ignore 1` in another: the first prints `investigate: stopped…`, exits non-zero, `claude` and its child are gone, the flag is gone) |
| Verification command run | ☑ pass | 2026-11-17 03:38 UTC, `cd rust && cargo test -p easy-personal-rs agent`: unit `test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 190 filtered out`, `tests/account_ignore_agent_run.rs` `1 passed; 7 filtered out` (the filter matches one name; the file alone, 03:37 UTC: `test result: ok. 8 passed; 0 failed`), `tests/agent_run_routes.rs` `2 passed`, every other target `ok`. `cargo clippy --workspace --all-targets --features window -- -D warnings` (after the commit, 2 m 50 s): `Finished \`dev\` profile … target(s) in 2m 50s`, no warning. `cargo fmt --check` exit 0 (before and after the comment edit). Leak gate `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py` from `rust/`, binary rebuilt at the final code commit `40d87cd` (03:46 UTC): `leak check: 0 hits over 42 canaries, 866 surfaces, 2023148 bytes`, no failed check line |
| Negative cases hold | ☑ pass | Another account's flag (`2`), the default account's empty flag, no flag, and an unknown account `99` (exit non-zero, flag `1` stays, status `agent run: on`): the four tests above, all green |
| verify | ☐ pass / ☐ fail / ☐ N/A | not run: the Supervisor starts the auto verify run |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | Stage 4 not run (Stage 3 report). Blast radius for the reviewer: one call in `rust/crates/app/src/cli_cmds.rs` `AccountAction::Ignore` (one dependent, `run_account`); the poll loop in `agent_run.rs` and the route in `writes.rs` are untouched |
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
