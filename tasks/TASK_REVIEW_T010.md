# TASK_REVIEW — T010: `improvement_plan` in the core

> Sibling of `tasks/TASK_GUIDE_T010.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/plan_parity.rs` (10: the four plan goldens (default and rich, API and MCP), group order and item order, key sets with `detail` / `cache_read_share` only when present, the empty case equal to Python's `{"groups": []}`, the `cache_read_share` vs `measured` explanation recomputed from the store, fixtures unchanged with no sidecars, two mutation tests) |
| Verification command run | ☑ pass | Supervisor ran the whole `cargo test`: lib 2, git_timeout 1, plan_parity 10, rich_parity 8, store_info 12, usage_parity 22, usage_tz_parity 9, warnings_parity 15, app 3+6+15+13, all ok; clippy `-D warnings` clean; rustfmt clean on both new files |
| Negative cases hold | ☑ pass | red run: 7 of 10 failed on a stub returning `{"groups": []}`. Mutations (agent): making the abstained record readable changes the plan against both default goldens; making all abstained rows readable changes it against both rich goldens. `git status --short -- src web tests pyproject.toml uv.lock` empty (Supervisor re-checked) |
| verify | ☑ pass | library function only. `plan::improvement_plan` over the default and rich stores equals the four Python goldens, with the frozen clock from each manifest — pass. The plan's `cache_read_share` (0.1895) differs from the `cache_hit_low` `measured` (0.1929) because it divides by all tokens, `measured` by prompt tokens only (no output); the port copies the hit's field and equals the golden |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `plan.rs`, one line of `lib.rs`, `tests/plan_parity.rs`, `rust/README.md`; `rules.rs`, `metrics.rs`, `parity/**`, `app/**` untouched |
| Full smoke suite still green (no regression) | ☑ pass | all suites above |
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

**BEFORE**: captured 2026-11-10T10:57:57Z by the Supervisor, before the agent's first command:
```
$ ls rust/crates/core/src; grep -c improvement_plan rust/crates/core/src/*.rs | tr '\n' ' '
lib.rs metrics.rs prices.rs rules.rs store.rs 0
(no plan.rs; no Rust function named improvement_plan)
```
(the Rust core has no plan function.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ ls rust/crates/core/src
lib.rs  metrics.rs  plan.rs  prices.rs  rules.rs  store.rs
$ cargo test -p easy-personal-core --test plan_parity   -> ok. 10 passed
```

**DELTA**: the Rust core answers `improvement_plan` with the same JSON as Python over the default and rich stores (library function; no surface wired yet).

**WITNESS**: Supervisor, 2026-11-10, re-ran the workspace suite independently of the implementing agent.

**Process note**: the agent's Python empty-case run included `ls ~/.easy-personal >/dev/null 2>&1 && echo skip`. The listing was discarded, nothing inside was read, and the run used a scratch `EASY_PERSONAL_HOME`, but it still touched the folder the rules say never to open. Disclosed by the agent; recorded here. No test opens it.
