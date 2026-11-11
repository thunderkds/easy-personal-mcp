# TASK_REVIEW — T028: claude_code parse layer in Rust — lines, JSON, time, labels, signals, records

> Sibling of `tasks/TASK_GUIDE_T028.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/ingest_parse_corpus.rs` (time, label, word, secret, bash, host corpora), `ingest_parse_json.rs` (11: Python json.loads leniencies, depth, ints), `ingest_parse_record.rs` (framing, dispatch, privacy canaries, fuzz, `bash_word` robustness), `ingest_parse_replay.rs` (S01 45 rows, S06 169 rows, S03, S05, S06a/b/d), `ingest_parse_perf.rs` (12 MB memory in a child process, ignored 200 MB speed test); corpora `rust/parity/ingest/golden/corpus-time.json` (2,059) and `corpus-label.json` (1,468) from `rust/parity/gen_ingest_corpus.py` |
| Verification command run | ☑ pass | Supervisor, own runs 2026-11-11T06:50:26Z and 06:59:53Z: `cargo test --workspace --no-fail-fast` = 356 passed, 0 failed, 10 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago` (Pago_Pago four times: all 356 passed); `cargo fmt --check` ok; clippy `-D warnings` clean in both builds (a stray exit 1 in my own loop was zsh not splitting an unquoted `--features window`; run properly it is exit 0, also after a forced rebuild); `gen_ingest_corpus.py --check` = `2 files, 0 different`; `git status` for `src tests web rust/crates/app writer.rs Cargo files S01 golden` = 0 entries |
| Negative cases hold | ☑ pass | agent-reported mutation checks, each red then restored byte-equal: `$` quirk broken → `label_corpus_equals_python`; only 3 fractional digits → `time_corpus_equals_python`; bool counted as int → `usage_needs_four_exact_ints`; NaN invalidates the line → `nan_and_infinity_are_floats`; tool_result text reaching a field → `canaries_never_reach_an_event`; follow-up: lone-surrogate rule removed → `dispatch_cases` red; `bash_word` with mismatched width → `bash_word_never_panics_on_any_bytes` red (after adding a multi-character case, the first mutation had stayed green). Not re-run by the Supervisor |
| verify | ☑ N/A | a library with no runtime surface yet (the `ingest` command is T029); the real-oracle evidence is the replay of the Python goldens (S01 45 rows, S06 169 rows equal) and the two corpora. The `/verify` recipe was not run |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | read by the Supervisor: `signals.rs` `bash_word` and its helper, and a grep of `rust/crates/core/src/ingest/*.rs` for `unsafe`, `unwrap`, `expect`, `panic!` (none outside tests); the rest (`json.rs` 530 lines, `time.rs` 336, `secret.rs` 87, `claude_code.rs` 263) was not read line by line: proven by the corpora, the replay, 10,000 fuzzed lines and the mutation table. Scope: new files under `rust/crates/core/src/ingest/`, five test files, `lib.rs`, the corpus script and two corpora, `rust/README.md`; nothing in `src/`, `tests/`, `writer.rs`, `rust/crates/app/`, Cargo files (no new crate). code-review report at `reports/code-review_main_20261111T140327.html`: 0 P0, 0 P1, 0 P2, 5 P3. security-review at `reports/security-review_main_20261111T140328.html`: nothing at confidence 8 or above |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 356 passed, 0 failed under three zones (Supervisor); all earlier tests unchanged |
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

**BEFORE**: captured by backend-developer before the first T028 code change (the verification command, test lines condensed to `test result` lines):

```
$ date -u; . "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check && ./parity/.venv/bin/python parity/gen_ingest_corpus.py --check
2026-11-11T06:17:04Z
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
test result: ok. ... (25 test binaries, all ok, 0 failed)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s      <- clippy clean
./parity/.venv/bin/python: can't open file '/home/thunderkds/workspace/pets/easy-personal-mcp/rust/parity/gen_ingest_corpus.py': [Errno 2] No such file or directory
exit=2
$ grep -c "pub mod ingest" rust/crates/core/src/lib.rs
1            <- matches `pub mod ingest_lock;` (line 2), not an ingest module
$ date -u; grep -n "pub mod ingest" rust/crates/core/src/lib.rs; grep -cx "pub mod ingest;" rust/crates/core/src/lib.rs; ls rust/crates/core/src/ingest; git status --short -- rust/crates rust/parity | wc -l
2026-11-11T06:17:33Z
2:pub mod ingest_lock;
0
ls: cannot access 'rust/crates/core/src/ingest': No such file or directory
0
```

**AFTER**: Supervisor run 2026-11-11T06:59:53Z: `cargo test --workspace` = 356 passed, 0 failed (three zones); `gen_ingest_corpus.py --check` = `--check: 2 files, 0 different`; the agent's own run of the verification command ended `--check: 2 files, 0 different`; `grep -cx "pub mod ingest;" rust/crates/core/src/lib.rs` = 1 (the guide's plain `grep -c "pub mod ingest"` also matches `pub mod ingest_lock;`, so BEFORE used the exact-line form).

**DELTA**: the Rust core can read a Claude Code transcript file and produce exactly the events and tool signals the Python app would store (proven on 214 oracle rows and 3,500 corpus strings), keeping no message text, and surviving the inputs that crash Python.

**WITNESS**: Supervisor re-ran the tests, lint and corpus check after both implementing agents; tool calls recorded in `memory/event-trace/T028.jsonl`.

## Choices for the owner (decided in `[admin]` mode, ⚠ decided for you — please review)

1. **Rust differs from Python only where Python crashes and sticks its ingest:** nesting deeper than 57,890 levels (all-NULL abstained row); an integer beyond int64 in a usage field (record abstains, type/version/ts/session kept); a list- or object-valued `type` (abstains, `record_type` NULL); a timestamp whose UTC value leaves years 1-9999 (no timestamp, a versioned record abstains); a `cwd` holding a lone surrogate (record abstains).
2. A secret-like value is **not** blocked from being a stored label (Python has no such check); only the Bash first word and the WebFetch host are screened, exactly as in Python.
3. The depth limit 57,890 was measured on CPython 3.14.4 with an 8 MB stack; a different interpreter would measure differently (the case is a crash in Python, so Rust is never worse).
4. Three Unicode tables are pinned to Python 3.14 / Unicode 16.0 and checked by a test.
