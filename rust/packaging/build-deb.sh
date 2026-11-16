#!/usr/bin/env bash
# T087: builds rust/target/deb/easy-personal-rs_<version>_amd64.deb from this repo.
#
#   rust/packaging/build-deb.sh            build on this computer (installs on this Ubuntu version or newer)
#   rust/packaging/build-deb.sh --docker   T093: build the programs in an Ubuntu 22.04 container (installs on 22.04, 24.04, 26.04)
#
# Steps: build the dashboard page, build the release programs with the window, lay out a staging folder, run
# `dpkg-shlibdeps`, write DEBIAN/control, `dpkg-deb --build --root-owner-group`. It installs nothing and needs no root.
# The version is the crate version from `cargo metadata`. Files only: the package has no maintainer scripts.
# Override the Maintainer field with EP_DEB_MAINTAINER.
#
# `--docker` runs this same script a second time inside the image of `rust/packaging/Dockerfile` (`--in-container`, an
# internal flag): the dashboard page is built on the host first, and the container does the cargo build, the staging,
# `dpkg-shlibdeps` and `dpkg-deb`. It needs Docker and no sudo. Both builds write the same file name; the one that ran
# last is in rust/target/deb.
set -eu

mode=host
case "${1:-}" in
"") ;;
--docker) mode=docker ;;
--in-container) mode=container ;;
*)
    echo "usage: build-deb.sh [--docker]" >&2
    exit 2
    ;;
esac

here=$(cd "$(dirname "$0")" && pwd)
rust_dir=$(dirname "$here")
repo=$(dirname "$rust_dir")
out=$rust_dir/target/deb
stage=$out/stage
maintainer=${EP_DEB_MAINTAINER:-thunderkds <thunderkds@users.noreply.github.com>}
static_src=$repo/src/easy_personal/dashboard/static

if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi

if [ "$mode" = docker ]; then
    command -v docker >/dev/null 2>&1 || { echo "build-deb: --docker needs docker, and it is not installed" >&2; exit 1; }
    docker info >/dev/null 2>&1 || { echo "build-deb: --docker needs the docker daemon, and it is not reachable" >&2; exit 1; }
fi

version=$(cargo metadata --no-deps --format-version 1 --manifest-path "$rust_dir/Cargo.toml" |
    python3 -I -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "easy-personal-rs"))')
[ -n "$version" ] || { echo "build-deb: cannot read the crate version" >&2; exit 1; }
deb=$out/easy-personal-rs_${version}_amd64.deb

if [ "$mode" != container ]; then
    echo "== dashboard page"
    npm --prefix "$repo/web" ci
    npm --prefix "$repo/web" run build
fi
[ -f "$static_src/index.html" ] || { echo "build-deb: $static_src/index.html is missing after the web build" >&2; exit 1; }

if [ "$mode" = docker ]; then
    echo "== Ubuntu 22.04 build image"
    image=easy-personal-deb-jammy
    docker build -q -t "$image" "$here" >/dev/null
    echo "== container build (as user $(id -u):$(id -g), target folder rust/target/jammy)"
    # The changelog date comes from the host: inside the container a worktree's .git file points outside the mount.
    stamp=${EP_DEB_STAMP:-$(git -C "$repo" log -1 --format=%ct 2>/dev/null || date +%s)}
    mkdir -p "$out"
    docker run --rm --user "$(id -u):$(id -g)" -v "$repo:/work" -w /work \
        -e HOME=/tmp -e CARGO_HOME=/work/rust/target/jammy-cargo -e RUSTUP_TOOLCHAIN=stable \
        -e EP_DEB_STAMP="$stamp" -e EP_DEB_MAINTAINER="$maintainer" \
        "$image" rust/packaging/build-deb.sh --in-container
    [ -f "$deb" ] || { echo "build-deb: the container did not write $deb" >&2; exit 1; }
    exit 0
fi

echo "== release programs (window feature)"
target=$rust_dir/target/release
if [ "$mode" = container ]; then
    # Never share rust/target/release with the host build: its build scripts are linked against the host's glibc.
    export CARGO_TARGET_DIR=$rust_dir/target/jammy
    target=$CARGO_TARGET_DIR/release
fi
cargo build --release --locked --manifest-path "$rust_dir/Cargo.toml" -p easy-personal-rs --features window
for p in easy-personal-rs easy-personal-launch; do
    [ -x "$target/$p" ] || { echo "build-deb: $target/$p was not built" >&2; exit 1; }
done

echo "== staging tree"
rm -rf "$stage"   # only ever inside rust/target/deb
mkdir -p "$stage/DEBIAN" "$stage/usr/bin" "$stage/usr/share/easy-personal-rs" "$stage/usr/share/applications" \
    "$stage/usr/share/doc/easy-personal-rs"
for p in easy-personal-rs easy-personal-launch; do
    install -m 0755 "$target/$p" "$stage/usr/bin/$p"
    strip --strip-unneeded "$stage/usr/bin/$p"
done
cp -r "$static_src" "$stage/usr/share/easy-personal-rs/static"
# The Vite build leaves a `.web-stamp` file; the server never serves a dotfile, so it stays out.
find "$stage/usr/share/easy-personal-rs/static" -mindepth 1 -name '.*' -delete
install -m 0644 "$here/easy-personal.desktop" "$here/easy-personal-tray.desktop" "$stage/usr/share/applications/"
icons=$rust_dir/crates/app/icons
for pair in "32x32:32x32.png" "128x128:128x128.png" "256x256:128x128@2x.png"; do
    dir=$stage/usr/share/icons/hicolor/${pair%%:*}/apps
    mkdir -p "$dir"
    install -m 0644 "$icons/${pair#*:}" "$dir/easy-personal.png"
done
install -m 0644 "$here/copyright" "$stage/usr/share/doc/easy-personal-rs/copyright"
# The changelog date is the last commit's date, so two builds of one commit agree.
stamp=${EP_DEB_STAMP:-$(git -C "$repo" log -1 --format=%ct 2>/dev/null || date +%s)}
{
    printf 'easy-personal-rs (%s) unstable; urgency=low\n\n' "$version"
    printf '  * Ubuntu package of the Rust app: programs, dashboard page, desktop entries, icons.\n\n'
    printf ' -- %s  %s\n' "$maintainer" "$(date -R -d "@$stamp")"
} | gzip -9n >"$stage/usr/share/doc/easy-personal-rs/changelog.gz"
chmod 0644 "$stage/usr/share/doc/easy-personal-rs/changelog.gz"
# Whatever umask the build ran with: folders 0755, files 0644 (the programs are set again below).
find "$stage" -type d -exec chmod 0755 {} +
find "$stage/usr/share" -type f -exec chmod 0644 {} +
chmod 0755 "$stage/usr/bin/easy-personal-rs" "$stage/usr/bin/easy-personal-launch"

echo "== Depends (dpkg-shlibdeps)"
# `dpkg-shlibdeps` wants a debian/control in the working folder: make a throw-away one.
shlib=$(mktemp -d "$out/shlibdeps-XXXXXX")
trap 'rm -rf "$shlib"' EXIT
mkdir "$shlib/debian"
printf 'Source: easy-personal-rs\n\nPackage: easy-personal-rs\nArchitecture: amd64\n' >"$shlib/debian/control"
depends=$(cd "$shlib" && dpkg-shlibdeps -O -e"$stage/usr/bin/easy-personal-rs" -e"$stage/usr/bin/easy-personal-launch" |
    sed -n 's/^shlibs:Depends=//p')
[ -n "$depends" ] || { echo "build-deb: dpkg-shlibdeps found no dependency" >&2; exit 1; }
# The tray library is loaded at run time (dlopen), so dpkg-shlibdeps cannot see it.
depends="$depends, libayatana-appindicator3-1"

echo "== control"
size=$(du -sk --exclude=DEBIAN "$stage" | cut -f1)
cat >"$stage/DEBIAN/control" <<CONTROL
Package: easy-personal-rs
Version: $version
Architecture: amd64
Maintainer: $maintainer
Installed-Size: $size
Depends: $depends
Section: utils
Priority: optional
Homepage: https://github.com/thunderkds/easy-personal-mcp
Description: Usage and kit dashboard for Claude Code (Rust app)
 One program with a desktop window, a tray icon, a CLI and an MCP server.
 It reads Claude Code usage metadata on this computer only, and keeps connector
 secrets in the OS keychain. The package holds files only: it changes no user
 setting, timer or MCP registration.
CONTROL
chmod 0644 "$stage/DEBIAN/control"

echo "== dpkg-deb"
rm -f "$deb"
dpkg-deb --build --root-owner-group "$stage" "$deb"
echo "built: $deb"
