# TASK_REVIEW — T015: warnings and improvement_plan tools in the MCP server

> Sibling of `tasks/TASK_GUIDE_T015.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/mcp.rs` unit tests (9, in-process with the frozen clock: warnings and plan equal the default and rich goldens, a changed fixture value and a changed clock break the comparison, the clock passed in is the clock used, filter handling, filter checked before the store is opened) and `rust/crates/app/tests/mcp.rs` (19: `tools/list` equals the golden for all five tools, a newer store is a tool error for both new tools, a filter is a tool error and the server keeps serving, an empty store gives empty `hits` / `groups`, top-level keys over the fixture, no store or a directory gives a tool error with no path) |
| Verification command run | ☑ pass | Supervisor ran the whole `cargo test --no-fail-fast`: 157 passed, 0 failed, 4 ignored (T014 and T015 both landed); `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean |
| Negative cases hold | ☑ pass | red run: 8 of 12 unit tests and 6 of 19 spawn tests failed before the tools existed. Mutations (agent): rich-store `output_tokens * 3` breaks both golden comparisons; a later clock breaks `warnings`; `evaluate(conn, now + 3 days)` by hand fails the default and rich golden tests (file restored). Two existing assertions changed (the tool count 3 -> 5 with its test renamed; two more tools added to the newer-store loop) and nothing was loosened. `git status --short -- src web tests pyproject.toml uv.lock` empty (Supervisor re-checked) |
| verify | ☑ pass | Supervisor, the real `mcp` binary over a copy of `store-rich/store-v11.sqlite`: `tools/list` -> `['ping','usage_summary','usage_breakdown','warnings','improvement_plan']`; `warnings {}` -> result with `abstained` (with the real clock the 2026-01 data is outside the window, so every rule abstains — hence the in-process goldens); `improvement_plan {account:"work"}` -> `Error executing tool improvement_plan: the workspace and account filters are not available in this build yet`; `{workspace:null, account:""}` -> `{"groups":[]}`; stderr 0 bytes; the store copy's sha256 unchanged and no sidecar — pass. Not proven against the real Claude Code client |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `rust/crates/app/src/mcp.rs`, `tests/mcp.rs`, `rust/README.md`; no Cargo change; `server.rs`, `api.rs`, `core/**`, `parity/**` untouched |
| Full smoke suite still green (no regression) | ☑ pass | 157 passed, 0 failed (all suites, both new slices together) |
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

**BEFORE**: captured 2026-11-10T11:23:00Z by the Supervisor, before the agent's first command:
```
$ ... easy-personal-rs mcp   # initialize, then tools/list
['ping', 'usage_summary', 'usage_breakdown']
```
(the server lists three tools.)

**AFTER**: captured 2026-11-10 by the Supervisor (same method as BEFORE):
```
['ping', 'usage_summary', 'usage_breakdown', 'warnings', 'improvement_plan']
```

**DELTA**: an MCP client can call `warnings` and `improvement_plan` on the Rust server and get the same JSON as the Python server (unscoped), next to the three usage tools.

**WITNESS**: Supervisor, 2026-11-10, re-ran the whole suite and the by-hand MCP run independently of the implementing agent.
