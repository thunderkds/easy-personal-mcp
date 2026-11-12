# TASK_REVIEW — T052: Rust P3 waves 1-2 review fixes: crontab step overflow, DEL escape, secret word boundary, capped account reads and stale text

> Sibling of `tasks/TASK_GUIDE_T052.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` or `[admin]` tag; with `[admin]`, the `Not done:` line names `merge (admin: after the auto verify pass)`,
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
`Memory:` records the diff-driven memory pass you run before you stop: each entry written, or `nothing new`.
After an auto verify pass the hook, never you, adds `Closed: row moved to Done <date> — …` and moves the row,
or adds `Paused: done not moved — <reason>`. It moves the row only when this block holds `Mode: auto`, a
numbered decision, `Checks:`, `Memory:`, the `Not done:` auto verify slot with its result, and `Not done:`
naming the merge — no `<...>` left (`docs/claude-md/tracks.md` `### Auto close-out`).

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/src/jobs.rs` and `reports.rs` and `secret.rs` unit tests (`a_step_too_big_to_add_stops_the_field_instead_of_overflowing`, `word_is_pythons_w_for_every_code_point`, others), Python-made oracle cases and goldens (+974 lines, 0 deletions) |
| Verification command run | ☑ pass | Author, commit dfeae09, three TZ runs: 601 passed, 1 failed (the known `workspaces_parity::edge_store_python_values`); `leak check: 0 hits over 42 canaries`; clippy (x2) and fmt clean. Not re-run by the Supervisor. |
| Negative cases hold | ☑ pass | BEFORE block holds the red run on the unfixed code (`cargo test -p easy-personal-core --lib`: 24 passed, 5 failed, including `attempt to add with overflow` in `jobs.rs` and 273 ranges where `word` differs from Python's `\w`). |
| verify | ☑ pass | Stage 5 independent verify on dfeae09 (qa-expert, 2026-11-12T08:09Z-08:23Z, TZ unset, fixtures and temp dirs only). `cargo build` ok; `cargo test --workspace --no-fail-fast` 601 passed, 1 failed (the known `workspaces_parity::edge_store_python_values`), 26 ignored; `leak_rust.py` `leak check: 0 hits over 42 canaries, 200 surfaces, 908174 bytes` (exit 0); `--self-test` ok; clippy default and `--features window` exit 0; `cargo fmt --check` exit 0; `git status` clean except this file and the untracked `rust/parity/.venv` symlink. AFTER run of the BEFORE commands: `--lib` 29 passed, 0 failed; `jobs_parity` 7/7, `save_report_parity` 7/7, `lazy_fetch_env` 1/1, `writer_no_create` 1/1 pass. Through the built binary: `GET /api/jobs` over a fake crontab with step `2^63-1` (`1/..` and `*/..`), `*/0`, `*/-1`, `*/70` answers 200, no panic, and the six next-run answers equal Python's `next_cron`; MCP `save_report` refuses `U+24B6 sk-abc` and `U+0345 sk-abc` (isError true, row count unchanged) and saves a clean summary (2 rows to 3). Caveat: the first full run on the default `/tmp` (tmpfs 95 % full, other sessions' folders) failed `ingest_coexist::serve_mcp_and_store_info_answer_while_a_rust_ingest_runs` with `SQLite error DiskFull` (3 of 5 isolated reruns on `/tmp` too; one showed `/api/warnings` 503); with `TMPDIR` on the disk volume it passes alone and in the full run. Environmental, not a T052 change; the suite leaves `easy-personal-rs-t041-*` and `ep-t033-cfg-*` temp folders behind. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Independent read-only reviewers (code and security) on dfeae09: no P0, P1 or P2; two P3s (Python 3.14 refuses `int()` text over 4300 digits, accepted; `read_json` opens before the type check, conf 50, recorded only). Reports: reports/code-review_T052_20261112T090200.html and reports/security-review_T052_20261112T090300.html in the main checkout. |
| Full smoke suite still green (no regression) | ☑ pass | Same author runs: 601 passed, 1 failed (known). |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier not run for this task. |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |
| **UI: Responsiveness at target viewports** | ☑ N/A | No UI file changed (Rust routes, docs and tests only). |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-12T04:19Z-04:20Z in this worktree, before any implementation commit; the new red tests and the
new Python-made oracle cases were written first, no `src` file of the fixes was touched yet). The guide names no
Demonstration command, so the commands are `cargo test -p easy-personal-core --lib` and `cargo test -p easy-personal-core
--test <name>` from `rust/`, against the unfixed code:

```
$ cargo test -p easy-personal-core --lib            # 2026-11-12T04:19:53Z   (24 passed; 5 failed)
---- jobs::tests::a_step_too_big_to_add_stops_the_field_instead_of_overflowing
panicked at crates/core/src/jobs.rs:327:13:  attempt to add with overflow
---- reports::tests::a_malformed_checked_entry_makes_the_whole_count_zero
panicked at crates/core/src/reports.rs:663:9:  assertion `left == right` failed  left: 1  right: 0
---- secret::tests::a_secret_behind_a_non_word_alphabetic_mark_is_found      panicked: "Ⓐsk-abc"
---- secret::tests::word_is_pythons_w_for_every_code_point
273 range(s) where `word` differs from Python's `\w`: [(345, 345), (363, 36F), (5B0, 5BD), ... (24B6, 24E9), ... (1F130, 1F149), (1F150, 1F169), (1F170, 1F189), ... (323B0, 33479)]
---- accounts::read_json_tests::the_cap_is_applied_to_the_bytes_read_not_to_a_size_asked_for_earlier
panicked at crates/core/src/accounts.rs:279:9:  more than 1 byte read against a cap of 1
$ cargo test -p easy-personal-core --test jobs_parity            # 2026-11-12T04:20:27Z
every_jobs_case_equals_python_and_asks_only_the_same_read_only_calls ... FAILED   panicked at jobs.rs:327:13: attempt to add with overflow
$ cargo test -p easy-personal-core --test save_report_parity
the_secret_check_matches_pythons_on_the_corpus ... FAILED    "Ⓐsk-abc"  left: false  right: true
every_case_stores_or_refuses_what_python_did ... FAILED      del_in_fix: the stored row  fixes_json  left: "...\u{7f}..." (raw DEL)  right: "...\\u007f..."
$ cargo test -p easy-personal-core --test lazy_fetch_env
every_git_run_says_never_to_fetch_lazily ... FAILED   GIT_NO_LAZY_FETCH as each git run saw it: ["unset", "unset"]
$ cargo test -p easy-personal-core --test writer_no_create
error[E0432]: unresolved import `easy_personal_core::writer::open_existing_for_write`   (does not compile: the handle does not exist yet)
$ rust/parity/.venv/bin/python rust/parity/leak_rust.py --self-test
self-test tree digest     FAILED      (the digest skipped every file named argv.log, not only <plant>/bin/argv.log)
```

**AFTER** (captured 2026-11-12T08:16Z-08:22Z by qa-expert on commit dfeae09, TZ unset, `TMPDIR` on the disk volume because `/tmp` was 95 % full; same commands as BEFORE, from `rust/`):

```
$ cargo test -p easy-personal-core --lib            # 2026-11-12T08:16:28Z
test accounts::read_json_tests::the_cap_is_applied_to_the_bytes_read_not_to_a_size_asked_for_earlier ... ok
test jobs::tests::a_step_too_big_to_add_stops_the_field_instead_of_overflowing ... ok
test reports::tests::a_malformed_checked_entry_makes_the_whole_count_zero ... ok
test secret::tests::a_secret_behind_a_non_word_alphabetic_mark_is_found ... ok
test secret::tests::word_is_pythons_w_for_every_code_point ... ok
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
$ cargo test -p easy-personal-core --test jobs_parity          # 2026-11-12T08:16:30Z
test every_jobs_case_equals_python_and_asks_only_the_same_read_only_calls ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.02s
$ cargo test -p easy-personal-core --test save_report_parity   # 2026-11-12T08:16:40Z
test the_secret_check_matches_pythons_on_the_corpus ... ok
test every_case_stores_or_refuses_what_python_did ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.14s
$ cargo test -p easy-personal-core --test lazy_fetch_env       # 2026-11-12T08:16:49Z
test every_git_run_says_never_to_fetch_lazily ... ok
$ cargo test -p easy-personal-core --test writer_no_create     # 2026-11-12T08:16:49Z
test a_missing_store_is_refused_and_not_created ... ok
$ rust/parity/.venv/bin/python rust/parity/leak_rust.py --self-test   # 2026-11-12T08:16:50Z
self-test tree digest     ok
self-test: ok (raw, json, \u, url, base64, base64url at 3 alignments, hex; sqlite; masking)

# user-visible, built binary `serve`, fake `crontab`/`systemctl` on a scratch PATH, `env -i`, temp store copy   # 2026-11-12T08:19:29Z
GET /api/jobs   HTTP 200   (serve.err empty, no panic)
  schedule '1/9223372036854775807 * * * *'  next_run 2026-11-12T09:01:00Z
  schedule '*/9223372036854775807 * * * *'  next_run 2026-11-12T09:00:00Z
  schedule '*/0 * * * *'                    next_run None
  schedule '*/-1 * * * *'                   next_run None
  schedule '*/70 * * * *'                   next_run 2026-11-12T09:00:00Z
  schedule '*/5 * * * *'                    next_run 2026-11-12T08:20:00Z
# Python's next_cron(expr, datetime(2026,11,12,8,19)) gives 09:01, 09:00, None, None, 09:00, 08:20: equal.

# user-visible, built binary `mcp` over a temp store copy (2 saved reports)   # 2026-11-12T08:22:28Z
save_report summary "the key is sk-abc"            isError True  "summary: looks like a secret ...; nothing was saved"  rows 2
save_report summary "U+24B6 + sk-abc"              isError True  "summary: looks like a secret ...; nothing was saved"  rows 2   (saved before the fix)
save_report summary "U+0345 + sk-abc"              isError True  "summary: looks like a secret ...; nothing was saved"  rows 2   (saved before the fix)
save_report summary "Cache reuse is low on ..."    isError False {"id":3,"saved_utc":"2026-11-12T08:22:35Z"}            rows 3
```

**DELTA**: A crontab line with a huge step (`*/9223372036854775807`) now lists with the same next run as Python instead of crashing the jobs route, and a key hidden behind a circled letter or combining mark (`\u24b6sk-abc`) is now refused by `save_report` instead of being stored.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
