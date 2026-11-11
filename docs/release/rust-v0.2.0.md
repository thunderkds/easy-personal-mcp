# Release plan — `rust-v0.2.0` (the Rust app: P2 complete — ingest of the kit sources, the scheduler, hardening)

> **This is a plan and a recommendation. The operator executes it.** Nothing in this file was run against the owner's real data. The Supervisor never pushes `main`, tags or publishes; the auto-mode gate blocks it and the owner is the operator.

**Proposed tag:** `rust-v0.2.0` (the Rust component; the Python app keeps its own `v0.x` tags, for example `v0.2.0` is Python's). **Crate version:** `0.2.0` (`rust/Cargo.toml`, T039). **Commit to tag:** `0fb232e` (the commit that added this plan; **approved by the owner, 2026-11-11**). Tag exactly this commit: later commits only record the approval and change no code.
**Deployment target** (`rust/PROJECT_SPEC.md`): local machine only, Ubuntu first. No deploy command is defined for the Rust app: the install method is the one the owner confirmed for 0.1.0 (copy the release binary to `~/.local/bin`).

**Relation to `rust-v0.1.0`:** that release was approved at `77df906` but **was never tagged** (`git tag` and `git ls-remote --tags origin 'rust-v*'` show no `rust-v` tag on 2026-11-11). 0.2.0 contains everything in 0.1.0. Recommended: **skip tagging 0.1.0 and publish only `rust-v0.2.0`** (one release, one set of notes). If you prefer both, tag `rust-v0.1.0` on `77df906` first as `docs/release/rust-v0.1.0.md` says, then this one.

---

## 1. Release scope summary

Since `77df906` (the 0.1.0 release commit): 11 commits, all under `rust/`, `tasks/`, `memory/`, `skills/verify/`, plus plans.

| Tasks | What it delivers |
|---|---|
| T033 | `ingest` also fills `kit_trace_events`, `ui_sessions` and `verifier_scores` (Python's rules, oracle `parity/gen_ingest_kit.py`, 11 files 0 different); `not_ingested` is now `["context"]` |
| T034 | `schedule install\|remove`: the daily systemd user timer (same unit names as Python, command line runs the Rust binary; oracle `parity/gen_schedule.py`, 12 files 0 different) |
| T036 | the `display` test flake fixed (8 of 100 red under load before, 0 of 200 after) |
| T037 | `/verify` recipe covers the ingest (twice) and the leak gate |
| T038 | the `folders.rs` comment made true; the readers coexistence test uses a deadline |
| T039 | crate and window version `0.2.0` |

Plus everything in `rust-v0.1.0` (T001-T032, T035: read core, dashboard server, six MCP tools, desktop window, events ingest). **33 + 6 = 39 cards Done** on `rust/PROJECT_KANBAN.md`.

**Not in this release (honest list):** `context` ingest; the dashboard's start-up ingest; the `live` tail and `/api/live`; **17 dashboard GET routes still answer 404** (reports, first-run, accounts, sync, kit*, board, memory-graph, jobs, dirs, connectors*, live; the earlier "12" was low) and six MCP tools (`kit_summary`, `kit_findings`, `connector_*`, `save_report`) and the `investigate` prompt; every write surface (workspaces, accounts, connectors, sync, keychain, launcher); macOS and Windows. They are planned in `BRAINSTORMING_LOG_RUSTP3.md` (T040-T051).

## 2. Release notes (for people)

**`easy-personal-rs` 0.2.0, a Rust build of the easy-personal tools. Linux (Ubuntu) only.** It works on the same data folder as the Python app and can be used next to it.

*Features*
- `ingest` now also reads the kit traces, the easy-ui sessions and the easy-verifier scores, so the kit, UI and verifier views fill after a Rust-only ingest (T033).
- `schedule install` / `schedule remove`: run the ingest daily on a systemd user timer, like `easy-personal schedule` (T034). It uses the **same unit names as Python**: installing from one replaces the other's timer (one scheduled ingest per machine).
- From 0.1.0: `ingest`, `serve`, `mcp` (6 tools), `store-info`, `app` (window, build with `--features window`).

*Fixes (differences from Python, all because Python crashes or hangs)*
- A FIFO, a deeply nested line, an out-of-range time or a folder named `x.jsonl` can no longer stick the ingest (Rust skips or abstains and counts it).
- A failing `systemctl` or an unwritable unit folder is a clean one-line exit 1 (Python prints a traceback).

*Known limits*
- No `context` ingest; the dashboard tabs that need the missing routes stay empty or error against the Rust server: use the Python dashboard for those (`easy-personal dashboard`).
- Needs a store at schema v11 (made by the Python app 0.4.0); an older store is refused, a missing store is created.
- The timer's command is the absolute path of the binary you installed from: if you move it, install again.
- The window needs `libwebkit2gtk-4.1-0` and `libgtk-3-0t64` and the built dashboard (`npm --prefix web ci && npm --prefix web run build`).

*Internal*: 461 Rust tests pass under three time zones; clippy and fmt clean; leak gate 0 hits over 42 canaries and 116 surfaces with the kit, ui and verifier rows now ingested; ingest is 2.5x faster than Python on 200 MB with equal tables (T032 numbers, unchanged by T033 within noise).

## 3. Smoke test (defined BEFORE the deploy step)

Run on a **scratch folder**, never the real store. Pass condition in each line.

```bash
cd rust && . "$HOME/.cargo/env" && cargo build --release --locked
B=$PWD/target/release/easy-personal-rs
S=$(mktemp -d); mkdir "$S/home" "$S/agents" "$S/xdg"
export EASY_PERSONAL_HOME="$S/home" EASY_PERSONAL_AGENT_ROOT="$S/agents" XDG_CONFIG_HOME="$S/xdg"
$B --version                                            # PASS: easy-personal-rs 0.2.0
$B ingest --source ../tests/fixtures/claude_code        # PASS: files 6, new_events 39, skipped 0, not_ingested ["context"], exit 0
$B ingest --source ../tests/fixtures/claude_code        # PASS: new_events 0
$B store-info                                           # PASS: user_version 11, known 11, pending no
$B schedule install --dry-run                           # PASS: two units printed, ExecStart ends "easy-personal-rs ingest", exit 0, nothing written under $S/xdg
```

*Run by the Supervisor on the release binary, scratch folder, 2026-11-11 (commit `5d09c0d`): `--version` `easy-personal-rs 0.2.0`; ingest `files 6, new_events 39, skipped 0`, `kit_trace {files 0, new_events 0}`, `ui` and `verifier` `no data`; second ingest `new_events 0`; `store-info` `user_version 11`, `known 11`, `pending no`; `schedule install --dry-run` printed both units with `ExecStart=<release binary> ingest`, nothing written (`xdg` folder empty). Leak gate `0 hits over 42 canaries, 116 surfaces, 847452 bytes`; `gen_ingest_kit.py --check` `11 files, 0 different`; `gen_schedule.py --check` `12 files, 0 different`. The plan is proven runnable; the operator still runs it on the real machine.*

Then, from `rust/`: `cargo test --workspace --locked` → **PASS:** 0 failed (461 passed, 26 ignored on the build machine, under `TZ` unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago`). And where a display exists: `cargo build --release --features window` into a separate target (`CARGO_TARGET_DIR=target-window`), then `app --self-test --static-dir ../src/easy_personal/dashboard/static` → **PASS:** `self-test: ok`, exit 0 (the dashboard bundle built first).

## 4. Deployment plan (operator)

**Pre-deploy checks**
1. On `main`, the tree clean for the release files (`git status` shows nothing under `rust/`); the commit equals the approved one.
2. Stage 5 evidence is green for every in-scope task: T033, T034, T036-T039 are in Done on `rust/PROJECT_KANBAN.md` with `tasks/TASK_REVIEW_Txxx.md` and reports; the owner decisions ("decided for you") are in the T033/T034 reviews and `rust/README.md`.
3. The smoke test above passes.
4. **Back up the real store first** (Rust ingest only appends rows, a backup costs nothing): `python3 -c "import sqlite3,os; p=os.path.expanduser('~/.easy-personal/store.sqlite'); s=sqlite3.connect('file:'+p+'?mode=ro',uri=True); d=sqlite3.connect(p+'.pre-rust'); s.backup(d); print(d.execute('PRAGMA integrity_check').fetchone())"` (read-only on the real store; or stop every `easy-personal*` process and `cp` the three files). **Pass:** `('ok',)`. **Also** (new in this release, the kit sources replace whole tables): the backup covers `ui_sessions` and `verifier_scores` too.

**Ordered steps**
1. `cd rust && cargo build --release --locked`. *Reverse:* delete the binary.
2. Optional window build (separate target) and the dashboard bundle as in the 0.1.0 plan. *Reverse:* delete the two folders.
3. **Install:** `mkdir -p ~/.local/bin && cp rust/target/release/easy-personal-rs ~/.local/bin/easy-personal-rs`; `command -v easy-personal-rs`. *Reverse:* `rm ~/.local/bin/easy-personal-rs`.
4. First real run, only after the backup: `easy-personal-rs ingest`. **Pass:** exit 0, `skipped` 0 or explained; then `easy-personal ingest` (Python) prints `new_events: 0` and its own `kit_trace`, `ui`, `verifier` objects equal in meaning to Rust's.
5. **Scheduler (optional, after step 4 passes):** `easy-personal-rs schedule install --dry-run` and read the units; then `easy-personal-rs schedule install`. It **replaces** a Python-installed timer of the same name. **Pass:** `systemctl --user list-timers easy-personal-ingest.timer` lists it. *Reverse:* `easy-personal-rs schedule remove` (or `easy-personal schedule remove`; then re-install the Python timer if you want it).
6. Publish (operator only): `git push origin main`; `git tag rust-v0.2.0 <approved commit>`; `git push origin rust-v0.2.0`; `gh release create rust-v0.2.0 --title "easy-personal-rs 0.2.0 (Rust, Ubuntu)" --notes-file docs/release/rust-v0.2.0.md`. *Reverse:* `gh release delete rust-v0.2.0 --yes`; `git push origin :refs/tags/rust-v0.2.0`; `git tag -d rust-v0.2.0`.

**Post-deploy health check** (the pass condition is the gate)

| Check | Command | Pass condition |
|---|---|---|
| Version | `easy-personal-rs --version` | `easy-personal-rs 0.2.0` |
| Store | `easy-personal-rs store-info` | `user_version: 11`, `known: 11`, `pending: no` |
| Ingest idempotent | `easy-personal-rs ingest` twice | second run `"new_events": 0`, exit 0 |
| Python still happy | `easy-personal ping`; `easy-personal ingest` | exit 0; `new_events: 0` after a Rust ingest |
| Kit tables | `easy-personal-rs store-info` | `kit_trace_events`, `ui_sessions`, `verifier_scores` counts equal what the Python app showed before (or grew only by new files) |
| Server and guard | `easy-personal-rs serve --static-dir <bundle>`; `curl -fsS -H "x-dashboard-token: <token>" <url>/api/version` | `{"version":"0.2.0"}`; without the token 403 |
| MCP | `tools/list` then `tools/call usage_summary` over stdio | 6 tools; `isError: false` |
| Timer (if installed) | `systemctl --user list-timers easy-personal-ingest.timer` | the timer is listed; `systemctl --user cat easy-personal-ingest.service` shows `ExecStart=<your binary> ingest` |
| Window (if built) | `easy-personal-rs app --self-test --static-dir <bundle>` | `self-test: ok`, exit 0 |
| Privacy | `rust/parity/leak_rust.py` | `leak check: 0 hits` |

## 5. Rollback plan

**Trigger (any one):** a health-check line fails; `PRAGMA integrity_check` on the real store is not `ok` after a Rust ingest; the Python app can no longer open the store; Rust's ingest count differs from what the Python ingest then adds; the kit tables are emptied or changed in a way the Python app would not (the replace-all sources `ui_sessions` and `verifier_scores`); the installed timer runs and fails (`systemctl --user status easy-personal-ingest.service`).

**Reverse steps, in order**
1. Remove the timer if installed: `easy-personal-rs schedule remove`. If you also had the Python timer before, `easy-personal schedule install` puts it back.
2. Stop using the binary: `rm ~/.local/bin/easy-personal-rs`. The Python app, its dashboard and its timer (once reinstalled) are untouched: nothing in `src/` changed and the Rust app never changes the schema.
3. If the store is suspect: stop every `easy-personal*` process, copy `~/.easy-personal/store.sqlite.pre-rust` over `~/.easy-personal/store.sqlite`, delete the stale `-wal` and `-shm`, run `easy-personal ingest` (Python) once. **Pass:** `PRAGMA integrity_check` `ok` and `easy-personal --version` `0.4.0`.
4. Undo the publication (if step 6 ran): the three commands under step 6 *Reverse*.
5. Nothing to migrate down: no Rust migration exists (`user_version` is 11 before and after). Rows the Rust ingest added are indistinguishable from Python's by design; restoring the backup is the only way to remove them.

## 6. Pre-flight GO gate (recommendation)

| Gate | State |
|---|---|
| Stage 5 evidence green for every in-scope task | **Yes**: T033, T034, T036-T039 Done with evidence; the Supervisor re-ran tests, lint, oracles and real-process checks for each |
| Rollback plan with triggers and exact reverse steps | **Yes** (section 5) |
| Post-deploy health check with a concrete pass condition | **Yes** (section 4) |
| Deploy command known | **Yes**: confirmed by the owner for 0.1.0 (copy to `~/.local/bin`), unchanged |
| Production approval recorded for the version and commit | **Yes**: owner, 2026-11-11, answer "Approve rust-v0.2.0 @ 0fb232e" (copied verbatim into the Release Log row of `RUNBOOK.md`) |

**Verdict: GO** (all five gates hold). Tag exactly `0fb232e`: the commits after it only record this approval and change no code. This is a recommendation; **the operator executes**.

## 7. Risks to watch during this release

- The first real Rust `ingest` **with the replace-all sources**: `ui_sessions` and `verifier_scores` are rewritten like Python does. The backup and the "Python agrees" check cover it.
- The scheduler replaces a Python timer of the same name: read `schedule install --dry-run` first.
- Two writers: Rust and Python share `ingest.lock`; never open the store with a tool that ignores it during an ingest.
- `serve` and `app` bind `127.0.0.1` only; the token is per run.
- A `--features window` binary needs GTK/WebKit even for `serve`/`mcp`: use the default binary for CLI and MCP.
- The Rust dashboard is incomplete (17 routes missing): for those tabs use the Python dashboard.
