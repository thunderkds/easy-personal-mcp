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

## Auto decisions

Mode: auto
1. PR_SET_PDEATHSIG for terminal children stays SIGKILL (a tray crash ends `claude` hard; a clean quit is SIGHUP, then SIGKILL after 5 s) — DEBATED: keep SIGKILL (Advocate 1: keep SIGKILL and add a pid/procStart liveness filter in Live; Advocate 2: SIGHUP for terminal children only; Advocate 3: keep SIGKILL, no code change, guarded by `the_child_dies_with_the_thread_that_started_it`); settled by `docs/ddr/0009-terminal-in-the-app.md:24` "quitting the tray ends it. No daemon." — SIGHUP could leave an orphan `claude` with no backstop. Follow-up (not T133): Live drops `sessions/<pid>.json` whose pid is dead (`session_procs.rs`) ⚠ decided for you — please review
2. The PTY is opened with `posix_openpt(O_RDWR|O_NOCTTY|O_CLOEXEC)` + `ptsname_r`, not the planned `openpty` (Path A kept, no PTY crate) — DEBATED: keep `posix_openpt` (Advocate 1: keep it, record a DDR-0009 amendment; Advocate 2: keep it on Linux, mark the module Linux-only, macOS choice later; Advocate 3: keep it, add a test that no PTY fd reaches another child); all three agree; `openpty` sets close-on-exec only after the fds exist, and app sessions spawn from other threads (`rust/crates/app/src/term_session.rs:6` "in the moment before a flag is set (`openpty` gives fds that are not close-on-exec)"). Follow-ups: DDR-0009 amendment line, the cross-child fd test ⚠ decided for you — please review
3. The Supervisor committed the implementer's finished tree (`0abdae6`) and Evidence (`d7388b4`) after the hook stopped the agent at its step limit — RULE: `CLAUDE.md` § Hard-Stop Gates ("The Supervisor writes the code itself, in the task's worktree")
4. Review scope = the branch diff `main...d7388b4` only, not the main checkout's uncommitted kit edits — RULE: `docs/claude-md/pipeline-stages.md:233` "Bound the review to the change's **blast radius**"
5. Pull moment 3 (`pull_moment.py review`) skipped; its guide-changed check done locally instead (spawn hash `7f734a56…` = now, no commit to the guide since `main`) — DEBATED: skip (Advocate 1: skip, hash check by hand, ask the owner about board mode; Advocate 2: skip, hash + `git log` check, ask the owner to set mode `local`; Advocate 3: skip, write nothing, keep the T-row count); all three agree; `scripts/board_sync.py:149` "tasks = board.list_tasks()" rebuilds the board from Linear (HUN rows only) and drops every T-row, and `scripts/pull_moment.py:24` "a pull moment never blocks work" ⚠ decided for you — please review
Checks: tests `cd rust && cargo test --offline --workspace && cargo clippy --offline --workspace -- -D warnings` — 99 suites, 1398 passed, 0 failed, clippy exit 0, fmt clean; leak gate — 0 hits over 42 canaries; reviews code-review — 0 P0/P1/P2, 2 P3 (final group kill after reap copies `app_session.rs:1599`; module comment should name the crash path); security-review — no findings (branch diff); reviewer spawn `Reviewer: T133` on `d7388b4` — PASS, AC 0-13 met, 0 P0/P1/P2 code, 6 P3 (slot freed after `end()`, master fd kept on instant exit, 1 MiB pre-auth frame, lag close, no repo-busy check, group kill after reap) recorded as follow-ups; real run `serve` + scripted WS client (AFTER above) — printed 201, 403 for evil Origin, 101, `hi` seen, `stty size 30 100`, exit frame, no stub left (auto-checked, not /verify)
Memory: memory/decisions.md — 2026-11-22 T133 terminal server: built, PDEATHSIG stays SIGKILL (3-Advocate Debate), T134 page in parallel
6. Board mode set to `local` (the `"board"` key removed from `.claude/board.json`), so no pull, including the merge gate's, rewrites `PROJECT_KANBAN.md` — RULE: owner answer 2026-11-22 ("Switch to local"), asked because the merge gate's pull (`.claude/hooks/pre_bash_block_unsafe_merge.py:3011` "warning = pull_cache(ROOT)") would have dropped every T-row
Not done: merge (admin: after the auto verify pass); auto verify: ☑ pass (session cf6bc4c7-0458-4061-9bb1-58ee6fbfa1e8, commit f487b19347a262ddc5d60031b6c41c6a01248199, log .claude/verify-runs/T133-f487b19347a2.log)
Closed: row moved to Done 2026-11-22 — auto verify pass (session cf6bc4c7-0458-4061-9bb1-58ee6fbfa1e8, commit f487b19347a262ddc5d60031b6c41c6a01248199); Gates 5 and 6 checked

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `rust/crates/app/src/term_session.rs` (13 tests, incl. AC 0 gate `step_0_the_libc_pty_gives_a_session_leader_on_its_tty_keys_sigwinch_and_sighup_at_close`, ring bound, PDEATHSIG), `rust/crates/app/src/term_routes.rs` (9 route/WS tests: auth frame, evil Origin 403, 186-key order, resize clamp, input cap notice, two pages, shared cap 409), `rust/parity/leak_rust.py` (terminal checks, AC 11) |
| Verification command run | ☑ pass | `cargo test --offline --workspace && cargo clippy --offline --workspace -- -D warnings` 2026-11-22T15:07:41Z..15:13:22Z: 99 suites, passed 1398, failed 0; `test exit 0`; `clippy exit 0`; `clippy --all-targets -D warnings` clean; `fmt-clean`. Leak gate: `leak check: 0 hits over 42 canaries, 1466 surfaces, 3254986 bytes`; `check terminal bytes on the guarded socket ok (3 of 3 planted strings)`; `check terminal socket foreign origin 403 ok`; `check terminal children end with the server ok (2 children, 0 left, exit 0)` |
| Negative cases hold | ☑ pass | mutations by the implementer, each reverted: M1 cap not shared → 5th-start test fails (409 vs 202); M2 guard carve-out off → 5 of 9 `term_routes` tests fail; M3 list carries the replay → leak gate exit 1 `HIT T133 term_secret as raw in [terms api GET /api/term-sessions -> 200]` (outputs: scratchpad `t133/leak-mut.txt`) |
| verify | ☑ pass | auto-run by hook, session cf6bc4c7-0458-4061-9bb1-58ee6fbfa1e8, commit f487b19347a262ddc5d60031b6c41c6a01248199, log .claude/verify-runs/T133-f487b19347a2.log — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewer (fresh context) read the branch diff `main...d7388b4` (HEAD `4d05665` adds only this file): all of `term_session.rs`, `term_routes.rs`, the guard carve-out in `server.rs`, the shared count and extracted helpers in `app_session.rs` / `app_session_routes.rs`, `agent_routes.rs` (shutdown), Cargo manifests and lock, `leak_rust.py`, `run_parity.py`. Skipped: the main checkout's uncommitted kit edits, `web/**` (T134), Python, the unchanged rest of `app_session.rs`. The `code-review` and `security-review` skills were not callable as tools; `skills/code-review/SKILL.md` and `references/security-reviewer.md` were applied by hand. See "Reviewer findings (T133)" below. |
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

---

## Reviewer findings (T133)

Reviewer: fresh context, 2026-11-22, commit under review `d7388b43a3e44b07829e498d31060609225a4fdf` (branch HEAD `4d05665`). `code-review` and `security-review` were applied by hand from `skills/code-review/SKILL.md` and `skills/code-review/references/security-reviewer.md` (the Skill tool was not callable). No code was changed.

**Verdict: PASS. No P0, P1 or P2 code finding. The merge can go ahead on the code.** One process item (below, P2-process) blocks the auto-mode close-out, not the code.

### Commands I ran (real output)
- `cd rust && cargo test --offline --workspace` : `test exit 0`, 99 suites, 1398 passed, 0 failed (5:57).
- `cargo clippy --offline --workspace -- -D warnings` : exit 0. `cargo fmt --check` : clean.
- Leak gate, run by me with `CLAUDE_CONFIG_DIR`/`CLAUDECODE` unset (the harness refuses a real data folder): `leak check: 0 hits over 42 canaries, 1466 surfaces, 3255642 bytes`; `check terminal bytes on the guarded socket ok (3 of 3 planted strings)`; `check terminal socket foreign origin 403 ok`; `check terminal children end with the server ok (2 children, 0 left, exit 0)`; `check T133 terminal strings scanned ok (3 strings, 1465 surfaces, 1 guarded socket capture allowed to hold them)`.
- Not run: `rust/parity/run_parity.py` (needs the Python server side); `cargo deny` / `cargo audit` (not installed, no `deny.toml` in the repo).

### Findings

| Sev | Where | Finding | Boundary / result |
|---|---|---|---|
| P2 (process) | this file, `## Auto decisions` 2 and 5; hook line `Paused:` | Risk is High; decisions 2 (`openpty` vs `posix_openpt`) and 5 (pull moment skipped) are `SAFER OPTION` with the flag "decided for you", which the auto-mode check refuses (must be `DEBATED` with 3 Advocates). The hook has already written `Paused:` (report 27fecc4466ed). Not a code defect. | Supervisor decides: run the Advocates, or restate both as `RULE:` with a written rule, or drop the flag. |
| P3 | `term_session.rs:686` vs `:765-772` | If the child exits at once (a `claude` that fails at start), `supervise` can run `close_master()` before `launch` stores the master at `:686`. The master fd then stays in `Term.master` until the entry is evicted (max 12 ended entries). Bounded fd retention only; no data or safety impact. | Fix: store the master before the thread starts the child, or have `launch` skip it when `ended()`. |
| P3 | `term_session.rs:765-772` | `term.end()` (state `exited`) runs before `drop(slot)`. A caller that polls the list, sees `exited`, and starts at once can get a 409 for the few microseconds before the slot frees. `the_fifth_start_of_either_kind_is_409...` (`term_routes.rs:641`) does exactly that, so it can flake under load. | Fix: drop the slot before `end()`. |
| P3 | `term_session.rs:758` (`signal_group` after reap) | Group SIGKILL after `try_wait` reaped the leader copies `app_session.rs:1599`; the pgid could in theory be recycled when the group is empty. Same pattern as app sessions; already noted by the implementer. | Accept; note only. |
| P3 | `term_routes.rs:52` | Before auth a socket may send one frame up to 1 MiB and hold a task for 5 s. Host and Origin are already checked, a local process still needs no token to open sockets. Hardening only (no data reached). | Optional: cap the first frame at about 4 KiB. |
| P3 | `term_routes.rs:245`, `term_session.rs:61` | A page more than 256 messages behind is closed (`Lagged`) and must reattach. Correct by design, but T134 must reconnect on a close and replay. | Carry into T134 (reconnect on close; the replay restores the screen). |
| P3 | `app_session_routes.rs:430` vs `term_routes.rs:92` | A terminal start has no `RUNNING_HERE` / `repo_quiet` analogue: it can start in a repo where an app session runs. Not an AC; it follows "full parity" only loosely. | Owner call; leave. |
| Note | `rust/Cargo.lock` | AC 12 asks that any new crate is named with its licence. Added to the graph by axum `ws` and the dev-deps (licences from `cargo metadata`): `tokio-tungstenite 0.29.0` MIT, `tungstenite 0.29.0` MIT OR Apache-2.0, `data-encoding 2.11.1` MIT, `base64 0.22.1` MIT OR Apache-2.0, `rand 0.9.5` / `rand_chacha 0.9.0` / `rand_core 0.9.5` MIT OR Apache-2.0, `sha1 0.10.7` MIT OR Apache-2.0 (now a dep of axum), dev-only `futures-util 0.3.34` MIT OR Apache-2.0. All permissive. No `cargo deny`; none exists in the repo. | AC 12 met. |

### Security review (by hand; lower-trust actor, boundary, resource, result)
- Another website in the owner's browser, to the terminal socket: Origin must equal `http://127.0.0.1:<port>` and Host must match (`server.rs:105-120`); tests `the_socket_needs_a_good_host_and_origin...` and the leak gate's foreign Origin 403 show it. Rejected as a finding.
- A local process with no token: the guard lets `GET /api/term/<hex>` through only without the token header (`server.rs:128`, hex id of 1 to 64 digits, `GET` only); it then must send the token in the first frame within 5 s (constant-time compare, `term_routes.rs:160`); the id is looked up after auth, so a probe learns nothing. The token in the URL is ignored (test). Rejected.
- Repeated or wrong token header: still 403 (`(Some, None)` arm, `_ => false`). Rejected.
- Argument and env injection into the shell-capable child: argv is built only from `MODELS` and `MODES` (`term_session.rs argv`); the form is the one `start` uses (`form()`), unknown keys are 400 (`write_body`); no prompt, no free argument; `env_clear` + `child_env` scrub, `TERM`/`COLORTERM` set, `MCP_TOOL_TIMEOUT` dropped. Rejected. `bypassPermissions` at start needs no second confirm, same as the existing app-session start (parity, owner picked it in the form).
- Typed bytes: any attached page can type by design (owner, question 47); 64 KiB/s cap with one notice; the writer is non-blocking (`try_send`), so a stuck child cannot block a tokio worker. Rejected.
- Screen bytes: held in memory only; no `Debug`, no logging in either new file (grep: no `println`, `eprintln`, `dbg!`, `tracing`); the list route sends no screen byte (test and leak mutation M3 by the implementer); leak gate 0 hits (my run). Rejected.
- Process lifetime: `PR_SET_PDEATHSIG` SIGKILL in `pre_exec` from the supervising thread; `Agent::shutdown` stops terminals (`agent_routes.rs`); tests `the_servers_shutdown_ends_every_terminal_child`, `the_child_dies_with_the_thread_that_started_it` and the leak gate (0 children left) pass. Rejected.
- fd inheritance: master and slave opened with `O_CLOEXEC` (test asserts `FD_CLOEXEC`, `term_session.rs:872`), so an app session or `git` spawned at the same time cannot inherit them. Rejected.
- `unsafe`: 6 blocks in `term_session.rs`, each with a SAFETY comment; `pre_exec` calls only `setsid`, `ioctl`, `prctl`, `getppid`. No finding.

### Acceptance Criteria 0-13

| AC | Status | Evidence (code and test, all run by me in the 1398/0 pass) |
|---|---|---|
| 0 | Met | `term_session.rs:864` `step_0_the_libc_pty_gives_a_session_leader_on_its_tty_keys_sigwinch_and_sighup_at_close`: setsid leader, controlling tty, keys, SIGWINCH, close then SIGHUP. Path A (libc, no PTY crate) kept; Path B not needed |
| 1 | Met | `term_routes.rs:487` start 201 + `{id}`; argv, env (`TERM`, `COLORTERM`, account folder, scrub), cwd asserted; `form()` and `stored_repo()` shared, not copied |
| 2 | Met | `:555` compares 14 refusal cases, 3 body shapes, evil Origin, rebinding Host and setting off against `POST /api/app-sessions`: same codes and texts; Chat-only fields are 400 `unknown field` |
| 3 | Met | `:641` two Chat + two Terminal fill the cap, a fifth of either is 409 with the same text; `Sessions::take_slot` shares the count under the one lock (`app_session.rs:1307`, `:1393`). See P3 on the slot-free ordering |
| 4 | Met | `:487` list holds id, repo, account, started, state, code, pages, no screen byte (`"ready"` and `/dev/pts` absent) |
| 5 | Met | `:677` unknown id 404, `confirmed` needed, a stub deaf to SIGHUP is killed after 5 s (5 s <= took < 8 s), reason `stopped` |
| 6 | Met | `:731` evil Origin 403, rebinding Host 403, wrong header token 403, non-hex id 403; wrong token, binary first frame, URL-only token, silent socket all close with no data (silent after 5 s); right token works |
| 7 | Met | `:818` 186 single-key frames arrive in order; resize 100x30, clamp to 500x5, zeros ignored; over 64 KiB/s: exactly one notice; `pages` frame on attach. `exit` frame in `:901` |
| 8 | Met | `:901` both pages see output, both type, last resize wins (90x20), both get `exit` code 7 |
| 9 | Met | `:979` replay holds the first bytes and the later ones then a SIGWINCH nudge; `term_session.rs:988` 3 MiB written holds at most `RING + PREFIX` |
| 10 | Met | `:901` bytes dropped at the last detach after exit, entry stays `exited`, `pgrep` finds no stub; `:1010` shutdown ends every child (also a SIGHUP-deaf one); `term_session.rs:1219` PDEATHSIG |
| 11 | Met | My leak gate run: 0 hits over 42 canaries, 3 of 3 planted strings only on the guarded socket. The tray and popups have no code path to `Terms` (only `agent_routes.rs`, `app_session.rs`, `server.rs`, `main.rs` reference it) |
| 12 | Met | `rust/Cargo.toml`: only `ws` added to axum; other crates named and licensed in the Note row; no supply-chain tool exists |
| 13 | Met | Workspace tests and clippy `-D warnings` pass (my run); the four routes added to `rust_only_surfaces` in `run_parity.py`; the parity script itself was not run by me |

### Merge answer
The code can merge: all 14 criteria are met, tests, clippy and the leak gate are green in my own run, and there is no P0/P1/P2 code finding. Before the auto-mode close-out the Supervisor must resolve the P2-process item (decisions 2 and 5). The three cheap P3 fixes (slot before `end()`, master at launch, first-frame cap) can ride on T134 or a follow-up.
