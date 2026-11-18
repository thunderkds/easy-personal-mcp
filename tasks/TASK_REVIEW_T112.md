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
Checks: build `48c4377` (agent): vitest 37 files / 1293 passed, build ok, check_static ok; ux-evaluate (agent harness) pass except B-e body text 14 px (older, app-wide); security-review (independent, `c71ffdf..48c4377`): no finding at or above confidence 8; code-review (independent): 0 P0/P1, 1 P2 (the "text typed while the request is out" test types nothing), 5 P3 — all fixed in `5664316` with decision 3 (server flag `babf7f7`); security-review of `139d60a..5664316`: no finding ≥ 8; Supervisor re-run at `5664316`: vitest 37 files / 1294 passed, build ok, check_static ok, `cargo test --workspace --no-fail-fast` 1263 passed / 0 failed, clippy `--features window` clean, leak 0 hits over 42 canaries, 1214 surfaces
Memory: memory/decisions.md — reply to idle sessions entry: T112 server flag `reply_bridge` (owner choice) (written with the merge)
Not done: merge (admin: after the auto verify pass); real-terminal smoke (owner, after installing the new package); accepted: `reply_bridge` does not check that the answer socket is open (a CLI install while the app runs shows the toggle; replies work after an app restart)

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/ReplyBox.test.jsx` (AC 1-7: last text, countdown, Enter/Shift+Enter/IME, 8,000 code points, Send 200/409/400/500, text not sent after 200, release, watch timer with fake timers, mark toggle from `reply_bridge`/`reply_marked`, no storage, no token in URL), `web/src/AnswerBridge.test.jsx` (AC 8 `on (no reply)`), `rust/crates/app/tests/answer_bridge.rs` `reply_bridge_follows_the_hook_status_of_the_sessions_account_by_id`, `rust/crates/core/src/live.rs` tail test. Run at `5664316`: `npx vitest run` → `Test Files  37 passed (37)` / `Tests  1294 passed (1294)`; `cargo test --workspace --no-fail-fast` → `passed 1263 failed 0` |
| Verification command run | ☑ pass | `cd web && npx vitest run` → 1294 passed; `npm run build` → `✓ built in 138ms`; `python3 scripts/check_static.py` → `check_static: ok` |
| Negative cases hold | ☑ pass | RED first: new ReplyBox tests failed (no `./ReplyBox.jsx`); fix round 5 failed / 41 passed before the fixes; server flag test failed with "the live stream never got there" with the src changes stashed; no toggle without `reply_bridge`, at your turn, or with an answer card |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review on `c71ffdf..48c4377` + callers; security-review on `c71ffdf..48c4377` and `139d60a..5664316`; server flag limited to `live.rs`, `hooks_cmd.rs`, `answer_bridge.rs`, `live_feed.rs`, `api.rs` (owner decision) |
| Full smoke suite still green (no regression) | ☑ pass | web 1294 / 1294; Rust 1263 / 0; leak 0 hits |
| **Companion score (easy-verifier)** | ☑ N/A | not run: low-signal on this repo (memory handoff 2026-11-14); independent code and security reviews used instead |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | ux-evaluate (frontend agent, the real `LiveView` with fixture data): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/7f4cd3c0-cdf6-469c-a97f-525e670398f9/scratchpad/ux/hold-375.png`, `hold-1280.png`, `marked-375.png`, `marked-1280.png`, `sent-375.png`, `sent-1280.png`, `gone-375.png`, `gone-1280.png`; Supervisor read `hold-375.png` (last text wraps a 90-char token, box, Send, Hand back to terminal, "waits 1m 18s more", toggle) and `marked-1280.png` (pressed toggle + "marked: it waits for you at its next turn end"). Baseline B-e (14 px body) fail (older, app-wide) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | `appsession.css` uses `tokens.css` variables and the existing `.field`, `.field-error`, `.actions` classes only (code review); same accent, button style and radius as the prompt card |
| **UI: Responsiveness at target viewports** | ☑ pass | 375 and 1280: no horizontal scroll, long token wraps, Send / Hand back / toggle ~44 px at 375 (shots above) |

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

**AFTER** (`5664316`): `npx vitest run` → 1294 passed; a session with `pending_reply` shows the reply box with the last text, Send, Hand back to terminal and the countdown; a marked idle session shows "marked: it waits for you at its next turn end" (`hold-375.png`, `marked-1280.png`).

**DELTA**: from the Live view the owner can type a request into a terminal session that just went idle (or that they marked *Reply from the app*), and it continues in that same terminal session.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
