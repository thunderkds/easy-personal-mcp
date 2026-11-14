# TASK_REVIEW — T066: Retire Python follow-ups (connector input, legacy/doctor "claude cannot run", keychain order)

> Sibling of `tasks/TASK_GUIDE_T066.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | (filled by backend-developer, 2026-11-14, on `5d5bc0c`; reviewer to confirm) AC1 `rust/crates/app/tests/connector_cli.rs::a_piped_secret_keeps_a_trailing_cr_as_python_getpass_does`; AC2 `connector_cli.rs::with_no_dev_tty_a_terminal_stdin_has_echo_off_during_the_read_and_back_after`, `::ctrl_c_with_a_terminal_stdin_restores_echo_and_writes_nothing`, `rust/crates/app/src/connector_cmd.rs` `tests::a_failing_tcsetattr_is_refused_and_nothing_is_read`, `::echo_is_off_during_the_read_and_restored_after`, `::a_pipe_is_not_a_terminal`; AC3 `connector_cli.rs::adopt_legacy_says_when_claude_could_not_run_and_adopts_nothing`, `rust/crates/app/tests/doctor_cli.rs::a_claude_that_answers_no_entry_keeps_the_old_lines`; AC4 `doctor_cli.rs::a_claude_that_cannot_run_is_one_fail_line_never_no_entry` (replaces T060's per-entry test); AC5 `connector_cli.rs::script_on_path` (both pty tests skip with a printed reason when `script` is absent); AC6 `rust/crates/app/src/connector_writes.rs` `tests::a_store_refusal_at_the_row_delete_keeps_the_keychain_item`, `::a_keychain_refusal_keeps_the_row`, `::a_remove_deletes_the_row_and_the_item`, `rust/crates/app/src/claude_cli.rs` `tests::a_timeout_kills_claudes_whole_process_group`. Output: AFTER below |
| Verification command run | ☐ pass / ☐ fail | (backend-developer, 2026-11-14, `--offline`, reviewer to re-run) `cargo test --workspace --no-fail-fast` on `3e8f98a` + fmt: exit 0, `84 suites, 896 passed, 0 failed, 29 ignored` (main: 882); `cargo clippy --workspace --all-targets -- -D warnings` exit 0; `--features window` exit 0 (`Finished`); `cargo fmt --check` exit 0; `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py`: `leak check: 0 hits over 42 canaries, 778 surfaces, 1754081 bytes`; `git diff --stat main...HEAD -- rust/Cargo.toml rust/Cargo.lock src tests web rust/parity`: empty |
| Negative cases hold | ☐ pass / ☐ fail | |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured by backend-developer on branch T066 after the new tests were written and before any implementation change; each new AC test fails on the code of `main`, `/usr/bin/script` is hard-coded on `main`):

```
date: 2026-11-14T15:53:22Z  commit: ba4d307 (tests only, no implementation change)
$ grep -rn /usr/bin/script crates  # on main (git show main:...)
89:    let mut cmd = Command::new("/usr/bin/script");
$ cargo test -p easy-personal-rs --test connector_cli
test a_piped_secret_keeps_a_trailing_cr_as_python_getpass_does ... FAILED
test adopt_legacy_says_when_claude_could_not_run_and_adopts_nothing ... FAILED
test the_sync_command_the_account_hint_names_exists ... ok
test the_secret_is_never_in_a_claude_argument_stdout_stderr_or_the_store ... ok
test a_claude_that_cannot_run_removes_and_forgets_nothing ... ok
test with_no_terminal_and_no_stdin_every_question_is_answered_at_once ... ok
test a_secret_typed_at_a_terminal_is_not_echoed_and_goes_to_the_keychain_only ... ok
test a_name_the_rules_refuse_reaches_neither_the_keychain_nor_claude ... ok
test ctrl_c_at_the_secret_prompt_ends_with_pythons_text_and_writes_nothing ... ok
test ctrl_c_with_a_terminal_stdin_restores_echo_and_writes_nothing ... FAILED
test with_no_dev_tty_a_terminal_stdin_has_echo_off_during_the_read_and_back_after ... FAILED
thread 'a_piped_secret_keeps_a_trailing_cr_as_python_getpass_does' (91776) panicked at crates/app/tests/connector_cli.rs:442:9:
  left: "abc"
 right: "abc\r"
thread 'adopt_legacy_says_when_claude_could_not_run_and_adopts_nothing' (91778) panicked at crates/app/tests/connector_cli.rs:605:5:
  left: Some(0)
 right: Some(1)
thread 'ctrl_c_with_a_terminal_stdin_restores_echo_and_writes_nothing' (91780) panicked at crates/app/tests/connector_cli.rs:541:13:
echo was never turned off on the terminal stdin; stderr: "Warning: Password input may be echoed.\nBot token (xoxb-...): "
thread 'with_no_dev_tty_a_terminal_stdin_has_echo_off_during_the_read_and_back_after' (91836) panicked at crates/app/tests/connector_cli.rs:541:13:
echo was never turned off on the terminal stdin; stderr: "Warning: Password input may be echoed.\nBot token (xoxb-...): "
test result: FAILED. 7 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.05s
$ cargo test -p easy-personal-rs --test doctor_cli
test a_missing_store_is_refused_and_nothing_is_created ... ok
test a_claude_that_cannot_run_is_one_fail_line_never_no_entry ... FAILED
test a_keychain_that_refuses_a_read_is_a_fail_line ... ok
test a_claude_that_answers_no_entry_keeps_the_old_lines ... ok
test doctor_reads_only ... ok
thread 'a_claude_that_cannot_run_is_one_fail_line_never_no_entry' (91989) panicked at crates/app/tests/doctor_cli.rs:78:5:
  left: ["FAIL discord: `claude mcp get` could not run (it did not start or timed out); not checked", "FAIL gmail: `claude mcp get` could not run (it did not start or timed out); not checked", "FAIL sendgrid: `claude mcp get` could not run (it did not start or timed out); not checked", "FAIL slack: `claude mcp get` could not run (it did not start or timed out); not checked", "FAIL teams: `claude mcp get` could not run (it did not start or timed out); not checked", "FAIL Claude work acme-slack: `claude mcp get` could not run (it did not start or timed out); not checked"]
 right: ["FAIL claude: `claude` could not run (it did not start or timed out); entries not checked"]
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
$ cargo test -p easy-personal-rs --bin easy-personal-rs -- connector_writes::tests claude_cli::tests
test connector_writes::tests::a_store_refusal_at_the_row_delete_keeps_the_keychain_item ... FAILED
test claude_cli::tests::a_timeout_kills_claudes_whole_process_group ... FAILED
thread 'connector_writes::tests::a_store_refusal_at_the_row_delete_keeps_the_keychain_item' (92206) panicked at crates/app/src/connector_writes.rs:740:9:
assertion `left == right` failed: no secret without its row
  left: None
 right: Some("{}")
thread 'claude_cli::tests::a_timeout_kills_claudes_whole_process_group' (92177) panicked at crates/app/src/claude_cli.rs:298:13:
grandchild 92199 still runs
test result: FAILED. 11 passed; 2 failed; 0 ignored; 0 measured; 116 filtered out; finished in 5.51s
```

**AFTER** (backend-developer, same tests on the branch after the implementation commits):

```
date: 2026-11-14T16:05:03Z  commit: 5d5bc0c
$ grep -rn /usr/bin/script rust/ --exclude-dir=target
(exit 1: no hit)
$ cargo test -p easy-personal-rs --test connector_cli --test doctor_cli
test ctrl_c_with_a_terminal_stdin_restores_echo_and_writes_nothing ... ok
test the_secret_is_never_in_a_claude_argument_stdout_stderr_or_the_store ... ok
test a_claude_that_cannot_run_removes_and_forgets_nothing ... ok
test the_sync_command_the_account_hint_names_exists ... ok
test a_piped_secret_keeps_a_trailing_cr_as_python_getpass_does ... ok
test adopt_legacy_says_when_claude_could_not_run_and_adopts_nothing ... ok
test with_no_dev_tty_a_terminal_stdin_has_echo_off_during_the_read_and_back_after ... ok
test a_secret_typed_at_a_terminal_is_not_echoed_and_goes_to_the_keychain_only ... ok
test ctrl_c_at_the_secret_prompt_ends_with_pythons_text_and_writes_nothing ... ok
test with_no_terminal_and_no_stdin_every_question_is_answered_at_once ... ok
test a_name_the_rules_refuse_reaches_neither_the_keychain_nor_claude ... ok
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test a_missing_store_is_refused_and_nothing_is_created ... ok
test a_claude_that_cannot_run_is_one_fail_line_never_no_entry ... ok
test a_keychain_that_refuses_a_read_is_a_fail_line ... ok
test a_claude_that_answers_no_entry_keeps_the_old_lines ... ok
test doctor_reads_only ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
$ cargo test -p easy-personal-rs --bin easy-personal-rs -- connector_writes::tests claude_cli::tests connector_cmd::tests
test connector_cmd::tests::a_failing_tcsetattr_is_refused_and_nothing_is_read ... ok
test connector_cmd::tests::echo_is_off_during_the_read_and_restored_after ... ok
test connector_cmd::tests::a_pipe_is_not_a_terminal ... ok
test connector_writes::tests::a_keychain_refusal_keeps_the_row ... ok
test connector_writes::tests::a_remove_deletes_the_row_and_the_item ... ok
test connector_writes::tests::a_store_refusal_at_the_row_delete_keeps_the_keychain_item ... ok
test claude_cli::tests::a_timeout_kills_claudes_whole_process_group ... ok
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 116 filtered out; finished in 0.52s
$ git diff --stat main...HEAD -- rust/Cargo.toml rust/Cargo.lock src tests web rust/parity
(empty)
```

**DELTA**: per AC: (1) a piped secret `abc\r\n` is stored as `abc\r`, as Python stores it; (2) with no `/dev/tty` but a terminal as stdin, echo is off during the read and back after (also after SIGINT), and a failing `tcsetattr` is refused with a fixed text before anything is read; (3) `adopt-legacy` with a `claude` that cannot run says so and adopts nothing, and the doctor note says the legacy check did not run; (4) `doctor` prints one `FAIL claude` line instead of 6; (5) no test hard-codes `/usr/bin/script`; (6) a store refusal at remove keeps the keychain item (Python loses it), a keychain refusal keeps the row, and a timed-out `claude` leaves no grandchild running.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T066.jsonl`, never the
implementing agent alone]
