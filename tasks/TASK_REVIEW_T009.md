# TASK_REVIEW — T009: Local-timezone parity for usage_summary / usage_breakdown

> Sibling of `tasks/TASK_GUIDE_T009.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/usage_tz_parity.rs` (9 + 2 child-process tests: Bangkok, New York across both DST changes, leading-colon `TZ`, unset `TZ`, unknown and empty `TZ`, offset-less forms pinned to UTC, the oracle differs between zones, a mutation test); goldens in `rust/parity/golden-tz/{Asia/Bangkok,America/New_York,Nowhere/Invalid}` made by `gen_golden.py --tz` |
| Verification command run | ☑ pass | Supervisor ran the whole `cargo test` under `TZ` unset (Bangkok), `UTC`, `America/New_York` and `Asia/Bangkok`: 0 failures each (core lib 2, git_timeout 1, store_info 12, usage_parity 22, usage_tz_parity 9, warnings_parity 15, app 3+6+15+13). `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean |
| Negative cases hold | ☑ pass | red run: 5 of 8 new tests failed while the core read UTC (Bangkok differed in 12 golden files). Mutation: a changed `output_tokens` in a New York golden fails the comparison naming that file; the unmutated copy passes. Brute force: `local_to_utc` vs Python `astimezone(UTC)` over 25,088 wall-clock times around the offset changes of 15 zones (incl. Lord_Howe, Chatham, Casablanca), 0 mismatches (scratch, not committed). Default generator run: `diff -r` against `rust/parity/golden` empty, `store-v11.sqlite` identical (Supervisor re-ran). `git status --short -- src web tests pyproject.toml uv.lock` empty |
| verify | ☑ pass | library functions only. With the machine zone (Bangkok, +07) the core equals Python's goldens for day keys, offset-less date and datetime bounds, the 23- and 25-hour New York days, the skipped (02:30) and repeated (01:30) hours, the leap day and the year boundary — pass. Unknown or empty `TZ` reads UTC like Python — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed `metrics.rs` (zone-aware `utc_bound`, `bounds`, `keys`, `local_to_utc`, `local_day`), `usage_tz_parity.rs`, the one changed test in `usage_parity.rs`, `gen_golden.py` (`--tz` only), `golden-tz/**`, both Cargo.toml files, `spike-T009.md`, READMEs; `rules.rs`, `app/**`, `golden/**` untouched |
| Full smoke suite still green (no regression) | ☑ pass | all suites above, including T006 `warnings_parity` and T008 `mcp` |
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

**BEFORE**: captured 2026-11-10T10:13:24Z by the Supervisor, before the agent's first command:
```
$ grep -n "read as UTC" rust/README.md
56:Day keys and an offset-less `since`/`until` are read as UTC (Python reads the machine's zone).
$ grep -n "(UTC)" rust/crates/core/src/metrics.rs
535:/// Tokens per day (UTC) / model / project (folder name of the owner project, FR-057) / tool for [since, until).
```
(the core groups `by=day` and reads offset-less bounds as UTC; Python uses the OS zone.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ grep -n "local" rust/README.md | grep -i "since"   # the UTC sentence is replaced
$ TZ=Asia/Bangkok cargo test   -> usage_tz_parity: 9 passed; usage_parity: 22 passed; 0 failures in the workspace
```

**DELTA**: the Rust core groups `by=day` and reads offset-less `since` / `until` in the machine's own zone, DST-aware, and returns the same JSON as Python in Bangkok and New York; the usage gate from T005 is lifted.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/T009.jsonl`, never the
implementing agent alone]
