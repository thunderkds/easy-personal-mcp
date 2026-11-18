# TASK_REVIEW — T110: Reply to idle sessions 0 — spike

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
1. Build order: a spike on the installed `claude` before any FR text or build task (the design depends on how the terminal behaves while a Stop hook holds) — RULE: `tasks/TASK_GUIDE_T102.md:22` precedent ("Before any build task, prove on the installed `claude` … the routes FR-109..FR-114 depend on")
2. Route: a `Stop` hook that blocks with the owner's text (option A); *Resume in app* stays for long conversations; tmux/terminal input stays out — FACT: PRD.md:332 "Decided by the owner 2026-11-18 (\"if the agent is idle, I can input the request to the live session also, can we do that\""
3. The hold rule — DEBATED: unsettled (Advocate 1: (ii) only sessions the owner marked "reply from the app", cap 120 s, cites tasks/evidence-T110/SPIKE.md:167; Advocate 2: (i) every turn end while a Live page watches, cap 60 s, cites PRD.md:211 FR-114; Advocate 3: (iii) hybrid, 90 s grace while a page watches, longer while the page shows that session or up to 10 min when marked, cites SPIKE.md:124) — no written rule settled it; the owner chose **hybrid** (Advocate 3) on 2026-11-18 ("Hybrid (Recommended)"), now PRD FR-121/FR-122 and question 42
Checks: spike `test -s tasks/evidence-T110/SPIKE.md && grep -cE '^\| *[0-9]+ ' …` → 11 (8 AC rows + 3 table rows), 7 scripts, all `py_compile` ok; scope: only `rust/spikes/reply-sessions/` and `tasks/evidence-T110/`; secret scan of added lines: 0 hits; all 8 ACs GO (claude 2.1.327); security-review (independent, scoped to the T110 diff): 1 Medium — fixed `/tmp/t110` base another local user could create first (hook swap) → fixed: `tempfile.mkdtemp` 0700 per run; no other finding ≥ 8
Memory: memory/decisions.md — 2026-11-18 reply to idle sessions: spike GO, hybrid hold rule (written with the T110 merge)
Not done: merge (admin: after the auto verify pass); auto verify: ☑ pass (session 9b0f7a91-4ff6-4469-8cb6-299bf0afd124, commit 4b46f628abc1fa27b230b6b134a8e86a8f1e82e7, log .claude/verify-runs/T110-4b46f628abc1.log); T111 (server) and T112 (Live view) build the feature
Paused: auto-mode check failed — no test run in memory/event-trace/T110.jsonl — write /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID with the task ID (minimal-track step 1): `mkdir -p /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d && printf '%s\n%s\n' "T110" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID`, then rerun the task's tests so the trace records them under the task, then stop; Risk High: decision 2 (`FACT`) is a choice — its FACT has no Citation (`path:line "quoted text"`) — so it must be `DEBATED` with exactly 3 Advocates named; Risk High, but no decision was DEBATED with 3 Advocates; Risk High, but the trace has no `security-review` Skill call [report 2b93d518b0e8]
Paused: auto-mode check failed — no test run in memory/event-trace/T110.jsonl — write /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID with the task ID (minimal-track step 1): `mkdir -p /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d && printf '%s\n%s\n' "T110" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > /home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/.claude/hooks/.state/active_task.d/$CLAUDE_CODE_SESSION_ID`, then rerun the task's tests so the trace records them under the task, then stop [report 2d7821796e0f]
Paused: done not moved — Gate 5: the tests row is not `☑ pass` with pasted output; Gate 6: the `UI: Visual regression` row is blank; Gate 6: the `UI: Design-system compliance` row is blank; Gate 6: the `UI: Responsiveness` row is blank; Gate 5: no test run in memory/event-trace/T110.jsonl; report incomplete: auto verify slot

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | Spike: each AC is a probe script with its recorded output in `tasks/evidence-T110/SPIKE.md`: `rust/spikes/reply-sessions/p1_headless.py` (AC 1, 2, 7), `p1b_shapes.py` (transcript shape), `p2_latency_text.py` (AC 3, 6), `p3_loop_transcripts.py` (loop, killed hook), `p4_interactive.py` (AC 4, 5 under a PTY), `stop_hook.py`, `cc.py`. Re-run 2026-11-18 at `4b46f62`: `test -s tasks/evidence-T110/SPIKE.md && grep -cE '^\| *[0-9]+ ' …` → `11`; `python3 -m py_compile rust/spikes/reply-sessions/*.py` → no failure |
| Verification command run | ☑ pass | `test -s tasks/evidence-T110/SPIKE.md && grep -cE '^\| *[0-9]+ ' tasks/evidence-T110/SPIKE.md && ls rust/spikes/reply-sessions/` → `11` (8 AC rows + 3 table rows), then `cc.py p1_headless.py p1b_shapes.py p2_latency_text.py p3_loop_transcripts.py p4_interactive.py stop_hook.py` |
| Negative cases hold | ☑ pass | Recorded in SPIKE.md: a Stop hook with exit 0 and no output ends the turn as usual (AC 3); `stop_hook_active` true → no block, exactly one extra turn (AC 2); an always-block hook loops (no Claude Code cap, so the app must guard); a 30 s timeout kills a 60 s hook quietly (AC 5); Esc/Ctrl+C kill a holding hook with nothing typed lost (AC 4) |
| verify | ☑ pass | auto-run by hook, session 9b0f7a91-4ff6-4469-8cb6-299bf0afd124, commit 4b46f628abc1fa27b230b6b134a8e86a8f1e82e7, log .claude/verify-runs/T110-4b46f628abc1.log — pass; done not moved: Gate 5: the tests row is not `☑ pass` with pasted output; Gate 6: the `UI: Visual regression` row is blank; Gate 6: the `UI: Design-system compliance` row is blank; Gate 6: the `UI: Responsiveness` row is blank; Gate 5: no test run in memory/event-trace/T110.jsonl |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Only new files under `rust/spikes/reply-sessions/` and `tasks/evidence-T110/`; no app code; secret scan of added lines 0 hits; security-review scoped to `git diff 4219ef5 HEAD`: 1 Medium fixed (`4b46f62`) |
| Full smoke suite still green (no regression) | ☑ N/A | No app code changed (spike files only) |
| **Companion score (easy-verifier)** | ☑ N/A | Spike scripts, not product code |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | No UI change (spike scripts and findings only) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | No UI change |
| **UI: Responsiveness at target viewports** | ☑ N/A | No UI change |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before the spike's first commit; worktree `T110` at `4219ef5`):
```
$ date -u; git log -1 --oneline; ls tasks/evidence-T110 rust/spikes/reply-sessions; claude --version
2026-11-18T04:14:00Z
4219ef5 T110 planned: reply to idle sessions spike (PRD question 42 decided by the owner)
ls: cannot access 'tasks/evidence-T110': No such file or directory
ls: cannot access 'rust/spikes/reply-sessions': No such file or directory
2.1.327 (Claude Code)
```
(No probe and no finding exist: whether a Stop hook can carry the owner's request into a terminal session is unproven.)

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
