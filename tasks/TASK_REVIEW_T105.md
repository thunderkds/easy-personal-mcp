# TASK_REVIEW — T105: Write sessions 3, Resume in app, fork, Continue (backend half)

> Sibling of `tasks/TASK_GUIDE_T105.md`. Stage 3 parts only (the Supervisor fills Stage 4/5). This file covers AC 1, AC 2 and the Rust part of AC 4; the page buttons (AC 3) come in a second step.

## Auto decisions

Mode: auto
1. The folder of a resumed session is the project folder of its **first** stored record (`events.project_path`), never a page-given path; the page sends no `repo`, `workspace` or `worktree` with `resume` (400). RULE: `PRD.md` FR-112 / FR-116 "the page sends only the form fields"; TASK_GUIDE AC 1 "never a page-given path"
2. "Active" is the newer of the store's last record and the live tail's last record of the session, under 120 s old (`live::RESUME_QUIET_S`, `live::is_recent`, `LiveTail::last_record`): the store lags the tail by one ingest, and a store-only check would let a plain resume into a window that is open. RULE: `PRD.md` FR-112 "a record in the last 2 minutes"; question 41
3. An app session's own Claude session cannot be resumed **or forked** while it is going (409, own fixed message); a plain resume counts as running from its start (its `claude_id` is set at the plan, so two quick clicks cannot start it twice; the check is made under the registry lock in `Sessions::start`). RULE: TASK_GUIDE scope "an app session's own session cannot be resumed while it is still going (409)"
4. `resume` needs the same `prompt` as a new session (the form is unchanged); `account` is the same optional field (default: the agent run account). The page half passes the session's account. A `resume` of an id that exists under another account is a `claude` "no conversation found" end of the app session, not a refusal here
5. `continue: true` with `worktree: true` is 400 (a new worktree has no conversation to continue); `continue` with `resume` or `fork`, and `fork` without `resume`, are 400; `continue: false` and `fork: false` are the same as absent
6. The session id must start with a hex digit (the guide's "hex digits and `-` only" would let `--` through; it is in the guide's own reject list)
Checks: tests `cargo test -p easy-personal-rs --bin easy-personal-rs app_session` 61 passed, 0 failed; workspace 1172 passed, 0 failed over 96 targets; fmt and clippy clean; leak gate 0 hits over 42 canaries, 1017 surfaces; no run with the real `claude` (stand-in only)
Not done: the page buttons (AC 3, second step); merge

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (AC 1, AC 2, AC 4 Rust part) | `rust/crates/app/src/app_session_routes.rs` `mod tests`: AC 1 `a_quiet_session_is_resumed_with_resume_in_its_stored_folder_and_a_page_given_path_is_refused` (argv `--resume <id>`, the child's cwd is the stored folder, 6 page-given path fields are 400), `an_unknown_session_is_404_and_a_folder_that_is_gone_is_400_and_nothing_starts`, `a_session_id_with_a_shell_character_a_space_an_option_or_over_64_characters_is_400_and_nothing_starts` (`a;b`, `ab cd`, `--`, `-ab`, `--resume`, `$(id)`, backtick, newline, `../a`, 65 characters, empty, non-string: 400 with and without fork; 64 characters accepted), `continue_adds_continue_in_a_stored_repo_and_is_never_mixed_with_resume_fork_or_a_worktree`, `the_resume_fields_keep_the_guard_and_the_setting_off_is_403`; AC 2 `a_session_with_a_record_in_the_last_two_minutes_is_409_without_fork_and_forks_with_it` (409 exact message, 119 s active, 121 s quiet, fork argv `--resume <id> --fork-session`), `a_record_only_the_live_tail_has_makes_a_session_active` (a real `LiveTail` over a temp transcript tree); own session `the_session_of_a_going_app_session_cannot_be_resumed_but_can_once_it_has_ended`; `rust/crates/app/src/app_session.rs` `mod tests`: `the_argv_of_a_resume_a_fork_and_a_continue_holds_the_checked_id_and_fixed_words`, `a_resume_id_is_hex_and_dashes_with_a_hex_first_character_and_at_most_64`; `rust/crates/core/src/live.rs` `a_record_is_recent_for_two_minutes` |
| Verification command run | ☑ pass | 2026-11-17, T105 worktree, `cd rust`: `cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings`: `Finished` with no warning; `cargo test -p easy-personal-rs --bin easy-personal-rs app_session`: `test result: ok. 61 passed; 0 failed`; `cargo test --workspace --no-fail-fast`: 96 targets, 1172 passed, 0 failed (the two timing tests passed in the run; alone: `live_parity` 15 passed incl. `pairing_100k_calls_is_fast`, `memgraph_parity` 27 passed incl. `two_hundred_thousand_links_are_counted_in_time`); leak gate `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py`: `check T103 app session strings scanned     ok (11 strings, 999 surfaces, 18 guarded answers allowed to hold them)` and `leak check: 0 hits over 42 canaries, 1017 surfaces, 2358297 bytes` |
| Negative cases hold | ☑ pass | id with `;`, a space, `--`, over 64 characters: 400 and the stand-in `claude` never started (`log/argv` absent); unknown session 404; folder gone or relative 400; a session active in the last 2 minutes without fork 409; an app session's own session 409 (with and without fork); `continue` mixed with `resume` / `fork` / `worktree` 400; no `confirmed` (false, "true", 1) 400, no token or a foreign Origin 403, setting off 403 |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pending | Supervisor, Stage 4. Blast radius: `start` in `app_session_routes.rs` (every start), `Plan` / `argv` / `Sessions::start` in `app_session.rs`, two additive methods on `Api` (`api.rs`), two additive items in `core/src/live.rs` |
| Full smoke suite still green (no regression) | ☑ pass | workspace 1172 passed, 0 failed |
| **Companion score (easy-verifier)** | ☑ N/A | not run in Stage 3 |
| **UI: Visual regression / Design-system / Responsiveness** | ☑ N/A | no UI in this half |

---

## Demonstration

**BEFORE** (captured 2026-11-17T15:39:30Z on branch T105 at commit e34dfc0, before any implementation commit; default-feature debug build; temp `EASY_PERSONAL_HOME`, a stand-in `claude` on `PATH`, agent runs on):

```
$ easy-personal-rs serve      # dashboard: http://127.0.0.1:33413/#<token>
$ POST /api/app-sessions resume (quiet session)    {"resume":"aaaa1111-0001","model":"default","mode":"default","prompt":"hi","confirmed":true}
{"error":"unknown field"} [400]
$ POST /api/app-sessions continue                  {"continue":true,"repo":"<quiet-repo>","workspace":"acme",...,"confirmed":true}
{"error":"unknown field"} [400]
```

**AFTER** (2026-11-17T15:42:09Z, HEAD 513cd79, same debug build, temp `EASY_PERSONAL_HOME` with three stored sessions: `aaaa1111-0001` quiet in `quiet-repo`, `bbbb2222-0002` with a record 30 s ago in `repo`, `cccc3333-0003` quiet in a deleted folder; a stand-in `claude` that logs its argument list and folder; paths shortened):

```
2026-11-17T15:42:09Z
$ easy-personal-rs serve      # dashboard: http://127.0.0.1:39621/#<token>
$ POST /api/app-sessions  resume quiet, hex id aaaa1111-0001
{"id":"374d714b4e0a469f"} [202]
$ POST /api/app-sessions  resume busy (record 30 s ago), no fork
{"error":"this session is still active in another window; fork it instead"} [409]
$ POST /api/app-sessions  resume busy with fork
{"id":"6c03fb01a54df480"} [202]
$ POST /api/app-sessions  resume unknown id
{"error":"no such Claude session"} [404]
$ POST /api/app-sessions  resume, folder deleted
{"error":"the folder of this session no longer exists"} [400]
$ POST /api/app-sessions  resume id "a;b"
{"error":"resume must be a session id: hex digits and - only, at most 64 characters"} [400]
$ POST /api/app-sessions  resume id "--"
{"error":"resume must be a session id: hex digits and - only, at most 64 characters"} [400]
$ POST /api/app-sessions  resume and continue
{"error":"resume and continue cannot be used together"} [400]
$ POST /api/app-sessions  continue
{"id":"b26bb3e82f0715da"} [202]
argv the stand-in claude got (one line per start), and its folder:
-p --input-format stream-json --output-format stream-json --verbose --resume aaaa1111-0001 --permission-prompt-tool mcp__easy-personal-app__permission_prompt --mcp-config <tmp>/home/ep/app-sessions/374d714b4e0a469f.mcp.json
-p --input-format stream-json --output-format stream-json --verbose --resume bbbb2222-0002 --fork-session --permission-prompt-tool mcp__easy-personal-app__permission_prompt --mcp-config <tmp>/home/ep/app-sessions/6c03fb01a54df480.mcp.json
-p --input-format stream-json --output-format stream-json --verbose --continue --permission-prompt-tool mcp__easy-personal-app__permission_prompt --mcp-config <tmp>/home/ep/app-sessions/b26bb3e82f0715da.mcp.json
<tmp>/quiet-repo
<tmp>/repo
<tmp>/quiet-repo
```

**DELTA**: `POST /api/app-sessions` now resumes a stored Claude session in its own stored folder (`--resume`), forks one that is open elsewhere (`--resume ... --fork-session`, 409 without fork), and continues the latest conversation of a stored repo (`--continue`); before, `resume` and `continue` were unknown fields.

**WITNESS**: backend-developer agent, 2026-11-17 (AFTER at 15:42:09Z; verification command and leak gate on the T105 worktree). A run with the real `claude` was not made; the stand-in recorded the argument lists above.
