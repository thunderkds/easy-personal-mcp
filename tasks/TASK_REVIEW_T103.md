# TASK_REVIEW — T103: Write sessions 1, app session runner and routes

> Sibling of `tasks/TASK_GUIDE_T103.md`. Stage 3 parts only (the Supervisor fills Stage 4/5).

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (AC 1-9) | `rust/crates/app/src/app_session.rs` `mod tests` (18: AC 1, 2 `the_argv_holds_fixed_words_...`, `the_child_environment_drops_billing_...`, `a_session_runs_the_fake_with_the_argv_the_temp_config_...`; AC 3 `a_worktree_is_made_on_a_new_easy_branch_...`; AC 4 `interrupt_is_a_control_request_on_stdin_...`, `stop_is_sigterm_to_the_group_and_sigkill_after_the_grace_...`, `a_child_that_obeys_sigterm_ends_at_once`, `a_fifth_concurrent_session_is_refused_...`; AC 5 `the_stream_gives_text_tool_labels_and_outcomes_...`, `a_result_of_a_local_command_...`; AC 6 `a_prompt_waits_for_the_owner_and_the_retry_...`, `deny_gives_the_note_and_a_question_is_answered_...`, `the_end_of_a_turn_or_of_the_session_cancels_...`, `a_summary_is_one_clean_line_...`; AC 9 `text_is_dropped_when_an_ended_session_is_closed_and_all_of_it_at_shutdown`; edges `no_claude_and_a_child_that_exits_at_once_...`, `turning_the_setting_off_stops_...`, `the_owners_text_is_sent_as_typed_...`); `rust/crates/app/src/app_session_routes.rs` `mod tests` (14: AC 1 `start_answers_202_...`, `a_shell_text_an_option_an_unknown_model_or_mode_and_a_path_outside_the_store_are_400_...`, `the_setting_off_is_403_...`; AC 2 `the_child_gets_no_billing_variable_...`; AC 3 `a_worktree_session_runs_on_its_own_branch_...`; AC 4 `message_interrupt_and_stop_reach_the_child_...`, `a_fifth_concurrent_session_is_409_...`; AC 5 `the_session_shows_text_labels_and_outcomes_...`; AC 6 `a_prompt_call_waits_two_seconds_with_no_answer_then_the_retry_gets_the_one_answer`, `a_deny_note_and_a_question_answer_go_back_...`, `the_prompt_route_needs_the_sessions_own_bearer_...`; AC 7 `every_route_needs_the_token_a_good_host_and_origin_and_each_write_needs_confirmed_true`; AC 9 `close_drops_an_ended_sessions_text_...`; edge `turning_the_setting_off_ends_a_going_session`); AC 8 `rust/parity/leak_rust.py` (`app_sessions`, `session_scan`, routes in `API_CALLS` / `GUARD_CALLS` / `WRITE_CALLS`) |
| Verification command run | ☑ pass | 2026-11-17, T103 worktree, `cd rust`: `cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings`: `Finished` with no warning; `cargo test --workspace --no-fail-fast`: exit 0, 96 targets, 1138 passed, 0 failed (app unit tests `test result: ok. 290 passed; 0 failed`, of which 32 are `app_session`); leak gate `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py`: `leak check: 0 hits over 42 canaries, 1029 surfaces, 2357330 bytes` with `check T103 app session strings scanned ok (11 strings, 999 surfaces, 30 guarded answers allowed to hold them)` |
| Negative cases hold | ☑ pass | Injection: `;`, `--...`, `$(id)`, backtick, `../repo`, empty and wrong-typed text as repo / workspace / model / mode / account, an unknown model, mode, account, a path outside the store, a missing folder: 400 each and the fake `claude` never started (`a_shell_text_an_option_...`). No `confirmed` (missing, false, "true", 1): 400 on every write; no token / wrong token / wrong Host / foreign Origin / token in the query: 403 and nothing started or stopped. Prompt route: no bearer, wrong bearer, another session's bearer, page token only: 401; GET / DELETE / PUT: 405. SIGINT never sent: the fake is alive after an interrupt. A child that ignores SIGTERM is ended by SIGKILL after the grace. Leak negative control: with the scan's allow-list switched off the same run reports 238 failed checks (`HIT T103 session_answer / session_assistant / session_card_command / ...`), so the scan does see the planted text |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pending | Supervisor, Stage 4 |
| Full smoke suite still green (no regression) | ☑ pass | workspace 1138 passed, 0 failed (T101 baseline 1104 plus the new tests); the one edited old test is `tests/agent_run_routes.rs` (`serve --help` now says "Not read-only for two features") |
| **Companion score (easy-verifier)** | ☑ N/A | not run in Stage 3 |
| **UI: Visual regression** | ☑ N/A | no UI in this task (T104) |
| **UI: Design-system compliance** | ☑ N/A | no UI in this task |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI in this task |

---

## Demonstration

**BEFORE** (captured 2026-11-17T11:50:18Z on branch T103 at commit b7c05b3, before any implementation commit; default-feature debug build; `EASY_PERSONAL_HOME` a fresh empty folder):

```
2026-11-17T11:50:18Z
$ easy-personal-rs serve      # dashboard: http://127.0.0.1:32775/#<token>
$ curl -s -o /dev/null -w "%{http_code}\n" -H "X-Dashboard-Token: <token>" http://127.0.0.1:32775/api/app-sessions
404
$ curl -s -o /dev/null -w "%{http_code}\n" -X POST -H "Content-Type: application/json" -H "X-Dashboard-Token: <token>" -d '{}' http://127.0.0.1:32775/api/app-sessions
404
$ curl -s -H "X-Dashboard-Token: <token>" http://127.0.0.1:32775/api/agent-run
{"account":null,"claude_found":true,"enabled":false,"last":null,"running":false} 200
```

**AFTER** (2026-11-17T12:27:26Z, HEAD of T103 after the implementation commits, same debug build, temp `EASY_PERSONAL_HOME`, an agent run account on (`agent-run on`), a stand-in `claude` on `PATH`; paths shortened):

```
$ GET /api/app-sessions
{"max":4,"sessions":[]} [200]
$ POST /api/app-sessions (no confirmed)
{"error":"confirmed must be true"} [400]
$ POST /api/app-sessions (shell text as repo "; touch /tmp/x")
{"error":"repo must be a repo of the workspace"} [400]
$ POST /api/app-sessions {repo, workspace, model:"opus", mode:"plan", worktree:false, prompt:"hello", confirmed:true}
{"id":"b53d57b9ada514cd"} [202]
$ GET /api/app-sessions/<id>
{"account":null,"branch":null,"changed_files":[],"folder":"repo","id":"b53d57b9ada514cd","items":[{"kind":"user","text":"hello"},{"kind":"assistant","text":"you said: hello"},{"kind":"tool","label":"ls","name":"Bash","outcome":"ok"}],"mode":"plan","model":"opus","reason":null,"session_id":"demo-session-1","state":"idle","worktree":false} [200]
$ POST .../message {"text":"/ping"}  -> 200 ; POST .../interrupt -> 200 ; POST .../stop -> 200
$ GET /api/app-sessions/<id>   (after the stop)  ...,"reason":"stopped",... (6 items: both turns)
argv the stand-in claude got:
-p --input-format stream-json --output-format stream-json --verbose --model opus --permission-mode plan --permission-prompt-tool mcp__easy-personal-app__permission_prompt --mcp-config <tmp>/demo/home/app-sessions/b53d57b9ada514cd.mcp.json
$ GET /api/app-sessions with no token -> 403
```

The tool input `ls -la` of the stand-in's tool call is in none of the answers (only its label `ls`). Prompt cards, the HTTP permission route and the leak check are in the tests and in `parity/leak_rust.py` (`check app session ...` lines above), not in this curl demo.

**DELTA**: the server now starts the owner's own `claude` as a session the page can talk to (start, message, interrupt, stop, close, answer a permission prompt) over guarded `/api/app-sessions` routes, with its permission prompts served by a private loopback route of the same process; before, those paths were 404 and the app only ran the fixed five-tool agent job.

**WITNESS**: backend-developer agent, 2026-11-17 (Demonstration AFTER at 12:27:26Z; verification command and leak gate on the T103 worktree). A run with the real `claude` is T104's AC; nothing here started it.
