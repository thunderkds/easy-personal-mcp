# TASK_GUIDE — T032: Ingest speed and memory gates (NFR-005), measured against Python
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: qa-automation
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

Before writing anything:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the 2026-11-11 entry "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/qa.md`
5. Read the Python gates: `tests/sources/test_ingest_perf.py`, `tests/ops/test_ingest_memory.py`, `docs/spikes/transcript-shapes.md`, and NFR-005 in `PRD.md` and `rust/docs/SPEC.md`
6. Read the Rust you measure: `rust/crates/app/src/ingest_cmd.rs`, `rust/crates/core/src/ingest/folders.rs` (note `read_from` collects a file's `Line`s before the single `save_file`: the open P2 finding of T035's review), `rust/crates/core/src/writer.rs` (`save_file`), `rust/crates/core/tests/ingest_parse_perf.rs` (the parse-layer numbers: 690 MB/s generated, 203 MB/s on real-shaped lines, +1.5 MiB for 12 MB)
7. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Data is generated into temp folders on the Linux filesystem

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): the Rust ingest (P2a) must meet the ingest speed and memory targets (NFR-005) that the Python app is held to.

**Restated intent**:
> A repeatable benchmark generates transcript data, runs the **release** Rust `ingest` and the Python `ingest` over it, and reports real numbers for the three gates (cold ingest of 200 MB at 8 MB/s or better, an unchanged re-run under 2 s, peak memory under 8 MiB above baseline for a 12 MB file) with the Python figures beside them; if Rust misses a gate because of a defined, local cause, a minimal fix is made and proven not to change any table.

**Out of scope**: speed work on read-side code (metrics, API); new features; multi-threading; changing the schema or the dedup key; changes outside the benchmark and, **only if a gate fails**, the one change described in criterion 5.

**Requirement Refs**: NFR-005 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T029 (the `ingest` command), T035, T027
**Entry point**: `rust/parity/bench_ingest.py` (benchmark driver, parity venv) and `cargo test -p easy-personal-rs --test ingest_perf -- --ignored` (the Rust-side gates)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New `rust/parity/bench_ingest.py` (parity venv; deterministic, seeded) generates, into a temp folder, transcript files of realistic shape (about 3.8 KB per line as in `docs/spikes/transcript-shapes.md`: mixed `user`, `assistant` with usage, tool use, metadata lines) in these sets: **A** 200 MB over many files, **B** one 12 MB file, **C** set A again unchanged; it measures wall time (`time.perf_counter`) and peak RSS (`/proc/<pid>/status` `VmHWM` of the child, or `resource.getrusage(RUSAGE_CHILDREN)`) for each run | measured, not asserted |
| 2 | **Runs and report**: Rust **release** binary (`cargo build --release`) cold ingest of A, re-run of C, ingest of B; the same three for the **Python** `ingest` from the parity venv (fresh data dir each); each cold run repeated 3 times, median reported; the report is a table: run, implementation, seconds, MB/s, peak RSS MiB, `new_events`; plus the machine facts (CPU model, cores, filesystem of the temp folder, git/SQLite versions). Paste it | comparable numbers |
| 3 | **Gates** (report pass/fail with the real numbers, do not tune the thresholds): cold ingest of A ≥ **8 MB/s**; unchanged re-run (C) < **2 s**; peak RSS above the idle baseline for B < **8 MiB**. Also report the memory of A (the 200 MB run) so the growth with file size is visible | NFR-005 |
| 4 | **Same data**: after each Rust and Python cold ingest of A and B, the `events`, `tool_signals` and `source_files` tables are **equal** between the two implementations (compare as the T026 goldens do; ids, machine_id and inodes excluded). A speed number is only valid if the tables match | speed without drift |
| 5 | **If a gate fails**: find the cause by measurement (a flame graph or counters, not a guess), and make the **smallest** fix. The known candidate: `folders.rs::read_from` collects a whole file's lines before the one transaction, so memory grows with the events of one file; the fix would be to stream lines into `save_file` inside its single transaction (an iterator-taking variant of `Store::save_file`), keeping **one transaction per file** and every table identical. Any such change must keep **all** existing tests green under three time zones (the S01-S14 golden replays included) and add a test that pins the new memory bound; list every file touched; if the gate cannot be met without a larger change, stop, report the numbers and the options, and do not make it | measured fix only |
| 6 | **Rust-side gates in cargo** (`rust/crates/app/tests/ingest_perf.rs`, `#[ignore]`d, release-profile hints in comments): the 12 MB memory gate and the unchanged-rerun gate over a generated set, measured in a child process (`VmHWM`), with thresholds from NFR-005 and a generous CI margin documented; the 200 MB run stays in the Python driver | repeatable in cargo |
| 7 | **No regression elsewhere**: `cargo test --workspace` under `TZ` unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago` (paste all three); clippy and fmt clean (also `--features window`); `git status` shows only the files you list; the Python tree is untouched (`git status -- src tests web` empty); no new crate | quality |
| 8 | **Docs**: a "Ingest speed and memory" section in `rust/parity/README.md`: how to run, the table of numbers, and the machine; the gates and their results go into your report | usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T032.md`.
> **Locked oracle**: the Python ingest and `docs/spikes/transcript-shapes.md` (read-only).

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | set A, release binary | ≥ 8 MB/s, tables equal Python's | `bench_ingest.py` |
| 2 | set C (unchanged) | < 2 s | `bench_ingest.py` |
| 3 | set B (12 MB) | < 8 MiB above baseline | `bench_ingest.py` and the ignored cargo test |
| 4 | `git status -- src tests web` | empty | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain (release build), git and rust/parity/.venv; the 200 MB set takes minutes
. "$HOME/.cargo/env" && cd rust && cargo build --release && ./parity/.venv/bin/python parity/bench_ingest.py && cargo test -p easy-personal-rs --test ingest_perf -- --ignored
```

---

## Approach

**Pattern reference**: `rust/crates/core/tests/ingest_parse_perf.rs` (child-process `VmHWM`), `tests/sources/test_ingest_perf.py`, `rust/parity/gen_ingest.py` (guards).
**Vital slice**: generator and the Rust run of B (the memory gate) → A and C → the Python runs and the equality check → fix only if a gate fails → cargo gates → docs.
**Cut list**: no parallel ingest, no mmap, no schema change, no changes to read code.

---

## Edge Case Checklist

- [ ] Peak RSS is measured on the child only, above an idle baseline taken from the same binary doing `--version`, so the numbers are comparable
- [ ] The generator's output is valid transcript JSON, never real content, no secrets or paths
- [ ] The 200 MB set is written under the temp folder only and the script says how much disk it needs (and refuses below 1 GB free)
- [ ] The OS page cache is noted (cold vs warm) in the report; do not call a warm run "cold"
- [ ] Wrap every run in `timeout` (the Python run of A may take minutes); report a timeout as a result
- [ ] Stale references: `rust/parity/README.md` and `BRAINSTORMING_LOG_RUSTP2.md` wording about NFR-005 (README only)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/parity/bench_ingest.py` (new) | generator, runs, equality check, report |
| `rust/crates/app/tests/ingest_perf.rs` (new) | the ignored Rust-side gates |
| `rust/parity/README.md` | the "Ingest speed and memory" section |
| `rust/crates/core/src/ingest/folders.rs`, `rust/crates/core/src/writer.rs` | **only if criterion 5 triggers**, the minimal streaming change, listed in the report |
| `tasks/TASK_REVIEW_T032.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/parity/ingest/**`, `gen_ingest.py`, `gen_ingest_corpus.py`, the older goldens | the oracle |
| `rust/crates/core/src/ingest/{claude_code,json,time,label,secret,signals,accounts}.rs`, `rust/crates/app/src/**` | not part of this task |
| `skills/verify/SKILL.md`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Memory gate on B first (it decides whether criterion 5 triggers), then A and C for both implementations, the equality check, the optional fix with the full test matrix, the ignored cargo gates. Paste every table and output.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T032.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] The benchmark table (both implementations), the gates verdicts and the machine facts pasted
- [ ] Supervisor notified: task ready for Stage 4 review
