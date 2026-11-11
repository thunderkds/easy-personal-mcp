# TASK_REVIEW — T017: workspace and account scope in the core

> Sibling of `tasks/TASK_GUIDE_T017.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/scope_parity.rs` (15 tests: 41 goldens counted from the manifest 8/11/6/5/6/5; empty string = no filter; account by id/label/any case; unknown scope; no-match workspace; NULL-account events; store bytes unchanged; 3 mutation tests) and 2 unit tests in `rust/crates/core/src/scope.rs` |
| Verification command run | ☑ pass | Supervisor, own runs: `cargo test -p easy-personal-core --test scope_parity -- --include-ignored` printed `15 passed; 0 failed; 0 ignored` and `scoped usage entries compared: 19`; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean. Whole `cargo test --workspace` with TZ unset: every suite ok, 0 failed. Oracle hash over the 310 files `aa6fcaed…bff92` equals the T011 value; `git status --short -- src web tests pyproject.toml uv.lock` is empty |
| Negative cases hold | ☑ pass | red run by the agent before the code (stub scoped functions that ignore the scope): 5 of 12 tests failed and both comparison tests listed all 41 entries. Mutation tests in the repo (Supervisor read them: each edits a temp copy and asserts the comparison fails and names the entry): one account-2 event moved to account 1 -> `account_work` and `account_default` differ; a workspace repo row deleted -> `workspace_personal` differs (usage and 22 warnings/plan); every account-2 event moved to account 1 -> the rules entries differ. By hand the agent dropped `OR agent_account_id IS NULL`: `events_with_no_account_belong_to_the_default_account` failed |
| verify | ☑ pass | library functions only (no surface yet), so no runtime to drive; the 41 scoped goldens equal Python's output, and the agent's extra cross-check of 1,140 scoped cases against Python on a scratch copy of the rich store (380 of them errors) all equal — pass. The agent's cross-check is its own report, not rerun by the Supervisor |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/crates/core/src/{scope,metrics,rules,plan,lib}.rs`, `tests/scope_parity.rs`, `rust/README.md`; the app crate (`api.rs`, `mcp.rs`) was only read as the caller. code-review `reports/code-review_main_20261110T203000.html` (no P0; 1 P1 that belongs to T015's tests; 2 P2; 3 P3) and security-review `reports/security-review_main_20261110T204000.html` (nothing at confidence 8+) |
| Full smoke suite still green (no regression) | ⚠ partial | TZ unset: all suites pass, no existing test changed. `TZ=Pacific/Kiritimati` and `TZ=Pacific/Pago_Pago`: the core suites pass; 2 tests in `rust/crates/app/tests/mcp.rs` fail (`usage_breakdown_equals_the_goldens_for_each_dimension`, `requests_sent_back_to_back_are_all_answered`) because they spawn the binary with the inherited TZ and compare `by=day` to UTC goldens. Pre-existing (T015's tests; the agent reproduced it on a pristine copy). Assigned to T019 (pin TZ=UTC) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier is not installed in this session |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | no UI |
| **UI: Responsiveness at target viewports** | ☑ N/A | no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-11-10T11:58:10Z by the Supervisor, before the agent's first command:
```
$ grep -c 'scoped' rust/crates/core/src/{metrics,rules,plan}.rs
metrics.rs:0 rules.rs:0 plan.rs:0 5://! (nothing in the goldens uses them): the `workspace` and `account` scope filters, the `prices` argument
(no scoped function exists; `workspace` appears nowhere in metrics.rs arguments)
```
(the core cannot filter by workspace or account.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ grep -n "pub fn .*_scoped" rust/crates/core/src/{metrics,rules,plan}.rs
metrics.rs:367:pub fn usage_summary_scoped(   metrics.rs:652:pub fn usage_breakdown_scoped(
rules.rs:1689:pub fn evaluate_scoped(       plan.rs:44:pub fn improvement_plan_scoped(
$ cargo test -p easy-personal-core --test scope_parity     # 15 passed; 0 failed
```

**DELTA**: the Rust core can now limit usage, warnings and the plan to one workspace and/or one account, with the same JSON and error text as Python for all 41 scoped goldens; HTTP and MCP wiring (T019) can replace the 501 and the tool error.

**WITNESS**: Supervisor, 2026-11-10, reran the scope suite, the whole workspace under three zones, the oracle hash and the Python-tree check independently of the implementing agent.
