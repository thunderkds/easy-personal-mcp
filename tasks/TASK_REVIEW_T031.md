# TASK_REVIEW — T031: Coexistence tests — two writers, one store, no corruption, no surprises

> Sibling of `tasks/TASK_GUIDE_T031.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/ingest_coexist.rs` (23 tests: 13 Rust-only run by default, 10 with the real Python app `#[ignore]`d), helpers `rust/crates/app/tests/ingest_coexist_common/mod.rs` (guarded `Spec`/`Proc`, barrier races, `checked_dump`, seeded large set, serve and mcp clients) |
| Verification command run | ☑ pass | Supervisor 2026-11-11: `cargo test --workspace --no-fail-fast` = 419 passed, 0 failed, 23 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago` (08:12-08:16Z); after the follow-up `cargo fmt --check` ok, clippy `-D warnings` exit 0 (default and `--features window`); the ignored half `--ignored --test-threads=1` = `10 passed; 0 failed` in 402.82 s (08:48-08:55Z); the previously failing kill test passed twice more (70 s each); no `easy-personal-rs` process left. My first run of the ignored half (before the follow-up) had failed 1 of 10 (`ingest.lock is kept`, a wrong test assumption, fixed) |
| Negative cases hold | ☑ pass | agent-reported: with the lock bypassed (two data folders share one store through a symlink) `the_race_oracle_sees_two_writers_when_the_lock_is_bypassed` observes two writers (`new_events 11200 and 2800` at attempt 2) while integrity stays ok; two `should_panic` guard tests (`EASY_PERSONAL_HOME is not set`, `EASY_PERSONAL_AGENT_ROOT is outside the temp folder`); follow-up mutation: deleting `ingest.lock` after a kill that saved files makes `ingest.lock is kept` fail, file restored byte-equal. Not re-run by the Supervisor |
| verify | ☑ pass | these are real processes by design: the Rust binary and the real Python ingest raced on one store (160 races in 8 cases of 20 (Rust against Rust 3 cases, Rust against Python 3, Python against Python 2 as the control), every case 20 of 20 ending in the golden S01 or S06 tables with integrity ok), killed with SIGKILL (22 kills: 6 of 7, 6 of 7, 7 of 8 landing while running) and resumed to the clean tables, while `serve`, `mcp` and `store-info` answered (54 and 126 polls, 50 counted each, no 5xx, no "locked"). The `/verify` recipe itself was not edited |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | read by the Supervisor: `kill_then_resume`, `precreate_store`, the kill-point selection and the failing assertion; the other tests were re-run, not read line by line. Scope: `rust/crates/app/tests/ingest_coexist.rs`, `ingest_coexist_common/mod.rs`, a short `rust/README.md` note, this review file; no product code, `src/`, `tests/`, `rust/parity/`. code-review `reports/code-review_main_20261111T155538.html`: 0 P0, 0 P1, 1 P2, 3 P3. security-review `reports/security-review_main_20261111T155539.html`: nothing at confidence 8 or above |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 419 passed, 0 failed under three zones (Supervisor); every earlier test unchanged |
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

**BEFORE** (captured 2026-11-11T07:42:16Z, before any T031 file existed, in `rust/`):

```
$ date -u; cargo test -p easy-personal-rs --test ingest_coexist
2026-11-11T07:42:16Z
error: no test target named `ingest_coexist` in `easy-personal-rs` package
help: available test targets:
    app
    ...
    ingest_cli
    mcp
    server
    window
    workspaces

$ cargo test --workspace 2>&1 | grep '^test result:'   (summed)   finished 2026-11-11T07:42:43Z
workspace: passed=406 failed=0 ignored=13 (sum of 34 test result lines)
```

**AFTER**: Supervisor 2026-11-11: `cargo test --workspace` = 419 passed, 0 failed (three zones); the ignored real-Python half `10 passed; 0 failed` (402.82 s); kill-and-resume after the fix prints, per point, `died before the lock step` or `past the lock step` (2 of 8 landed before the lock step in each run: fixed 305 ms and seed 77).

**DELTA**: the claim that the Rust and Python ingests can safely share one store is now tested with real processes (simultaneous writers, SIGKILL and resume, readers during writes, copied files, version guards, odd environments) and no defect was found.

**WITNESS**: Supervisor re-ran the workspace tests, lint and the whole real-Python half after the implementing agents; tool calls recorded in `memory/event-trace/T031.jsonl`.

## Notes for the owner

1. **Defects found in the product: none.** The one failure the Supervisor saw was a wrong assumption in the test (a lock file that does not exist when Python dies during its import phase), fixed.
2. **Deviation from the guide (⚠ decided for you):** the whole file runs in about 66 s for the default half and about 400 s for the ignored Python half, not under 3 minutes: the Python ingest of a 6 MB set is slow. The Python half is an ignored manual gate.
3. **Known fragile test:** `py_serve_mcp_and_store_info_answer_while_a_python_ingest_runs` (cap of 15 rounds to collect 50 polls) failed once in an agent run; it passes on re-run. Relax the cap if it flakes in CI.
4. **Disk:** the tests need free space on `/tmp`; on this machine `/tmp` is a RAM-backed 7.8 GB filesystem that was full of scratch copies (cleared).
