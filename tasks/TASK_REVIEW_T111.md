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
4. A marked session's state outside a hold reaches the page as a per-session `reply_marked: true` in the `GET /api/live` snapshot (added in `add_pending`, also when nothing else is pending; only for listed sessions; never in `LiveTail::snapshot()` read by the tray and alerts; marks of sessions that leave the list are pruned) — DEBATED: A (Advocate 1: A, high, cites rust/crates/app/src/answer_bridge.rs:432 "FR-121: sessions the owner marked *Reply from the app*; in memory, dropped at app exit"; Advocate 2: A, high (asked for the smallest change), cites docs/ddr/0005-live-question-text-show-never-store.md:19 "`LiveTail::snapshot()`, which the tray and the alert engine read, never holds it"; Advocate 3: A, high (owner with two windows and reloads), cites rust/crates/app/src/answer_bridge.rs:735; re-run 2026-11-18 on `4d2807e` with the task attributed: all three A again, citing rust/crates/app/src/live_feed.rs:497-499 and the tail-snapshot test at :1534) ⚠ decided for you — please review
Checks: build `e9ceeb7`: cargo test --workspace 1258 passed / 0 failed, clippy ± window clean, fmt ok, leak 0 hits over 42 canaries and 1214 surfaces (agent); security-review (independent, scoped to `git diff 446fecf e9ceeb7`): no finding at or above confidence 8 (guarded routes + `confirmed`, 128-bit hold id, `SO_PEERCRED` both ends, `json!` output re-checked by `is_stop_output`, add-only settings write); code-review (independent): 0 P0/P1/P2, 3 P3 → fixed in `4d2807e` (write-confirmed replies and answers, `isMeta` parity note, MAX_HOLDS test) plus decision 4 (`reply_marked`); security-review of `08bf0dd..4d2807e`: no finding ≥ 8; accepted P3: a write slower than the 6 s ack wait can reach the hook while the owner is told 409 (still only to its own hook, once). Supervisor re-run at `4d2807e`: `cargo test --workspace --no-fail-fast` 1261 passed / 0 failed; clippy `--features window` clean; leak 0 hits over 42 canaries, 1214 surfaces
Memory: memory/decisions.md — reply to idle sessions entry extended with T111 (written with the merge)
Not done: merge (admin: after the auto verify pass); auto verify: ☑ pass (session 60258345-209b-4fb8-b6b1-fb484ca70364, commit 2eb8a28008575076c8fc3fd3adc1827b62f04987, log .claude/verify-runs/T111-2eb8a2800857.log); real-terminal smoke (owner, with T112); T112 page
Paused: done not moved — report incomplete: auto verify slot

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/answer_bridge.rs` tests (hold rule, watch, mark, reply/release, stream close, app exit, killed hook, no crossing, write-confirmed delivery, MAX_HOLDS), `rust/crates/app/tests/answer_bridge.rs` (binary: install three entries, Stop handed back cases, e2e reply, 403/400/405 on the 4 routes), `rust/crates/app/src/hooks_cmd.rs` tests (third entry, `on (no reply)`, hand-edited Stop kept), `rust/crates/app/src/live_feed.rs` tests (`pending_reply`, `reply_marked`, tail snapshot clean), `rust/crates/core/tests/live_waiting.rs` (isMeta skipped). Run 2026-11-18 at `4d2807e`: `cargo test --workspace --no-fail-fast` → `passed 1261 failed 0` |
| Verification command run | ☑ pass | `cargo fmt --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` and `-p easy-personal-rs --features window --all-targets -- -D warnings` clean; `cargo test --workspace --no-fail-fast` → 1261 passed, 0 failed; `parity/leak_rust.py` → `leak check: 0 hits over 42 canaries, 1214 surfaces, 2593190 bytes` |
| Negative cases hold | ☑ pass | RED seen before each implementation (compile errors for new APIs, then assertion failures: e.g. `live_feed.rs:1498 left: Null, right: true`, `answer_bridge.rs:2639 nothing was written`); fail-open cases: no app, no stream, unlisted, sdk, `stop_hook_active`, malformed stdin → exit 0, empty stdout (timed median 1.8 ms) |
| verify | ☑ pass | auto-run by hook, session 60258345-209b-4fb8-b6b1-fb484ca70364, commit 2eb8a28008575076c8fc3fd3adc1827b62f04987, log .claude/verify-runs/T111-2eb8a2800857.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review on `git diff 446fecf e9ceeb7` + callers; security-review on `446fecf..e9ceeb7` and `08bf0dd..4d2807e`; T106 `answer()` touched by the shared delivery helper (owner-approved scope, tests pass) |
| Full smoke suite still green (no regression) | ☑ pass | `cargo test --workspace --no-fail-fast` 1261 passed / 0 failed (was 1237 on main before T111) |
| **Companion score (easy-verifier)** | ☑ N/A | not run: low-signal on this repo (memory handoff 2026-11-14); independent code and security reviews used instead |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Server only; the page is T112 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Server only; the page is T112 |
| **UI: Responsiveness at target viewports** | ☑ N/A | Server only; the page is T112 |

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

**AFTER** (`4d2807e`): the binary e2e test `a_reply_from_the_app_reaches_the_held_stop_hook_and_leaks_nowhere` holds a Stop hook, sends `POST /api/reply`, and `hook ask` prints exactly `{"decision":"block","reason":"/status please\n  second \"line\" é"}`; `cargo test --workspace --no-fail-fast` → 1261 passed, 0 failed.

**DELTA**: the app can hold an idle terminal session at its turn end (90 s, longer while watched, up to 10 min when marked) and send the owner's typed request into it; the page half is T112.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
