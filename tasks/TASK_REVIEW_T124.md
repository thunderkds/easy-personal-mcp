# TASK_REVIEW — T124: Live like Claude 2a — sub-agent logs (server)

> Sibling of `tasks/TASK_GUIDE_T[NNN].md`. Everything here is **filled by the reviewer at Stage
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
With `/verify (yours)`, your pass goes in the `verify` row below: Result `☑ pass`, Notes `/verify by the user, session <id>, commit <sha>`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its proposal>; Advocate 2: <its proposal>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Memory: <each entry the memory pass wrote, as `file — entry` | nothing new>
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours), with [admin] only after the one retry also ended blocked>

-->

## Auto decisions

Mode: auto
1. Sub-agent transcripts shown under the same rules as the main thread (extend the fifth NFR-002 exception) — DEBATED: same rules (Advocate 1: same rules, parity; Advocate 2: a narrower sixth exception, conceded its live-only limit; Advocate 3: same rules through one `agent=` parameter); settled by the owner's written rule `memory/owner-wants-parity-not-narrowing.md` ⚠ decided for you — please review
2. Meta `description` never read; listing reads only agentType, toolUseId, spawnDepth, finished — RULE: PRD.md:228 (FR-129) and docs/ddr/0008-live-subagent-logs-and-steering.md:13
3. Reuse the existing transcript route with `agent=` plus one listing route (nested and still-running agents need ids before their call has a result) — DEBATED: `?agent=` + listing (Advocate 1: keep it, one guard/parser/caps, listing needed for nested and running agents; Advocate 2: keep it, but T125 must not reuse offsets across files and must treat a missing meta as unknown, and the log line 54 was stale; Advocate 3: keep it, smallest surface, a route family duplicates the guard and leak gate) ⚠ decided for you — please review
4. Direction Path A of `BRAINSTORMING_LOG_LIVE2.md` (with the FR-129 narrowing) — DEBATED: Path A (Advocate 1: Path A, the only path for live, terminal and past sessions; Advocate 2: Path A, B breaks per-file paging, C covers app sessions only; Advocate 3: Path A as narrowed by PRD.md:228) ⚠ decided for you — please review
5. Stage 4 fixes: stale log lines 29, 54 and the id pattern marked superseded (`6bc268c` + this commit) — RULE: `.claude/skills/code-review/SKILL.md` § Phase 4 — Apply Safe Fixes
Checks: tests `cargo test --workspace --no-fail-fast` — 1345 passed / 0 failed (Supervisor); reviews code-review — 0 P0/P1, 1 P2 fixed, 1 P3 (T125: call the listing when the panel opens, not every 2 s); security-review — no finding >= 8 (independent opus reviewer); real run leak_rust.py — printed `leak check: 0 hits over 42 canaries, 1328 surfaces` (implementer)
Memory: memory/decisions.md — 2026-11-20 — T124-T128 Live like Claude 2 resumed and Stage 4 under [admin] (T124: ?agent= + listing; notes for T125); memory/learnings.md — 2026-11-20 — leak_rust.py needs the main .venv python and env -u CLAUDE_CONFIG_DIR
Not done: merge (admin: after the auto verify pass); auto verify: running

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/core/src/transcript.rs` (tests: sidechain flag, agent id pattern, listing four meta fields + no `description`, defaults for missing/oversized/invalid meta, listing regular files newest-first max 200); `rust/crates/core/tests/transcript_golden.rs` + `transcript_golden/agent.jsonl`, `agent.golden.json` (AC 5); `rust/crates/app/src/transcript_route.rs` (live + past + no folder + hostile + symlinked folders + hostile past path + read-anew/tail untouched + guard, AC 1-4, 6, 7); `rust/parity/leak_rust.py` (AC 6) |
| Verification command run | ☑ pass | 2026-11-19T22:55Z in `rust/` of the worktree: `cargo fmt --check` -> ok; `cargo clippy --workspace --all-targets -- -D warnings` -> `Finished`, 0 warnings; `cargo clippy -p easy-personal-rs --features window --all-targets -- -D warnings` -> `Finished`, 0 warnings; `cargo test --workspace` -> EXIT=0, 99 `test result: ok` lines, 1345 passed, 0 failed; `env -u CLAUDE_CONFIG_DIR <repo .venv>/bin/python parity/leak_rust.py` -> EXIT=0, `check T124 main answer holds no sub-agent string  ok`, `check T124 agents list names the stream, no transcript text  ok`, `check T124 agent transcript shows the agent's text  ok`, `check T124 meta description never read  ok`, 4 x 404 checks ok, `leak check: 0 hits over 42 canaries, 1328 surfaces, 2945809 bytes`. (The gate refuses a real CLAUDE_CONFIG_DIR, so the agent's own value was unset for that run; it needs the repo venv for `mcp`.) |
| Negative cases hold | ☑ pass | tests prove: `..`, `a/b`, `%2F`, dotted, 65-char, empty ids, symlinked file, directory, FIFO, over-long line, gone file, symlinked `subagents` or stem folder, out-of-root past path -> 404 one line, never 500; guard 403 on no/wrong token, bad Host, foreign Origin, query token. `git diff --stat` (non-empty): 7 files, +897/-34 before the review file |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | `git diff main...T124`: transcript_route.rs, api.rs, core transcript.rs, golden test, leak_rust.py + callers of transcript::read; web untouched |
| Full smoke suite still green (no regression) | ☑ pass | `cargo test --workspace --no-fail-fast` 1345 passed / 0 failed (Supervisor, 2026-11-20) |
| **Companion score (easy-verifier)** | ☑ N/A | easy-verifier refuses repo paths since 2026-11-01 (memory/learnings.md, easy-verifier registered); advisory only |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | backend-only task: no web file changed (`git diff --stat main...T124 -- web` is empty); the panel is T125 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | backend-only task: no CSS or component changed |
| **UI: Responsiveness at target viewports** | ☑ N/A | backend-only task: nothing rendered changed |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any implementation commit; `main` the branch point):
```
$ date -u; git log -1 --oneline; grep -n 'agent=\|/agents' rust/crates/app/src/transcript_route.rs rust/crates/app/src/api.rs | wc -l; grep -n 'isSidechain' rust/crates/core/src/transcript.rs | head -2
2026-11-19T11:18:18Z
af4aa48 Board + memory: T122, T123 Done (merged); font decision
0
226:    if record["isSidechain"] == true || !matches!(kind, Some("user" | "assistant")) {
695:            format!("{}\n", json!({"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"sub"}]}})),
```

**AFTER**:
```
$ date -u; grep -c 'agent=\|/agents' rust/crates/app/src/transcript_route.rs rust/crates/app/src/api.rs
Thu Nov 19 10:55:18 PM UTC 2026
(transcript_route.rs and api.rs now name the `agent=` parameter and the `/agents` route; the sidechain flag is `parse`/`read`'s third argument: main file passes false, the agent file true)
check T124 agents list names the stream, no transcript text  ok (status 200, {'depth': 1, 'finished': True, 'id': 'a1addd7ec56e8f010', 'parent_call': 'toolu_t124_parent', 'size': 1194, 'type': 'Explore'})
leak check: 0 hits over 42 canaries, 1328 surfaces, 2945809 bytes
```

**DELTA**: The server can now list a session's sub-agents and return one sub-agent's own log (live or past), with its sidechain records, behind the same guard and caps as the main transcript.

**WITNESS**: backend-developer (resumed run), 2026-11-19T22:55Z, commands as above; the Supervisor confirms from `memory/event-trace/T124.jsonl`.
