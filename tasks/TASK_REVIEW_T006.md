# TASK_REVIEW — T006: P1 slice — rule pack and warnings in the core

> Sibling of `tasks/TASK_GUIDE_T006.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/warnings_parity.rs` (15: API and MCP warnings goldens, read-only check, mutation check, parser tests incl. no-secret-in-error, 36 synthetic cases whose expected answers are Python's own output over the same stores) and `rules::tests` in `rules.rs` |
| Verification command run | ☑ pass | Supervisor re-ran `cargo test -p easy-personal-core --test warnings_parity --lib`: 15 and 2 pass; `cargo clippy -p easy-personal-core --all-targets -- -D warnings` clean; `rustfmt --check` clean on both files. Agent's full run: usage_parity 22, store_info 12, git_timeout 1, app 3+6+15+13 all pass |
| Negative cases hold | ☑ pass | red run: both golden tests failed with `not implemented`. Agent mutated `rules.rs` 15 ways (`<`→`<=`, `>`→`>=`, the 5% and 10% lines, even-count median, compaction test, 60 s skew, history boundary, padding, stale cause order, version tie, unsorted abstained list): the suite failed each time. Fixture sha256 `2289d17f…` unchanged, no `-wal`/`-shm`. `git status --short -- src web tests pyproject.toml` empty |
| verify | ☑ pass | library function only (no CLI/API wiring): `rules::evaluate` over the fixture store equals `api__warnings__all` and `mcp__warnings__all` as parsed JSON (1 hit, 5 abstained) — pass. Not proven: `workspace`/`account` filters, `config.yaml` overrides, the richer-store paths beyond the 36 synthetic cases (T007) |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed `rules.rs`, `lib.rs` (one line), `tests/warnings_parity.rs`, `rust/README.md`; `metrics.rs`, `store.rs`, `app/**`, `parity/**` untouched |
| Full smoke suite still green (no regression) | ☑ pass | the agent's whole-workspace run was green; T009 is editing `metrics.rs` now, so the Supervisor re-ran only the T006 suites |
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

**BEFORE**: captured 2026-11-10T08:47:57Z by the Supervisor, before the agent's first command:
```
$ ls rust/crates/core/src; grep -c rules rust/crates/core/src/lib.rs
lib.rs
metrics.rs
prices.rs
store.rs
rules mentioned in lib.rs: 0
```
(no `rules.rs`; `lib.rs` does not mention `rules`.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ cargo test -p easy-personal-core --test warnings_parity --lib
test result: ok. 2 passed (lib)   test result: ok. 15 passed (warnings_parity)
```

**DELTA**: the Rust core evaluates the seven rules and returns the same `warnings` JSON as Python over the same store (library function; no surface wired yet).

**WITNESS**: Supervisor, 2026-11-10, re-ran the suites independently of the implementing agent.
