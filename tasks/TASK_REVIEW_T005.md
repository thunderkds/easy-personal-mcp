# TASK_REVIEW — T005: P1 slice — price parser and usage_summary / usage_breakdown

> Sibling of `tasks/TASK_GUIDE_T005.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/usage_parity.rs` (22: all 7 usage goldens, 2 error texts, priced path, rounding vectors, fixture bytes unchanged, git-repo project rules), `rust/crates/core/tests/git_timeout.rs`, `rust/crates/core/tests/store_info.rs` (+1 variant-name test) |
| Verification command run | ☑ pass | Supervisor re-ran (before and after the review fixes): core `usage_parity` 22, `store_info` 12, `git_timeout` 1 (10.02 s), unit 1; app 3 + 6 + 13; clippy `-D warnings` and `cargo fmt --check` clean |
| Negative cases hold | ☑ pass | agent's red runs: 22 of 22 parity tests failed on stubs; `git_timeout` failed after 120 s on the old `run_git`; `store_info` new test failed with the fixed text. Mutation: `changing_one_fixture_value_breaks_the_golden_comparison` (input_tokens 10→11 changes the summary). Rounding: 21 Python vectors, plus the agent's 300,000-value check (0 mismatches vs a naive method's 2,965). Cross-check: 400-message synthetic store, 36 of 36 runs identical to Python. `git status --short -- src web tests pyproject.toml` empty before and after |
| verify | ☑ pass | library functions only (no CLI/API wiring in this task); the parity tests call them over the fixture store and equal the Python goldens — pass. A hung `git` returns within about 5 s (real-timeout test) — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed `rust/crates/core/src/{metrics,prices,store}.rs`, the three test files and `rust/README.md`; `rust/crates/app/**` and `rust/parity/**` untouched |
| Full smoke suite still green (no regression) | ☑ pass | T002 `store_info` (12), T004 app tests (22) still green |
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

**BEFORE**: captured 2026-11-10T08:35:36Z by the Supervisor, before the agent's first command:
```
$ ls rust/crates/core/src; grep -c 'metrics|prices' rust/crates/core/src/lib.rs
lib.rs
store.rs
grep -c count: 0
```
(`lib.rs` mentions neither `metrics` nor `prices`, so neither module exists.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ cargo test -p easy-personal-core --test git_timeout --test store_info --test usage_parity --lib
test result: ok. 1 passed (lib)   ok. 1 passed; finished in 10.02s (git_timeout)
test result: ok. 12 passed (store_info)   ok. 22 passed (usage_parity)
```

**DELTA**: the Rust core now answers `usage_summary` and `usage_breakdown` (by day, model, project, tool) with the same JSON as the Python goldens, priced from a hand-ported price-file parser, and a hung `git` can no longer hang a project breakdown.

**WITNESS**: Supervisor re-ran the tests after the agent's report and again after the review fixes, and read `run_git` line by line (2026-11-10).
