# TASK_REVIEW — T042

> Sibling of `tasks/TASK_GUIDE_T042.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | [paste the red run against the prior content, or the mutation diff (`git diff --stat`, non-empty) before each verdict] |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
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

**BEFORE**: captured 2026-11-11T12:18:19Z by the T042 implementing agent, before any implementation file existed.
Debug binary `rust/target/debug/easy-personal-rs` built from the guide's base commit, `serve` and `mcp` over a temp copy of
`rust/parity/store-rich/store-v11.sqlite` (script kept in the agent's scratchpad), token sent as `X-Dashboard-Token`:

```
2026-11-11T12:18:19Z
GET /api/kit            -> status 404
GET /api/kit/workspaces -> status 404
GET /api/kit/reports    -> status 404
--- mcp tools/list and tools/call kit_summary
tools: ['improvement_plan', 'ping', 'session_outliers', 'usage_breakdown', 'usage_summary', 'warnings']
kit_summary call: {"jsonrpc": "2.0", "id": 3, "error": {"code": -32602, "message": "Unknown tool: kit_summary"}}
```

Python's answers for the same routes are the goldens `rust/parity/golden-rich/api__kit__all.json`, `api__kit_workspaces__all.json`,
`api__kit_reports__all.json` and `mcp__kit_summary__all.json` (HTTP 200 and a body).

**AFTER**: Agent (12:38:39Z, same script as BEFORE, temp copy of the default store, token): `GET /api/kit`, `/api/kit/workspaces`, `/api/kit/reports` 200 and equal to the goldens; `tools/list` includes `kit_summary` and the call returns its result.
Agent counts: golden-rich 17 / 17 equal, golden 4 / 4 equal, hand-made store (git repos, scores, ui rows, stale and fresh index, report folders) 16 / 16 equal to Python's answers.
Supervisor re-runs in main after merging T042 on top of T046 (three-way cherry-pick; conflicts in `api.rs`, `mcp.rs`, `tests/mcp.rs`, `leak_rust.py` and `README.md` resolved by keeping both sides in Python's tool order), 2026-11-11: `kit_parity` 11 passed (the seventeen goldens, the four default goldens, the hand-made store); `mcp` 28 passed; `leak_rust.py` 0 hits over 42 canaries, 141 surfaces; `cargo test --workspace --no-fail-fast` TZ unset / Pacific/Kiritimati / Pacific/Pago_Pago 503 passed, 0 failed, 26 ignored each; clippy default and window clean; fmt clean. `git` runs with an argument list, `-c core.fsmonitor=false`, `GIT_OPTIONAL_LOCKS=0` and a timeout; the module reads folder names and `lstat` only (no file opened, no symlink followed).

**DELTA**: The Rust dashboard server and MCP serve the Kit data (per-project kit stage, trace counts, verifier and UI parts, workspace roll-up, report folders) like Python does, read-only.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
