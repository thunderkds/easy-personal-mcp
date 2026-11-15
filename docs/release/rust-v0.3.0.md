# Release plan — `rust-v0.3.0` (the Rust app: Python feature parity, cutover, Watch + tray, Ubuntu .deb)

> **This is a plan and a recommendation. The operator executes it.** The owner asked for it on 2026-11-15 ("release new version app", then picked "0.3.0, tag + .deb"). The Supervisor prepares the branch, the notes and the package; the owner approves the commit; a push and a GitHub Release page go out only on the owner's word.

**Proposed tag:** `rust-v0.3.0` (the Python app keeps its own `v0.x` tags). **Crate version:** `0.3.0` (`rust/Cargo.toml`, `rust/crates/app/tauri.conf.json`, `rust/Cargo.lock`). **Commit to tag:** the merge of branch `release-rs-0.3.0` into `main` (named in the Release Log row once merged and approved).
**Deployment target** (`rust/PROJECT_SPEC.md`): local machine only, Ubuntu. Two install methods: the cutover install (`cargo install` into `~/.local/share/easy-personal-rs`, `rust/docs/CUTOVER.md` Step 1, already on the owner's machine) and, new in this release, the `.deb` package (`rust/packaging/README.md`).

**Relation to earlier plans:** `rust-v0.1.0` and `rust-v0.2.0` were approved but **never tagged** (`git tag` shows only Python's `v0.2.0`). 0.3.0 contains both. Recommended: tag only `rust-v0.3.0`.

---

## 1. Release scope summary

Since `0fb232e` (the 0.2.0 plan commit): 323 commits (no merges), cards T040-T062, T064-T087 Done on `rust/PROJECT_KANBAN.md`; HUN-244 (store health) and HUN-245 (Copy command) ported or shipped on the shared web page. T063 (remove the Python app) is **not** in this release: it waits for the week of use (to about 2026-11-22) and the owner's gate.

| Tasks | What it delivers |
|---|---|
| T040-T048, T053, T054 | Every dashboard read route Python has (reports, first run, accounts, sync, kit, board, memory graph, dirs, jobs, report files) and the store-only workspace and account writes; MCP `kit_summary`, `kit_findings`, `connector_*`, `save_report`, prompt `investigate` |
| T049, T051, T056, T057 | Live tail engine, `GET /api/live` (SSE), start-up ingest in `serve`/`app` (`--no-ingest`), `mcp --transport http` (loopback), `context` ingest |
| T050, T065, T066, T078 | OS keychain, connector launcher (`easy-personal-launch`), connector writes, sync apply, legacy adopt |
| T058-T061, T067 | Python's CLI commands in Rust (`ping`, `usage`, `outliers`, `kit-findings`, ...), `store-check` and `/api/store/health`, `connector` and `doctor` commands, full parity run (7 findings closed) |
| T062, T086 | Cutover guide `rust/docs/CUTOVER.md`; Rust is the default on the owner's machine since 2026-11-15 |
| T068-T071 | UI redesign: status chips, First run rework, per-screen polish, icon set |
| T072-T077, T083-T085 | Watch: machine status, plan-limit watch, alerts with desktop notifications, effectiveness, `tray` command, Watch view |
| T087 | Ubuntu `.deb` package (`rust/packaging/build-deb.sh`) |
| T052, T055, T064, T079-T082 | Review fixes and load-flaky tests hardened |

## 2. Release notes (for people)

**`easy-personal-rs` 0.3.0. Linux (Ubuntu) only.** The Rust app now does everything the Python app does, and it is the default on the owner's machine. It reads and writes the same store (`~/.easy-personal/store.sqlite`, schema v11).

*Features*
- **Full dashboard and MCP server.** Every page and route of the Python dashboard, all MCP tools and the `investigate` prompt (T040-T048, T051).
- **Connectors.** Connector secrets stay in the OS keychain; the `easy-personal-launch` launcher starts them; `connector` and `doctor` commands (T050, T060).
- **Live view.** Live sessions over `/api/live` (T049, T056).
- **Watch and tray.** Machine status, 5 h / 7 d plan-limit watch, alerts with desktop notifications, effectiveness numbers, a Watch page with 24 h graphs, and `easy-personal-rs tray` with a tray icon and autostart (T072-T077, T083-T084).
- **Store health.** `store-check` (and `--full`) (T059, T086).
- **Copy command.** Each repo row has a Copy button for a ready `cd -- '<repo>' && <command>` line (HUN-245).
- **Ubuntu package.** `rust/packaging/build-deb.sh` builds `easy-personal-rs_0.3.0_amd64.deb`. It holds files only; each user runs `schedule install`, `claude mcp add` and `tray --autostart on` themselves (T087).

*Fixes*
- Flaky tests under load fixed (T064, T079-T082); review fixes T052-T055, T078; the alert engine never blocks server start on a hung D-Bus (T084).

*Known limits*
- Ubuntu only; no macOS or Windows build.
- The tray icon needs a tray host: on Ubuntu GNOME, the "Ubuntu AppIndicators" extension. Without it the tray prints `no tray area (no StatusNotifier host)`; alerts still work.
- `store-check` without `--full` may not finish on a large store (about 500 MB); use `--full` (T086).
- The `.desktop` entries and the window need `libwebkit2gtk-4.1-0` and `libgtk-3-0t64` (the package pulls them in).
- The Python app is still in the repo (T063 removes it after the owner's gate).

*Internal*: the leak gate covers the alert surface (T085); parity run over every Python command and route (T061).

## 3. Smoke test (defined BEFORE the deploy step)

Run on the built package, in temp folders, never the real store:

```bash
rust/packaging/build-deb.sh           # PASS: exit 0, "built: .../rust/target/deb/easy-personal-rs_0.3.0_amd64.deb"
rust/packaging/test_deb.sh            # PASS: last line "PASS: ...easy-personal-rs_0.3.0_amd64.deb", exit 0
cd rust && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check   # PASS: exit 0
cargo test --workspace                # PASS: 0 failed (run with a temp HOME plus the real CARGO_HOME/RUSTUP_HOME)
```

`test_deb.sh` checks: `--version` prints `easy-personal-rs 0.3.0`; `doctor` names the packaged launcher; `dashboard` serves `GET /` 200 from the packaged page; the file list, modes, owner and `Depends`.

*Run by the Supervisor on 2026-11-15 on branch `release-rs-0.3.0`: results are in the Release Log row of `RUNBOOK.md`.*

## 4. Deployment plan (operator)

**Pre-deploy checks**
1. `main` holds the merge of `release-rs-0.3.0`; that commit is the approved one.
2. Every in-scope card is Done on `rust/PROJECT_KANBAN.md` with its `tasks/TASK_REVIEW_Txxx.md`.
3. The smoke test passes.
4. Back up the store: `easy-personal-rs store-check --full` (PASS: healthy), then the WAL-safe backup of `rust/docs/CUTOVER.md` Step 0.3 into `~/.easy-personal/store.sqlite.before-0.3.0`.

**Ordered steps — pick one install method**
- **A. Cutover install (owner's machine today):** `cargo install --locked --path rust/crates/app --features window --target-dir rust/target --root ~/.local/share/easy-personal-rs`, then copy the built page: `npm --prefix web run build` and replace `~/.local/share/easy-personal-rs/static` with `src/easy_personal/dashboard/static`. Restart the tray (stop it by its PID, start `easy-personal-rs tray --static-dir ~/.local/share/easy-personal-rs/static`). *Reverse:* reinstall the previous commit the same way.
- **B. Package:** `sudo apt install ./rust/target/deb/easy-personal-rs_0.3.0_amd64.deb`, then the per-user steps in `rust/packaging/README.md` (MCP registration and timer with `/usr/bin/easy-personal-rs`; keep only one of A or B registered). *Reverse:* `sudo apt remove easy-personal-rs`, then re-register A's path.

**Publish (operator only, on the owner's word):**
`git tag -a rust-v0.3.0 <approved commit> -m "easy-personal-rs 0.3.0"`; `git push github main rust-v0.3.0`; with `gh` logged in: `gh release create rust-v0.3.0 rust/target/deb/easy-personal-rs_0.3.0_amd64.deb --title "easy-personal-rs 0.3.0 (Rust, Ubuntu)" --notes-file docs/release/rust-v0.3.0.md`. Run the secret scan of the pushed range first. *Reverse:* `gh release delete rust-v0.3.0 --yes`; `git push github :refs/tags/rust-v0.3.0`; `git tag -d rust-v0.3.0`.

**Post-deploy health check** (the pass condition is the gate)

| Check | Command | Pass condition |
|---|---|---|
| Version | `easy-personal-rs --version` | `easy-personal-rs 0.3.0` |
| Store | `easy-personal-rs store-check --full` | healthy, exit 0 |
| Doctor | `easy-personal-rs doctor` | exit 0; the launcher line names the launcher beside the installed binary |
| Dashboard | the tray's or `dashboard`'s URL, `curl -s -o /dev/null -w '%{http_code}'` | `200` |
| MCP | `claude mcp list` in each account that has it | `easy-personal` Connected |
| Timer | `systemctl --user list-timers easy-personal-ingest.timer` | listed, next run set |

## 5. Rollback plan

**Trigger (any one):** a health-check line fails; `store-check --full` is not healthy; the MCP server does not connect; the timer's run fails (`systemctl --user status easy-personal-ingest.service`).

**Reverse steps, in order**
1. Reinstall the previous Rust build: A — `cargo install` from the commit before the merge; B — `sudo apt remove easy-personal-rs`.
2. If the store is suspect: stop every `easy-personal*` process (by PID), restore `~/.easy-personal/store.sqlite.before-0.3.0` as `rust/docs/CUTOVER.md` "Rollback" says, run `easy-personal-rs store-check --full`.
3. Full return to Python: `rust/docs/CUTOVER.md` "Rollback" (notes in `~/easy-personal-cutover-notes/`).
4. Undo the publication: the three commands under *Publish — Reverse*.
5. Nothing to migrate down: the schema stays v11.

## 6. Pre-flight GO gate (recommendation)

| Gate | State |
|---|---|
| Stage 5 evidence green for every in-scope task | Yes: all cards Done with review files (see the board) |
| Rollback plan with triggers and exact reverse steps | Yes (section 5) |
| Post-deploy health check with a concrete pass condition | Yes (section 4) |
| Deploy command known | Yes: cutover install (T062) and the package (T087) |
| Production approval recorded for the version and commit | Asked after the merge; recorded in the `RUNBOOK.md` Release Log row |

## 7. Risks to watch

- Two installs at once (A and B): `~/.local/bin` comes first on `PATH`. Keep one MCP registration and one timer.
- The tray icon is invisible without a tray host (GNOME AppIndicators extension).
- The week of use (T062) runs to about 2026-11-22: T063 stays out of this release.
