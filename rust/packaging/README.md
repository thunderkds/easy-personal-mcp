# Ubuntu package (.deb)

`build-deb.sh` builds `easy-personal-rs_<version>_amd64.deb` from this repo. The package holds files only. It runs no script when you install or remove it, and it changes nothing in your home folder: no MCP registration, no timer, no store, no autostart entry.

`<version>` is the crate version (`0.2.0` today). The script reads it from `cargo metadata`.

## Build

From the repo root, on Ubuntu 24.04 or newer with Rust stable, Node and `dpkg-dev` installed:

```bash
rust/packaging/build-deb.sh
```

It runs `npm --prefix web ci && npm --prefix web run build`, then `cargo build --release --locked -p easy-personal-rs --features window`, lays out a staging folder, finds the library dependencies with `dpkg-shlibdeps`, and runs `dpkg-deb --build --root-owner-group`. The first build takes several minutes. It needs no root and installs nothing.

Output: `rust/target/deb/easy-personal-rs_0.2.0_amd64.deb` (the folder is git-ignored). Set `EP_DEB_MAINTAINER="Name <mail>"` to change the `Maintainer` field.

Check the result: `rust/packaging/test_deb.sh`. It is offline and works in temp folders. It lists the files, modes and owner, the `Depends` line and the `.desktop` files, runs the unpacked program, and proves the check fails when one file is missing.

## What the package holds

| Path | What |
|---|---|
| `/usr/bin/easy-personal-rs` | The program: window, tray, CLI, MCP server |
| `/usr/bin/easy-personal-launch` | The connector launcher. It sits beside the program, so the program finds it |
| `/usr/share/easy-personal-rs/static/` | The built dashboard page |
| `/usr/share/applications/easy-personal.desktop` | Menu entry that opens the window |
| `/usr/share/applications/easy-personal-tray.desktop` | Menu entry that starts the tray |
| `/usr/share/icons/hicolor/{32x32,128x128,256x256}/apps/easy-personal.png` | Icons |
| `/usr/share/doc/easy-personal-rs/` | `copyright`, `changelog.gz` |

Both `.desktop` files call `/usr/bin/easy-personal-rs` with `--static-dir /usr/share/easy-personal-rs/static`. They use the full path, so the menu always starts the packaged program, whatever comes first on your `PATH`.

`Depends` comes from `dpkg-shlibdeps` (GTK 3, WebKitGTK 4.1 and the C library) plus `libayatana-appindicator3-1`. The tray loads that library at run time, so `dpkg-shlibdeps` cannot see it.

## Install and remove

```bash
sudo apt install ./rust/target/deb/easy-personal-rs_0.2.0_amd64.deb
sudo apt remove easy-personal-rs
```

`apt` installs the listed libraries on its own. Remove deletes only the files above. Your store (`~/.easy-personal`), timer, MCP registration and autostart entry stay as they were, because the package never made them.

## After the install: per-user steps

Run these as your own user. The package does none of them.

```bash
easy-personal-rs --version
easy-personal-rs schedule install                                    # daily ingest timer (systemd user timer)
claude mcp add --scope user easy-personal -- /usr/bin/easy-personal-rs mcp
easy-personal-rs tray --static-dir /usr/share/easy-personal-rs/static --autostart on   # optional: tray at login
```

The tray icon needs a tray host. On Ubuntu GNOME that is the "Ubuntu AppIndicators" extension: if the tray prints `no tray area (no StatusNotifier host)`, run `gnome-extensions enable ubuntu-appindicators@ubuntu.com` and log in again. Alerts work without it.

For a second Claude account, put `CLAUDE_CONFIG_DIR="$HOME/.claude-work"` (its folder) in front of the `claude` command. The dashboard needs the page folder: `easy-personal-rs dashboard --static-dir /usr/share/easy-personal-rs/static`, or use the menu entry.

## Relation to the cutover install

The cutover (`rust/docs/CUTOVER.md`) put the program in `~/.local/share/easy-personal-rs` and linked `~/.local/bin/easy-personal-rs` to it. The package does not touch that install. Both can exist at once.

- `~/.local/bin` comes before `/usr/bin` on `PATH`, so a bare `easy-personal-rs` still runs the cutover copy.
- `/usr/bin/easy-personal-rs` runs the packaged copy. It finds `/usr/bin/easy-personal-launch` beside itself.
- To move to the package, register it again with the full path: `claude mcp remove easy-personal --scope user`, then `claude mcp add --scope user easy-personal -- /usr/bin/easy-personal-rs mcp`; and run `/usr/bin/easy-personal-rs schedule install` so the timer starts the packaged program. Check with `/usr/bin/easy-personal-rs doctor`.
- Both programs use the same store. Keep one timer and one MCP registration, so only one program writes.

## Not built

`.dmg`, `.exe` or `.msi`, an AppImage, an apt repository, package signing and CI release builds.
