# TASK_REVIEW — T002: P0 foundation (workspace, `--version`, read-only store guard)

> Sibling of `tasks/TASK_GUIDE_T002.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/store_info.rs` (11), `rust/crates/app/tests/cli.rs` (6); includes the Python-migration drift guard `known_migration_is_the_last_python_migration` |
| Verification command run | ☑ pass | Supervisor re-ran 2026-11-10: `cargo build` ok; `cargo test` 11 (core) + 6 (app) passed; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` ok; `--version` → `easy-personal-rs 0.1.0`; missing store → `{"file":"store.sqlite","exists":false}` exit 0, nothing created |
| Negative cases hold | ☑ pass | agent's red run before implementation: core 10 of 11 failed, app 6 of 6 failed (stubs); version-12 store → exit 2, file bytes unchanged; read-only write attempt fails; `git status --short -- src web tests pyproject.toml` empty |
| verify | ☑ pass | ran the built binary: `--version` and `store-info` on a temp store and a missing store behave as the guide says — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed only `rust/Cargo.toml`, `rust-toolchain.toml`, `README.md`, `crates/**`; every other uncommitted change in the tree ignored |
| Full smoke suite still green (no regression) | ☑ N/A | no Python code changed; Rust tests above |
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

**BEFORE**: captured 2026-11-10T08:15:22Z by the Supervisor, before the agent's first command:
```
$ ls rust/Cargo.toml rust/crates; which easy-personal-rs
ls: cannot access 'rust/Cargo.toml': No such file or directory
ls: cannot access 'rust/crates': No such file or directory
exit=1
```
(no Cargo workspace and no `easy-personal-rs` binary exist.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ cd rust && cargo test
test result: ok. 11 passed (store_info)   test result: ok. 6 passed (cli)
$ ./target/debug/easy-personal-rs --version
easy-personal-rs 0.1.0
$ EASY_PERSONAL_HOME=<empty temp>/h ./target/debug/easy-personal-rs store-info --json
{"file":"store.sqlite","exists":false}   (exit 0, no file created)
```

**DELTA**: a Rust binary now exists that builds on Ubuntu, reports its version and reads the shared Python store read-only, refusing one newer than migration 11.

**WITNESS**: Supervisor re-ran the build, tests, clippy and the binary after the backend agent's report (2026-11-10); the agent's own figures were not taken on trust.
