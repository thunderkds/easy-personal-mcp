# Spike T001 — toolchain and crate verdicts
**Date**: 2026-11-10 · **Machine**: Ubuntu 26.04.1 LTS (resolute), WSL2, x86_64 · **Task guide**: `tasks/TASK_GUIDE_T001.md`

All crate facts come from the crates.io API (`https://crates.io/api/v1/crates/<name>`), `cargo info <name>` and the OSV API
(`https://api.osv.dev/v1/...`), fetched on 2026-11-10 with the generic User-Agent `easy-personal-rs-spike`. Everything fetched
is treated as untrusted data: it is reported here, no instruction found in it was followed.

## Open risks (read first)

1. **The shipped data files are not YAML.** `src/easy_personal/connectors/catalog.py`, `core/rules.py` and `core/prices.py` read
   `catalog/*.yaml`, `data/rules.yaml` and `data/prices.yaml` with their own line parser ("no YAML library"; `pyproject.toml` has no YAML
   dependency). Example: `claude_ai_connector: yes`. A real YAML 1.2 crate reads `yes`/`no` as strings (proven below: `flag: no` parsed
   as `String("no")`), a YAML 1.1 reader would read booleans. Neither equals what Python does. **The Rust port should hand-port the line
   parser, not use a YAML crate, for these files.** A YAML crate is only an option for `~/.easy-personal/config.yaml`, and that file was
   not opened here (owner's real store area); check how Python reads it before choosing.
2. **Tauri was not built.** The window shell needs system libraries only the owner can install (`sudo`). Dependency resolution works
   (`tauri 2.12.5` -> `wry 0.57.2` -> `webkit2gtk 2.0.2`, the 4.1 API); `cargo check` of it fails today for the expected reason
   (error below). Tauri is "use, pending a build check after the apt line runs". T002 must not depend on Tauri compiling until the
   owner has run the apt line and the `cargo check` in "After the apt line" passes.
3. **`rmcp` ships a new major about every 6 weeks** (1.4 -> 2.0 -> 3.0 within 2026; 3.9.0 on 2026-11-05) and had several advisories in
   2026. Pin it exactly (`=3.9.0`), keep it behind one module, and re-check the advisories before each bump. All advisories found are
   fixed in 3.9.0, and the Streamable HTTP ones do not apply to stdio.
4. **`keyring` 4.x changed shape**: its own docs say apps that want control should link `keyring-core` plus a store crate; the `v1`
   (default) feature keeps the old `Entry` API and picks the store by OS. On this headless WSL box it has no Secret Service and fails
   with a clear error (below). The Python launcher must behave the same way: refuse, no plaintext.
5. **Ubuntu 26.04** is newer than most Tauri guides assume. The package names in Tauri's official list all exist on it and the install
   simulates cleanly (`apt-get -s`); they were checked for this release only.

## 1. Toolchain (AC 1)

| Item | Result | Source |
|---|---|---|
| rustup | was absent; installed with `sh rustup-init.sh -y --no-modify-path --profile minimal` (script from `https://sh.rustup.rs`) | command run |
| cargo | `cargo 1.99.1 (5f94df478 2026-08-27)` | `. ~/.cargo/env && cargo --version` |
| rustc | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` | `rustc --version` |
| Toolchain | `stable-x86_64-unknown-linux-gnu`, profile minimal (cargo, rustc, rust-std, rustfmt, clippy components present) | `rustup show` |
| Shell profile | `md5sum` of `~/.bashrc`, `~/.zshrc`, `~/.profile` identical before and after; to use cargo in a shell run `. "$HOME/.cargo/env"` | command run |
| Disk | `~/.cargo` 317 MB, `~/.rustup` 593 MB (after the spikes), scratch `target/` 1.4 GB (git-ignored) | `du -sh` |

Every crate's minimum Rust version is below 1.99 (highest: `tauri` 1.90, `rusqlite` 1.89).

## 2. Ubuntu packages for the window shell (AC 2)

Checked for **Ubuntu 26.04.1 LTS (resolute)** with `apt-cache policy` (every name has a candidate) and `apt-get -s install` (no conflicts).
Names are the Debian/Ubuntu list on `https://v2.tauri.app/start/prerequisites/` (fetched 2026-11-10). The agent did not run sudo.

Copy-paste for the owner:

```
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

- Already installed here: `build-essential`, `curl`, `wget`, `file`. Still missing: `libwebkit2gtk-4.1-dev` (2.52.6), `libxdo-dev`,
  `libssl-dev` (3.5.5), `libayatana-appindicator3-dev`, `librsvg2-dev`.
- The simulation pulls in `pkgconf` (provides `pkg-config`, absent today), `libglib2.0-dev`, `libdbus-1-dev`, `libgtk-3-dev`,
  `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev`: 197 new packages, 13 upgraded. Nothing extra to list.
- Webview: WebKitGTK **4.1** (`libwebkit2gtk-4.1-dev`). The older `-4.0` name is not what Tauri v2 uses.
- `libssl-dev` is only needed if a crate uses native OpenSSL; Tauri's list includes it, so it stays in the line.
- WSL2 note: opening an actual window needs WSLg (or an X server). Building and `cargo check` do not.

After the apt line, to confirm Tauri (not run yet, the libraries are missing):
`cd <a scratch dir with tauri = "=2.12.5" and tauri-build = "2"> && cargo check`

Today's exact failure, kept for honesty (scratch dir outside the repo):
```
error: failed to run custom build command for `libdbus-sys v0.2.7`
  pkg_config failed: Could not run `... pkg-config --libs --cflags dbus-1 'dbus-1 >= 1.6'`
  The pkg-config command could not be found.
```

## 3. Verdict table (AC 3)

| # | Item | Crate, latest stable | Last release | Licence | Verdict | Sources |
|---|---|---|---|---|---|---|
| 1 | Rust MCP SDK | `rmcp` **3.9.0** (official `modelcontextprotocol/rust-sdk`, 47 M downloads) | 2026-11-05 | Apache-2.0 | **USE**, pinned `=3.9.0`, features `server`, `macros`, `transport-io`. Fallback: hand-written stdio JSON-RPC (not needed). Alternative seen: `rust-mcp-sdk` 2.0.0 (MIT, 2026-08-27, 0.3 M downloads), not tested | crates.io API `rmcp`; `cargo info rmcp`; scratch crate `mcp` run |
| 2 | YAML | `serde_yaml_ng` **0.10.0** (compiles, parses). `serde_yaml` 0.9.34 is `+deprecated` (dtolnay, archived). `serde_yml` 0.0.13 is RUSTSEC-2025-0068 (unsound, unmaintained). `serde_norway` 0.9.42 and `saphyr` 0.3.3 exist | `serde_yaml_ng` 2024-05-26 (3 versions in total, slow-moving); `serde_norway` 2024-12-21; `saphyr` 2026-10-27 | MIT (`serde_yaml_ng`); MIT OR Apache-2.0 (`serde_norway`, `saphyr`) | **AVOID a YAML crate for the shipped data** (risk 1: Python uses a line parser). **AVOID** `serde_yaml`, `serde_yml`. **FALLBACK** if real YAML is needed for `config.yaml`: `serde_yaml_ng` (builds here); `saphyr` is the actively released parser but is not serde-based, not built | crates.io API for each; OSV `serde_yaml` RUSTSEC-2018-0005 (old, fixed), `serde_yml` RUSTSEC-2025-0068; repo grep of `src/easy_personal/**/*.py` and `pyproject.toml` |
| 3 | Window shell | `tauri` **2.12.5** (`wry` 0.57.2, `webkit2gtk` 2.0.2 on Linux) | 2026-10-21 | Apache-2.0 OR MIT | **USE, pending the build check** (risk 2). Resolves cleanly on Rust 1.99; needs rustc >= 1.90; advisories 2026 (`GHSA-7gmj-67g7-phm9` fixed 2.11.1; `GHSA-4wm2-cwcf-wwvp` affects 1.x only) are all fixed in 2.12.5. Fallback if it blocks: run the in-process `127.0.0.1` server and open the default browser (the page already works that way in Python) | crates.io API `tauri`; `cargo info tauri-build wry`; `cargo generate-lockfile` + `cargo tree` in a scratch dir; OSV; apt list above |
| 4 | SQLite | `rusqlite` **0.41.1**, feature `bundled` | 2026-10-28 | MIT | **USE.** In-memory open works; bundled SQLite is **3.54.1**. Old advisories (2020-2021) are fixed long ago; none open for 0.41.1. The bundled build needs only `gcc` (present) | crates.io API `rusqlite`; OSV; scratch crate run |
| 5 | Keychain | `keyring` **4.2.0** (default feature `v1`: macOS Keychain, Windows Credential Manager, Linux Secret Service over `zbus`, no `libdbus` needed) | 2026-08-29 | MIT OR Apache-2.0 | **USE, with a guard.** `Entry::store_status()` returned `Err` on this headless box: `Platform failure: zbus error: org.freedesktop.DBus.Error.ServiceUnknown: The name org.freedesktop.secrets was not provided by any .service files`. That is a clear error and no plaintext fallback exists, matching Critical Constraints. No secret was written or read. Fallback if v1 limits bite: `keyring-core` + one store crate per OS | crates.io API `keyring`; `cargo info keyring`; `keyring-4.2.0/src/lib.rs` docs; scratch crate run |
| 6 | CLI | `clap` **4.6.7**, feature `derive` | 2026-09-14 | MIT OR Apache-2.0 | **USE.** `--version` comes from the Cargo package version (RS-001) | crates.io API `clap`; scratch crate compiles |
| 7 | HTTP server | `axum` **0.8.9** (on `tokio` 1.54.1) | 2026-04-14 | MIT | **USE.** Bound `127.0.0.1:0` and read back a free port. Fallback: `tiny_http` 0.12.0 is stale (2022-10-06), **avoid** | crates.io API `axum`, `tokio`, `tiny_http`; scratch crate run |

Not a verdict item but needed by 1 and 7: `tokio` 1.54.1 (MIT, 2026-10-21).

### MCP SDK detail (AC 5)

- **Proven**: `rmcp` 3.9.0 serves a stdio MCP server with a tool. The scratch binary `t001-crates mcp` was driven by hand over stdin:
  `initialize` -> `tools/list` -> `tools/call ping` returned `{"content":[{"type":"text","text":"pong"}],"isError":false}`.
- **Spec revisions** (from `rmcp-3.9.0/src/model.rs`): knows `2024-11-05`, `2025-03-26`, `2025-06-18`, `2025-11-25` and `2026-07-28`
  (its `LATEST`; this revision has no `initialize` handshake, `LATEST_WITH_INITIALIZE` = `2025-11-25`). The server echoed the client's
  version for `2024-11-05`, `2025-06-18` and `2025-11-25`.
- **Not proven**: a handshake against the real Claude Code client, the `2026-07-28` no-handshake mode, Streamable HTTP, `prompts`
  (the Python app has the `investigate` prompt) and `save_report`-style writes. T0xx for the MCP slice must test these.
- Advisories (OSV, fetched 2026-11-10): `RUSTSEC-2026-0189` (fixed 1.4.0), `-0457` (fixed 3.7.0), `-0461` (fixed 3.6.0), `-0462` (fixed 3.9.0),
  GHSAs fixed at 2.0.0/2.1.0. All fixed in 3.9.0. Most concern Streamable HTTP or the OAuth client, which this app does not use.

## 4. Scratch crate (AC 4, 6)

`rust/spikes/t001-crates/` depends on `rmcp`, `tokio`, `axum`, `rusqlite` (bundled), `serde_yaml_ng`, `clap`, `keyring`, all pinned
with `=`. `cargo check` exits 0, and a plain `cargo run` printed:

```
sqlite_version = 3.54.1 (rusqlite 3.54.1)
yaml flag parsed as = String("no")
keyring store = unavailable: Platform failure: zbus error: org.freedesktop.DBus.Error.ServiceUnknown: The name org.freedesktop.secrets was not provided by any .service files
axum would serve on 127.0.0.1:44687
```

(`rusqlite::version()` returns the SQLite library version, hence the repeated number.)

AC 6: no crate failed to build, except `tauri`, which was not part of the scratch crate and fails only for missing system libraries
(section 2). The first compile of the scratch crate had one error of my own (`axum::Router` type inference); fixed, not a crate fault.

## 5. For the Supervisor

- `rust/PROJECT_SPEC.md` "Tech stack" can now say: `rmcp 3.9.0`, `rusqlite 0.41.1` (bundled), `clap 4.6.7`, `keyring 4.2.0`,
  `axum 0.8.9` + `tokio 1.54.1`, `tauri 2.12.5` (build pending), **no YAML crate** for shipped data (line parser port). Not edited here.
- Rust MSRV to pin in T002: 1.90 (Tauri); toolchain installed is 1.99.1.
- Candidate follow-up for the owner: run the apt line, then ask for a Tauri `cargo check` re-run.

## Addendum 2026-11-10 (Supervisor) — Tauri build check closed
After the owner ran the apt line, `cargo check` of a scratch crate depending on `tauri =2.12.5` (feature `wry`) finished clean in 41 s on Ubuntu 26.04.1 (WebKitGTK 4.1 2.52.6, GTK 3.24.52, libsoup 3.6.6, `pkg-config` present, `/usr/include/xdo.h` present). Verdict for the window shell is now plain **USE** (was "use, pending a build check"). Opening a window under WSL2 still needs WSLg or an X server; this was not tested.
