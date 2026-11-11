# TASK_REVIEW — T001: Environment and crate spike

> Sibling of `tasks/TASK_GUIDE_T001.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ N/A | spike with no product code: nothing to test; the checks are the toolchain and crate runs in the rows below |
| Verification command run | ☑ pass | Supervisor re-ran: `cargo 1.99.1`, `rustc 1.99.1`, scratch `cargo check` exit 0 (`Finished dev profile`), `rust/docs/spike-T001.md` 119 lines |
| Negative cases hold | ☑ pass | keyring on headless Ubuntu printed a clear `unavailable` error and wrote nothing; `git status --short -- src web tests pyproject.toml` empty before and after (Supervisor re-checked); `~/.bashrc/.zshrc/.profile` last modified 2026-11-08, before T001 ran, and the one cargo line in `.profile` predates it |
| verify | ☑ N/A | no runtime feature in this task (environment and research); the toolchain run above is the check — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed `rust/spikes/t001-crates/src/main.rs` (72 lines) and `rust/docs/spike-T001.md`; no product code exists in this task. code-review `reports/code-review_main_20261111T112728.html`, security-review `reports/security-review_main_20261111T112729.html` (Supervisor, inline, 2026-11-11) |
| Full smoke suite still green (no regression) | ☑ N/A | no Python or Rust product code changed |
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

**BEFORE**: captured 2026-11-10T08:07:58Z by the Supervisor, before the agent's first command:
```
$ which cargo rustc
cargo not found
rustc not found
$ ls ~/.cargo ~/.rustup
ls: cannot access '/home/thunderkds/.cargo': No such file or directory
ls: cannot access '/home/thunderkds/.rustup': No such file or directory
```
(`rust/docs/spike-T001.md` and `rust/spikes/t001-crates/` did not exist.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ . ~/.cargo/env && cargo --version && rustc --version
cargo 1.99.1 (5f94df478 2026-08-27)
rustc 1.99.1 (58cf8f9c3 2026-10-13)
$ (cd rust/spikes/t001-crates && cargo check)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s
$ wc -l rust/docs/spike-T001.md
119 rust/docs/spike-T001.md
```

**DELTA**: Rust stable 1.99.1 is installed and every stack candidate has a checked verdict, so the P0 foundation can start; Tauri's build is still unproven until the owner runs the apt line.

**WITNESS**: Supervisor re-ran the commands above on 2026-11-10 after the common-infrastructure agent's report; the agent's own figures were not taken on trust.
