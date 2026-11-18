# TASK_REVIEW — T116: Live like Claude 2 — process files, Stop, queue

> Sibling of `tasks/TASK_GUIDE_T116.md`. Everything here is **filled by the reviewer at Stage
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

-->

## Auto decisions

Mode: auto
1. Queue for a busy terminal session, delivered by `hook ask` on the next Stop — FACT: PRD.md:344 "trace = **full transcript**, shown never stored; busy input = **queue until the turn end**"
2. (Replaced by 3.) The plan's SIGTERM-only rule — now SIGTERM by default, SIGKILL only on an explicit Force stop
3. Stop of a terminal session that survives SIGTERM: a separate **Force stop** (SIGKILL after its own confirm, same pidfd and identity checks), never an automatic escalation — DEBATED: (c) Force stop (Advocate 1: separate Force stop, the app does not own a terminal session's process, `session_procs.rs:11-16`; Advocate 2: separate Force stop, Stop stays a polite request as in Claude's own terminal, PRD FR-124 needed amending; Advocate 3: separate Force stop, already built with the same checks, smallest safe change) ⚠ decided for you — please review
Checks: tests `cargo test --workspace --no-fail-fast` — 1278 passed, 1 failed (`store_check` 600k-row setup under load; alone `ctrl_c_during_full…` ok, T116 does not touch it); fmt + both clippy runs clean; leak gate — 0 hits over 42 canaries, 1235 surfaces; reviews code-review — 0 P0/P1, 1 P2 + 2 P3 fixed, re-check found a test-build break + 2 leftovers, fixed in `8244e91`; security-review — no finding ≥ 8
Memory: memory/decisions.md — "2026-11-18 — Live like Claude: transcript on demand, queue, Stop/Force stop, past sessions, one panel" (+ MEMORY.md index line), `5c38bf4` on main
Not done: merge (admin: after the auto verify pass); owner smoke test (queue a message to a busy terminal session; stop a scratch terminal session); auto verify: ☑ pass (session 8019ed72-aaf6-4ff7-9d3f-c3c12ef3dd94, commit 0d2e50bf2a5a9e587acad17763ccec28839f989b, log .claude/verify-runs/T116-0d2e50bf2a5a.log)
Closed: row moved to Done 2026-11-18 — auto verify pass (session 8019ed72-aaf6-4ff7-9d3f-c3c12ef3dd94, commit 0d2e50bf2a5a9e587acad17763ccec28839f989b); Gates 5 and 6 checked

<!-- template:
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

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/session_procs.rs` tests (8: 7 fields kept, `*.key`/oversize/non-JSON/symlink/FIFO/pid mismatch skipped, dummy `claude` process: wrong name/start/gone refused, SIGTERM, `force` → SIGKILL, pure `identity_ok` uid mismatch, pid ≤ 1), `rust/crates/app/tests/answer_bridge.rs` e2e (stop 200/409/404/400/403; queue 200/replace/cancel/400/404/409, delivery as `{"decision":"block",…}` with no Live stream, not after the session leaves the list, take-before-write with edit counter), live_feed `proc`/`queued` snapshot test; `leak_rust.py` queue/name/cwd/key canaries |
| Verification command run | ☑ pass | at `8244e91`: `cargo fmt --check` ok; clippy workspace and `--features window` `Finished`; `cargo test --workspace --no-fail-fast` 1278 passed / 1 failed (`store_check` load, passes alone); `leak check: 0 hits over 42 canaries, 1235 surfaces, 2613075 bytes` |
| Negative cases hold | ☑ pass | a non-claude process, a stale start time, an empty `procStart`, a gone pid, pid ≤ 1, another uid and an `sdk` session are all refused and the process keeps running; `force` without `confirmed` → 400, nothing signalled; queued text never delivered to an unlisted session; cwd and `.key` canaries on no surface |
| verify | ☑ pass | auto-run by hook, session 8019ed72-aaf6-4ff7-9d3f-c3c12ef3dd94, commit 0d2e50bf2a5a9e587acad17763ccec28839f989b, log .claude/verify-runs/T116-0d2e50bf2a5a.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | code-review and security-review on `main...T116` (session_procs.rs, answer_bridge.rs, live_feed.rs, hooks_cmd.rs, api.rs, tests, leak gate) + callers; re-check of the fix round |
| Full smoke suite still green (no regression) | ☑ pass | Rust 1278 + `store_check` alone; all T111 hook tests pass; leak 0 hits |
| **Companion score (easy-verifier)** | ☑ N/A | not run: independent code and security reviews used instead | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | server only; the page is T118 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | server only; the page is T118 |
| **UI: Responsiveness at target viewports** | ☑ N/A | server only; the page is T118 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Supervisor, before any T116 implementation commit; `main` at `ddb947f`, the branch point):
```
$ date -u; git log -1 --oneline; grep -rn 'sessions/{id}/stop\|reply/queue\|session_procs' rust/crates/app/src/ | wc -l; ls rust/crates/app/src/session_procs.rs
2026-11-18T10:15:50Z
ddb947f Plan: Live like Claude (question 45, FR-123..FR-128, DDR-0007, T115-T119)
9
ls: cannot access 'rust/crates/app/src/session_procs.rs': No such file or directory
```
(The 9 hits are all the app-session route `/api/app-sessions/{id}/stop` (FR-110) in `app_session_routes.rs`; there is no `/api/live/sessions/{id}/stop`, no `/api/reply/queue`, no `session_procs`. No way to stop a terminal session or queue text for a busy one; Claude Code's process files are not read.)
first implementation commit] OR [verbatim excerpt of the prior content, for non-executable changes]

**AFTER** (`8244e91`, 2026-11-18T12:30:54Z):
```
$ grep -n 'reply/queue\|sessions/{id}/stop' rust/crates/app/src/api.rs | head -3
307:        .route("/api/reply/queue", post(answer_bridge::reply_queue))
309:            "/api/live/sessions/{id}/stop",
```
`rust/crates/app/src/session_procs.rs` exists; tests above.

**DELTA**: the owner can stop (or, after a confirm, force stop) a terminal Claude Code session from the app, and type a message to a busy terminal session that is delivered the moment its turn ends.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
