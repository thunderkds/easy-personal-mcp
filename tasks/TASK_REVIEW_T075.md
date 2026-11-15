# TASK_REVIEW — T075: Watch 4: effectiveness on read

> Sibling of `tasks/TASK_GUIDE_T075.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | AC1-AC4: `rust/crates/core/tests/effectiveness.rs` (7 tests: attribution incl. sidechain of a branch session / of a `main` session / `worktree-agent-x` / `fix-foo` / HEAD / two ids / two repos; first merge of two wins, squash, fast-forward and merge-into-`side` not done, shared `worktree-agent-*` merge not done, no git = unknown; 0 good results = null, unknown price; an FR-061 good result; pruned store via `Writer::prune` -> oldest day, local day in Tokyo; read-only, period, scope, no path / session id; relative or missing project never reads the cwd's git) + `rust/crates/core/src/effectiveness.rs` unit test `task_ids_are_read_as_written_first_one_wins`. AC5: `rust/crates/app/src/api_tests.rs` `the_effectiveness_route_answers_the_core_and_maps_its_errors`; `rust/crates/app/tests/mcp.rs` `effectiveness_is_listed_read_only_and_answers_the_cores_json`, `effectiveness_with_no_store_is_a_tool_error_without_a_path_and_creates_nothing`; `rust/crates/app/tests/cli_cmds.rs` `effectiveness_prints_a_table_or_the_json_and_refuses_a_bad_scope` + missing / newer store / read-only lists; tool counts 12->13 in `mcp_http.rs`, `save_report.rs`, `mcp.rs` (name list, registration order); `/api/effectiveness` added to `FORBIDDEN_ROUTES` in `tests/server.rs` (guard, no echo, 503 store states). Output: `cargo test -q -p easy-personal-core --test effectiveness` -> `test result: ok. 7 passed; 0 failed`; app suites `mcp` 30 passed, `cli_cmds` 14 passed, `server` 52 passed, `mcp_http` 13 passed, `save_report` 7 passed, `kit` 7, `dirs_jobs` 5, `connectors` 13 (all 0 failed). |
| Verification command run | ☑ pass (with 2 notes) | Full run (see AFTER): `test binaries 85 passed 904 failed 5 ignored 29`; the 5 failures are timing / race tests in 4 binaries outside T075 (`jobs_parity` crontab `< 3.0 s`, `live_parity` 100k pairing, `memgraph_parity` `took 9.58s`, `ingest_coexist` x2) under load average 16-18 from parallel worktrees; each binary passes alone right after: `jobs_parity` 7 passed, `live_parity` 15 passed, `memgraph_parity` 27 passed, `ingest_coexist` 14 passed (0 failed); `ingest_coexist` also failed on clean main in BEFORE. Note 1: the guide's `cargo run -p easy-personal-rs` needs `--bin easy-personal-rs`. Note 2: the real run uses a fixture store (synthetic events on this repo's path; the owner's store is not opened), with done times from this repo's real local git. |
| Leak gate (`parity/leak_rust.py`, after merging main `e40bb8f`) | ☑ pass | `/api/effectiveness` (6 queries), MCP `effectiveness` (6 argument sets) and CLI `effectiveness` (5 calls) registered. `env -u CLAUDE_CONFIG_DIR .../rust/parity/.venv/bin/python parity/leak_rust.py; echo exit=$?` -> last line `leak check: 0 hits over 42 canaries, 811 surfaces, 1789641 bytes`, no `failed checks` line, `exit=0` (28 output lines name `effectiveness`). |
| Fix round: suite after merging main (HEAD `d9da3d2`) | ☑ pass | `cargo fmt --all --check`, `cargo clippy -q --workspace --all-targets` (no output), `cargo test --workspace --no-fail-fast`:<br>`fmt-ok` `clippy-done` `cargo test exit=0` `binaries 90 passed 937 failed 0 ignored 28`<br>No flaky failure this run. Real run on the fixture store (`EASY_PERSONAL_HOME=<fixture home> target/debug/easy-personal-rs effectiveness`): T070 is done again: `T070          easy-personal-mcp  373800   $0.3588                  1         0     -            2026-11-15T02:48:44Z  373800`; total line: `total - 5500200 $5.2275 + unknown price 4 0 - 6 of 7 done 293700.0`. |
| Negative cases hold | ☑ pass | bad `since` -> 400 / tool error / exit 2 `Invalid isoformat string: 'bad'`; unknown workspace -> `unknown workspace`; no token -> 403; no store / newer store -> fixed texts, nothing created, no path in the text; wrong argument type -> `argument `until` must be a string or null`; store bytes unchanged after API, MCP and CLI reads. |
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

**BEFORE**: captured by backend-developer on the clean tree (HEAD `244e983`, no T075 change; work stashed), before any T075 commit.
The guide's command as written cannot reach `effectiveness`: `cargo run -p easy-personal-rs` needs `--bin easy-personal-rs`
(the package has two binaries), so the same check was also run with `--bin`. The one failed test (`ingest_coexist`,
`a_killed_rust_ingest_leaves_whole_files_and_a_rust_rerun_equals_a_clean_run`) fails on clean main, before T075.

```
2026-11-15T02:54:11Z
HEAD 244e983, clean tree
$ cd rust && cargo test --workspace 2>&1 | tail -5 && cargo run -q -p easy-personal-rs -- effectiveness
    a_killed_rust_ingest_leaves_whole_files_and_a_rust_rerun_equals_a_clean_run

test result: FAILED. 13 passed; 1 failed; 10 ignored; 0 measured; 0 filtered out; finished in 112.92s

error: test failed, to rerun pass `-p easy-personal-rs --test ingest_coexist`
error: `cargo run` could not determine which binary to run. Use the `--bin` option to specify a binary, or the `default-run` manifest key.
available binaries: easy-personal-launch, easy-personal-rs
exit=101
2026-11-15T02:59:21Z
2026-11-15T02:59:33Z
$ EASY_PERSONAL_HOME=<empty temp dir> cargo run -q -p easy-personal-rs --bin easy-personal-rs -- effectiveness
info: syncing channel updates for stable-x86_64-unknown-linux-gnu
info: latest update on 2026-11-12 for version 1.100.0 (0ef1bc386 2026-11-09)
info: downloading 5 components
warn: the missing active toolchain `stable-x86_64-unknown-linux-gnu` has been auto-installed
warn: this might cause rustup commands to take longer time to finish than expected
info: you may opt out with `RUSTUP_AUTO_INSTALL=0` or `rustup set auto-install disable`
error: unrecognized subcommand 'effectiveness'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
exit=2
```

**AFTER**: run by backend-developer at HEAD `fb2216d` (fixture store: synthetic events whose project is this repo's main checkout; done times from its real local git).

```
2026-11-15T03:59:11Z
HEAD fb2216d
$ cd rust && cargo test --workspace --no-fail-fast 2>&1 | tail -5   (+ the sum of every "test result" line)
cargo test exit=101
test binaries 85 passed 904 failed 5 ignored 29
error: 4 targets failed:
    `-p easy-personal-core --test jobs_parity`
    `-p easy-personal-core --test live_parity`
    `-p easy-personal-core --test memgraph_parity`
    `-p easy-personal-rs --test ingest_coexist`
$ EASY_PERSONAL_HOME=<fixture home> cargo run -q -p easy-personal-rs --bin easy-personal-rs -- effectiveness
task          project            tokens   cost                     sessions  good  tokens/good  done                  tokens/task done
T071          easy-personal-mcp  427200   $0.4100                  1         0     -            2026-11-15T02:40:32Z  427200
T070          easy-personal-mcp  373800   $0.3588                  1         0     -            not done              -
T069          easy-personal-mcp  320400   $0.3075                  1         0     -            2026-11-14T16:40:44Z  320400
T068          easy-personal-mcp  267000   $0.2562                  1         0     -            2026-11-14T16:39:21Z  267000
T067          easy-personal-mcp  213600   $0.2050                  1         0     -            2026-11-14T16:34:21Z  213600
T066          easy-personal-mcp  160200   $0.1537                  1         0     -            2026-11-14T16:34:00Z  160200
T075          easy-personal-mcp  106800   $0.0512 + unknown price  1         0     -            not done              -
unattributed  -                  3631200  $3.4850                  3         0     -            -                     -
total         -                  5500200  $5.2275 + unknown price  4         0     -            5 of 7 done           277680.0
Oldest stored day: 2026-11-14 (older events may have been pruned with --retention-days)
exit=0
2026-11-15T04:14:44Z
```

Finding in the AFTER run: T070 shows `not done`, but a run at `29d2ff7` 36 min earlier showed `2026-11-15T02:48:44Z`.
Cause: `main` was fast-forwarded to `t078-keychain-leftovers` (reflog `10:56:09 merge t078-keychain-leftovers: Fast-forward`),
whose `b411c19 Merge branch 'main' into t078-keychain-leftovers` made the old main the second parent. So
`Merge branch 'T070'` is no longer on main's first-parent chain. The guide's rule (`--first-parent`) is kept as written; see the
report's Question.

**Fix round (⚠ decided for you — please review):** done time no longer reads `--first-parent` only. It reads every merge reachable from `main` and counts a subject `Merge branch '<branch>'` or `Merge branch '<branch>' into main`; a subject with ` into <other>` does not count; the first such merge by commit time wins. Decided by the Supervisor for the owner in [admin] mode, on backend-developer's recommendation, after the T070 finding above. New test case: T9 merged into main, then main fast-forwarded to `ff` (which had merged main into itself) -> T9 still done at its merge time; `Merge branch 'T4' into side` reachable from main -> T4 still not done (`rust/crates/core/tests/effectiveness.rs`, `done_is_the_first_merge_into_main_and_a_squash_or_fast_forward_is_not_done`).

**DELTA**: The owner can now see, per task branch, the tokens, the cost, the good results and the done time from git, computed on read. The API (`GET /api/effectiveness`), the CLI (`effectiveness`, `--json`) and the MCP tool (`effectiveness`) show them. Before, the command did not exist (`unrecognized subcommand 'effectiveness'`).

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T075.jsonl`, never the
implementing agent alone]
