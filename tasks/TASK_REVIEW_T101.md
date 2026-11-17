# TASK_REVIEW — T[NNN]: [Short Title]

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (AC 1-5, 7; AC 6, 8 are the Supervisor's at Stage 5) | `rust/crates/app/src/window.rs` `mod tests` (AC 1, 2: `only_a_plain_window_run_wants_a_tray`, `the_tray_runs_this_exe_with_the_same_folder`, `a_failed_tray_start_is_one_line_and_app_opens_its_own_window`, `the_tray_child_has_its_own_group_and_null_stdio`); `rust/crates/app/src/tray.rs` `mod tests` (AC 3, 4: `write_pid_leaves_the_pid_and_keeps_the_lock`, `read_pid_gives_none_without_a_number_and_waits_for_one`, `only_an_easy_personal_tray_passes_the_cmdline_check`, `a_gone_pid_and_this_test_process_are_not_trays`, `hand_over_signals_only_a_pid_that_is_a_tray`); `rust/crates/app/tests/tray_show.rs` (AC 5: `a_pid_that_is_not_a_tray_gets_no_signal`, `a_lock_with_no_pid_is_exit_1_after_the_wait`, `without_show_a_held_lock_is_unchanged`, plus `a_running_tray_gets_sigusr1_and_the_ask_exits_0` with a shell named `easy-personal-rs tray` as a stand-in); `rust/crates/app/tests/app.rs` (AC 7: `app --help` lists `--no-tray`, `tray --help` lists `--show`) |
| Verification command run | ☑ pass | 2026-11-17, in the T101 worktree, `cd rust`: `cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets --features window -- -D warnings` and without the feature: `Finished` with no warning; `cargo test --workspace` (default): 96 targets, 1104 passed, 0 failed; `cargo test --workspace --features window --no-fail-fast`: 96 targets, first run 1108 passed, 6 failed = 5 in `tests/app.rs` (see Negative cases) + `ingest_coexist::a_killed_rust_ingest_leaves_whole_files_and_a_rust_rerun_equals_a_clean_run` (`precreate_store` `NotFound` under load; alone: `test result: ok. 1 passed`, 93 s); after the `tests/app.rs` fix `--test app` is `test result: ok. 8 passed; 0 failed`. App unit tests: `test result: ok. 256 passed; 0 failed` (both builds). `tests/tray_show.rs` `--features window`: `test result: ok. 4 passed; 0 failed`. Leak gate `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py` from `rust/`: `leak check: 0 hits over 42 canaries, 866 surfaces, 2037147 bytes` |
| Negative cases hold | ☑ pass | RED before the helpers existed (`cargo test -p easy-personal-rs --bin easy-personal-rs tray::tests`): `error[E0425]: cannot find function` `write_pid` x2, `read_pid` x5, `cmdline_is_tray` x1, `is_tray` x4, `hand_over_to` x4, `error[E0433]` `Duration`/`Instant`; `window::tests`: `cannot find function own_window_line` x2, so `the_tray_runs_this_exe_with_the_same_folder` (now expects `--show`) was red too. GREEN after: the 5 tray tests and 4 window tests pass. No signal sent: `a_pid_that_is_not_a_tray_gets_no_signal` keeps a `sleep 30` alive (a `SIGUSR1` would end it) and `hand_over_signals_only_a_pid_that_is_a_tray` records no pid. Pre-existing, found on the window build: 5 `tests/app.rs` cases called `app` without `--no-ingest`, so `main.rs`'s start-up ingest (T051, unchanged here) printed its progress line first and `one_line` / the exit code failed (`assertion left == right failed: ingesting before start (waits if another ingest runs)... left: 2 right: 1`); fixed by adding `--no-ingest` to those 6 invocations |
| verify | ☑ pass | auto-run by hook, session fecf0cf3-0909-4a5a-9206-6f8d5e4f0bda, commit 63c0bf882ac704d0eef21f0b9312c5c46f49dab2, log .claude/verify-runs/T101-63c0bf882ac7.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor inline code review 2026-11-17 over `main...T101`: window.rs, tray.rs, main.rs, tests/app.rs, tests/window.rs, tests/tray_show.rs, both READMEs; stale-reference sweep (site/, README.md, rust/docs) 0 hits; lane_check `EXCEEDS (lines 687 > 500)`, mostly tests, accepted. Findings: 0 P0/P1; P2 silent launcher with an older tray (fixed `eb54b66`); P3 module doc width (fixed); P3 open: plain `app` still ingests before it hands over (kept: the tray does not ingest at start). Startup reads complete. Security pass (Medium): signal only to a pid whose `/proc/<pid>/cmdline` is `easy-personal-rs tray`, lock in the private runtime dir, `kill` limited to the same user, argv spawn with no shell, no secrets or network: 0 findings |
| Full smoke suite still green (no regression) | ☑ pass | workspace 1106 passed (default), 1115 passed + 1 known load flake `ingest_coexist` (passes alone) with `--features window`; Stage 5 D0-D2 above |
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

**BEFORE** (captured 2026-11-17T08:02:46Z on branch T101 at commit 87f0675, before any implementation commit; default-feature build; no tray was started or killed):

```
2026-11-17T08:02:46Z
$ grep -n -i tray crates/app/src/window.rs  (lines 146-260: fn run, start, open)
31:    /// engine when `alerts`): what `app` and `tray` both run before any Tauri code. With `clicks` (the `tray` only) a
(exit 0 : no spawn of a tray in run/start/open; only a doc comment on line 176 of start)
$ easy-personal-rs app --help
Open the dashboard in a window (needs a build with `--features window` and a display)

Usage: easy-personal-rs app [OPTIONS] --static-dir <DIR>

Options:
      --static-dir <DIR>  Serve the built dashboard's files from this folder, from the same origin as the API
      --no-ingest         Do not ingest before the start (default: ingest first)
      --alerts            Raise desktop alerts (a session waiting or failing, a plan limit, disk, RAM or battery) over the session D-Bus; they are listed at `GET /api/alerts`
  -h, --help              Print help
```

**AFTER** (code and unit/integration level; AC 6 and 8 on the owner's display are the Supervisor's Stage 5): `app --help` (default build, 2026-11-17) now lists `--no-tray  Open this process's own window and start no tray (by default \`app\` starts the tray, which owns the one dashboard window, and exits)`; `tray --help` lists `--show  Show the dashboard window: open it at start, or when a tray already runs ask that tray to show its window and exit`. `tests/tray_show.rs` (no display, temp `XDG_RUNTIME_DIR`): a held lock whose pid is a stand-in `easy-personal-rs tray` receives `SIGUSR1` and `tray --show` exits 0 with no output; a pid that is not a tray, or no pid, is exit 1 with one stderr line and no signal.

**Stage 4 fix round** (2026-11-17): P2, a launcher click while a tray that cannot be asked (an older one, no pid) holds the lock was silent: `hand_over_to_tray` now watches the spawned `tray --show` for up to 3 s (`HAND_OVER_WAIT`, `observe`) and decides with the pure `after_spawn`: still running or exit 0 = `app` exits 0; a non-zero exit = `easy-personal-rs: tray not started: the tray exited with exit status: 1` and `app` opens its own window. Tests: `app_exits_for_a_running_or_a_clean_tray_and_falls_back_for_a_failed_one` (all three cases plus a spawn error) and `observe_sees_a_failed_exit_a_clean_exit_and_a_running_child` (real `/bin/sh -c 'exit 1'`, `exit 0`, and a `sleep 30` the test ends) pass; `own_window_line` was replaced, so its test was rewritten (no separate RED run was captured for these two, they were written with the code). P3, `tray.rs` module doc rewrapped to the file's width. READMEs say `app` opens its own window for an older tray. Re-run: fmt 0, clippy both builds clean, `cargo test --workspace` 96 targets 1106 passed 0 failed; `--features window --no-fail-fast` 96 targets 1115 passed, 1 failed = `ingest_coexist::a_killed_rust_ingest_...` (`listing` `read_dir` NotFound at `ingest_coexist.rs:91`, the same load flake as before, in code this task does not touch; alone: `test result: ok. 1 passed`, 79 s); leak gate `leak check: 0 hits over 42 canaries, 866 surfaces, 2037897 bytes`.

**Stage 5 real-machine run** (Supervisor, owner's GNOME Wayland desktop, 2026-11-17, release build `--features window` of `eb54b66`, static dir `/usr/share/easy-personal-rs/static`, `--no-ingest`):
- D0 09:02:28Z, an installed 0.3.0 tray (no pid in its lock) running: `app` printed `easy-personal-rs: tray not started: the tray exited with exit status: 1` then `window open on http://127.0.0.1:35559/` (own-window fallback; ended by `timeout 15`).
- D1 09:02:57Z, the old tray ended (the Supervisor had started it): `app` exit 0; processes: one `easy-personal-rs tray --static-dir ... --show` (pid 1460688), no `app`; 1 `WebKitWebProcess` under the tray (its window).
- D2 09:03:04Z, `app` again: exit 0; still one tray (1460688), still 1 `WebKitWebProcess`; `$XDG_RUNTIME_DIR/easy-personal-tray.lock` = `1460688`. Hand-over took the signal path; no second window.
- Not run by the Supervisor (needs the owner's eyes and mouse): the focus of the existing window on D2, close the window then `app` reopens it, and a popup click landing in the same window (AC 6 visual, AC 8 last two steps).

**DELTA**: opening the app now also starts the tray, and the launcher, the tray menu and a popup click all use one dashboard window instead of the launcher opening a second one.

**WITNESS**: Supervisor session, 2026-11-17T09:02-09:03Z on the owner's desktop (D0-D2 above); owner check of focus, reopen and popup click pending.
## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass (AC 1-5, 7; AC 6, 8 are the Supervisor's at Stage 5) | `rust/crates/app/src/window.rs` `mod tests` (AC 1, 2: `only_a_plain_window_run_wants_a_tray`, `the_tray_runs_this_exe_with_the_same_folder`, `a_failed_tray_start_is_one_line_and_app_opens_its_own_window`, `the_tray_child_has_its_own_group_and_null_stdio`); `rust/crates/app/src/tray.rs` `mod tests` (AC 3, 4: `write_pid_leaves_the_pid_and_keeps_the_lock`, `read_pid_gives_none_without_a_number_and_waits_for_one`, `only_an_easy_personal_tray_passes_the_cmdline_check`, `a_gone_pid_and_this_test_process_are_not_trays`, `hand_over_signals_only_a_pid_that_is_a_tray`); `rust/crates/app/tests/tray_show.rs` (AC 5: `a_pid_that_is_not_a_tray_gets_no_signal`, `a_lock_with_no_pid_is_exit_1_after_the_wait`, `without_show_a_held_lock_is_unchanged`, plus `a_running_tray_gets_sigusr1_and_the_ask_exits_0` with a shell named `easy-personal-rs tray` as a stand-in); `rust/crates/app/tests/app.rs` (AC 7: `app --help` lists `--no-tray`, `tray --help` lists `--show`) |
| Verification command run | ☑ pass | 2026-11-17, in the T101 worktree, `cd rust`: `cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets --features window -- -D warnings` and without the feature: `Finished` with no warning; `cargo test --workspace` (default): 96 targets, 1104 passed, 0 failed; `cargo test --workspace --features window --no-fail-fast`: 96 targets, first run 1108 passed, 6 failed = 5 in `tests/app.rs` (see Negative cases) + `ingest_coexist::a_killed_rust_ingest_leaves_whole_files_and_a_rust_rerun_equals_a_clean_run` (`precreate_store` `NotFound` under load; alone: `test result: ok. 1 passed`, 93 s); after the `tests/app.rs` fix `--test app` is `test result: ok. 8 passed; 0 failed`. App unit tests: `test result: ok. 256 passed; 0 failed` (both builds). `tests/tray_show.rs` `--features window`: `test result: ok. 4 passed; 0 failed`. Leak gate `env -u CLAUDE_CONFIG_DIR <main>/rust/parity/.venv/bin/python parity/leak_rust.py` from `rust/`: `leak check: 0 hits over 42 canaries, 866 surfaces, 2037147 bytes` |
| Negative cases hold | ☑ pass | RED before the helpers existed (`cargo test -p easy-personal-rs --bin easy-personal-rs tray::tests`): `error[E0425]: cannot find function` `write_pid` x2, `read_pid` x5, `cmdline_is_tray` x1, `is_tray` x4, `hand_over_to` x4, `error[E0433]` `Duration`/`Instant`; `window::tests`: `cannot find function own_window_line` x2, so `the_tray_runs_this_exe_with_the_same_folder` (now expects `--show`) was red too. GREEN after: the 5 tray tests and 4 window tests pass. No signal sent: `a_pid_that_is_not_a_tray_gets_no_signal` keeps a `sleep 30` alive (a `SIGUSR1` would end it) and `hand_over_signals_only_a_pid_that_is_a_tray` records no pid. Pre-existing, found on the window build: 5 `tests/app.rs` cases called `app` without `--no-ingest`, so `main.rs`'s start-up ingest (T051, unchanged here) printed its progress line first and `one_line` / the exit code failed (`assertion left == right failed: ingesting before start (waits if another ingest runs)... left: 2 right: 1`); fixed by adding `--no-ingest` to those 6 invocations |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor inline code review 2026-11-17 over `main...T101`: window.rs, tray.rs, main.rs, tests/app.rs, tests/window.rs, tests/tray_show.rs, both READMEs; stale-reference sweep (site/, README.md, rust/docs) 0 hits; lane_check `EXCEEDS (lines 687 > 500)`, mostly tests, accepted. Findings: 0 P0/P1; P2 silent launcher with an older tray (fixed `eb54b66`); P3 module doc width (fixed); P3 open: plain `app` still ingests before it hands over (kept: the tray does not ingest at start). Startup reads complete. Security pass (Medium): signal only to a pid whose `/proc/<pid>/cmdline` is `easy-personal-rs tray`, lock in the private runtime dir, `kill` limited to the same user, argv spawn with no shell, no secrets or network: 0 findings |
| Full smoke suite still green (no regression) | ☑ pass | workspace 1106 passed (default), 1115 passed + 1 known load flake `ingest_coexist` (passes alone) with `--features window`; Stage 5 D0-D2 above |
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

**BEFORE** (captured 2026-11-17T08:02:46Z on branch T101 at commit 87f0675, before any implementation commit; default-feature build; no tray was started or killed):

```
2026-11-17T08:02:46Z
$ grep -n -i tray crates/app/src/window.rs  (lines 146-260: fn run, start, open)
31:    /// engine when `alerts`): what `app` and `tray` both run before any Tauri code. With `clicks` (the `tray` only) a
(exit 0 : no spawn of a tray in run/start/open; only a doc comment on line 176 of start)
$ easy-personal-rs app --help
Open the dashboard in a window (needs a build with `--features window` and a display)

Usage: easy-personal-rs app [OPTIONS] --static-dir <DIR>

Options:
      --static-dir <DIR>  Serve the built dashboard's files from this folder, from the same origin as the API
      --no-ingest         Do not ingest before the start (default: ingest first)
      --alerts            Raise desktop alerts (a session waiting or failing, a plan limit, disk, RAM or battery) over the session D-Bus; they are listed at `GET /api/alerts`
  -h, --help              Print help
```

**AFTER** (code and unit/integration level; AC 6 and 8 on the owner's display are the Supervisor's Stage 5): `app --help` (default build, 2026-11-17) now lists `--no-tray  Open this process's own window and start no tray (by default \`app\` starts the tray, which owns the one dashboard window, and exits)`; `tray --help` lists `--show  Show the dashboard window: open it at start, or when a tray already runs ask that tray to show its window and exit`. `tests/tray_show.rs` (no display, temp `XDG_RUNTIME_DIR`): a held lock whose pid is a stand-in `easy-personal-rs tray` receives `SIGUSR1` and `tray --show` exits 0 with no output; a pid that is not a tray, or no pid, is exit 1 with one stderr line and no signal.

**Stage 4 fix round** (2026-11-17): P2, a launcher click while a tray that cannot be asked (an older one, no pid) holds the lock was silent: `hand_over_to_tray` now watches the spawned `tray --show` for up to 3 s (`HAND_OVER_WAIT`, `observe`) and decides with the pure `after_spawn`: still running or exit 0 = `app` exits 0; a non-zero exit = `easy-personal-rs: tray not started: the tray exited with exit status: 1` and `app` opens its own window. Tests: `app_exits_for_a_running_or_a_clean_tray_and_falls_back_for_a_failed_one` (all three cases plus a spawn error) and `observe_sees_a_failed_exit_a_clean_exit_and_a_running_child` (real `/bin/sh -c 'exit 1'`, `exit 0`, and a `sleep 30` the test ends) pass; `own_window_line` was replaced, so its test was rewritten (no separate RED run was captured for these two, they were written with the code). P3, `tray.rs` module doc rewrapped to the file's width. READMEs say `app` opens its own window for an older tray. Re-run: fmt 0, clippy both builds clean, `cargo test --workspace` 96 targets 1106 passed 0 failed; `--features window --no-fail-fast` 96 targets 1115 passed, 1 failed = `ingest_coexist::a_killed_rust_ingest_...` (`listing` `read_dir` NotFound at `ingest_coexist.rs:91`, the same load flake as before, in code this task does not touch; alone: `test result: ok. 1 passed`, 79 s); leak gate `leak check: 0 hits over 42 canaries, 866 surfaces, 2037897 bytes`.

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T101.jsonl`, never the
implementing agent alone]
