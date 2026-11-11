# TASK_REVIEW — T012: Remove the code rules.rs copied from metrics.rs

> Sibling of `tasks/TASK_GUIDE_T012.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | no new behaviour: the existing suites are the net. Changed test: `rust/crates/core/tests/usage_parity.rs::breakdown_by_day_matches_golden` now re-runs itself in a child process with `TZ=UTC` (child test `breakdown_by_day_child`, `#[ignore]`) |
| Verification command run | ☑ pass | Supervisor ran core lib 2, git_timeout 1, store_info 12, usage_parity 22, usage_tz_parity 9, warnings_parity 15 under `TZ` unset, `Pacific/Kiritimati` (+14) and `Pacific/Pago_Pago` (-11): all ok; app 3+6+15+13 ok; clippy `-D warnings` and rustfmt clean. Counts equal the agent's before-run |
| Negative cases hold | ☑ pass | sensitivity check (agent): without the UTC pin the child test fails under Kiritimati with day `2026-01-16` vs golden `2026-01-15`. `grep` for `COPY of private helpers` finds 0; one `fn` each of `parse_iso`, `civil_from_days`, `project_map`, `run_git_within` in `rust/crates/core/src`. `git status --short -- src web tests pyproject.toml` empty |
| verify | ☑ pass | behaviour unchanged: the warnings goldens, the 36 synthetic cases and the usage and timezone goldens all pass over the shared helpers; the 10 s `latest_commit` limit and the 5 s default are kept through `run_git_within(cwd, args, timeout)` — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/crates/core/src/{metrics,rules}.rs` and `tests/usage_parity.rs`; `rust/parity/**`, `app/**`, `lib.rs` untouched |
| Full smoke suite still green (no regression) | ☑ pass | all of the above; T007's new test file was not run (it is not finished) |
| **Companion score (easy-verifier)** | ☑ N/A | not run: easy-verifier is not installed in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-10T10:34:42Z by the Supervisor, before the agent's first command:
```
$ grep -n "COPY of private helpers" rust/crates/core/src/rules.rs; wc -l rust/crates/core/src/rules.rs
1589:// ---- COPY of private helpers of metrics.rs ---------------------------------------------------------------------
2046 rust/crates/core/src/rules.rs
```
(two copies of parse_iso, project_map, run_git and folder exist (metrics.rs and rules.rs).)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ grep -rn "COPY of private" rust/crates/core/src | wc -l
0
$ wc -l rust/crates/core/src/rules.rs        # 2046 before
1644
```

**DELTA**: one copy of each helper remains, so a later fix cannot drift between `metrics.rs` and `rules.rs`; `rules.rs` is about 400 lines smaller; a zone-sensitive T005 test no longer fails in far-east zones.

**WITNESS**: Supervisor, 2026-11-10, re-ran the suites under three zones independently of the implementing agent.
