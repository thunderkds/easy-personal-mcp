# TASK_REVIEW — T112: Reply to idle sessions 2 — Live view

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
1. Mark state comes from the server's `reply_marked` (T111 decision 4, DEBATED A), not page memory — FACT: tasks/TASK_REVIEW_T111.md:39 "A marked session's state outside a hold reaches the page as a per-session `reply_marked: true` in the `GET /api/live` snapshot"
2. Every reply route is sent with `confirmed: true` (watch and mark too), as FR-116 requires of every write route — FACT: PRD.md:213 "Every write route of FR-109..FR-114 keeps the NFR-008 guard and needs `confirmed: true` in its body"
3. How the page knows a session is bridge-enabled (shows the *Reply from the app* toggle) — DEBATED: unsettled (Advocate 1: B, a server-computed per-session `reply_bridge` in the live snapshot, cites rust/crates/app/src/hooks_cmd.rs:503-504 vs rust/crates/core/src/live.rs:743 — labels come from different row sets, are not unique (schema_v11.sql:39), and the page reads `/api/hooks` once; Advocate 2: A, keep the page-side label match, cites web/src/LiveView.jsx:588 — a wrong match only shows a toggle that stores an unused mark, and B reopens the merged T111 server)
   → the owner chose **B** on 2026-11-18 ("Server flag (Recommended)"): the server adds `reply_bridge: true` per session in the live snapshot from the account's real hook status (by account id); the page uses it instead of the label match
Checks: build `48c4377` (agent): vitest 37 files / 1293 passed, build ok, check_static ok; ux-evaluate (agent harness) pass except B-e body text 14 px (older, app-wide); security-review (independent, `c71ffdf..48c4377`): no finding at or above confidence 8; code-review (independent): 0 P0/P1, 1 P2 (the "text typed while the request is out" test types nothing), 5 P3 — all fixed in the fix round with decision 3
Memory: nothing new yet
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

**BEFORE** (Supervisor, before any T112 implementation commit; worktree `T112` at `13508b5`):
```
$ date -u; git log -1 --oneline; grep -c "pending_reply\|reply_marked\|sendReply" web/src/*.jsx web/src/api.js | grep -v ':0'; ls web/src/ReplyBox.jsx
2026-11-18T05:31:21Z
13508b5 T112: review file from main
web: no pending_reply / reply_marked / sendReply anywhere
ls: cannot access 'web/src/ReplyBox.jsx': No such file or directory
```
(The server sends `pending_reply` and `reply_marked` since T111, but the page reads neither: no reply box, no *Reply from the app* toggle.)

**AFTER**: [same command, post-change] OR [verbatim excerpt of the new content]

**DELTA**: [one sentence — what a user can now do that they could not before]

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
