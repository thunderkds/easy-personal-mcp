# TASK_REVIEW — T027: Rust store writer — lock, one transaction per file, the exact upsert, v11 only

> Sibling of `tasks/TASK_GUIDE_T027.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/writer_open.rs` (375 lines: open rules, refusal table, pragmas, machine_id, created-store equality), `writer_lock.rs` (239: flock against `flock(1)` and Python, 0700, never deleted), `writer_save.rs` (809: one transaction, rollback, idempotency, back-fill, FK, retire, prune), `writer_parity.rs` (483: replays T026 goldens S02, S04, S06, S08b, S13, S14), `writer_python.rs` (121, ignored: real Python opens a Rust-created store), unit test `writer::tests::every_connection_has_the_python_pragmas` |
| Verification command run | ☑ pass | Supervisor, own runs 2026-11-11T06:11:37Z: `cargo test --workspace --no-fail-fast` = 322 passed, 0 failed, 9 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`; `cargo fmt --check` ok; clippy `-D warnings` 0 warnings (default and `--features window`). The guide's own command (`cargo test -p easy-personal-core -- --ignored`) cannot pass, even on the old tree (it runs helper tests); the guide now names `--test writer_python -- --ignored` |
| Negative cases hold | ☑ pass | agent-reported mutation checks, each red then restored (files byte-equal afterwards): M1 `try_lock` replaced by `Ok(())` → `writer_lock` 5 failed (incl. the flock-holder test); M2 upsert `WHERE` dropped → `writer_save` 3 failed (`the_same_batch_twice…`, `a_re_save_with_null_never_erases`, `re_reading_the_fixture_files_adds_nothing`); M3 commit before the offset → `a_failure_on_the_offset_rolls_back_the_whole_file` failed. Not re-run by the Supervisor. The agent wrote the tests right after the code (no red run before it) and says so |
| verify | ☑ pass | real runtime checks by the Supervisor: `cargo test -p easy-personal-core --test writer_python -- --ignored` = `python exit Some(0)`, `user_version 11`, `python_added 1`, `rust_added_for_the_python_row 0`, `... ok`: the real Python CLI opened a Rust-created store without migrating it (no `.bak-v*`), read Rust's `machine_id`, and Rust re-saving Python's row added 0. The lock tests ran against `flock(1)` and a real Python `fcntl.flock` holder (the agent's `/proc/locks` shows type FLOCK). The `/verify` recipe itself was not run (this task adds a library, no CLI yet) |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | read by the Supervisor: `writer.rs` open rules (168-243), the error mapping (63-114), `save_file` and its SQL (262-405), the `Display` and risky-call greps of `writer.rs` and `ingest_lock.rs`; the prune/test sections and the five test files were not read line by line (their results were re-run). Scope: only `rust/crates/core` (`src`, `tests`), `Cargo.toml`/`Cargo.lock` (one line: `getrandom` for the core, already locked, MIT OR Apache-2.0) and `rust/README.md`; nothing in `src/`, `tests/`, `rust/parity/`, `rust/crates/app/`. code-review `reports/code-review_main_20261111T131355.html`: 0 P0, 0 P1, 1 P2 (fixed), 4 P3. security-review `reports/security-review_main_20261111T131356.html`: nothing at confidence 8 or above |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 322 passed, 0 failed, 9 ignored under three time zones (Supervisor); no existing test changed; `store.rs` and the app crate untouched |
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

**BEFORE** (captured by backend-developer before any T027 code change; `rust/` tree as at `bed1eba`):

```
$ date -u; . "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && cargo test -p easy-personal-core -- --ignored && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
2026-11-11T05:28:56Z
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.40s
cargo test --workspace: every "test result:" line ok (0 failed in every binary)
cargo test -p easy-personal-core -- --ignored:
      1 test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.09s
      1 test usage_child ... FAILED
    Finished `dev` profile ... (clippy clean)
fmt ok
2026-11-11T05:29:14Z

$ grep -c "pub mod writer" rust/crates/core/src/lib.rs
0
```

Note: the guide's `cargo test -p easy-personal-core -- --ignored` step **fails on the current tree, before any change**:
the existing `#[ignore]` tests (`rich_parity::usage_child`, `sessions_parity::zone_child`,
`usage_tz_parity::zone_goldens_child`, ...) are child-process helpers that their parent test starts with its own env; run
directly they panic (`TZ=UTC ... --no-fail-fast -- --ignored`: `zone_child` and `zone_goldens_child` still fail). The
writer's ignored test is therefore run by target: `cargo test -p easy-personal-core --test writer_python -- --ignored`.

**AFTER**: Supervisor run 2026-11-11T06:11:37Z after the change: `cargo test --workspace`: 322 passed, 0 failed, 9 ignored (three zones); fmt ok; clippy 0; `grep -c "pub mod writer" rust/crates/core/src/lib.rs` = 1; the ignored real run printed `python exit Some(0)`, `user_version 11`, `python_added 1`, `rust_added_for_the_python_row 0`.

**DELTA**: the Rust core can open the shared v11 store for writing (or create it), take the same single-writer lock as Python, and save a file's rows and offset in one transaction exactly as Python does, so the two writers can share one store.

**WITNESS**: Supervisor re-ran the tests and the real Python run after the implementing agent; tool calls recorded in `memory/event-trace/T027.jsonl`.

## Choices for the owner (decided in `[admin]` mode, ⚠ decided for you — please review)

1. A store with tables at `user_version` 0 is refused as "older" (Python would migrate it): SAFER OPTION, Rust never migrates.
2. A file with no tables at `user_version` 1-10 is created fresh at v11.
3. A store newer than v11 is a clean exit-2 error (Python exits 1 with a traceback).
4. Transactions begin `IMMEDIATE` (Python: deferred at the first INSERT): same result, the count is read under the write lock.
