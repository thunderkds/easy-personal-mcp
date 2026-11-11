# TASK_REVIEW — T008: P1 slice — MCP stdio server

> Sibling of `tasks/TASK_GUIDE_T008.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/tests/mcp.rs` (15 tests spawning the real `mcp` binary over pipes: handshake, `tools/list` equal to the golden incl. `outputSchema`, all 7 `mcp__*` goldens as parsed JSON in `structuredContent` and `content[0].text`, null bounds, tool errors, unknown tool, `-32700`, EOF exit 0, protocol-version limit, no path in errors, fixture bytes unchanged, mutation test) |
| Verification command run | ☑ pass | Supervisor re-ran `cargo test -p easy-personal-rs`: unit 3, cli 6, mcp 15, server 13, all ok; `cargo clippy -p easy-personal-rs --no-deps --all-targets -- -D warnings` clean; `cargo fmt -p easy-personal-rs --check` clean. Whole-workspace `cargo test`/clippy fail only in T006's unfinished `core/src/rules.rs` (not this task) — re-run when T006 lands |
| Negative cases hold | ☑ pass | agent's red run: 13 of 13 failed with `unrecognized subcommand 'mcp'` before the code existed. Mutation: `a_changed_fixture_value_breaks_the_matching_comparison` (output_tokens +1 in a temp copy) makes both usage goldens differ. `git status --short -- src web tests pyproject.toml` empty (Supervisor re-checked) |
| verify | ☑ pass | Supervisor ran the real binary by hand over a copy of the fixture store (`EASY_PERSONAL_HOME` = temp dir): initialize, `usage_breakdown by=model` (result equals the golden shape), `ping`, `nope` -> `-32602 Unknown tool`, `not json` -> `-32700` with `id:null`; exit 0, stderr 0 bytes. Missing store: tool error `no store yet: …`, and the data folder stays empty (nothing created) — pass. Not proven against the real Claude Code client |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | reviewed `rust/crates/app/src/mcp.rs`, `tests/mcp.rs`, the `Mcp` arm in `main.rs`, both Cargo.toml files and `README.md`; `core`, `server.rs`, `parity/**` untouched |
| Full smoke suite still green (no regression) | ☑ pass | T004 server tests (13) and T002 cli tests (6) green; core suites green when the agent ran them (only T006's `warnings_parity` red, another task) |
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
$ cd rust && ./target/debug/easy-personal-rs mcp
error: unrecognized subcommand 'mcp'
```
(no `mcp` subcommand exists.)

**AFTER**: captured 2026-11-10 by the Supervisor (fixture-store copy, temp data folder):
```
$ printf '<initialize> <initialized> <usage_breakdown by=model> <ping>' | ./target/debug/easy-personal-rs mcp
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"easy-personal-rs","version":"0.1.0"}}}
{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"easy-personal core ok"}],"structuredContent":{"result":"easy-personal core ok"},"isError":false}}
exit=0 stderr_bytes=0
```

**DELTA**: an MCP client can start `easy-personal-rs mcp` over stdio and call `ping`, `usage_summary` and `usage_breakdown`, getting the same JSON as the Python server over the same store.

**WITNESS**: Supervisor, 2026-11-10, re-ran the tests and the live probe independently of the implementing agent.
