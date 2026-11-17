# TASK_REVIEW — T106: Write sessions 4 — answer bridge

> Sibling of `tasks/TASK_GUIDE_T106.md`. Stage 3 parts only (closing checks, 2026-11-17). The Stage 4/5 rows are not filled here.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (AC 1-9) | `rust/crates/app/src/hooks_cmd.rs` `mod tests` (AC 1, 2: `install_adds_exactly_two_entries_and_keeps_everything_else`, `the_block_shown_is_the_block_merged`, `a_file_that_cannot_be_merged_is_refused_and_left_as_it_is`, `installing_twice_adds_nothing`, `remove_drops_only_equal_entries_and_a_hand_edited_one_stays`, `a_gone_binary_reads_as_moved_and_install_replaces_the_entries`, `half_installed_is_changed_by_hand_and_install_completes_it`, `the_owners_hooks_are_never_taken_for_ours`, `a_symlinked_file_is_written_through_and_its_mode_kept`, `a_path_with_a_space_or_quote_is_quoted_and_read_back`); `rust/crates/app/src/answer_bridge.rs` `mod tests` (AC 4-8: `the_client_hands_back_for_no_socket_a_stale_one_and_another_owner`, `an_answer_reaches_the_waiting_hook_as_the_documented_json`, `the_two_hook_outputs_have_the_documented_shape`, `a_question_is_cleaned_and_capped_for_the_page_and_kept_raw_for_the_reply`, `a_secret_shaped_question_is_hidden_but_still_answerable`, `a_permission_shows_the_tool_and_one_line_and_no_other_field`, `a_permission_without_a_tool_use_id_is_keyed_by_tool_and_input_hash`, `a_release_ends_the_wait_with_no_reply`, `the_wait_running_out_hands_back_and_the_item_goes`, `the_app_stopping_mid_wait_hands_back_and_removes_the_socket`, `malformed_and_foreign_input_gets_no_reply_and_no_card`, `two_sessions_never_cross_and_the_first_answer_wins`, `the_socket_is_private_and_a_stale_one_is_replaced`, `only_a_documented_hook_output_is_printable`, `the_bridge_prints_no_prompt_text`); `rust/crates/app/src/live_feed.rs` `mod tests` (AC 5, 9: `the_stream_adds_pending_answer_to_the_session_whose_hook_waits`, `a_hook_of_another_session_is_not_shown_on_this_one`, `a_change_of_the_question_text_alone_sends_a_snapshot`); `rust/crates/core/src/live.rs` `mod tests` (`clean_ask_hides_a_secret_shaped_question_and_cuts_the_rest`, `clean_line_makes_one_capped_line_without_hidden_characters`, `clean_typed_keeps_line_breaks_and_cuts_to_the_limit`); `rust/crates/app/tests/answer_bridge.rs` (AC 1-9 over the real binary in temp homes: `hooks_install_status_and_remove_keep_every_other_key` (also the fixed restart line), `a_settings_file_that_is_not_json_is_refused_byte_identical`, `install_without_an_account_uses_the_default_one_and_never_makes_the_folder`, `hook_ask_with_no_app_is_silent_exit_zero_and_fast` (20 timed runs), `hook_ask_hands_back_for_a_stale_socket_bad_input_and_a_socket_that_is_not_private`, `an_answer_from_the_app_reaches_the_waiting_hook_and_leaks_nowhere`, `the_app_exiting_mid_wait_hands_the_hook_back_to_the_terminal`, `with_the_bridge_off_serve_makes_no_socket_and_turning_it_on_opens_one`); `rust/parity/leak_rust.py` `bridge()` / `bridge_scan()` (AC 9) |
| Verification command run | ☑ pass | 2026-11-17, T106 worktree, `cd rust`. `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings` and `cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings`: `Finished \`dev\` profile` with no warning. `cargo test --workspace --no-fail-fast` (last run, after the fixes below): `97 targets, passed 1149 failed 0 ignored 28`, exit 0. Leak gate (venv of the main checkout, `CLAUDE_CONFIG_DIR` unset): `--self-test` → `self-test: ok (raw, json, \u, url, base64, base64url at 3 alignments, hex; sqlite; masking)`; full run exit 0: `check bridge socket is 0600 in a 0700 folder   ok`, `check bridge pending_answer on GET /api/live    ok (2 hooks)`, `check bridge hook ask: exit 0, documented JSON  ok`, `check bridge second answer is a 409             ok (status 409)`, `check bridge socket removed at SIGINT           ok (exit 0)`, `check cli commands all registered   ok (25 commands)`, `check T106 bridge strings seen where allowed  ok (0 missing)`, `check T106 bridge strings scanned            ok (8 strings, 1030 + 1 + 2 surfaces)`, `leak check: 0 hits over 42 canaries, 1030 surfaces, 2305410 bytes`. Not run: the owner's real-terminal smoke (guide checklist) |
| Negative cases hold | ☑ pass | Found and fixed by this closing round. (1) `store_health_route::the_route_writes_nothing_and_names_no_folder` failed 4 of 4 (3 alone): `a file changed or appeared`. Cause: `serve` now reads the store at start (`hooks_cmd::any_on`) and a read-only open of a `-wal` store with no `-shm` creates the `-shm`. Fix: `any_on` reads as off when `-wal` exists and `-shm` does not (a store nobody has open); `PUT /api/hooks` opens the socket later and until then every hook is handed back (fail open). After: `test result: ok. 10 passed; 0 failed`. (2) Leak gate crashed in the new step: `ValueError: flush of closed file` (`communicate` on a hook process whose `stdin` was closed); fixed by `stdin = None` after the close. (3) Leak gate then reported `unregistered ['hook', 'hooks']` (added to `CLI_ELSEWHERE`, they are driven by `bridge()`) and 6 HITs of `bridge_header`, `bridge_label_other`, `bridge_description` on `hook ask` stdout. Not a leak: the documented PreToolUse output returns the tool's own input (`updatedInput`, `answer_bridge.rs` `pre_tool_use_output`) to Claude Code, so those strings belong there; `BRIDGE_HOOK` now says so, and the permission command stays forbidden on that stdout. Before the fixes: `check cli commands all registered   FAILED`, `7 failed checks`; after: `leak check: 0 hits`. (4) Flakes in code this task does not touch, each reported as seen: `core jobs_parity::a_crontab_of_ten_thousand_lines_with_hostile_lines_is_listed_fast` failed once under build load in the first agent's run and passed in all three runs here; `core memgraph_parity::two_hundred_thousand_links_are_counted_in_time` failed once here (`took 6.448224287s`, limit 5 s, while the workspace built) and passed alone (`test result: ok. 1 passed; ... finished in 4.12s`) and in both later full runs |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Full smoke suite still green (no regression) | ☑ pass | `cargo test --workspace --no-fail-fast`: 97 targets, 1149 passed, 0 failed, 28 ignored (default build, last run); leak gate 0 hits |

---

## Demonstration

**BEFORE** (captured 2026-11-17 on `main` at `637cf2c`, before the T106 implementation commit): `easy-personal-rs hooks` is an unknown command, and so is `hook`; the app crate has no code for them:

```
$ git show 637cf2c:rust/crates/app/src/main.rs | grep -c hooks_cmd
0
```

**AFTER** (2026-11-17T12:15:16Z, debug build of the T106 worktree, `HOME` and `EASY_PERSONAL_HOME` a temp folder):

```
$ easy-personal-rs hooks --help
Answer terminal sessions from the app: add or remove the two Claude Code hook entries of an account (off by default; add-only)

Usage: easy-personal-rs hooks <COMMAND>

Commands:
  install  Show the two entries, ask, then add them to an account's user settings.json (add-only; the old file is copied to backups/ first)
  remove   Remove the entries `install` added (an entry changed by hand is kept)
  status   Print on, off, changed by hand or binary moved for each confirmed Claude account (or one)
  help     Print this message or the help of the given subcommand(s)
$ easy-personal-rs hook ask </dev/null      # no app running
(no output) exit=0
```

`hooks install` also prints, after the entries and the "sessions already running are not covered" line: `A running app picks this up after a restart or when Settings turns the bridge on.` (test `hooks_install_status_and_remove_keep_every_other_key`).

**DELTA**: the CLI gains `hooks install|remove|status` and `hook ask`, and the app gains `GET`/`PUT /api/hooks`, `POST /api/answer` and `pending_answer` on `GET /api/live`. With the bridge off nothing new is created in the data folder; with it on, an answer typed in the app reaches the waiting terminal session as the documented hook JSON, and the planted strings reach only `GET /api/live` and `hook ask`'s stdout.
