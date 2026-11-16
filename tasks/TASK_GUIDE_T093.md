# TASK_GUIDE — T093: one .deb that installs on Ubuntu 22.04, 24.04 and 26.04 (build it in an Ubuntu 22.04 container)
**Date**: 2026-11-16
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: common-infrastructure
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Read `rust/packaging/build-deb.sh`, `rust/packaging/test_deb.sh`, `rust/packaging/README.md`, `tasks/TASK_GUIDE_T087.md` (the package this task changes the build of)
- **Never run anything against the owner's real setup**: no `sudo`, no `dpkg -i`/`apt install` of the package on this machine, no write under `/usr`, `~/.local`, `~/.config`, `~/.easy-personal`, `~/.claude*`; no `systemctl --user`, no `claude mcp add/remove`. An install check runs only **inside a throw-away container** (`docker run --rm`). Never run `real_keychain_items_cross_check` or any `#[ignore]` keychain test. Do not touch `~/.local/share/easy-personal-rs` (the owner's live install). Do not remove or prune any Docker image, container or volume that this task did not create.

---

## Requirement

**Original request** (owner, 2026-11-16, `[admin]`): "checking this error from another machine easy-personal-rs : Depends: libc6 (>= 2.39) but 2.35-0ubuntu3.18 is to be installed / Depends: libglib2.0-0t64 (>= 2.66.0) but it is not installable / Depends: libgtk-3-0t64 (>= 3.21.5) but it is not installable".

**Diagnosis** (Supervisor, 2026-11-16): the other machine is Ubuntu 22.04 (`libc6 2.35-0ubuntu3.18`). `build-deb.sh` builds on the host, Ubuntu 26.04 (glibc 2.43), so `dpkg-shlibdeps` writes `libc6 (>= 2.39)`, `libglib2.0-0t64`, `libgtk-3-0t64` (`dpkg-deb -f rust/target/deb/easy-personal-rs_0.3.0_amd64.deb Depends`). Ubuntu 22.04 has glibc 2.35 and no `t64` package names, so `apt` refuses. The binary itself also needs glibc symbols newer than 2.35, so forcing the install would not help.

**Restated intent**:
> 1. A second way to build the same package: inside an **Ubuntu 22.04 container** (Docker), so the binaries link against glibc 2.35 and `dpkg-shlibdeps` writes Ubuntu 22.04 package names. On 24.04 and 26.04 the `t64` packages `Provide` the old names (`libglib2.0-0`, `libgtk-3-0`), so this one `.deb` installs on 22.04, 24.04 and 26.04.
> 2. The host build stays as it is: `rust/packaging/build-deb.sh` with no argument behaves exactly as today (same steps, same output path). The container build is opt-in (a flag or env of `build-deb.sh`, or a small sibling script that calls it; your choice, say which and why). It needs only Docker on the host beyond today's tools.
> 3. Package content does not change: same file list, modes, owner, control fields other than `Depends`, no maintainer scripts. `rust/packaging/test_deb.sh` (offline) must pass on the container-built package unchanged in what it checks, plus one new check that can tell the two builds apart when asked (see AC 3).
> 4. A real install proof, in throw-away containers: the container-built `.deb` installs with `apt-get install -y ./<deb>` on `ubuntu:22.04` and on `ubuntu:24.04`, and `easy-personal-rs --version` prints `easy-personal-rs <version>` there. This check needs the network, so it is its own script (not part of the offline `test_deb.sh`).
> 5. Docs: `rust/packaging/README.md` and the repo `README.md` install part say which Ubuntu versions the package installs on, how to make the container build, and that a host build installs only on the host's Ubuntu version or newer.

**Out of scope**: Ubuntu 20.04 and older (no WebKitGTK 4.1), Debian, arm64, `.dmg`/`.exe`/AppImage, an apt repository or signing, CI release builds, publishing a release, a crate version bump, any change under `rust/crates/**`, the landing page `docs/`.

### Requirement Fidelity Gate
- [x] Restated intent confirmed: auto (Supervisor), 2026-11-16, `[admin]`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `tasks/TASK_GUIDE_T087.md` (the package), `rust/packaging/README.md:9` "on Ubuntu 24.04 or newer" (the limit this task lifts for installs), `PRD.md` NFR-001, NFR-002

---

## Dependencies & Reachability
**Depends on**: T087 (merged), T089 (merged)
**Entry point**: `rust/packaging/build-deb.sh` (container mode)

## Dependency rule
No new crate, no new npm package. One new host tool, Docker, only for the container build and the install check; the host build and `test_deb.sh` need nothing new. Inside the image: Ubuntu 22.04 packages from the Ubuntu archive, Rust from `rustup` (stable, as `rust/rust-toolchain.toml`), nothing else from a third-party apt source unless the web build needs Node there (see Approach).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | The container build (the documented command) exits 0 and writes `easy-personal-rs_<version>_amd64.deb`; `<version>` still comes from `cargo metadata` | intent 1 |
| 2 | Its `Depends` (`dpkg-deb -f <deb> Depends`): `libc6 (>= V)` with V ≤ 2.35; no package name ending in `t64`; holds `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libglib2.0-0`, `libayatana-appindicator3-1` | intent 1 |
| 3 | `rust/packaging/test_deb.sh` passes on the container-built package (`DEB=<deb>`). A new opt-in check (for example `EP_DEB_EXPECT_BASE=jammy`) fails on a package whose `Depends` has `libc6 (>= 2.39)` or a `t64` name, and passes on the container-built one; the test proves both (negative case on a re-packed control file, offline) | intents 3, 1 |
| 4 | Install proof script: in `docker run --rm ubuntu:22.04` and `ubuntu:24.04`, `apt-get update && apt-get install -y ./<deb>` exits 0 and `easy-personal-rs --version` prints `easy-personal-rs <version>`; output pasted for both | intent 4 |
| 5 | `rust/packaging/build-deb.sh` with no argument or env: same commands as before this task (diff shows no change of the default path's steps or output file) | intent 2 |
| 6 | The container build leaves no root-owned file in the repo (`find rust web src -user root` prints nothing) and does not break the next host `cargo build` (separate target folder for the container) | intent 2 |
| 7 | `rust/packaging/README.md` and `README.md`: installs on Ubuntu 22.04, 24.04, 26.04 from the container build; the build command; the host-build limit | intent 5 |
| 8 | `shellcheck` on every script touched or added, if installed: exit 0. `cargo fmt --check` and clippy are not needed: no Rust file changes | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
rust/packaging/build-deb.sh --docker && DEB=$(ls rust/target/deb/easy-personal-rs_*_amd64.deb | tail -1) EP_DEB_EXPECT_BASE=jammy rust/packaging/test_deb.sh && rust/packaging/test_deb_install.sh
```
(The flag, env and script names above are the Supervisor's sketch. If you choose other names, write the exact command you used at the top of your final report and in `rust/packaging/README.md`.)

### Evidence (filled by reviewer at Stage 4/5)
> Moved to `tasks/TASK_REVIEW_T093.md`.

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T093.md`. BEFORE (capture before your first commit): `dpkg-deb -f rust/target/deb/easy-personal-rs_0.3.0_amd64.deb Depends` on the host-built package, and the `apt-get install` failure of that package in `docker run --rm ubuntu:22.04` (the owner's error, reproduced).

## UI / Design Acceptance Criteria
N/A — no page change.

---

## Approach
Test first: write the new `Depends` check and the install proof script, run them against the existing host-built package and see them fail for the right reason (RED: `libc6 (>= 2.39)`, `t64`). Then the container build. Then the docs.

Sketch (change it if you find a simpler one that meets the ACs):
- A small `rust/packaging/Dockerfile` (`FROM ubuntu:22.04`): `build-essential`, `curl`, `pkg-config`, `dpkg-dev`, `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `libssl-dev`, `librsvg2-dev`, `libdbus-1-dev` (check what `cargo build --features window` really needs), and `rustup` stable.
- The web page is plain static files, the same on every Ubuntu. `nodejs` from the Ubuntu 22.04 archive is too old for Vite. Simplest: build the web page on the host (as today), and run only `cargo build`, the staging, `dpkg-shlibdeps` and `dpkg-deb` in the container. If you put Node in the image instead, say why.
- Run the container as the host user (`--user "$(id -u):$(id -g)"`) with the repo mounted, `CARGO_TARGET_DIR` in a folder of its own (for example `rust/target/jammy`), and a `CARGO_HOME` that is not the owner's `~/.cargo` (a folder under `rust/target/` or a named volume this task creates).
- Keep one `build-deb.sh`: the staging and control steps must not be copied into a second script.

## Edge Case Checklist
- Build scripts and proc-macros compiled on the host (glibc 2.43) cannot run in the container: never share `rust/target/release` between the two builds
- `--locked` must hold in the container too: same `Cargo.lock`, no lock change
- `dpkg-shlibdeps` must run in the container (it reads the container's package database), not on the host
- Docker not installed or the daemon not running: the container build fails with one clear line, exit not 0; the host build is not affected
- The container-built `.deb` has the same file name as the host-built one: say in the README which one is in `rust/target/deb/` after each command (overwrite, or a second name; your choice)
- First image build downloads about 1 GB and takes minutes; a second run reuses the image
- `ubuntu:24.04` install: `libgtk-3-0` and `libglib2.0-0` resolve through `Provides` of the `t64` packages; if `apt` refuses, stop and report (`Status: blocked`) with the exact text

## Files to Change (Predicted)
`rust/packaging/build-deb.sh`, `rust/packaging/Dockerfile` (new), `rust/packaging/test_deb.sh`, `rust/packaging/test_deb_install.sh` (new), `rust/packaging/README.md`, `README.md` (install part only), `.gitignore` or `.dockerignore` only if needed, `tasks/TASK_REVIEW_T093.md`

## Files Must NOT Touch
`rust/crates/**`, `rust/Cargo.toml`, `rust/Cargo.lock`, `web/**`, `src/easy_personal/**`, `docs/**`, `PRD.md`, any `PROJECT_KANBAN.md`, `memory/**`, `.claude/**`, `rust/README.md` except a line that states the Ubuntu version

## Test Plan
AC 1, 2, 4: pasted command output. AC 3: `test_deb.sh` output, positive and negative. AC 5: `git diff main -- rust/packaging/build-deb.sh` read by the reviewer. AC 6: the `find` output and a host `cargo build --release --locked -p easy-personal-rs --features window` that still works. AC 7 by reading. AC 8: `shellcheck` output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T093.md`
- [ ] Commits on branch `T093` only; final report with `Status:`

## Questions Log
