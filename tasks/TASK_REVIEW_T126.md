# TASK_REVIEW — T126: Live like Claude 2c — steer an app session

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

## Auto decisions

Mode: auto
1. Effort mid-session through `apply_flag_settings {effortLevel}`, all five levels — FACT: tasks/evidence-T126/SPIKE.md:103 "So all five levels of the fixed list (`low`, `medium`, `high`, `xhigh`, `max`) are seen to apply mid-session"
2. Model list = aliases + opusplan + accepted [1m] variants, fixed on the server — RULE: PRD.md:215 "No route lets the page name the program or free arguments"
3. Direction Path A, all five effort levels and bypassPermissions mid-session behind its second confirm — DEBATED: keep the build (Advocate 1: keep all five, add a medium/low spike row, note the header shows the last level sent; Advocate 2: narrow to high/xhigh/max and make bypass start-only until a re-spike; Advocate 3: keep all five, fewest requests already, optional medium/low row); settled by the run-4 re-spike (all levels proven, which removes Advocate 2's ground) and PRD.md:230 (FR-131 allows bypass mid-session with a second confirm) and memory/owner-wants-parity-not-narrowing.md ⚠ decided for you — please review
4. Stage 4 fix: DDR-0008 decision 3 names `apply_flag_settings` and the missing read-back (`c226092`) — RULE: `.claude/skills/code-review/SKILL.md` § Phase 4 — Apply Safe Fixes
Checks: tests `cargo test --workspace --no-fail-fast` — 1335 passed, 2 load timeouts in tests/answer_bridge.rs that pass alone (`cargo test -p easy-personal-rs --test answer_bridge` 18/18 twice; file not in the diff); `npm test` — 44 files, 1457/1457; reviews code-review — 0 P0/P1, 1 P2 fixed, 2 P3; security-review — no finding >= 8 (independent opus reviewer); real run re-spike `run.py` — printed `sent medium: answer success -> next turn medium`, `sent low: answer success -> next turn low`
Memory: memory/decisions.md — 2026-11-20 — T124-T128 Live like Claude 2 resumed and Stage 4 under [admin] (T126: effort mid-session, all levels proven); memory/learnings.md — 2026-11-20 — leak_rust.py needs the main .venv python and env -u CLAUDE_CONFIG_DIR
Not done: merge (admin: after the auto verify pass); auto verify: pass (commit c86551b, log .claude/verify-runs/T126-c86551b2c4b7.log)
Paused: done not moved — report incomplete: auto verify slot

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | AC 1: `tasks/evidence-T126/SPIKE.md` (real CLI 2.1.328). AC 2, 3: `rust/crates/app/src/app_session.rs` `model_mode_and_effort_are_control_requests_and_only_the_clis_success_changes_the_header`, `the_context_answer_is_numbers_only`; `rust/crates/app/src/app_session_routes.rs` `model_mode_effort_and_context_steer_a_going_session_from_fixed_lists_only`. AC 4: `the_argv_adds_a_listed_effort_and_each_listed_model_as_one_word_and_nothing_for_default`, `the_start_form_takes_a_listed_effort_or_none_and_refuses_any_other`, `web/src/NewSession.test.jsx`. AC 5, U1, U2: `web/src/LiveSteer.test.jsx` (8 tests). AC 6: `the_steering_routes_keep_the_guard`, leak gate |
| Verification command run | ☑ pass | 2026-11-19T22:47Z on `839ed22`: `cargo fmt --check` ok; both clippy runs `Finished` with no warnings; leak: `leak check: 0 hits over 42 canaries, 1314 surfaces, 2854281 bytes` (run as `env -u CLAUDE_CONFIG_DIR <repo>/.venv/bin/python parity/leak_rust.py`: system python3 has no `mcp`, and the gate refuses a real CLAUDE_CONFIG_DIR); web: `Test Files 44 passed (44)`, `Tests 1457 passed (1457)`; `✓ built`; `check_static: ok`. Rust `cargo test --workspace --no-fail-fast` (22:50Z): `1335 passed, 2 failed`. The 2 failures are debug-build timing tests in `crates/core` (`live_parity.rs` `pairing_100k_calls_is_fast`, `memgraph_parity.rs` `two_hundred_thousand_links_are_counted_in_time`); `crates/core` is untouched by T126. In an earlier full run, `answer_bridge.rs` hit its 15 s stream deadline twice under load and passed 18/18 when run alone. The timing tests were not rerun alone (agent step limit): Supervisor to confirm |
| Negative cases hold | ☑ pass | Mutation `web/src/api.js` `setAppMode` always sends `confirmed_bypass: true` (`git diff --stat`: `web/src/api.js` changed): `× any other mode is sent at once, with no bypass confirm in the body`, `Tests 1 failed / 7 passed (8)`; restored: `Tests 8 passed (8)`. The mutation first survived the 7 inherited tests, so this test was added. Server negatives (off-list values 400 with nothing sent, bypass without `confirmed_bypass` 400, 403 guard) are in the Rust tests above |
| verify | ☑ pass | auto-run by hook, session 67d6231e-3aff-4044-8bc2-f57910015046, commit c86551b2c4b7a9925259d4f5e8cdef0100df3c5f, log .claude/verify-runs/T126-c86551b2c4b7.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff main...T126`: app_session.rs, app_session_routes.rs, leak_rust.py, SessionPanel.jsx, NewSession.jsx, api.js, appsession.css + their tests; transcript and answer bridge not reviewed (untouched) |
| Full smoke suite still green (no regression) | ☑ pass | Rust 1335 + 2 load timeouts that pass alone (18/18 twice, file not in the diff); `npm test` 44 files 1457/1457 (Supervisor, 2026-11-20) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier refuses repo paths since 2026-11-01 (memory/learnings.md, easy-verifier registered); advisory only |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | ux-evaluate by the implementer, real SessionPanel/NewSession with app CSS and a fake fetch, 8 states at 375 and 1280: U1 pass (Model/Mode/Effort pickers in the header), U2 pass (refusal in red, picker reverts); 11 pass, 0 fail; shots /tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/b0e607b5-0126-47ad-b65d-c4fbbbdebda6/scratchpad/ux-t126/shots/ |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | new CSS in appsession.css uses var(--color-red), var(--space-*) and existing control styles only (B-c pass) |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280: no horizontal scroll (B-h), pickers, buttons and the bypass checkbox row 44 px tall at 375 (B-d) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any implementation commit; `main` the branch point):
```
$ date -u; git log -1 --oneline; grep -n 'set_model\|set_permission_mode\|--effort' -r rust/crates/app/src web/src | wc -l; grep -n 'pub(crate) const MODELS' rust/crates/app/src/app_session.rs
2026-11-19T11:18:19Z
af4aa48 Board + memory: T122, T123 Done (merged); font decision
0
61:pub(crate) const MODELS: [&str; 4] = ["default", "opus", "sonnet", "haiku"];
```

**AFTER** (backend-developer, branch `T126` at `839ed22`; same command, `-A3` added to show the wider list):
```
$ date -u +%Y-%m-%dT%H:%M:%SZ; git log -1 --oneline; grep -n 'set_model\|set_permission_mode\|--effort' -r rust/crates/app/src web/src | wc -l; grep -n -A3 'pub(crate) const MODELS' rust/crates/app/src/app_session.rs
2026-11-19T22:53:46Z
839ed22 T126: Model / Mode / Effort pickers and Context in a running app session's header; Effort on the New session form
23
62:pub(crate) const MODELS: [&str; 7] = [
63-    "default",
64-    "opus",
65-    "sonnet",
```
(the list continues `haiku`, `opusplan`, `opus[1m]`, `sonnet[1m]`; `haiku[1m]` is left out because the CLI refuses it, per `tasks/evidence-T126/SPIKE.md`)

**DELTA**: The owner can now switch a running app session's model, permission mode and effort from the panel header, and read its context use. The CLI's own answer is shown, and a refusal puts the picker back with the CLI's line. The owner can also start a session with an effort level and a wider model list.

**WITNESS**: AFTER captured by backend-developer (T126 resume) at 2026-11-19T22:53:46Z; tool calls are recorded under T126 in `memory/event-trace/T126.jsonl` (spawn binding). Independent witness: Supervisor at Stage 4/5.
