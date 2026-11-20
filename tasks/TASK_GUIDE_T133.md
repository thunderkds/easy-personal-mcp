# TASK_GUIDE — T133: Terminal 1 — PTY sessions and the terminal WebSocket (server)
**Date**: 2026-11-20
**Complexity Level**: C3
**Risk Level**: High (a route that types into a shell-capable process; a new process lifecycle; a sixth NFR-002 exception)
**Priority**: P2
**Assigned agents**: backend-developer
**Agent guides**: `agents/backend.md`

> Read relevant files from `docs/legacy/` before making any code changes. (This repo has no `docs/legacy/`; its sources of truth are `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` and `docs/ddr/`.)

---

## Mandatory Startup (Do Not Skip)
1. `PROJECT_SPEC.md`, `rust/PROJECT_SPEC.md`, `PRD.md` FR-109..FR-116, **FR-136..FR-138**, NFR-002, NFR-008, question 41 and 47
2. `docs/ddr/0009-terminal-in-the-app.md`, `docs/ddr/0006-write-sessions-cli-parity.md`, `BRAINSTORMING_LOG_TERMINAL.md` (Path A, edge case checklist)
3. `tasks/evidence-T130/SPIKE.md` (on branch `T130` until the owner merges it: `git show T130:tasks/evidence-T130/SPIKE.md`) — § "What a build would need", routes table, guard tests
4. `rust/crates/app/src/app_session.rs` (`MAX_RUNNING` `:53`, `child_env` `:263`, `pre_exec` + `PR_SET_PDEATHSIG` `:1445`, stop escalation), `rust/crates/app/src/app_session_routes.rs` (`start` `:430`, form-field checks, `confirmed`), `rust/crates/app/src/server.rs` (`guard_layer` `:139`, Host `:104`, Origin `:110`)
- Tests never start the real `claude` and never touch the owner's real `~/.claude*`: use a stub program (a small shell script that echoes, reads stdin, handles SIGWINCH/SIGHUP) via the existing test seam for the claude path.

## Requirement
Owner 2026-11-20 (T130 → AskUserQuestion): build the terminal **beside** Chat; sessions **end with the tray**; **all attached pages can type**; **full parity** (no hook or setting skipped). This task is the server half; T134 is the page.

## Dependencies & Reachability
**Depends on**: none (T130 spike evidence on branch `T130`)
**Entry point**: `POST /api/term-sessions`, `GET /api/term-sessions`, `POST /api/term-sessions/{id}/stop`, `GET /api/term/{id}` (WebSocket). Reached from the page in T134; this task proves them with route tests and a scripted WebSocket client.

## Acceptance Criteria
| # | Criterion | Evidence |
|---|---|---|
| 0 | **Step 0 (gate)**: a throwaway check that `libc::openpty` + `Command` with `pre_exec` (`setsid`, `ioctl(TIOCSCTTY)`, `PR_SET_PDEATHSIG`) passes spike Q1, Q2 and Q6 with the stub: own session leader, controlling tty, keys reach it, `TIOCSWINSZ` delivers SIGWINCH, master close → SIGHUP → exit. If it fails, use `portable-pty` 0.9.0 (Path B) and say why in the review | review Auto decisions + test |
| 1 | `POST /api/term-sessions` with `confirmed: true` and the FR-109 fields (repo, workspace, account, model, mode; each checked against its fixed list or the store exactly as `start` does) starts the program in a PTY with `child_env` + `TERM=xterm-256color`, `COLORTERM=truecolor`; no prompt, no free arguments; 201 + `{id}` | route tests |
| 2 | Missing/false `confirmed`, unknown field values, guard failures → the same codes and fixed texts as `POST /api/app-sessions` | route tests |
| 3 | One cap of `MAX_RUNNING` (4) across app sessions and terminal sessions: the 5th start of either kind → 409 with the fixed text | test mixing both kinds |
| 4 | `GET /api/term-sessions` lists id, repo, account, started, `running` or `exited` + code, `pages` (attached count); no screen bytes | route test |
| 5 | `POST /api/term-sessions/{id}/stop` (`confirmed`) closes the master, SIGKILLs the process group after 5 s; unknown id → 404 | test with a stub that ignores SIGHUP |
| 6 | WebSocket `GET /api/term/{id}`: the guard checks Host and Origin on the upgrade (evil Origin / rebinding Host → 403); first frame `{"t":"auth","token","cols","rows"}` within 5 s, else close with no data; wrong token → close with no data; the token is never read from the URL | WS tests (tokio-tungstenite client in tests) |
| 7 | In: binary = stdin bytes in order (186 single-key frames arrive 186/186 in order); text `resize` clamps cols 10..500, rows 5..200, ignores zero; input over 64 KiB/s per session is dropped with one `{"t":"notice"}`. Out: binary PTY bytes, `{"t":"exit","code"}`, `{"t":"pages","n"}` on attach/detach | WS tests with the stub |
| 8 | Two clients attached: both receive output; both can type; the last resize wins (FR-137, question 47) | WS test |
| 9 | Attach replays the first 4 KiB + the 1 MiB ring, then a resize nudge; the ring wraps without growing (memory bound test: 3 MiB written, held ≤ 1 MiB + 4 KiB) | test |
| 10 | Program exits by itself → every page gets `exit`; the entry stays `exited` and its bytes are dropped when the last page detaches after exit; at app exit every PTY child ends (PDEATHSIG) and no stub process is left | tests + `pgrep` in the test |
| 11 | Leak gate: a planted secret printed by the stub into the PTY is absent from the store, logs, MCP, CLI, `/api/alerts`, tray menu text and popups (extend the existing leak suite) | leak gate run, 0 hits |
| 12 | `rust/Cargo.toml`: axum gains the `ws` feature only; any other new crate is named in the review with licence; `cargo deny`/existing supply-chain check (if any) passes | diff + command |
| 13 | Workspace tests, clippy `-D warnings` pass; the routes are in the parity Rust-only list | commands below |

## Approach
Path A (`BRAINSTORMING_LOG_TERMINAL.md`). New `rust/crates/app/src/term_session.rs`: PTY open + spawn (reuse `child_env` and the PDEATHSIG thread pattern from `app_session.rs`), reader thread → prefix + ring + `tokio::sync::broadcast`, waiter → exit event, writer with the input cap; registry shares the running count with app sessions (expose one counter, do not copy the cap). New `rust/crates/app/src/term_routes.rs` for the four routes; mount in `server.rs` inside the guard layer. Copy no form-check code: call the one `start` uses.

## Files to change (predicted)
| File | Change |
|------|--------|
| `rust/crates/app/src/term_session.rs` (new) | PTY owner, ring, registry |
| `rust/crates/app/src/term_routes.rs` (new) | routes + WS |
| `rust/crates/app/src/app_session.rs` | share the running count only |
| `rust/crates/app/src/server.rs`, `rust/crates/app/src/main.rs` | mount routes / `mod` lines |
| `rust/Cargo.toml`, `rust/crates/app/Cargo.toml`, `rust/Cargo.lock` | axum `ws`; tokio-tungstenite as a dev-dependency for tests |
| leak suite files, `rust/parity/run_parity.py` | AC 11, AC 13 |

## Files must NOT touch
| File | Reason |
|------|--------|
| stream-json behaviour in `app_session.rs` / `app_session_routes.rs` (beyond the shared count) | Chat stays as it is (DDR-0009 §1) |
| `answer_bridge.rs`, `hooks_cmd.rs` | out of scope |
| `web/**` | T134 |
| owner's real `~/.claude*`, the real `claude` | DDR-0001; tests use a stub |
| `src/easy_personal/**` (Python) | not changed |

## Test plan
Rust unit + route + WS tests with the stub program; leak gate. `/verify`: real `serve` with a temp `HOME` and the stub as claude: start → WS attach with a scripted client → type `echo hi` → see `hi` → resize → stop → no process left.

## Verification Command
```bash
cd rust && cargo test --workspace && cargo clippy --workspace -- -D warnings
```

## Completion checklist
- [ ] Implementation done
- [ ] Lint passes (clippy)
- [ ] Tests pass
- [ ] Smoke test confirmed
- [ ] `memory/` updated (Supervisor)
- [ ] TASK_GUIDE marked complete
