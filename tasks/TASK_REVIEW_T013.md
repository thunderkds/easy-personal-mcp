# TASK_REVIEW — T013: Shared store opener and clock in the core

> Sibling of `tasks/TASK_GUIDE_T013.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/tests/store_open.rs` (7: missing store -> `NoStore` and nothing created, a directory -> `NotAFile`, a store at version 99 -> `Newer` with today's full text, the fixture opens read-only and its bytes stay the same, a symlink to a file counts as a file, a non-database file errors without the path, `now_micros` within seconds of `SystemTime`) |
| Verification command run | ☑ pass | Supervisor ran the whole `cargo test`: 123 passed, 0 failed, 4 ignored (116 before + the 7 new); `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean |
| Negative cases hold | ☑ pass | red run: `store_open.rs` failed to compile (`clock`, `open_checked` and `NoStore` missing). No existing test file was edited and all still pass (mcp 15 unchanged, store_info 12 unchanged). `git status --short -- src web tests pyproject.toml uv.lock` empty (Supervisor re-checked) |
| verify | ☑ pass | library change plus a refactor: `with_store` in `mcp.rs` is now `store_path()` -> `open_checked()` -> `read()`, and the 15 spawned-binary MCP tests pass unchanged, so the tool errors keep their texts — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `store.rs` (`NoStore`, `open_checked`), `clock.rs`, one line of `lib.rs`, `mcp.rs` `with_store`, `tests/store_open.rs`; `read_info` and `store-info` untouched |
| Full smoke suite still green (no regression) | ☑ pass | all 123 |
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

**BEFORE**: captured 2026-11-10T11:20:28Z by the Supervisor, before the agent's first command:
```
$ ls rust/crates/core/src; grep -c 'NoStore\|open_checked' rust/crates/core/src/store.rs; grep -n 'fn with_store' rust/crates/app/src/mcp.rs
lib.rs metrics.rs plan.rs prices.rs rules.rs store.rs 
0
293:fn with_store(
294-    read: impl FnOnce(&rusqlite::Connection) -> Result<Value, String>,
295-) -> Result<Value, String> {
296-    let path = store_path().map_err(|e| e.to_string())?;
297-    match std::fs::metadata(&path) {
298-        Ok(meta) if meta.is_file() => {}
299-        Ok(_) => return Err(StoreError::NotAFile.to_string()),
```
(no `clock.rs`, no `open_checked`; `mcp.rs` carries its own store checks.)

**AFTER**: captured 2026-11-10 by the Supervisor:
```
$ ls rust/crates/core/src      -> clock.rs lib.rs metrics.rs plan.rs prices.rs rules.rs store.rs
$ grep -n "fn with_store" -A5 rust/crates/app/src/mcp.rs
    let path = store_path().map_err(|e| e.to_string())?;
    let conn = open_checked(&path).map_err(|e| e.to_string())?;
    read(&conn)
```

**DELTA**: HTTP and MCP surfaces can now share one checked read-only store opener (missing, not-a-file and too-new stores are refused without a path) and one clock, so T014 and T015 do not each carry their own copy.

**WITNESS**: Supervisor, 2026-11-10, re-ran the whole suite independently of the implementing agent.
