# TASK_REVIEW — T032: Ingest speed and memory gates (NFR-005), measured against Python

> Sibling of `tasks/TASK_GUIDE_T032.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/parity/bench_ingest.py` (generator, Rust and Python runs, table-equality check, three NFR-005 gates), `rust/crates/app/tests/ingest_perf.rs` (2 ignored gates: `memory_12mb_under_8_mib`, `unchanged_rerun_under_2s`), unit test `writer::tests::a_read_error_part_way_rolls_the_file_back` |
| Verification command run | ☑ pass | Supervisor, own runs 2026-11-11 (quiet machine): `bench_ingest.py` at 09:29Z: A (209,715,201 bytes, 72 files) Rust 1.18 s = 169.7 MB/s vs Python 2.69 s = 74.5 MB/s; C (unchanged) Rust 0.01 s vs Python 0.45 s; B (12.6 MB) Rust +5.8 MiB vs Python +4.9 MiB above baseline; tables `equal` for every cold run; all three gates PASS for both implementations; ignored cargo gates `2 passed` (debug: delta 7788 KiB, re-run 0.031 s, 0 new events); `cargo test --workspace --no-fail-fast` = 420 passed, 0 failed, 25 ignored under TZ unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago` (09:41-09:46Z); fmt ok; clippy `-D warnings` exit 0 (default and `--features window`); `git status` for `src tests web` = 0 entries |
| Negative cases hold | ☑ pass | agent-reported red/green on the memory gate in release: the prior code put back gave `peak_kib=22140 delta_kib=17968` and `peak grew 17968 KiB` FAILED; the fix gave `peak_kib=9384 delta_kib=4928` ok. Measured growth before the fix about 0.47 KiB per event (5k lines 12,012 KiB; 40k lines 28,564 KiB), flat after (9.5-9.7 MiB). The rollback unit test proves a mid-file error stores nothing. Not re-run by the Supervisor (he ran the green side) |
| verify | ☑ pass | real runs of the release binary and the real Python ingest over the same 200 MB set and the same 12 MB file with equal tables (above); the ignored cargo gates ran the real binary under `/usr/bin/time`. The `/verify` recipe itself was not edited |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | read by the Supervisor: the whole diff of `writer.rs` (105 changed lines: `save_file_streamed`, the delegation, the unit test) and `folders.rs` (25 changed lines: `read_from`, `scan`); `bench_ingest.py` and `ingest_perf.rs` were not read line by line (their output was reproduced). Scope: those two core files, the two new files, the `rust/parity/README.md` section, this review file; `src`, `tests`, `web`, other core files, Cargo files untouched. code-review report `reports/code-review_main_20261111T164747.html`: 0 P0, 0 P1, 1 P2, 5 P3, 1 info. security-review `reports/security-review_main_20261111T164748.html`: nothing at confidence 8 or above |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 420 passed, 0 failed under three zones (Supervisor), including the S01-S14 golden replays and the T031 coexistence default half; the only two flaky tests met on the way were handled: T031's lock-bypass red control (made reliable) and one unproven display test flake (open P3) |
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

**BEFORE**: captured by qa-automation (T032) before any file of this task existed, in `rust/`:

```
$ date -u; ls parity/bench_ingest.py crates/app/tests/ingest_perf.rs; echo "exit=$?"
2026-11-11T08:57:52Z
ls: cannot access 'parity/bench_ingest.py': No such file or directory
ls: cannot access 'crates/app/tests/ingest_perf.rs': No such file or directory
exit=2
$ date -u; cargo build --release && ./parity/.venv/bin/python parity/bench_ingest.py; echo "exit=$?"; date -u
2026-11-11T08:57:52Z
   Compiling hyper-util v0.1.21
   Compiling axum v0.8.9
    Finished `release` profile [optimized] target(s) in 21.42s
./parity/.venv/bin/python: can't open file '/home/thunderkds/workspace/pets/easy-personal-mcp/rust/parity/bench_ingest.py': [Errno 2] No such file or directory
exit=2
2026-11-11T08:58:14Z
```

The Verification Command stops at its second step: there is no benchmark and no `ingest_perf` cargo test.

**AFTER**: Supervisor 2026-11-11T09:29Z: Rust 169.7 MB/s vs Python 74.5 MB/s on 200 MB (tables equal), unchanged re-run 0.01 s vs 0.45 s, 12 MB file +5.8 MiB vs +4.9 MiB; gates all PASS; `cargo test --workspace` = 420 passed, 0 failed (three zones).

**DELTA**: the Rust ingest is measured, not assumed, to meet NFR-005 (and to be about 2.3 times faster than Python with identical output), and its memory no longer grows with the size of one transcript file.

**WITNESS**: Supervisor re-ran the benchmark, the ignored gates and the workspace tests after the implementing agent; tool calls recorded in `memory/event-trace/T032.jsonl`.

## Notes for the owner

1. **Result:** all three NFR-005 gates pass for Rust on this machine (AMD Ryzen 7 5700X, 16 GB RAM, WSL2, ext4): 169.7 MB/s cold (gate 8), 0.01 s unchanged re-run (gate 2 s), +5.8 MiB for a 12 MB file (gate 8 MiB). Python also passes its own gates (74.5 MB/s, 0.45 s, +4.9 MiB).
2. **⚠ decided for you:** the streaming change moves git project resolution and file reading inside the write transaction (see the P2). Accepted because the alternative (collecting lines first) fails the memory gate; Python also streams inside its transaction.
3. **Open P3:** one unproven flake in `display::tests::when_both_fail_the_message_names_each_variable_and_no_path_or_value` (a just-dropped socket checked without the `refused_soon` wait its siblings use); 0 of 30 failures in isolation.
4. **Disk:** the benchmark leaves 1.4 GB under `rust/target/bench` (git-ignored, safe to delete).
