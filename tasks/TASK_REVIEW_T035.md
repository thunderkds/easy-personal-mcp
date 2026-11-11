# TASK_REVIEW — T035: Ingest of folders — discovery, accounts, project resolution, offsets, save_file

> Sibling of `tasks/TASK_GUIDE_T035.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/ingest_folders_parity.rs` (21 tests: goldens S01-S07, S08a, S10a/b, S13, S14, the four Rust-differs cases, offsets, accounts, order, skips), `ingest_folders_privacy.rs` (canary in store/WAL/SHM with a non-vacuity control), `ingest_folders_python.rs` (2 ignored: real Python coexistence), helper `ingest_folders_common/mod.rs` |
| Verification command run | ☑ pass | Supervisor, own runs 2026-11-11T07:21:38Z: `cargo test --workspace --no-fail-fast` = 378 passed, 0 failed, 12 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`; `cargo fmt --check` ok; clippy `-D warnings` exit 0 (default and `--features window`); ignored `ingest_folders_python` = `2 passed`; `git status` for `src tests web rust/crates/app rust/parity` the T028 parse files, `metrics.rs`, Cargo files = 0 entries |
| Negative cases hold | ☑ pass | agent-reported mutation checks, each red then restored (`cmp` identical, 21 of 21 green again): sort by string → `files_are_read_in_path_component_order` and `s06a_deep_nesting_abstains_and_later_files_are_read`; follow folder symlinks → `s06_cold_ingest_of_the_synthetic_set`; drop the `offset > st_size` reset → `a_shrunk_file_with_the_same_inode_restarts_from_zero`; `--show-toplevel` only → `s06_cold_ingest_of_the_synthetic_set`; skip the NULL-to-default update → `untagged_rows_are_given_to_the_default_account` (left 39, right 0). Not re-run by the Supervisor |
| verify | ☑ pass | real runtime check by the Supervisor 2026-11-11T07:21:38Z: `cargo test -p easy-personal-core --test ingest_folders_python -- --ignored`: Rust ingest of the S01 folder `files 6 new_events 39`, then the real Python ingest on the same store `exit 0 files 6 new_events 0`; and Python first (`new_events 39`) then Rust `files 6 new_events 0`: two implementations share one store. The `/verify` recipe itself was not run (no command yet: T029) |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | read by the Supervisor: all of `folders.rs` (239 lines: roots, discovery, project resolution, offsets, the per-file loop, retire); `accounts.rs` (96 lines) and the test files were not read line by line (their results were re-run). Scope: `rust/crates/core/src/ingest/{folders,accounts,mod}.rs`, `writer.rs` (+18 lines: `file_offsets`, `pub(crate) conn()`), four test files, `rust/README.md`; T028's parse files, `metrics.rs`, Cargo files, `rust/parity`, `src`, `tests`, `web`, the app crate untouched. code-review `reports/code-review_main_20261111T142356.html`: 0 P0, 0 P1, 1 P2, 4 P3. security-review `reports/security-review_main_20261111T142357.html`: nothing at confidence 8 or above |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 378 passed, 0 failed under three zones (Supervisor); every earlier test unchanged |
| **Companion score (easy-verifier)** | ☑ N/A | `score` not run in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-11T07:04:30Z (UTC), before any T035 code change, in `rust/`:

```
$ cargo build
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.86s
$ cargo test -p easy-personal-core --test ingest_folders_python -- --ignored
error: no test target named `ingest_folders_python` in `easy-personal-core` package
help: available test targets:
    git_timeout
    ingest_parse_corpus
    ... (writer_parity, writer_python, writer_save)
$ grep -cx "pub fn ingest_accounts" rust/crates/core/src/ingest/*.rs
rust/crates/core/src/ingest/json.rs:0
rust/crates/core/src/ingest/claude_code.rs:0
rust/crates/core/src/ingest/label.rs:0
rust/crates/core/src/ingest/mod.rs:0
rust/crates/core/src/ingest/secret.rs:0
rust/crates/core/src/ingest/signals.rs:0
rust/crates/core/src/ingest/time.rs:0
(grep exit 1: no match)
```

**AFTER**: Supervisor run 2026-11-11T07:21:38Z after the change: `cargo test --workspace` = 378 passed, 0 failed, 12 ignored (three zones); fmt ok; clippy 0 (both builds); the ignored coexistence run printed `rust ingest: files 6 new_events 39` / `python ingest: exit 0 files 6 new_events 0` and the reverse order; the agent's full Verification Command at 07:20:06Z ended with `2 passed` for the ignored test.

**DELTA**: the Rust core can ingest a whole set of Claude Code project folders into the shared store (same files, order, accounts, project identity and offsets as Python, 21 oracle scenarios equal row for row) and can safely run before or after the Python ingest on the same store.

**WITNESS**: Supervisor re-ran the tests, lint and the real Python coexistence test after the implementing agent; tool calls recorded in `memory/event-trace/T035.jsonl`.

## Choices for the owner (decided in `[admin]` mode, ⚠ decided for you — please review)

1. Where Rust differs from Python (all are Python crashes or silent drops that would stick or hide data): a `*.jsonl` that is a directory or other non-regular file is skipped and counted (Python: `IsADirectoryError`, stuck on every run); an unstattable, unreadable or non-UTF-8-path file is skipped, counted and its offset kept; a folder it cannot list is counted (Python ignores it silently); an inode beyond i64 or a negative stored offset is skipped; a read error in the middle of a file skips that file with nothing saved.
2. `ingest_accounts` lives in `ingest::folders` and calls `accounts::detect` itself, as Python's CLI does before it; the result has a third count, `skipped`, that Python does not have.
3. `Store::conn()` is a `pub(crate)` accessor added to `writer.rs` so the ingest can run its own SQL on the writer's connection.
4. A file's lines are collected, then saved in one transaction: memory grows with the events of one file (T032 measures it).
