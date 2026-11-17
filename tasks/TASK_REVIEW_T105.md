# TASK_REVIEW — T105: Write sessions 3, Resume in app, fork, Continue (backend half)

> Sibling of `tasks/TASK_GUIDE_T105.md`. Stage 3 parts only (the Supervisor fills Stage 4/5). First half: AC 1, AC 2 and the Rust part of AC 4. Second half (below, rows marked 2nd half): the page buttons (AC 3), the Vitest part of AC 4, and two backend items B1 and B2.

## Auto decisions

Mode: auto
1. A resumed session runs in the session's own folder: its transcript's start `cwd` when that resolves to the session's project, else the project folder (from the store, else the live tail); the page never names a folder (`repo`, `workspace`, `worktree` with `resume` → 400). RULE: `PRD.md:209` FR-112 "an app session with `--resume <id>` in the session's folder"
2. A session with a record in the last 2 minutes (store or live tail, whichever is newer) is only resumed as a fork; a plain resume answers 409 and the page switches to Fork with the fixed note. RULE: `PRD.md:209` FR-112 "A session with a record in the last 2 minutes offers it only with *fork* (`--fork-session`)"
3. `continue` answers 409 while the repo has a record under 2 minutes old (store or live tail) or a going app session, since `--continue` would attach to an open conversation. RULE: `PRD.md:209` FR-112 "Claude Code: two writers to one transcript lose messages"
4. The request carries only a checked session id (hex digits and `-`, first a hex digit, ≤ 64), flags and the fixed form fields; the argv is fixed words plus that id. RULE: `PRD.md:213` FR-116 "No route lets the page name the program or free arguments"
Checks: tests `cargo test -p easy-personal-rs --bin easy-personal-rs app_session` — 66 passed, 0 failed; `cargo test --workspace --no-fail-fast` — 1177 passed, 0 failed (96 targets); `cd web && npm test` — 1166 passed; fmt and clippy clean; leak gate 0 hits over 42 canaries, 1017 surfaces; reviews code-review + security (independent, 0 P0, 1 P1 + 1 P2 + 3 P3 — fixed in `2b46ad7`, the index note documented); real run: a real `serve` with a stand-in `claude` (Demonstration AFTER) — a real resume needs the owner
Memory: nothing new (the 2-minute rule and the store lag are recorded in memory/learnings.md and rust/README.md)
Not done: merge (admin: after the auto verify pass) — merge after T103 and T104 (this branch already holds both); auto verify: running (T182); owner real run

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (AC 1, AC 2, AC 4 Rust part) | `rust/crates/app/src/app_session_routes.rs` `mod tests`: AC 1 `a_quiet_session_is_resumed_with_resume_in_its_stored_folder_and_a_page_given_path_is_refused` (argv `--resume <id>`, the child's cwd is the stored folder, 6 page-given path fields are 400), `an_unknown_session_is_404_and_a_folder_that_is_gone_is_400_and_nothing_starts`, `a_session_id_with_a_shell_character_a_space_an_option_or_over_64_characters_is_400_and_nothing_starts` (`a;b`, `ab cd`, `--`, `-ab`, `--resume`, `$(id)`, backtick, newline, `../a`, 65 characters, empty, non-string: 400 with and without fork; 64 characters accepted), `continue_adds_continue_in_a_stored_repo_and_is_never_mixed_with_resume_fork_or_a_worktree`, `the_resume_fields_keep_the_guard_and_the_setting_off_is_403`; AC 2 `a_session_with_a_record_in_the_last_two_minutes_is_409_without_fork_and_forks_with_it` (409 exact message, 119 s active, 121 s quiet, fork argv `--resume <id> --fork-session`), `a_record_only_the_live_tail_has_makes_a_session_active` (a real `LiveTail` over a temp transcript tree); own session `the_session_of_a_going_app_session_cannot_be_resumed_but_can_once_it_has_ended`; `rust/crates/app/src/app_session.rs` `mod tests`: `the_argv_of_a_resume_a_fork_and_a_continue_holds_the_checked_id_and_fixed_words`, `a_resume_id_is_hex_and_dashes_with_a_hex_first_character_and_at_most_64`; `rust/crates/core/src/live.rs` `a_record_is_recent_for_two_minutes` |
| Verification command run | ☑ pass | 2026-11-17, T105 worktree, `cd rust`: `cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings`: `Finished` with no warning; `cargo test -p easy-personal-rs --bin easy-personal-rs app_session`: `test result: ok. 61 passed; 0 failed`; `cargo test --workspace --no-fail-fast`: 96 targets, 1172 passed, 0 failed (the two timing tests passed in the run; alone: `live_parity` 15 passed incl. `pairing_100k_calls_is_fast`, `memgraph_parity` 27 passed incl. `two_hundred_thousand_links_are_counted_in_time`); leak gate `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py`: `check T103 app session strings scanned     ok (11 strings, 999 surfaces, 18 guarded answers allowed to hold them)` and `leak check: 0 hits over 42 canaries, 1017 surfaces, 2358297 bytes` |
| Negative cases hold | ☑ pass | id with `;`, a space, `--`, over 64 characters: 400 and the stand-in `claude` never started (`log/argv` absent); unknown session 404; folder gone or relative 400; a session active in the last 2 minutes without fork 409; an app session's own session 409 (with and without fork); `continue` mixed with `resume` / `fork` / `worktree` 400; no `confirmed` (false, "true", 1) 400, no token or a foreign Origin 403, setting off 403 |
| **2nd half: Vitest covers AC 3 and the Vitest part of AC 4 (file paths pasted)** | ☑ pass | `web/src/NewSession.test.jsx` (`resume mode: Resume in app and Fork in app`, 11 tests: no repo or worktree field and the session named by short id and project; Resume posts `{resume, account, model, mode, prompt, confirmed}` and no repo, workspace, worktree or fork; Fork shows `FORK_NOTE` on the form and the confirm step and posts `fork: true`; a 409 "still active" switches to Fork with the note and the next Start sends `fork: true`; the bypass second step survives the switch; 404 `no such Claude session`, 404 `...not stored yet; try again after the next ingest`, 409 running in an app session and 400 folder gone show the server's words with no switch; a 409 on a fork that was asked for does not loop; markup is React text) and `continue mode: Continue in app` (3 tests: repo fixed, no worktree, posts `{continue: true, repo, workspace, ...}`; a 409 for an active folder shows the server's words; the new session body is unchanged); `web/src/AppSessionEntry.test.jsx` (`Live: Resume in app and Fork in app`, 8 tests: quiet at 121 s is Resume with no note, 119 s is Fork with the fixed note, the label follows the clock, an app session's row has neither, no handler or no app session route gives no button, no button before the app sessions are known, a click does not open the row, in-app flows for Resume and Fork; `Workspaces: Continue in app`, 2 tests: one button per repo naming workspace and repo, no handler no button, in-app flow posts `continue: true`) |
| **2nd half: B1 and B2 Rust tests** | ☑ pass | `rust/crates/app/src/app_session_routes.rs` `mod tests`: B1 `a_session_resumes_in_the_first_cwd_of_its_transcript_not_in_its_repo_and_falls_back_to_the_stored_folder` (a subfolder and a worktree start folder is where the child runs; a missing file, a symlink and a transcript with no `cwd` fall back to the stored folder; a start folder that is gone is 400, nothing starts); B2 `continue_is_409_while_any_record_of_the_repo_is_under_two_minutes_old_and_starts_when_quiet` (119 s is 409 with the fixed text, 121 s starts, a record of another folder does not count, a new session in the same folder is not guarded) |
| **2nd half: verification command run** | ☑ pass | 2026-11-17, T105 worktree. `cd web && npx vitest run`: `Test Files 33 passed (33)`, `Tests 1166 passed (1166)` (1141 before this half). `cd rust`: `cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings` no warning; `cargo test -p easy-personal-rs --bin easy-personal-rs app_session`: `63 passed; 0 failed` (61 before); `cargo test --workspace --no-fail-fast`: 96 targets, 1174 passed, 0 failed; the two timing tests passed inside that run (`pairing_100k_calls_is_fast`, `two_hundred_thousand_links_are_counted_in_time`), so no run alone was needed. `npm run build` not run (as told). `node_modules` of the worktree was a temporary symlink to the main checkout's, removed after the run |
| **Stage 4 fix round: verification command run** | ☑ pass | 2026-11-17, T105 worktree. Fixes: P1 `continue` is 409 for a store record, a live-tail record (`LiveTail::last_record_in`, matched on the resolved project folder the tail now keeps) or a going app session in the repo or its worktree base (`Sessions::runs_in`); P2 a session only the tail knows resumes from the tail's transcript and project folder (`LiveTail::session_origin`); P3 the transcript's start `cwd` is used only when `resolve_project` of it equals the project folder (a start folder that is gone stays 400); P3 the continue 409 text is `... open it from the Live view and use Fork in app`. Tests: `continue_is_409_for_a_record_only_the_live_tail_has_and_for_a_running_app_session_in_the_repo_and_quiet_is_202`, `continue_is_409_while_any_record_of_the_repo_is_under_two_minutes_old_and_starts_when_quiet` (now also a going app session in the repo), `a_session_known_only_to_the_live_tail_is_resumed_in_its_start_folder_and_404_only_when_the_tail_does_not_know_it`, `a_transcript_cwd_outside_the_project_is_ignored_and_the_run_is_in_the_project_folder`, `a_session_resumes_in_the_first_cwd_of_its_transcript_not_in_its_repo_and_falls_back_to_the_stored_folder` (now a real git repo with a subfolder and a worktree). `cd rust`: `cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings` no warning; `cargo test -p easy-personal-rs --bin easy-personal-rs app_session`: `test result: ok. 66 passed; 0 failed`; `cargo test --workspace --no-fail-fast`: 96 targets, 1177 passed, 0 failed (`pairing_100k_calls_is_fast` and `two_hundred_thousand_links_are_counted_in_time` ok in the run); `cd web && npm test`: `Tests 1166 passed (1166)` (the continue 409 test text updated; the page matches only on `STILL_ACTIVE`, unchanged); leak gate: `check T103 app session strings scanned     ok (11 strings, 999 surfaces, 18 guarded answers allowed to hold them)` and `leak check: 0 hits over 42 canaries, 1017 surfaces, 2359121 bytes` |
| **2nd half: UI: ux-evaluate (375 and 1280)** | ☑ pass (see table) | scratch harness outside the repo (Vite dev server on the worktree's `src/`, `fakeFetch` and the app session stub, headless Chrome); shots under the scratchpad `h105/shots/` (`live`, `resume`, `resume-confirm`, `fork`, `fork-confirm`, `fork409`, `notfound`, `workspaces`, `continue`, `continue-confirm`, each -375 and -1280); no UX rows in the guide: baseline checks only |

| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pending | Supervisor, Stage 4. Blast radius: `start` in `app_session_routes.rs` (every start), `Plan` / `argv` / `Sessions::start` in `app_session.rs`, two additive methods on `Api` (`api.rs`), two additive items in `core/src/live.rs` |
| Full smoke suite still green (no regression) | ☑ pass | workspace 1172 passed, 0 failed |
| **Companion score (easy-verifier)** | ☑ N/A | not run in Stage 3 |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | `ux-evaluate` by the page half: shots at 375 and 1280 for Live (Resume / Fork in app), the resume, fork and continue forms and confirm steps, the 409 switch, the 404 and Workspaces — all baseline checks pass (table in this file) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | only `tokens.css` variables in `appsession.css` (same rule set as T104) |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280: no sideways scroll; Continue in app buttons set to 44 px at 375 after the first shot |

### ux-evaluate (second half: baseline checks only)

| Check | Verdict | Reason |
|---|---|---|
| B-a one primary action in view | pass | forms: Review then Start session (accent) sit in view at both widths; Live and Workspaces have no new primary, the new buttons are secondary like Open in app |
| B-b hierarchy | pass | heading, session or repo identity line, fields, actions; on Live the button sits under the row head and the fork note above the bars |
| B-c spacing | pass | the repo list on a workspace card has the same gaps as the card's other blocks; the 375 Live row keeps the button, note and bars in one column |
| B-d touch targets | pass | `Resume in app` and `Fork in app` are 44 px high at 375 in the shot (the `live-open-app` rule). The `Continue in app` buttons were about 32 px in the first shot, so `.repo-continue button` got the same 44 px rule at 375 (`appsession.css`); that edit was made after the shots and was not shot again. Not measured with `tap_probe.py` |
| B-e readable text | pass | body text is the page's own; the fork note and the server's messages are 13 px and above, in the existing note colours |
| B-f states | pass | the 409 switch (`fork409`), 404 (`notfound`) and the fork note are on screen with the server's words and the next action (Start session, Back) |
| B-g feedback | pass | a refusal shows an alert under the confirm text; a switch to Fork changes the heading to "Start a fork of this session?" |
| B-h 375 layout | pass | no horizontal scroll; the repo rows wrap path and button |
| B-i one visual language | pass | same button, note (`prompt-cut`) and card styles as the first half |

Not looked at: the off state and a missing `claude` on these screens (covered by the first half's tests and unchanged code).

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

**AFTER (second half)** (captured 2026-11-17T16:20:36Z on branch T105 with B1 and B2 in the working tree, default-feature debug build; temp `EASY_PERSONAL_HOME`, a stand-in `claude` that logs its arguments and folder; stored sessions: `aaaa1111-0001` and `bbbb2222-0002` quiet for 10 minutes with project `quiet-repo` whose transcripts say they started in `quiet-repo/src/deep` and in a worktree folder, `cccc3333-0003` with a record 30 s ago in `repo`; paths shortened):

```
$ POST /api/app-sessions resume session started in a SUBFOLDER (stored project: the repo)
{"id":"1183268919fdedef"} [202]
$ POST /api/app-sessions resume session started in a git WORKTREE (stored project: the repo)
{"id":"5d462fec9509c427"} [202]
$ POST /api/app-sessions continue in repo with a record 30 s ago
{"error":"a session in this folder is still active in another window; resume it with Fork instead"} [409]
$ POST /api/app-sessions continue in quiet-repo (newest record 10 min old)
{"id":"94f633a5269395ad"} [202]
argv per start, then the folder each child ran in:
-p --input-format stream-json --output-format stream-json --verbose --resume aaaa1111-0001
-p --input-format stream-json --output-format stream-json --verbose --resume bbbb2222-0002
-p --input-format stream-json --output-format stream-json --verbose --continue
<tmp>/quiet-repo/src/deep
<tmp>/quiet-repo-wt
<tmp>/quiet-repo
```

The page: Live rows show Resume in app (no record for 2 minutes), Fork in app with the fixed note (record in the last 2 minutes) and Open in app (an app session); each opens the New session form in resume mode; Workspaces lists each repo with Continue in app. Evidence: the Vitest rows and the screenshots above.

**DELTA (second half)**: a resumed session now runs in the folder its transcript starts in (a subfolder or a worktree), not in its repo; `continue` is refused with 409 while the repo has a record under 2 minutes old; the page has the three buttons and the resume and continue forms.

**WITNESS (second half)**: frontend-developer agent, 2026-11-17 (AFTER at 16:20:36Z; Vitest, Rust gate and screenshots on the T105 worktree). A run with the real `claude` was not made.
