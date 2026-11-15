# TASK_GUIDE — T087: Ubuntu .deb package of the Rust app (files only; the owner installs it)
**Date**: 2026-11-15
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
**Assigned agent**: common-infrastructure
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Read `rust/docs/CUTOVER.md` (Step 1 install layout, the launcher beside the real binary), `rust/crates/app/Cargo.toml` (`[features] window`, the two `[[bin]]`), `rust/crates/app/tauri.conf.json`, `rust/crates/app/icons/README.md`
6. Read `rust/crates/app/src/claude_cli.rs` `launcher()` and `rust/crates/app/src/tray.rs` `--autostart` (which `Exec=` path it writes)
- **Never run anything against the owner's real setup**: no `sudo`, no `dpkg -i`/`apt install` of the package, no write under `/usr`, `~/.local`, `~/.config`, `~/.easy-personal`, `~/.claude*`; no `systemctl --user`, no `claude mcp add/remove`. Every real-run check uses `dpkg-deb -x` into a temp folder and temp `HOME`/`XDG_*`/`EASY_PERSONAL_HOME`. Never run `real_keychain_items_cross_check` or any `#[ignore]` keychain test. Do not touch `~/.local/share/easy-personal-rs` (the owner's live install).

---

## Requirement

**Original request** (owner, 2026-11-15): "do we have the job to package the app and build to the application like deb, dmg, or exe", then "start with the .deb first".

**Restated intent**:
> 1. One script, `rust/packaging/build-deb.sh`, builds `easy-personal-rs_<version>_amd64.deb` from this repo with no new tool and no new crate: `npm --prefix web ci && npm --prefix web run build`, `cargo build --release --locked -p easy-personal-rs --features window`, a staging tree, `dpkg-deb --build --root-owner-group`. Output goes to `rust/target/deb/` (git-ignored). `<version>` is the crate version (`0.2.0`), read from `cargo metadata`, never typed twice.
> 2. Package layout (⚠ decided for the owner, Supervisor 2026-11-15):
>    - `/usr/bin/easy-personal-rs` and `/usr/bin/easy-personal-launch` (real files side by side, so `launcher()` finds the Rust launcher beside the binary).
>    - `/usr/share/easy-personal-rs/static/` (the built dashboard page).
>    - `/usr/share/applications/easy-personal.desktop` (opens the window: `Exec=easy-personal-rs app --static-dir /usr/share/easy-personal-rs/static`) and `/usr/share/applications/easy-personal-tray.desktop` (`Exec=easy-personal-rs tray --static-dir /usr/share/easy-personal-rs/static`).
>    - `/usr/share/icons/hicolor/{32x32,128x128,256x256}/apps/easy-personal.png` from `rust/crates/app/icons/`.
>    - `/usr/share/doc/easy-personal-rs/copyright` and `changelog.gz` (Debian policy minimum).
> 3. `DEBIAN/control`: `Package: easy-personal-rs`, `Version`, `Architecture: amd64`, `Maintainer`, `Section: utils`, `Priority: optional`, `Description`, `Installed-Size`, and `Depends:` computed by `dpkg-shlibdeps` on both binaries **plus** `libayatana-appindicator3-1` (the tray library is loaded at run time, so `dpkg-shlibdeps` cannot see it). Files only: **no `postinst`/`prerm` scripts**. A system package runs as root and must not touch a user's MCP registration, timer, store or autostart; each user runs `easy-personal-rs schedule install`, `claude mcp add …` and `tray --autostart on` themselves.
> 4. `rust/packaging/README.md`: how to build, what the package holds, how to install (`sudo apt install ./rust/target/deb/easy-personal-rs_0.2.0_amd64.deb`) and remove (`sudo apt remove easy-personal-rs`), and the per-user steps after the install. It says how this relates to the current cutover install in `~/.local/share/easy-personal-rs` (the two can co-exist; `~/.local/bin` comes first on `PATH`; moving from one to the other means re-running the `claude mcp add` and `schedule install` with `/usr/bin/easy-personal-rs`). Do not edit `rust/README.md` or `rust/docs/CUTOVER.md` (other tasks edit them).
> 5. A test that the package is right, offline, in temp folders: `rust/packaging/test_deb.sh` (or a Rust integration test, your choice; say which and why) builds the .deb, then checks with `dpkg-deb -I`/`-c`/`-x`: the file list above exactly; modes (binaries 0755, others 0644, owner root); no file outside `/usr`; `Depends` holds `libwebkit2gtk-4.1-0`, `libgtk-3-0` (any `t64` form) and `libayatana-appindicator3-1`; both `.desktop` files pass `desktop-file-validate` if it is installed (skip with a printed note if not); the extracted `usr/bin/easy-personal-rs --version` prints `easy-personal-rs 0.2.0`; with temp `HOME`, `XDG_CONFIG_HOME` and `EASY_PERSONAL_HOME`, the extracted binary's `doctor` names the launcher at the extracted `usr/bin/easy-personal-launch`, and `dashboard --no-ingest --static-dir <extracted>/usr/share/easy-personal-rs/static` serves `GET /` 200 (stopped by its PID). It fails when a listed file is missing (show this once by deleting one from the staging tree).

**Out of scope**: `.dmg`, `.exe`/`.msi`, AppImage, an apt repository or signing, CI release builds, installing the package on this machine (the owner does it), changing Rust behaviour (`--static-dir` stays required: the `.desktop` files pass it), T063.

### Requirement Fidelity Gate
- [x] Restated intent agreed: owner 2026-11-15 "start with the .deb first"; layout and "files only" decided by the Supervisor (⚠ the owner may change them); scope agreed with the other supervisor (`rust-57`): new files under `rust/packaging/` only
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: NFR-001, NFR-002, NFR-006 in `PRD.md`; `rust/docs/PYTHON_RETIREMENT_PLAN.md` (packaging listed as not done)

---

## Dependencies & Reachability
**Depends on**: T062 (merged), T076 (merged)
**Entry point**: `rust/packaging/build-deb.sh`

## Dependency rule
No new crate, no new npm package, no new system tool (`dpkg-deb`, `dpkg-shlibdeps` are already installed; `desktop-file-validate` only if present).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `rust/packaging/build-deb.sh` builds `rust/target/deb/easy-personal-rs_0.2.0_amd64.deb`, exit 0; the version comes from `cargo metadata` | intent 1 |
| 2 | `dpkg-deb -c` lists exactly the intent-2 files; modes and owner as stated; nothing outside `/usr` | intent 2 |
| 3 | `dpkg-deb -I`: control fields as stated; `Depends` holds webkit2gtk-4.1, gtk-3 and `libayatana-appindicator3-1`; no maintainer scripts | intent 3 |
| 4 | Extracted binary: `--version` prints `easy-personal-rs 0.2.0`; `doctor` (temp `HOME`) names `<extract>/usr/bin/easy-personal-launch`; `dashboard --no-ingest --static-dir <extract>/usr/share/easy-personal-rs/static` gives `GET /` 200 | intents 2, 5 |
| 5 | The package test passes, and fails when one listed file is removed from the staging tree | intent 5 |
| 6 | Both `.desktop` files valid (`desktop-file-validate`, or a printed skip note) | intent 2 |
| 7 | `rust/packaging/README.md` covers build, install, remove, the per-user steps and the relation to the cutover install | intent 4 |
| 8 | `cargo test --workspace`, clippy `-D warnings`, `cargo fmt --check` still pass; `shellcheck` on the scripts if installed | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
rust/packaging/build-deb.sh && rust/packaging/test_deb.sh && cd rust && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

### Evidence (filled by reviewer at Stage 4/5)
> Moved to `tasks/TASK_REVIEW_T087.md`.

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T087.md`. BEFORE: `ls rust/packaging` (absent) and `"bundle": { "active": false }` in `tauri.conf.json`, captured before the first commit.

## UI / Design Acceptance Criteria
N/A — no page change; the `.desktop` entries reuse the existing icons.

---

## Approach
Test first: write the package test against a missing script (RED), then the build script, then the README. Plain POSIX `sh` or `bash` with `set -eu`; quote every path. Smallest script that works.

## Edge Case Checklist
- The web build writes into `src/easy_personal/dashboard/static` (git-ignored): the script copies from there and fails loudly if `index.html` is missing
- `cargo` not on `PATH`: the script sources `"$HOME/.cargo/env"` only if `cargo` is missing
- A stale staging folder from an earlier run: removed first (only inside `rust/target/deb/`)
- `dpkg-shlibdeps` needs a `debian/control` in the working folder: run it inside a temp folder made for it

## Files to Change (Predicted)
`rust/packaging/build-deb.sh` (new), `rust/packaging/test_deb.sh` (new), `rust/packaging/README.md` (new), `rust/packaging/easy-personal.desktop` and `rust/packaging/easy-personal-tray.desktop` (new), `rust/packaging/copyright` (new), `tasks/TASK_REVIEW_T087.md`

## Files Must NOT Touch
`rust/README.md`, `rust/docs/CUTOVER.md`, `rust/crates/**/src/**`, `rust/crates/app/Cargo.toml`, `rust/crates/app/tauri.conf.json`, `web/**`, `src/easy_personal/**`, `PRD.md`, any `PROJECT_KANBAN.md`, `memory/**`, `.claude/**`

## Test Plan
AC1-AC6 through the package test with pasted output; AC7 by reading; AC8 the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T087.md`
- [ ] Commits on branch `t087-deb` only; final report with `Status:`
