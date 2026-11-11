# TASK_REVIEW — T018: Core session_timeline and session_outliers (with scope)

> Sibling of `tasks/TASK_GUIDE_T018.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/sessions_parity.rs` (23 test functions: 23 goldens counted from both manifests, default=5 and rich=18, with exact file-name sets; hand-made edge store with Python's answers embedded; non-UTC cases; unknown scope; hostile text as a bound value; store bytes unchanged; 3 mutation tests). The guide said 18 goldens: that was the rich oracle only (Supervisor's miscount) |
| Verification command run | ☑ pass | Supervisor, own runs: `cargo test -p easy-personal-core --test sessions_parity` prints `session entries compared: default=5 rich=18`; whole `cargo test --workspace --no-fail-fast` = 213 passed, 0 failed, 8 ignored under TZ unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean; oracle hash over 310 files `aa6fcaed…bff92` equal to T011's; `git status --short -- src web tests pyproject.toml uv.lock` empty. (Run with `--include-ignored`, the child tests `zone_child` and `utc_child` fail by design: they need their parent's environment.) |
| Negative cases hold | ☑ pass | agent's red run (the four functions stubbed to null): `3 passed; 17 failed`, and `sessions_child` listed `23 session entries differ`. Mutation tests in the repo (Supervisor read the pattern): a changed `ts_utc` names `rich/api__usage_sessions__all.json`; `tool_names` made additive (`|| ',Edit'`) names `rich/api__usage_outliers__all.json` and `rich/mcp__session_outliers__all.json`; a changed `ts_utc` in the default store names `default/api__usage_sessions__all.json` |
| verify | ☑ pass | library functions only, no surface yet. The agent's extra cross-check: six random stores (about 500 events each, ties, NULLs, abstained), 24 calls each, Python and Rust canonical JSON byte-identical; a non-UTC case from Python under `TZ=Asia/Kolkata` — pass. Not rerun by the Supervisor |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/crates/core/src/{sessions,metrics,lib}.rs`, `tests/sessions_parity.rs`, `rust/README.md`; `crates/app/**` untouched by this task. code-review `reports/code-review_main_20261110T211000.html` (no P0/P1; 4 P3; 1 info) and security-review `reports/security-review_main_20261110T211100.html` (nothing at confidence 8+) |
| Full smoke suite still green (no regression) | ☑ pass | the whole workspace is green in the three zones with T019's tests in (213/213); no existing test changed by this task |
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

**BEFORE**: captured 2026-11-10T13:29:12Z by the Supervisor, after the agents were spawned and before any file of theirs existed:
```
$ grep -c "session_timeline\|session_outliers" rust/crates/core/src/*.rs
lib.rs:0 clock.rs:0 plan.rs:0 metrics.rs:0 prices.rs:0 rules.rs:0 scope.rs:0 store.rs:0
```
(the core has no per-session function.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ cargo test -p easy-personal-core --test sessions_parity
session entries compared: default=5 rich=18      # 21 passed in a direct run
$ cargo test --workspace --no-fail-fast          # TZ unset, Kiritimati, Pago_Pago
passed 213 failed 0 ignored 8   (each)
```

**DELTA**: the Rust core can now answer the per-session outlier list and the session timeline (with workspace/account scope and detail_sessions) with the same JSON as Python for 23 goldens; T020 can serve them over HTTP and MCP.

**WITNESS**: Supervisor, 2026-11-10, reran the suite under three zones, the oracle hash and the Python-tree check independently of the implementing agent.
