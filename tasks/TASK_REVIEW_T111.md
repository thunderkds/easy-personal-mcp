# TASK_REVIEW — T111: Reply to idle sessions 1 — server

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
1. Route and hold rule as PRD FR-121 (Stop hook block-with-reason; hybrid hold 90 s / watched / marked up to 10 min) — FACT: PRD.md:332 "Hold rule decided by the owner 2026-11-18 after an unsettled 3-Advocate Debate: **hybrid**"
2. One bridge: extend `answer_bridge.rs` and its socket instead of a second bridge — RULE: CLAUDE.md:186 "Simplicity First"
3. Branch `T111` cut from `T110` so the spike findings are in the worktree — RULE: tasks/TASK_GUIDE_T111.md:24 "**Depends on**: T110 (spike GO)"
4. A marked session's state outside a hold reaches the page as a per-session `reply_marked: true` in the `GET /api/live` snapshot (added in `add_pending`, also when nothing else is pending; only for listed sessions; never in `LiveTail::snapshot()` read by the tray and alerts; marks of sessions that leave the list are pruned) — DEBATED: A (Advocate 1: A, high, cites rust/crates/app/src/answer_bridge.rs:432 "FR-121: sessions the owner marked *Reply from the app*; in memory, dropped at app exit"; Advocate 2: A, high (asked for the smallest change), cites docs/ddr/0005-live-question-text-show-never-store.md:19 "`LiveTail::snapshot()`, which the tray and the alert engine read, never holds it"; Advocate 3: A, high (owner with two windows and reloads), cites rust/crates/app/src/answer_bridge.rs:735) ⚠ decided for you — please review
Checks: build `e9ceeb7`: cargo test --workspace 1258 passed / 0 failed, clippy ± window clean, fmt ok, leak 0 hits over 42 canaries and 1214 surfaces (agent); security-review (independent, scoped to `git diff 446fecf e9ceeb7`): no finding at or above confidence 8 (guarded routes + `confirmed`, 128-bit hold id, `SO_PEERCRED` both ends, `json!` output re-checked by `is_stop_output`, add-only settings write); code-review running
Memory: nothing new yet (the pass runs at the merge)
Not done: build; Stage 4; auto verify; merge (admin: after the auto verify pass)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☐ pass / ☐ fail | [test file path(s) — required before Done] |
| Verification command run | ☐ pass / ☐ fail | [paste actual output] |
| Negative cases hold | ☐ pass / ☐ fail | [paste the red run against the prior content, or the mutation diff (`git diff --stat`, non-empty) before each verdict] |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
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

**BEFORE** (Supervisor, before any T111 implementation commit; worktree `T111` at `4948ba2`):
```
$ date -u; git log -1 --oneline; grep -n '"/api/reply' -r rust/crates/app/src
$ echo '{"session_id":"x","hook_event_name":"Stop","stop_hook_active":false,"transcript_path":"/nonexistent"}' | /usr/bin/easy-personal-rs hook ask; echo "exit $?"
2026-11-18T04:37:57Z
4948ba2 T111: guide, review file and PRD FR-121/FR-122 from main
(no /api/reply route)
exit 0, stdout empty
```
(No `/api/reply*` route exists, and `hook ask` ignores a `Stop` input: an idle terminal session cannot get a request from the app.)

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
