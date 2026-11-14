# TASK_REVIEW — T067: Close the 7 T061 parity findings and stop the runner leaking on SIGTERM

> Sibling of `tasks/TASK_GUIDE_T067.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `rust/parity/test_parity_report.py` (36 tests; `./parity/.venv/bin/python parity/test_parity_report.py` -> `passed: 0 failing`): one rule test with mutations per decided finding (`test_version_rule_...`, `test_ctrl_c_rule_...`, `test_prompt_rule_...`, `test_protocol_rule_...`, `test_write_only_route_rule_...`, `test_the_usage_error_texts_are_explained_by_item_14_and_only_when_exact`), `test_each_t067_item_is_in_the_readme_marked_decided_for_you_and_has_a_rule`, `test_the_report_has_no_new_difference_and_no_surface_left_unrun`, five signal tests (`test_sigterm_during_a_running_server_...`, `test_sighup_...`, `test_sigterm_while_a_serve_starts_kills_it`, `test_sigterm_before_any_server_exists_...`, `test_sigterm_in_the_full_run_path_...`). Rust: `crates/app/tests/live.rs` `a_new_transcript_line_arrives_as_a_live_event_within_2_s` asserts `["text/event-stream; charset=utf-8"]` (red before the fix: `left: ["text/event-stream"]`), `crates/app/src/live_feed.rs` `the_live_check_reads_a_chunked_first_snapshot_and_refuses_everything_else` reads the charset form. With the `stop_on_signals()` call replaced by `pass`, the four process signal tests go red (exit -15 / -1 not 143 / 129; child alive; `['parity-world-9x4owgql']` left) ||
| Verification command run | pass | `run_parity.py --check` -> `the report equals a fresh run`, exit 0 (2026-11-14T16:01:30Z to 16:04:35Z, started in the foreground, the harness moved it to the background after 180 s) and again as a shell `&` job: `bg job exit 0`, `the report equals a fresh run` (16:04:43Z to 16:07:19Z). Report line 5: `Counts: equal 26 + equal except JSON key order 29, differs README 32, differs new 0, not run 0.` (before: `equal 25 + 29, differs README 26, differs new 7, not run 0`). `cargo test --workspace --no-fail-fast`: exit 0, 884 passed, 0 failed, 29 ignored (sum of the `test result` lines). `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: Finished, no warning ||
| Negative cases hold | pass | each rule test has the mutation rows (one other byte changed -> `("new", [])`): other `--version` text, other exit code, stderr added; access-log plus a traceback line; Rust stdout not empty; Rust exit 1; Python exit -9; another prompt message; wrong code; extra stderr line; one more capability; message with spaces; `403` for `405`; `Gone` for `Not Found`; `'bananas'`, `projects`, exit 1, stdout `x` for the usage texts ||
| verify | pass | AFTER, same command as BEFORE with the T067 runner: SIGTERM `runner exit code: 143`, `started children still alive: 0`, `new world folders left in /tmp: []`; SIGHUP `runner exit code: 129`, `started children still alive: 0`, `new world folders left in /tmp: []` (2026-11-14T16:14:19Z..16:14:23Z) - pass ||
| Review scope bounded to the change's blast radius (affected set, not whole repo) | pass | reviewed: `rust/parity/run_parity.py`, `rust/parity/test_parity_report.py`, `rust/docs/PARITY_T061.md` (regenerated), `rust/README.md` (6 new items, 1 changed sentence), `crates/app/src/api.rs` (one header string), `crates/app/src/live_feed.rs` (self-test check accepts the charset), `crates/app/tests/live.rs`. Not touched: `src/**`, `rust/parity/gen_*.py`, connector/doctor/keychain code (T066) ||
| Full smoke suite still green (no regression) | pass | `cargo test --workspace --no-fail-fast`: 884 passed, 0 failed, 29 ignored; `test_parity_report.py`: `passed: 0 failing`; `run_parity.py --check`: equal ||
| **Companion score (easy-verifier)** | N/A | not run in this QA pass; Stage 4 `code-review` and the easy-verifier `score` stay with the Supervisor ||
| **UI: Visual regression (diff or verdict pasted)** | N/A | no UI ||
| **UI: Design-system compliance (tokens/colors/typography verified)** | N/A | no UI ||
| **UI: Responsiveness at target viewports** | N/A | no UI ||

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-14T15:47Z..15:49Z, before any T067 implementation commit (the runner code in the T067 worktree is byte-identical to `main` at `7f901f8`, checked with `cmp`; the SIGTERM runs used the main checkout's runner and binary).

1. Committed report, `rust/docs/PARITY_T061.md` line 5:
```
Counts: equal 25 + equal except JSON key order 29, differs README 26, differs new 7, not run 0.
```
2. `/api/live` charset, `run_parity.py --only 'route: GET /api/live / first-event'` (2026-11-14T15:47:13Z):
```
NEW    route: GET /api/live / first-event [UTC]
  step 0 /type
     py: "text/event-stream; charset=utf-8"
     rs: "text/event-stream"
```
3. SIGTERM to the runner while a server child is up (scratch driver `sigprobe.py`: start `run_parity.py --only 'cli: mcp / http-handshake-and-ping'`, send the signal when the runner has a child, wait 1.5 s, look for what is left):
```
$ python3 sigprobe.py <main>/rust TERM 'cli: mcp / http-handshake-and-ping' mcp     # 2026-11-14T15:48:35Z
descendants of the runner before the signal:
  66066 .../rust/parity/.venv/bin/python -c import sys; from easy_personal. ...
runner exit code: -15
after TERM: started children still alive: 1
  66066 .../rust/parity/.venv/bin/python -c import sys; from easy_personal. ...
new world folders left in /tmp: ['/tmp/parity-world-zdp95vhx']
```
SIGHUP (2026-11-14T15:48:41Z): `runner exit code: -1`, `started children still alive: 1` (pid 66101), `new world folders left in /tmp: ['/tmp/parity-world-b7y5ciwu']`. (The driver killed those PIDs and removed the folders afterwards; only PIDs it saw were killed.)

**AFTER**: captured 2026-11-14T16:14Z with the T067 runner (same driver, same probe, same signal):

1. `rust/docs/PARITY_T061.md` line 5 (made by `run_parity.py`, equal to a fresh run by `--check`):
```
Counts: equal 26 + equal except JSON key order 29, differs README 32, differs new 0, not run 0.
```
2. `route: GET /api/live` is `equal` (2 probes); `crates/app/tests/live.rs` asserts `content-type: text/event-stream; charset=utf-8`.
3. Signals:
```
$ python3 sigprobe.py <T067>/rust TERM 'cli: mcp / http-handshake-and-ping' mcp     # 2026-11-14T16:14:19Z
runner exit code: 143
after TERM: started children still alive: 0
new world folders left in /tmp: []
$ ... HUP ...
runner exit code: 129
after HUP: started children still alive: 0
new world folders left in /tmp: []
```
(the child seen before each signal was the Python `mcp --transport http` server.)

**DELTA**: the parity run now ends with no open difference (every one of the seven T061 findings is aligned or named by a README item the runner checks), and a SIGTERM or SIGHUP to the runner no longer leaves a server or a world folder behind.

**WITNESS**: qa-expert (this session, 2026-11-14T15:47Z to 16:14Z); the event trace `memory/event-trace/T067.jsonl` is written by the hook and was not read here. The same agent wrote the runner rules, the tests and ran them: the Supervisor's Stage 4 review is the independent check.
