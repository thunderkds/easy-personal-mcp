# TASK_REVIEW — T038

> Sibling of `tasks/TASK_GUIDE_T038.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | [paste the red run against the prior content, or the mutation diff (`git diff --stat`, non-empty) before each verdict] |
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

**BEFORE** (captured 2026-11-11, before any edit; worktree HEAD 548bd15):

Prior top doc comment of `rust/crates/core/src/ingest/folders.rs` (lines 1-10), verbatim:

```rust
//! The `claude_code` source over real folders (P2a, T035): Python `sources/claude_code.py` `ingest_accounts`,
//! `account_roots` and `_scan`, plus `core/discovery.py` `resolve_project`. Finds every transcript, picks its account,
//! resolves each `cwd` to its project through `git`, resumes from the stored offset, and saves each file through
//! [`Store::save_file`] (one transaction per file).
//!
//! Rust differs from Python only where Python crashes and then stays stuck on every later run (`TASK_REVIEW_T026.md`,
//! findings): an entry named `*.jsonl` that is not a regular file (S06c, `IsADirectoryError`), an entry it cannot
//! stat, open or read (`PermissionError`, `OSError`), a path that is not UTF-8 (Python cannot store it), and an inode
//! beyond `i64`. Each is skipped and counted in [`Summary::skipped`]; the files after it are read; an existing file
//! skipped this way keeps its stored offset (it is not retired). No error or count carries a path.
```

Prior assertion in `rust/crates/app/tests/ingest_coexist.rs` (`readers_while_writing`, lines 534-537), verbatim:

```rust
            assert!(
                rounds < 15,
                "{label}: 15 rounds without 50 polls during an ingest"
            );
```

Readers test, 5 runs (`cargo test -p easy-personal-rs --test ingest_coexist serve_mcp_and_store_info_answer_while_a_rust_ingest_runs -- --nocapture`), 2026-11-11 11:33:47Z to 11:34:46Z (UTC):

```
run 1 start 2026-11-11T11:33:47Z
readers during rust ingest: 54 polls ... 50 sent while an ingest ran with its first rows committed, over 11 rounds in 12.409128736s; ...
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 12.46s
run 2 start 2026-11-11T11:34:00Z
readers during rust ingest: 52 polls ... 50 sent ..., over 8 rounds in 8.958043921s; ...
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 9.00s
run 3 start 2026-11-11T11:34:09Z
readers during rust ingest: 54 polls ... 50 sent ..., over 11 rounds in 12.101097684s; ...
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 12.15s
run 4 start 2026-11-11T11:34:21Z
readers during rust ingest: 52 polls ... 50 sent ..., over 11 rounds in 11.324620473s; ...
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 11.37s
run 5 start 2026-11-11T11:34:33Z
readers during rust ingest: 54 polls ... 50 sent ..., over 11 rounds in 12.99636232s; ...
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 13.04s
2026-11-11T11:34:46Z
```

Five of five passed, using 8 to 11 of the 15 allowed rounds, so the round cap had little room on this machine.

**AFTER**: `folders.rs` top comment now names `Store::save_file_streamed` and states the retire rule correctly (a stored path that became a folder, or lies under an unlistable folder, is retired); `git diff -w` shows `//!` lines only. `ingest_coexist.rs`: `rounds < 15` became `started.elapsed() < READERS_DEADLINE` (120 s). Agent: readers test 5 of 5 (rounds 11, 10, 10, 11, 8), ignored half 10 passed; with the deadline set to 3 s the test fails with a message that has no path. Supervisor re-run in main: `cargo test -p easy-personal-rs --test ingest_coexist` 13 passed, 0 failed, 10 ignored.

**DELTA**: The readers coexistence test no longer fails on a slow machine because of a round count, and the `folders.rs` comment no longer misstates what the code does.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
