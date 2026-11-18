# TASK_REVIEW — T114: Find `claude` when the app's PATH lacks `~/.local/bin`

> Sibling of `tasks/TASK_GUIDE_T114.md`. Everything here is **filled by the reviewer at Stage
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

## Auto decisions

Mode: auto
1. Task renumbered T113 → T114 after another live session claimed T113 on the board (its row + `BRAINSTORMING_LOG_WORKFLOWPLACE.md:63` "Stage 2: `tasks/TASK_GUIDE_T113.md` + KANBAN row (T113)") — FACT: BRAINSTORMING_LOG_WORKFLOWPLACE.md:63 "Stage 2: `tasks/TASK_GUIDE_T113.md` + KANBAN row (T113)." ⚠ my first write of `tasks/TASK_GUIDE_T113.md` (17:00:07) may have replaced that session's guide; that session must re-check its file
2. Fallback locations `~/.local/bin/claude`, then `~/.claude/local/claude`; PATH still wins — FACT: owner confirmed the mental model 2026-11-18 ("yes, fix it")
3. Fix only the Rust app; Python `src/` unchanged — FACT: rust/crates/app/src/agent_routes.rs:123 "\"claude_found\": runner.claude().is_some()," (the tray that showed the message is `/usr/bin/easy-personal-rs tray`) ⚠ decided for you — please review (Python keeps PATH-only lookup)
Checks: pending
Memory: pending
Not done: merge (admin: after the auto verify pass); auto verify: ☑ pass (session eee5b05b-25ed-4f9e-8c27-0bbc9703a975, commit 56a3f3f86be0e6f5c80197ad01768ca41c887d7d, log .claude/verify-runs/T114-56a3f3f86be0.log)
Paused: auto-mode check failed — Risk Medium: decision 1 (`FACT`) is a choice — it is flagged ⚠ — so it must be `DEBATED` with exactly 2 Advocates named; Risk Medium: decision 2 (`FACT`) is a choice — its FACT has no Citation (`path:line "quoted text"`) — so it must be `DEBATED` with exactly 2 Advocates named; Risk Medium: decision 3 (`FACT`) is a choice — it is flagged ⚠ — so it must be `DEBATED` with exactly 2 Advocates named; extended track, but the trace has no `code-review` Skill call (Stage 4 runs in auto mode); Risk Medium, but the trace has no `security-review` Skill call [report d0f88133403d]
Closed: row moved to Done 2026-11-18 — auto verify pass (session eee5b05b-25ed-4f9e-8c27-0bbc9703a975, commit 56a3f3f86be0e6f5c80197ad01768ca41c887d7d); Gates 5 and 6 checked

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/claude_cli.rs` (tests: `claude_on_path_is_found`, `claude_off_path_is_found_in_local_bin_then_in_dot_claude_local`, `claude_on_path_wins_over_the_fallback`, `a_fallback_that_is_a_directory_or_not_executable_is_not_found`, `no_home_or_an_empty_home_means_no_fallback`, `the_real_claude_looks_in_home_too`) = AC1, AC4, AC5; `rust/crates/app/src/agent_routes.rs` (`claude_found_is_true_when_claude_is_only_in_the_homes_local_bin`) = AC3; AC2 by the three callers' diffs (`agent_run.rs`, `app_session.rs`, `SystemClaude`); AC6: `git diff f0c9f75 --stat` holds no `core/src/jobs.rs` and the launcher lookup is untouched |
| Verification command run | ☑ pass (with one known flake) | `cargo fmt --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` -> `Finished dev profile`; `cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings` -> `Finished dev profile ... in 59.53s`. `cargo test --workspace --no-fail-fast`: 1329 passed, 2 failed, 29 ignored; the 2 failures are `store_check::ctrl_c_during_full_stops_with_130_no_panic_text_and_no_process_left` and `a_second_ctrl_c_ends_the_process_at_once_with_130` (signal tests that flake under load, `memory/MEMORY.md` T113 note; they touch no `claude` lookup). Rerun alone: `cargo test -p easy-personal-rs --test store_check -- --test-threads=1` -> `test result: ok. 12 passed; 0 failed; 1 ignored`. A plain `cargo test --workspace` stops at that target (exit 101), so its `tail -5` is not clean on this machine under load |
| Negative cases hold | ☑ pass | Red run (commit `c8159bb`, stub `find_claude` PATH-only): `claude_off_path_is_found_in_local_bin_then_in_dot_claude_local` left `None` right `Some(.../home/.claude/local/claude)`; `the_real_claude_looks_in_home_too` and `claude_found_is_true_when_claude_is_only_in_the_homes_local_bin` (left `false`, right `true`) FAILED; 19 passed. Mutations on the fix (`git diff --stat` non-empty each time, file restored after, `cmp` identical): `.exists()` instead of `is_executable` -> `a_fallback_that_is_a_directory_or_not_executable_is_not_found` + the route test FAILED; fallback tried before PATH -> `claude_on_path_wins_over_the_fallback` FAILED; fallbacks swapped -> `claude_off_path_is_found_in_local_bin_then_in_dot_claude_local` FAILED. Surviving mutant: dropping the empty-`HOME` filter is not caught (it would only read `.local/bin/claude` relative to the test's working folder, which no hermetic test may plant); the filter stays as a one-line guard |
| verify | ☑ pass | auto-run by hook, session eee5b05b-25ed-4f9e-8c27-0bbc9703a975, commit 56a3f3f86be0e6f5c80197ad01768ca41c887d7d, log .claude/verify-runs/T114-56a3f3f86be0.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Supervisor 2026-11-18: code-review over main..T114 (claude_cli, agent_run, app_session, agent_routes, README) + callers (doctor_cmd, session_procs, connector_writes, core jobs.rs): 0 P0/P1, 1 P2 (connector_writes.rs:465 "not on PATH" wording, kept for Python parity until T063), 2 P3 (stale docs agent_run.rs:12,169; claude_cli.rs:36) — report `reports/code-review_T114_20261118T222921.html`; security-review scoped to main..T114: no finding >= 8 |
| Full smoke suite still green (no regression) | ☑ pass | see the Verification row: 1329 passed; only the 2 load-flaky `store_check` signal tests failed in the parallel run and pass alone. Integration tests that spawn the binary use a temp `HOME` (`tests/connector_world/mod.rs:132` env built from nothing), so the new fallback never reaches the real `~/.local/bin/claude` that exists on this machine |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI change (`web/**` untouched) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI change |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI change |
| Repro loop | ☑ pass | BEFORE the fix (2026-11-18T15:09:26Z): `env -i HOME=<fake> PATH=/usr/bin:/bin <binary> serve --no-ingest`, `curl /api/agent-run` -> `{"account":null,"claude_found":false,...}` with an executable `<fake>/.local/bin/claude` present. Same binary with `PATH=/usr/bin:/bin:<fake>/.local/bin` -> `claude_found:true` (cause confirmed). AFTER the fix (2026-11-18T15:13Z, rebuilt): original PATH -> `"claude_found":true` |
| Regression test | ☑ pass | `rust/crates/app/src/agent_routes.rs::claude_found_is_true_when_claude_is_only_in_the_homes_local_bin`; `rust/crates/app/src/claude_cli.rs` tests listed in the first row |
| Smoke suite | ☑ pass | the repro loop above, run on the built binary under a minimal env with a fake HOME (never the real one) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (2026-11-18T15:09:26Z, before any implementation commit; fake HOME holds an executable `.local/bin/claude`, PATH is `/usr/bin:/bin`):
```
$ env -i HOME=<fake> PATH=/usr/bin:/bin easy-personal-rs serve --no-ingest   # then GET /api/agent-run
{"account":null,"claude_found":false,"enabled":false,"last":null,"running":false}
```

**AFTER** (2026-11-18T15:13Z, same command, fix built):
```
{"account":null,"claude_found":true,"enabled":false,"last":null,"running":false}
```

**DELTA**: the desktop app started by `systemd --user` (no `~/.local/bin` on its PATH) now finds `claude` at `~/.local/bin/claude` or `~/.claude/local/claude`, so App sessions, Agent run, connectors and doctor work there.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]


## Supervisor check (2026-11-18T15:32Z)
Installed `/usr/bin/easy-personal-rs` (before) vs T114 build `56a3f3f` (after), `env -i HOME=<fake> PATH=/usr/bin:/bin serve --no-ingest`, `GET /api/agent-run`: before `claude_found:false`, after `claude_found:true`, after with the fake removed `claude_found:false`. `cargo test -p easy-personal-rs --bin easy-personal-rs -- claude_cli:: claude_found`: 15 passed, 0 failed.
