# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/agent_routes.rs` (12 route tests, `mod tests`: AC 1 `each_route_without_the_token_…`, AC 2 `the_setting_is_off_by_default_…` and `the_account_shows_whether_its_folder_is_there`, AC 3 `with_no_account_chosen_a_post_is_403_…`, AC 4 `the_body_rules_of_the_other_write_routes_hold`, AC 5 `a_run_is_202_then_running_…`, AC 6 `stop_ends_the_run_as_stopped_…`, `a_run_of_another_process_shows_as_running_…`, `the_server_stopping_during_a_run_…`, AC 7 `an_account_that_is_ignored_or_gone_…`, `the_ignore_hook_removes_the_flag_…`, AC 9 `a_planted_secret_in_the_output_and_the_environment_is_in_no_response_body`); `rust/crates/app/src/agent_run.rs` (AC 2, 7, 8: `the_flag_holds_the_account_id_…`, `a_run_with_no_account_given_uses_the_stored_one_and_a_dead_id_starts_nothing`, `the_stop_flag_ends_a_going_run_…`, `on_account_writes_the_id_status_prints_it_and_now_uses_it`; T090's tests in the same module still pass); `rust/crates/app/tests/agent_run_routes.rs` (real `serve` binary: `serve_help_says_it_is_not_read_only_…`, `ctrl_c_on_the_server_during_a_run_…`, `a_kill_of_the_server_during_a_run_…`, `ignoring_the_agent_run_account_through_the_account_routes_…`); `rust/parity/leak_rust.py` (AC 9 gate) |
| Verification command run | ☑ pass (T091 targets; one environment failure not from T091) | 2026-11-17 (the second full run started 02:42 UTC), `cd rust && cargo test --workspace --no-fail-fast` (the guide's `cargo test --workspace` stops at the first failing target, so `--no-fail-fast` shows the rest): app unit tests `test result: ok. 218 passed; 0 failed` (28 of them `agent_`); `tests/agent_run_routes.rs` `4 passed`; every target ok except `-p easy-personal-rs --test connector_cli_parity` (`5 problems`, all `sync_no_launcher` / `doctor_no_launcher`: `/usr/bin/easy-personal-launch` and `~/.local/bin/easy-personal-launch` of the installed .deb are on `PATH`; T090's review file records the same failure with its changes stashed; not run on `main` here). `memgraph_parity two_hundred_thousand_links_are_counted_in_time` failed once in the first full run (`took 11.87s`, under load) and passed in the second full run and alone: `test result: ok. 27 passed`. `cargo fmt --check` exit 0. `cargo clippy --workspace --all-targets -- -D warnings` exit 0. Leak gate `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py` (from `rust/`, binary rebuilt at the final commit): `leak check: 0 hits over 42 canaries, 866 surfaces, 2021390 bytes`, exit 0, no `failed checks` |
| Negative cases hold | ☑ pass | In-process tests (above) and a real `serve` on a temp `EASY_PERSONAL_HOME` with a fake `claude` that would write a marker file (no run started; marker absent): no account chosen, POST 403 with `agent runs are off: choose an agent run account in Settings`; PUT unknown account 400 `the account is not a confirmed Claude account`; PUT without `confirmed` 400; POST other job 400; no token 403; foreign Origin 403; stop with no run 200, no change. 409 (second POST; a run of another process) and 415, 413, 400 body cases: `agent_routes::tests::a_run_is_202_…`, `a_run_of_another_process_…`, `the_body_rules_of_the_other_write_routes_hold`. One gap found and fixed in this session: `agent-run now --account <x>` with a flag that names a gone or Codex account started a run (the test failed: `99 with --account left: Ok right: Off`); now off, commit `fa6f83d` |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass (see the Verification row for the one environment failure) | the `cargo test --workspace --no-fail-fast` above: 93 `test result: ok` lines and 1 `FAILED` in the second run, the failed one is `connector_cli_parity` (launcher on `PATH`). `parity/run_parity.py --check` (run in this worktree with `main`'s venv): `the report differs from a fresh run`; a fresh report against `rust/docs/PARITY_T061.md` differs in four lines only, all Rust-only CLI names added to the list: `cli: agent-run` (T090), `cli: effectiveness`, `cli: machine`, `cli: tray` (not T091). The four new routes are not in that report at all (its inventory is read from Python), so they add no difference. The committed report was not changed |
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

**BEFORE** (2026-11-16T10:17:57Z-10:19:21Z UTC, branch `T091` at `137ce1f`, no implementation commit yet; a route test that calls the real router with a good Host and token):
```
$ cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs agent_routes
---- agent_routes::tests::get_agent_run_exists stdout ----
thread 'agent_routes::tests::get_agent_run_exists' panicked at crates/app/src/agent_routes.rs:20:9:
assertion `left == right` failed: GET /api/agent-run
  left: 404
 right: 200
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 202 filtered out
```

**AFTER** (backend-developer, 2026-11-17, branch `T091`; the BEFORE command, then a real `serve` on a temp `HOME` and `EASY_PERSONAL_HOME` with a fake `claude` that would write a marker file; `EASY_PERSONAL_HOME` holds no store, so no account exists and no run can start):
```
$ cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs agent_routes
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 206 filtered out

$ easy-personal-rs serve --no-ingest      # then curl with the printed token, Host 127.0.0.1:<port>
Tue Nov 17 02:42:49 AM UTC 2026
$ GET /api/agent-run
{"account":null,"claude_found":true,"enabled":false,"last":null,"running":false} -> 200
$ POST /api/agent-run {"job":"investigate"}
{"error":"agent runs are off: choose an agent run account in Settings"} -> 403
$ PUT /api/agent-run/setting {"account":"1","confirmed":true}
{"error":"the account is not a confirmed Claude account"} -> 400
$ PUT /api/agent-run/setting {"account":"1"}
{"error":"confirmed must be true: agent runs use the Claude subscription login of the account and count against its plan limits"} -> 400
$ POST /api/agent-run {"job":"other"}
{"error":"job must be \"investigate\""} -> 400
$ POST /api/agent-run/stop {}
{"account":null,"claude_found":true,"enabled":false,"last":null,"running":false} -> 200
$ POST /api/agent-run (no token)
{"error":"forbidden"} -> 403
$ POST /api/agent-run (foreign Origin)
{"error":"forbidden"} -> 403
claude ran: no marker file
flag file: none
server exit=0        # kill -INT
```
A 202 run with a chosen account, the 409, the stop and Ctrl-C leaving no process are shown by the in-process tests and by `tests/agent_run_routes.rs` (4 passed), which use a fake `claude` and a temp store with a Claude account. No real `claude` was started.

**DELTA**: A page (or any client with the dashboard token) can read the agent run account, choose it (with `confirmed: true`), start the fixed `investigate` run, and stop it, with no terminal command; the setting is off until chosen, and the server's end, Ctrl-C or ignoring the account ends a going run.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T091.jsonl`, never the
implementing agent alone]
