#!/usr/bin/env bash
# T087: checks the .deb that build-deb.sh makes. Offline; every real run happens in a temp folder.
#
#   rust/packaging/test_deb.sh            use rust/target/deb/easy-personal-rs_<version>_amd64.deb; build it first if absent
#   DEB=/path/x.deb rust/packaging/test_deb.sh   check that file instead
#
#   EP_DEB_EXPECT_BASE=jammy rust/packaging/test_deb.sh   also require the Ubuntu 22.04 `Depends` (T093): `libc6 (>= V)` with
#                                         V <= 2.35 and no `t64` package name, so the package installs on 22.04, 24.04 and 26.04
#
# It also proves the check can fail: it unpacks the package, deletes one listed file, packs it again and expects
# the same check to reject the copy.
#
# Why a shell test and not a Rust test: the package is files and `dpkg-deb` output. A Rust test would shell out to the
# same tools and add a cargo target for nothing (no new crate, no new tool).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
rust_dir=$(dirname "$here")
if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
version=$(cargo metadata --no-deps --format-version 1 --manifest-path "$rust_dir/Cargo.toml" |
    python3 -I -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "easy-personal-rs"))')
deb=${DEB:-$rust_dir/target/deb/easy-personal-rs_${version}_amd64.deb}
if [ ! -f "$deb" ]; then
    echo "no package at $deb: building it"
    "$here/build-deb.sh"
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/t087-test-XXXXXX")
server_pid=""
cleanup() {
    if [ -n "$server_pid" ]; then kill "$server_pid" 2>/dev/null || true; fi
    rm -rf "$work"
}
trap cleanup EXIT

static_root=usr/share/easy-personal-rs/static
# Every regular file of the package except the dashboard page (its files are checked as a group below).
expected_files="usr/bin/easy-personal-launch
usr/bin/easy-personal-rs
usr/share/applications/easy-personal-tray.desktop
usr/share/applications/easy-personal.desktop
usr/share/doc/easy-personal-rs/changelog.gz
usr/share/doc/easy-personal-rs/copyright
usr/share/icons/hicolor/128x128/apps/easy-personal.png
usr/share/icons/hicolor/256x256/apps/easy-personal.png
usr/share/icons/hicolor/32x32/apps/easy-personal.png"

failures=0
fail() {
    echo "FAIL: $*" >&2
    failures=$((failures + 1))
}

# check_package <deb>: the static checks (file list, modes, owner, control). Prints FAIL lines; returns 1 on any.
check_package() {
    local pkg=$1 listing
    failures=0
    listing=$(dpkg-deb -c "$pkg" | awk '{ f = $6; for (i = 7; i <= NF; i++) f = f " " $i; sub(/^\.\//, "", f); print $1 "\t" $2 "\t" f }')

    # nothing but regular files and directories, nothing outside /usr
    echo "$listing" | while IFS=$'\t' read -r mode owner path; do
        case $mode in
        -* | d*) ;;
        *) echo "FAIL: $path is not a regular file or folder ($mode)" >&2 ;;
        esac
    done | grep -q FAIL && fail "the package holds a link or a special file"
    echo "$listing" | awk -F'\t' '$3 != "" && $3 != "." && $3 !~ /^usr(\/|$)/ { print "FAIL: outside /usr: " $3 }' | grep FAIL >&2 && fail "a path is outside /usr"

    # exact file list
    local got want
    got=$(echo "$listing" | awk -F'\t' -v root="$static_root/" '$1 ~ /^-/ && index($3, root) != 1 { print $3 }' | LC_ALL=C sort)
    want=$(echo "$expected_files" | LC_ALL=C sort)
    if [ "$got" != "$want" ]; then
        fail "file list differs from the listed files:"
        diff <(echo "$want") <(echo "$got") >&2 || true
    fi
    echo "$listing" | awk -F'\t' -v root="$static_root/" '$1 ~ /^-/ && index($3, root) == 1 { n++ } END { exit n > 0 ? 0 : 1 }' ||
        fail "no file under $static_root"
    echo "$listing" | awk -F'\t' -v want="$static_root/index.html" '$1 ~ /^-/ && $3 == want { f = 1 } END { exit f ? 0 : 1 }' ||
        fail "$static_root/index.html is missing"

    # the server answers 404 for a dotfile, so none belongs in the page folder (Vite leaves a `.web-stamp`)
    echo "$listing" | awk -F'\t' -v root="$static_root/" 'index($3, root) == 1 && $3 ~ /\/\.[^\/]+$/ { print $3 }' | grep -q . &&
        fail "a dotfile is in $static_root"

    # modes and owner
    echo "$listing" | while IFS=$'\t' read -r mode owner path; do
        [ -n "$path" ] || continue
        want_mode=-rw-r--r--
        case $path in
        usr/bin/easy-personal-rs | usr/bin/easy-personal-launch) want_mode=-rwxr-xr-x ;;
        esac
        case $mode in d*) want_mode=drwxr-xr-x ;; esac
        [ "$mode" = "$want_mode" ] || echo "FAIL: $path mode $mode, want $want_mode" >&2
        [ "$owner" = root/root ] || echo "FAIL: $path owner $owner, want root/root" >&2
    done 2>&1 | grep FAIL >&2 && fail "a mode or an owner is wrong"

    # control
    local control
    control=$(dpkg-deb -f "$pkg")
    field() { echo "$control" | sed -n "s/^$1: //p"; }
    [ "$(field Package)" = easy-personal-rs ] || fail "Package is '$(field Package)'"
    [ "$(field Version)" = "$version" ] || fail "Version is '$(field Version)', want $version"
    [ "$(field Architecture)" = amd64 ] || fail "Architecture is '$(field Architecture)'"
    [ "$(field Section)" = utils ] || fail "Section is '$(field Section)'"
    [ "$(field Priority)" = optional ] || fail "Priority is '$(field Priority)'"
    for f in Maintainer Description Installed-Size; do
        [ -n "$(field "$f")" ] || fail "control field $f is empty"
    done
    local depends
    depends=$(field Depends)
    echo "$depends" | grep -Eq '(^|, )libwebkit2gtk-4\.1-0( |,|$)' || fail "Depends lacks libwebkit2gtk-4.1-0: $depends"
    echo "$depends" | grep -Eq '(^|, )libgtk-3-0(t64)?( |,|$)' || fail "Depends lacks libgtk-3-0 (or libgtk-3-0t64): $depends"
    echo "$depends" | grep -Eq '(^|, )libayatana-appindicator3-1( |,|$)' || fail "Depends lacks libayatana-appindicator3-1: $depends"
    # no maintainer scripts: the control archive holds `control` and `md5sums` only
    local extra
    extra=$(dpkg-deb --ctrl-tarfile "$pkg" | tar -t | sed 's|^\./||' | grep -Ev '^(control|md5sums|)$' || true)
    [ -z "$extra" ] || fail "control archive holds more than control and md5sums: $extra"
    [ "$failures" -eq 0 ]
}

# check_base_jammy <deb>: `Depends` must be installable on Ubuntu 22.04 (glibc 2.35, no `t64` package names).
# Prints FAIL lines; returns 1 on any.
check_base_jammy() {
    local pkg=$1 depends libc bad=0
    depends=$(dpkg-deb -f "$pkg" Depends)
    libc=$(echo "$depends" | sed -n 's/.*\(^\|, \)libc6 (>= \([0-9.]*\)).*/\2/p')
    if [ -z "$libc" ]; then
        echo "FAIL: Depends has no 'libc6 (>= V)': $depends" >&2
        bad=1
    elif [ "$(printf '%s\n2.35\n' "$libc" | LC_ALL=C sort -V | tail -n1)" != 2.35 ]; then
        echo "FAIL: Depends has libc6 (>= $libc), newer than 2.35 (Ubuntu 22.04): $depends" >&2
        bad=1
    fi
    if echo "$depends" | grep -Eq '[A-Za-z0-9.+-]+t64( |,|$)'; then
        echo "FAIL: Depends has a t64 package name (not installable on Ubuntu 22.04): $depends" >&2
        bad=1
    fi
    for lib in libwebkit2gtk-4.1-0 libgtk-3-0 libglib2.0-0 libayatana-appindicator3-1; do
        echo "$depends" | grep -Eq "(^|, )$lib( |,|\$)" || { echo "FAIL: Depends lacks $lib: $depends" >&2; bad=1; }
    done
    [ "$bad" -eq 0 ]
}

echo "== package: $deb"
echo "== 1. file list, modes, owner, control"
if check_package "$deb"; then
    echo "ok: file list, modes, owner, control"
else
    echo "the package check failed" >&2
    exit 1
fi

if [ "${EP_DEB_EXPECT_BASE:-}" = jammy ]; then
    echo "== 1b. Depends installable on Ubuntu 22.04 (EP_DEB_EXPECT_BASE=jammy)"
    check_base_jammy "$deb" || { echo "the Ubuntu 22.04 Depends check failed" >&2; exit 1; }
    echo "ok: Depends: $(dpkg-deb -f "$deb" Depends)"
elif [ -n "${EP_DEB_EXPECT_BASE:-}" ]; then
    echo "FAIL: EP_DEB_EXPECT_BASE is '$EP_DEB_EXPECT_BASE'; the only value is jammy" >&2
    exit 1
fi

echo "== 2. both .desktop files"
root="$work/extract"
mkdir -p "$root"
dpkg-deb -x "$deb" "$root"
for d in easy-personal easy-personal-tray; do
    file="$root/usr/share/applications/$d.desktop"
    grep -q '^Exec=.*--static-dir /usr/share/easy-personal-rs/static$' "$file" || { echo "FAIL: $d.desktop has no --static-dir Exec" >&2; exit 1; }
done
if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$root/usr/share/applications/easy-personal.desktop" "$root/usr/share/applications/easy-personal-tray.desktop"
    echo "ok: desktop-file-validate passed on both files"
else
    echo "note: desktop-file-validate is not installed; the .desktop check is skipped"
fi

echo "== 3. extracted program"
bin="$root/usr/bin/easy-personal-rs"
got=$("$bin" --version)
[ "$got" = "easy-personal-rs $version" ] || { echo "FAIL: --version printed '$got'" >&2; exit 1; }
echo "ok: --version printed '$got'"

# Temp HOME and data folders; no session bus (so the keychain check cannot reach the owner's keychain) and a PATH
# without the owner's ~/.local/bin (so no real `claude` runs).
home="$work/home"
mkdir -p "$home" "$work/config" "$work/run" "$work/data"
run_clean() {
    env -i PATH=/usr/bin:/bin HOME="$home" XDG_CONFIG_HOME="$work/config" XDG_RUNTIME_DIR="$work/run" \
        EASY_PERSONAL_HOME="$work/data" "$@"
}

# `doctor` reads the store and stops without one: an ingest of the empty temp HOME makes it.
run_clean "$bin" ingest >"$work/ingest.out" 2>&1 || { echo "FAIL: ingest into the temp store failed:" >&2; cat "$work/ingest.out" >&2; exit 1; }
# `doctor` exits 1 on a FAIL line (no keychain here): only its launcher line matters.
doctor_out=$(run_clean "$bin" doctor 2>&1 || true)
want_launcher="$root/usr/bin/easy-personal-launch"
echo "$doctor_out" | grep -F "launcher" | grep -qF "$want_launcher" ||
    { echo "FAIL: doctor does not name $want_launcher:" >&2; echo "$doctor_out" >&2; exit 1; }
echo "ok: doctor: $(echo "$doctor_out" | grep -F launcher)"

echo "== 4. dashboard serves the page from the package"
# Not through `run_clean`: a function in the background runs in a subshell, and `$!` would be the subshell, not the server.
env -i PATH=/usr/bin:/bin HOME="$home" XDG_CONFIG_HOME="$work/config" XDG_RUNTIME_DIR="$work/run" EASY_PERSONAL_HOME="$work/data" \
    "$bin" dashboard --no-ingest --static-dir "$root/$static_root" >"$work/server.out" 2>"$work/server.err" &
server_pid=$!
url=""
for _ in $(seq 1 50); do
    url=$(sed -n 's|^dashboard: \(http://127\.0\.0\.1:[0-9]*/\).*|\1|p' "$work/server.out" | head -n1)
    [ -n "$url" ] && break
    kill -0 "$server_pid" 2>/dev/null || break
    sleep 0.2
done
[ -n "$url" ] || { echo "FAIL: the server printed no address" >&2; cat "$work/server.out" "$work/server.err" >&2; exit 1; }
code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 "$url")
[ "$code" = 200 ] || { echo "FAIL: GET $url gave $code" >&2; exit 1; }
echo "ok: GET $url gave $code"
kill "$server_pid"
wait "$server_pid" 2>/dev/null || true
if kill -0 "$server_pid" 2>/dev/null; then echo "FAIL: server $server_pid did not stop" >&2; exit 1; fi
echo "ok: server $server_pid stopped"
server_pid=""

echo "== 5. the check fails when a listed file is missing"
neg="$work/neg"
mkdir -p "$neg"
dpkg-deb -R "$deb" "$neg/tree"
rm "$neg/tree/usr/share/applications/easy-personal-tray.desktop"
dpkg-deb --build --root-owner-group "$neg/tree" "$neg/missing.deb" >/dev/null
if check_package "$neg/missing.deb" 2>"$neg/err"; then
    echo "FAIL: the check accepted a package without easy-personal-tray.desktop" >&2
    exit 1
fi
grep -q 'easy-personal-tray.desktop' "$neg/err" || { echo "FAIL: the rejection does not name the missing file" >&2; cat "$neg/err" >&2; exit 1; }
echo "ok: rejected, and it names the file:"
sed 's/^/    /' "$neg/err"

echo "== 6. the jammy check fails on a host-style Depends (libc6 >= 2.39, t64 names)"
mkdir -p "$neg/host"
dpkg-deb -R "$deb" "$neg/host/tree"
sed -i -E '/^Depends:/ { s/libc6 \(>= [0-9.]+\)/libc6 (>= 2.39)/; s/libgtk-3-0( |,|$)/libgtk-3-0t64\1/; s/libglib2\.0-0( |,|$)/libglib2.0-0t64\1/ }' \
    "$neg/host/tree/DEBIAN/control"
dpkg-deb --build --root-owner-group "$neg/host/tree" "$neg/host.deb" >/dev/null
if check_base_jammy "$neg/host.deb" 2>"$neg/host.err"; then
    echo "FAIL: the jammy check accepted a Depends with libc6 (>= 2.39) and t64 names" >&2
    exit 1
fi
if ! grep -q 'libc6 (>= 2.39)' "$neg/host.err" || ! grep -q 't64' "$neg/host.err"; then
    echo "FAIL: the rejection does not name libc6 and t64" >&2
    cat "$neg/host.err" >&2
    exit 1
fi
echo "ok: rejected, and it names libc6 and t64:"
sed 's/^/    /' "$neg/host.err"

echo "PASS: $deb"
