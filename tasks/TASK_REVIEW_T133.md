# TASK_REVIEW — T133: Terminal 1 — PTY sessions and the terminal WebSocket (server)

> Sibling of `tasks/TASK_GUIDE_T133.md`. Everything here is **filled by the reviewer at Stage
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

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/term_session.rs` (13 tests, incl. AC 0 gate `step_0_the_libc_pty_gives_a_session_leader_on_its_tty_keys_sigwinch_and_sighup_at_close`, ring bound, PDEATHSIG), `rust/crates/app/src/term_routes.rs` (9 route/WS tests: auth frame, evil Origin 403, 186-key order, resize clamp, input cap notice, two pages, shared cap 409), `rust/parity/leak_rust.py` (terminal checks, AC 11) |
| Verification command run | ☑ pass | `cargo test --offline --workspace && cargo clippy --offline --workspace -- -D warnings` 2026-11-22T15:07:41Z..15:13:22Z: 99 suites, passed 1398, failed 0; `test exit 0`; `clippy exit 0`; `clippy --all-targets -D warnings` clean; `fmt-clean`. Leak gate: `leak check: 0 hits over 42 canaries, 1466 surfaces, 3254986 bytes`; `check terminal bytes on the guarded socket ok (3 of 3 planted strings)`; `check terminal socket foreign origin 403 ok`; `check terminal children end with the server ok (2 children, 0 left, exit 0)` |
| Negative cases hold | ☑ pass | mutations by the implementer, each reverted: M1 cap not shared → 5th-start test fails (409 vs 202); M2 guard carve-out off → 5 of 9 `term_routes` tests fail; M3 list carries the replay → leak gate exit 1 `HIT T133 term_secret as raw in [terms api GET /api/term-sessions -> 200]` (outputs: scratchpad `t133/leak-mut.txt`) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [leave it: the auto verify hook writes this row, with its run record. If you ran /verify yourself, tick pass and make the whole Notes cell "/verify by the user, session <id>, commit <sha>" (the full 40-character commit). The merge gate counts no other row, even with the word "pass" in it] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☑ pass | whole Rust workspace 1398/0 (above) |
| **Companion score (easy-verifier)** | ☑ N/A | not run: easy-verifier refused every repo path since 2026-11-01 (memory learnings) |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | server-only task; the page is T134 |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | server-only task; the page is T134 |
| **UI: Responsiveness at target viewports** | ☑ N/A | server-only task; the page is T134 |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: real `serve` (`rust/target/debug/easy-personal-rs serve`, built from `006e926`, `env -i PATH=/usr/bin:/bin HOME=<scratch>/home`), curl from the same host, captured before any T133 commit (token replaced by `<token>`):

```
dashboard: http://127.0.0.1:39025/#<token>
2026-11-22T09:09:58Z
006e926f02bc5fe83c4d18d6d67cf474c3e902aa
$ curl -X POST /api/term-sessions (token, confirmed:true)
404
$ curl GET /api/term-sessions
404
$ curl GET /api/term/abc (WebSocket upgrade, no token header)
403
$ curl GET /api/term/abc (token header)
404
```

**AFTER**: real `serve` built from the T133 tree, temp HOME, stub `claude` = `exec /bin/sh -i` (scratchpad `t133/after.txt`):

```
2026-11-22T15:06:16Z serve up; dashboard line printed (token hidden)
$ POST /api/workspaces -> 201
$ POST /api/term-sessions -> 201 {"id":"6fb3a4e42a2b140f"}
$ WS /api/term/{id} Origin evil -> HTTP/1.1 403 Forbidden
$ WS /api/term/{id} -> HTTP/1.1 101 Switching Protocols
  text frames: ['{"n":1,"t":"pages"}'] | typed echo h''i, output hi seen: True
  after resize, stty size seen 30 100: True
$ POST /api/term-sessions/{id}/stop -> 200
  exit frame: ['{"code":null,"t":"exit"}']
$ GET /api/term-sessions -> {"max":4,"sessions":[{"account":null,"branch":null,"code":null,"id":"6fb3a4e42a2b140f","pages":1,"reason":"stopped","repo":"<scratch>/demo/repo","started":"2026-11-22T15:06:16Z","state":"exited"}]}
  stub pid 189267 left: False | serve exit 0
2026-11-22T15:06:17Z
```

**DELTA**: the app can now start a session in a real terminal (PTY) and any attached page can watch and type into it over a guarded WebSocket; before, all four routes answered 404/403.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
