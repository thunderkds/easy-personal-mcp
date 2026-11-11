# Release plan — `rust-v0.1.0` (the Rust app, P0 + P1 + P1b + P2a)

> **This is a plan and a recommendation. The operator executes it.** Nothing in this file was run against the owner's real data. The Supervisor never pushes `main`, tags, or publishes: those steps are the operator's.

**Proposed tag:** `rust-v0.1.0` (a new component; the Python app keeps its own `v0.x` tags and `RUNBOOK.md` rows). **Crate version:** `0.1.0` (`rust/Cargo.toml`).
**Commit to tag:** `77df906` (the commit that added this plan; **approved by the owner, 2026-11-11**). Tag exactly this commit: later commits only record the approval in documentation.
**Deployment target** (`rust/PROJECT_SPEC.md`): local machine only, Ubuntu first, then macOS, Windows last. **No deploy/install command was defined for the Rust app; the owner chose on 2026-11-11: "Copy to ~/.local/bin (Recommended)" (step 3).**

---

## 1. Release scope summary

Merged since the last push to `origin` before the Rust port (`a928bb1`), 26 commits (plus the one that adds this file), all under `rust/`, `tasks/`, `memory/`, `docs/`, plus one skill edit (`skills/verify/SKILL.md`, T024). The Python app (`src/`, `web/`, `tests/`) is **unchanged**.

| Phase | Tasks | What it delivers |
|---|---|---|
| P0 foundation | T001, T002, T003 | Cargo workspace, `--version`, read-only store open with a `user_version` guard, the parity oracle (Python's answers frozen as goldens) |
| P1 read core | T004-T022 | Loopback HTTP server (Host/Origin/token guard, CSP), nine `/api` GET routes, six MCP read tools, usage/warnings/plan/sessions/workspaces in the core with `workspace`/`account` scope, static dashboard files |
| P1b window | T016, T023 | `app`: a native desktop window (Tauri v2) on the loopback server, behind the `window` Cargo feature, no IPC, navigation locked |
| P2a ingest | T026-T032, T035 | `ingest`: the Rust app fills the shared store itself (events source), Python-exact, behind the same lock, with an oracle, a leak gate, coexistence tests and speed/memory gates |
| tooling | T024, T025 | A Rust step in the `/verify` recipe; README wording |

**Not in this release (honest list):** `kit_trace`, `ui`, `verifier`, `context` ingest (T033 and later); the `live` tail; `schedule install`; 12 dashboard routes still answer 404 (accounts, connectors, dirs, kit report files/workspaces, live, memory-graph, reports, sync); six MCP tools (`kit_summary`, `kit_findings`, `connector_*`, `save_report`) and the `investigate` prompt; every write surface (P3); macOS and Windows (P4, P5); packaging (`.deb`, AppImage); any Rust migration (the Rust app **never migrates**: it needs a store at schema v11 and refuses any other).

## 2. Release notes (for people)

**New: `easy-personal-rs`, a Rust build of the easy-personal tools. Linux (Ubuntu) only for now.** It works on the same data folder as the Python app and can be used next to it.

*Features*
- `easy-personal-rs ingest` reads Claude Code transcripts into the same store, with the same configuration, lock, JSON and exit codes as `easy-personal ingest` (T026-T032, T035). About **2.3x faster** (200 MB: 169.7 MB/s against 74.5 MB/s on the build machine), unchanged re-run 0.01 s, memory flat with file size. The Python ingest and the Rust ingest can run before or after each other on one store; running both at once is safe (the lock makes one wait).
- `easy-personal-rs serve`, `mcp`, `store-info`: read-only dashboard API, six MCP tools and a store check (T004-T022).
- `easy-personal-rs app` (build with `--features window`): the dashboard in a native window, no browser (T016, T023).

*Fixes (differences from Python, all because Python crashes or silently drops)*
- One odd line or entry can no longer stick the ingest for good: a very deeply nested line, an integer beyond 64 bits, a list-valued record type, an out-of-range timestamp, a `cwd` with a lone surrogate, a directory named `x.jsonl`. Rust abstains or skips, counts it, and carries on.
- A store newer than schema v11 gives a clean message (exit 2); Python prints a traceback.

*Known limits*
- Ingests the events source only; the kit, UI, verifier and context sources stay with the Python app (`easy-personal-rs ingest` prints `"not_ingested": [...]` so this is never silent).
- Needs a store at schema v11 (made by the Python app 0.4.0); an older store is refused with "run the Python app once", a missing store is created.
- The window needs the Ubuntu packages `libwebkit2gtk-4.1-0` and `libgtk-3-0t64`, and only works with the built dashboard (`npm --prefix web ci && npm --prefix web run build`).

*Internal*
- A parity oracle (Python's real answers frozen: 21 ingest scenarios, about 200 read-golden files, 3,527 differential strings), a leak gate (42 planted secrets, 116 surfaces, 0 hits), 23 two-writer tests, speed/memory gates. 420 Rust tests pass under three time zones; clippy and fmt clean.

## 3. Smoke test (defined BEFORE the deploy step)

Run on a **scratch folder**, never the real store. Pass condition in each line.

```bash
cd rust && . "$HOME/.cargo/env" && cargo build --release
B=$PWD/target/release/easy-personal-rs
S=$(mktemp -d); mkdir "$S/home" "$S/agents"            # scratch data dir and agent root
export EASY_PERSONAL_HOME="$S/home" EASY_PERSONAL_AGENT_ROOT="$S/agents"
$B --version                                            # PASS: easy-personal-rs 0.1.0
$B ingest --source ../tests/fixtures/claude_code        # PASS: files 6, new_events 39, skipped 0, exit 0
$B ingest --source ../tests/fixtures/claude_code        # PASS: new_events 0
$B store-info                                           # PASS: user_version 11, known 11, pending no
```

Then, from `rust/`: `./parity/.venv/bin/python parity/leak_rust.py` → **PASS:** `leak check: 0 hits over 42 canaries, 116 surfaces`. And `cargo test --workspace` → **PASS:** 0 failed (420 passed, 25 ignored on the build machine). And, where a display exists, `cargo build --release --features window` then `app --self-test --static-dir ../src/easy_personal/dashboard/static` → **PASS:** `self-test: ok`, exit 0 (the dashboard bundle built first).

## 4. Deployment plan (operator)

**Pre-deploy checks**
1. On `main`, working tree clean for the release files (`git status` shows nothing under `rust/`); the commit equals the one approved.
2. Stage 5 evidence is green for every in-scope task: every card T001-T032 and T035 (33 cards) is in Done on `rust/PROJECT_KANBAN.md` with its `tasks/TASK_REVIEW_Txxx.md` and reports; `rust/docs/closeout-2026-11-11.md` lists the owner decisions.
3. The smoke test above passes.
4. **Back up the real store first** (the Rust ingest only appends rows, but a backup costs nothing): `python3 -c "import sqlite3,os; p=os.path.expanduser('~/.easy-personal/store.sqlite'); s=sqlite3.connect('file:'+p+'?mode=ro',uri=True); d=sqlite3.connect(p+'.pre-rust'); s.backup(d); print(d.execute('PRAGMA integrity_check').fetchone())"` (the `sqlite3` command-line tool is not installed on the build machine; this reads the real store read-only and writes a consistent copy; or stop every `easy-personal*` process and `cp` the three files `store.sqlite`, `-wal`, `-shm`). **Pass:** it prints `('ok',)`.

**Ordered steps**
1. `cd rust && cargo build --release` → `rust/target/release/easy-personal-rs`. *Reverse:* delete the binary.
2. Optional window build, into a separate target so the default binary is not replaced: `CARGO_TARGET_DIR=target-window cargo build --release --features window`; and the dashboard bundle: `npm --prefix web ci && npm --prefix web run build`. *Reverse:* delete the two folders.
3. **Install (owner-confirmed 2026-11-11, answer: "Copy to ~/.local/bin (Recommended)"):** `mkdir -p ~/.local/bin && cp rust/target/release/easy-personal-rs ~/.local/bin/easy-personal-rs`; make sure `~/.local/bin` is on `PATH` (`command -v easy-personal-rs`). *Reverse:* `rm ~/.local/bin/easy-personal-rs`.
4. First real run, only after the backup: `easy-personal-rs ingest` (reads the default `~/.claude` account exactly as the Python command does; prints the JSON). **Pass:** exit 0, `skipped` is 0 or explained; then `easy-personal ingest` (Python) prints `new_events: 0` (nothing the Rust run missed) and exits 0.
5. Publish (operator only): `git push origin main`; `git tag rust-v0.1.0 77df906`; `git push origin rust-v0.1.0`; `gh release create rust-v0.1.0 --title "easy-personal-rs 0.1.0 (Rust, Ubuntu)" --notes-file docs/release/rust-v0.1.0.md`. *Reverse:* `gh release delete rust-v0.1.0 --yes`; `git push origin :refs/tags/rust-v0.1.0`; `git tag -d rust-v0.1.0`.

**Post-deploy health check** (the pass condition is the gate; "installed" is not success)

| Check | Command | Pass condition |
|---|---|---|
| Version | `easy-personal-rs --version` | `easy-personal-rs 0.1.0` |
| Store | `easy-personal-rs store-info` | `user_version: 11`, `known: 11`, `pending: no` |
| Ingest idempotent | `easy-personal-rs ingest` twice | second run `"new_events": 0`, exit 0 |
| Python still happy | `easy-personal ping`; `easy-personal ingest` | exit 0; `new_events: 0` after a Rust ingest |
| Server and guard | `easy-personal-rs serve --static-dir <bundle>`; `curl -fsS -H "x-dashboard-token: <token>" <url>/api/version` | `{"version":"0.1.0"}`; the same call without the token returns 403 |
| MCP | `tools/list` then `tools/call usage_summary` over stdio | 6 tools; `isError: false` |
| Window (if built) | `easy-personal-rs app --self-test --static-dir <bundle>` | `self-test: ok`, exit 0 |
| Privacy | `rust/parity/leak_rust.py` | `leak check: 0 hits` |

## 5. Rollback plan

**Trigger (any one):** a health-check line above fails; `PRAGMA integrity_check` on the real store is not `ok` after a Rust ingest; the Python app can no longer open the store; the Rust ingest reports a count that differs from what the Python ingest then adds (a missed or doubled row).

**Reverse steps, in order**
1. Stop using the binary: `rm ~/.local/bin/easy-personal-rs` (step 3) and, if scheduled by hand, any cron/timer line that calls it. The Python app, its timer and its dashboard are untouched and keep working: nothing in `src/` changed and the Rust app never changes the schema.
2. If the store is suspect: stop every `easy-personal*` process, then restore the backup: copy `~/.easy-personal/store.sqlite.pre-rust` over `~/.easy-personal/store.sqlite` and delete the stale `store.sqlite-wal` and `-shm`; run `easy-personal ingest` (Python) once, which re-reads the transcripts from the restored offsets. **Pass:** `PRAGMA integrity_check` `ok` and `easy-personal --version` `0.4.0` (Python).
3. Undo the publication (if step 5 ran): the three commands under step 5 *Reverse*.
4. Nothing to migrate down: the Rust app never ran a migration (`user_version` is 11 before and after). Note for the operator: rows the Rust ingest added are indistinguishable from Python's by design (same keys), so restoring the backup is the only way to remove them.

## 6. Pre-flight GO gate (recommendation)

| Gate | State |
|---|---|
| Stage 5 evidence green for every in-scope task | **Yes**: all 33 cards (T001-T032 and T035) Done with evidence; Supervisor re-ran tests, lint and real-process checks for each |
| Rollback plan with triggers and exact reverse steps | **Yes** (section 5) |
| Post-deploy health check with a concrete pass condition | **Yes** (section 4) |
| Deploy command known | **Yes**: not in `PROJECT_SPEC.md`, but confirmed by the owner on 2026-11-11 (copy the release binary to `~/.local/bin`, step 3) |
| Production approval recorded for the version and commit | **Yes**: owner, 2026-11-11, answer "Approve rust-v0.1.0 @ 77df906" (copied verbatim into the Release Log row of `RUNBOOK.md`) |

**Verdict: GO** (all five gates hold). This is a recommendation; **the operator executes**: the Supervisor does not push, tag or publish.

## 7. Risks to watch during this release (`Known Risk Areas`)

- **The first real ingest into the owner's store** is the first time Rust writes there: hence the backup and the "Python adds 0" check.
- Two writers: always use `flock`-aware tools; never open the store with a tool that ignores `ingest.lock` while an ingest runs.
- `serve` and `app` bind `127.0.0.1` only; the token is per run; do not expose them.
- A `--features window` binary needs GTK/WebKit libraries even to run `serve`/`mcp`: use the default binary for CLI and MCP.
- Five MPL-2.0 crates are in the Tauri tree (owner accepted, 2026-11-11).
