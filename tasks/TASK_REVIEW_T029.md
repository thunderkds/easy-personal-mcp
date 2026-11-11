# TASK_REVIEW — T029: easy-personal-rs ingest — the command, the lock, the JSON, the exit codes

> Sibling of `tasks/TASK_GUIDE_T029.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/ingest_cli.rs` (23 tests over the real binary plus 1 ignored real-Python run: help and usage errors, env, order, busy lock, refused stores, JSON text, S01-S07 S08a/b S10 replays, retention, two-process lock, canary scan), unit tests in `rust/crates/app/src/ingest_cmd.rs` (timeout parsing, path normalisation, `the_lock_is_free_as_soon_as_the_locked_part_returns`) |
| Verification command run | ☑ pass | Supervisor, own runs 2026-11-11T07:39:07Z after the follow-up: `cargo test --workspace --no-fail-fast` = 406 passed, 0 failed, 13 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`; `cargo fmt --check` ok; clippy `-D warnings` exit 0 (default and `--features window`); ignored `ingest_cli` real-Python test = `1 passed`; `cargo tree` for the default build: 0 Tauri/GTK/WebKit lines (agent); changed files only `main.rs`, `ingest_cmd.rs`, `tests/ingest_cli.rs`, `rust/README.md`; `rust/crates/core`, `rust/parity`, `src`, `tests`, `web` and Cargo files untouched |
| Negative cases hold | ☑ pass | agent-reported mutation checks, each red then restored byte-equal: path normalisation removed → the relative-source test; refused-store exit 2 changed to 1 → `s11_s12_…`; store opened before the lock → 16 of 23 tests including `s09`; follow-up: lock leaked past `ingest_locked` (`mem::forget`) → `the_lock_is_free_as_soon_as_the_locked_part_returns` red. A print inside the locked scope is not caught by that test (stated limit). Not re-run by the Supervisor |
| verify | ☑ pass | real runs by the Supervisor 2026-11-11 with literal binary paths, a temp data dir and agent root, `--source` on the committed fixtures: first run printed `files 6, new_events 39, skipped 0, not_ingested [context, kit_trace, ui, verifier]`, exit 0; second run `new_events 0`; `store-info` showed `user_version: 11`, `pending: no`; the Rust MCP `usage_summary` then read the Rust-filled store (abstained total 3, cache_read_input_tokens 1000). Plus the ignored test: Rust then Python and Python then Rust on one store, the second adds 0. The `/verify` recipe itself was not edited or run; the ingest step to add is listed in the agent's report and below |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | read by the Supervisor: the whole flow of `ingest_cmd.rs` (env, lock, store, ingest, prune, print, exit-code mapping); `ingest_cli.rs` and the README table were not read line by line (their results were re-run). Scope as listed above. code-review `reports/code-review_main_20261111T144116.html`: 0 P0, 0 P1, 1 P2 (fixed), 3 P3, 1 info. security-review `reports/security-review_main_20261111T144117.html`: nothing at confidence 8 or above |
| Full smoke suite still green (no regression) | ☑ pass | whole workspace 406 passed, 0 failed under three zones (Supervisor); every earlier test unchanged |
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

**BEFORE** (captured 2026-11-11 by the implementing agent, before any code change; main checkout, `cd rust`, `. "$HOME/.cargo/env"`):

```
$ date -u            -> 2026-11-11T07:25:31Z
$ cargo run -q -- ingest --help
error: unrecognized subcommand 'ingest'

Usage: easy-personal-rs <COMMAND>

For more information, try '--help'.
(exit 2)

$ cargo build                                  -> Finished `dev` profile [unoptimized + debuginfo] target(s)
$ cargo test --workspace   (2026-11-11T07:26:00Z)
  33 suites, passed=378 failed=0 ignored=12
$ cargo test -p easy-personal-rs --test ingest_cli -- --ignored   (2026-11-11T07:25:57Z, first failing step)
error: no test target named `ingest_cli` in `easy-personal-rs` package
help: available test targets:
    app
    cli
    mcp
    server
    window
    workspaces
```

**AFTER**: Supervisor, 2026-11-11, real binary on the committed fixtures into a temp home: `easy-personal-rs ingest --source tests/fixtures/claude_code` printed `{"files": 6, "new_events": 39, "not_ingested": ["context","kit_trace","ui","verifier"], "skipped": 0}` (indent-2 layout), exit 0; again: `new_events: 0`; `store-info` `user_version: 11`; the Rust MCP `usage_summary` read the store; `cargo test --workspace` = 406 passed, 0 failed (three zones).

**DELTA**: `easy-personal-rs ingest` fills the shared store from Claude Code transcripts with the same environment, lock, JSON and exit codes as `easy-personal ingest`, so the Rust app can now be used end to end (ingest, then serve, mcp or the window) without the Python app, for the events source.

**WITNESS**: Supervisor re-ran the tests, lint and the real binary after both implementing agents; tool calls recorded in `memory/event-trace/T029.jsonl`.

## Choices for the owner (decided in `[admin]` mode, ⚠ decided for you — please review)

1. The JSON prints `files`, `new_events`, `skipped` (+ `pruned`) and `"not_ingested": ["context","kit_trace","ui","verifier"]`; Python's output has those four sources' keys, Rust prints only what it did.
2. A store newer than v11 exits 2 with a clean message (Python: traceback, exit 1); an older store exits 2 with "run the Python app once" (Python migrates it): Rust never migrates in P2.
3. Skipped entries (a directory named `x.jsonl`, unreadable files, non-UTF-8 paths) are counted and announced with one fixed stderr note; Python crashes on some and ignores others silently.
4. A malformed `EASY_PERSONAL_LOCK_TIMEOUT` is a clean exit-1 error; `inf`/`nan` is capped at about 31 years; `HOME` unset is a clean error (no passwd fallback).

## `/verify` ingest step to add (for a later task; `skills/verify/SKILL.md` was not edited)

In a scratch folder with `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT` inside it: `easy-personal-rs ingest --source tests/fixtures/claude_code` expects `files 6, new_events 39, skipped 0`, exit 0; again expects `new_events 0`; `--retention-days 1` expects `pruned 26`; with another process holding `flock` on `<home>/ingest.lock` and `EASY_PERSONAL_LOCK_TIMEOUT=0.2`, exit 75 with the exact stderr line; a store set to `user_version` 12 or 10 exits 2 with the bytes unchanged; then `cargo test -p easy-personal-rs --test ingest_cli -- --ignored`.
