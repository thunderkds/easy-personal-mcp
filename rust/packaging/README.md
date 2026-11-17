# Ubuntu package (.deb)

`build-deb.sh` builds `easy-personal-rs_<version>_amd64.deb` from this repo. The package holds files only. It runs no script when you install or remove it, and it changes nothing in your home folder: no MCP registration, no timer, no store, no autostart entry.

`<version>` is the crate version (`0.3.0` today). The script reads it from `cargo metadata`.

## Build

There are two builds. Both give the same package content; they differ in `Depends` only.

| Build | Command | Needs | Installs on |
|---|---|---|---|
| Container (one package for all) | `rust/packaging/build-deb.sh --docker` | Docker, Rust stable, Node | Ubuntu 22.04, 24.04 and 26.04 |
| Host | `rust/packaging/build-deb.sh` | Rust stable, Node, `dpkg-dev` | the Ubuntu version of the build computer, or newer |

The host build links against the glibc of the build computer, and `dpkg-shlibdeps` writes the package names of that Ubuntu version. Built on Ubuntu 26.04 it needs `libc6 (>= 2.39)`, `libglib2.0-0t64` and `libgtk-3-0t64`, so `apt` on 22.04 refuses it (`Depends: libc6 (>= 2.39) but 2.35-0ubuntu3.15 is to be installed`). The container build runs in `ubuntu:22.04`, so it needs `libc6 (>= 2.35)` and the old library names (`libglib2.0-0`, `libgtk-3-0`). On 24.04 and 26.04 the `t64` packages `Provide` those names, so the same package installs there. Ubuntu 20.04 and older have no WebKitGTK 4.1 and are not supported.

Both commands write `rust/target/deb/easy-personal-rs_<version>_amd64.deb`. The build that ran last is the file there. Build the package you want to install last, or copy it away.

The container build (`--docker`):

1. builds the dashboard page on the host (`npm`), as the host build does. The page is plain files and the same on every Ubuntu;
2. builds the image `easy-personal-deb-jammy` from `rust/packaging/Dockerfile` (Ubuntu 22.04, the dev libraries, Rust stable). The first image build downloads about 1 GB; later runs reuse the image. To get a newer Rust, run `docker rmi easy-personal-deb-jammy` and build again;
3. runs the same `build-deb.sh` inside the container (`--in-container`, an internal flag) as your own user: no root, and no root-owned file in the repo. The cargo build writes to `rust/target/jammy` and the crate cache to `rust/target/jammy-cargo`, so it never touches the host `rust/target/release` or `~/.cargo`.

It needs Docker and no `sudo` on the host. Without Docker it stops with `build-deb: --docker needs docker, and it is not installed`.

The host build runs `npm --prefix web ci && npm --prefix web run build`, then `cargo build --release --locked -p easy-personal-rs --features window`, lays out a staging folder, finds the library dependencies with `dpkg-shlibdeps`, and runs `dpkg-deb --build --root-owner-group`. The first build takes several minutes. It needs no root and installs nothing.

Output: `rust/target/deb/easy-personal-rs_0.3.0_amd64.deb` (the folder is git-ignored). Set `EP_DEB_MAINTAINER="Name <mail>"` to change the `Maintainer` field.

Check the result: `rust/packaging/test_deb.sh`. It is offline and works in temp folders. It lists the files, modes and owner, the `Depends` line and the `.desktop` files, runs the unpacked program, and proves the check fails when one file is missing.

To also require the Ubuntu 22.04 `Depends` (`libc6 (>= V)` with V up to 2.35, no `t64` name), set `EP_DEB_EXPECT_BASE=jammy`: `EP_DEB_EXPECT_BASE=jammy rust/packaging/test_deb.sh`. A host build made on 24.04 or newer fails that check. The test also proves, without the variable, that the check rejects a `Depends` with `libc6 (>= 2.39)` and `t64` names.

Install proof (needs Docker and the network, so it is not part of `test_deb.sh`): `rust/packaging/test_deb_install.sh`. In a throw-away `ubuntu:22.04` container and in an `ubuntu:24.04` container it runs `apt-get install -y ./<deb>` and `easy-personal-rs --version`. It installs nothing on your computer. `DEB=/path/x.deb` checks another file; `EP_INSTALL_IMAGES="ubuntu:22.04"` picks the images.

The full check of the container build:

```bash
rust/packaging/build-deb.sh --docker && EP_DEB_EXPECT_BASE=jammy rust/packaging/test_deb.sh && rust/packaging/test_deb_install.sh
```

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

`Depends` comes from `dpkg-shlibdeps` (GTK 3, WebKitGTK 4.1 and the C library; the names depend on the build, see Build) plus `libayatana-appindicator3-1`. The tray loads that library at run time, so `dpkg-shlibdeps` cannot see it.

## Install and remove

```bash
sudo apt install ./rust/target/deb/easy-personal-rs_0.3.0_amd64.deb
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

Opening the app (the `easy-personal` menu entry or `easy-personal-rs app`) starts the tray, which owns the one dashboard window and opens it (a second open shows that window); `app --no-tray` opens its own window and starts no tray, and `--autostart on` above only adds the tray at login. After upgrading, quit an older tray once so the new one can take over. The tray icon needs a tray host. On Ubuntu GNOME that is the "Ubuntu AppIndicators" extension: if the tray prints `no tray area (no StatusNotifier host)`, run `gnome-extensions enable ubuntu-appindicators@ubuntu.com` and log in again. Alerts work without it.

For a second Claude account, put `CLAUDE_CONFIG_DIR="$HOME/.claude-work"` (its folder) in front of the `claude` command. The dashboard needs the page folder: `easy-personal-rs dashboard --static-dir /usr/share/easy-personal-rs/static`, or use the menu entry.

## Relation to the cutover install

The cutover (`rust/docs/CUTOVER.md`) put the program in `~/.local/share/easy-personal-rs` and linked `~/.local/bin/easy-personal-rs` to it. The package does not touch that install. Both can exist at once.

- `~/.local/bin` comes before `/usr/bin` on `PATH`, so a bare `easy-personal-rs` still runs the cutover copy.
- `/usr/bin/easy-personal-rs` runs the packaged copy. It finds `/usr/bin/easy-personal-launch` beside itself.
- To move to the package, register it again with the full path: `claude mcp remove easy-personal --scope user`, then `claude mcp add --scope user easy-personal -- /usr/bin/easy-personal-rs mcp`; and run `/usr/bin/easy-personal-rs schedule install` so the timer starts the packaged program. Check with `/usr/bin/easy-personal-rs doctor`.
- Both programs use the same store. Keep one timer and one MCP registration, so only one program writes.

## Not built

Ubuntu 20.04 and older, Debian, arm64, `.dmg`, `.exe` or `.msi`, an AppImage, an apt repository, package signing and CI release builds.
