#!/usr/bin/env bash
# T093: proves the .deb installs on Ubuntu 22.04 and 24.04. Needs Docker and the network (the images and the Ubuntu
# archive), so it is not part of the offline `test_deb.sh`. Every install runs inside a throw-away `docker run --rm`
# container; nothing is installed on this machine.
#
#   rust/packaging/test_deb_install.sh                    check rust/target/deb/easy-personal-rs_<version>_amd64.deb
#   DEB=/path/x.deb rust/packaging/test_deb_install.sh    check that file instead
#   EP_INSTALL_IMAGES="ubuntu:22.04" rust/packaging/test_deb_install.sh   other images (default: ubuntu:22.04 ubuntu:24.04)
#
# In each container: `apt-get update`, `apt-get install -y ./<deb>`, then `easy-personal-rs --version` must print
# `easy-personal-rs <version>` (the Version field of the package).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
rust_dir=$(dirname "$here")
images=${EP_INSTALL_IMAGES:-ubuntu:22.04 ubuntu:24.04}

command -v docker >/dev/null 2>&1 || { echo "test_deb_install: docker is not installed" >&2; exit 1; }
docker info >/dev/null 2>&1 || { echo "test_deb_install: the docker daemon is not reachable" >&2; exit 1; }

if [ -n "${DEB:-}" ]; then
    deb=$DEB
else
    deb=$(find "$rust_dir/target/deb" -maxdepth 1 -name 'easy-personal-rs_*_amd64.deb' 2>/dev/null | LC_ALL=C sort | tail -n1)
fi
[ -n "$deb" ] && [ -f "$deb" ] || { echo "test_deb_install: no package (build it first, or set DEB=)" >&2; exit 1; }
deb=$(cd "$(dirname "$deb")" && pwd)/$(basename "$deb")
version=$(dpkg-deb -f "$deb" Version)
echo "== package: $deb (version $version)"
echo "== Depends: $(dpkg-deb -f "$deb" Depends)"

indent() { sed 's/^/    /'; }
failed=0
for image in $images; do
    echo "== $image: apt-get install -y ./$(basename "$deb")"
    # The folder holding the package is mounted read-only; the container is removed when it ends.
    if got=$(docker run --rm -e DEBIAN_FRONTEND=noninteractive -v "$(dirname "$deb"):/pkg:ro" -w /pkg "$image" sh -c '
        set -e
        . /etc/os-release
        echo "os: $PRETTY_NAME (glibc $(ldd --version | sed -n "1s/.* //p"))"
        apt-get update -qq >/dev/null
        apt-get install -y ./'"$(basename "$deb")"' >/tmp/apt.log 2>&1 || { cat /tmp/apt.log; echo "APT_FAILED"; exit 1; }
        tail -n 3 /tmp/apt.log
        easy-personal-rs --version
    ' 2>&1); then
        printf '%s\n' "$got" | indent
        if [ "$(echo "$got" | tail -n1)" = "easy-personal-rs $version" ]; then
            echo "ok: $image printed 'easy-personal-rs $version'"
        else
            echo "FAIL: $image: --version did not print 'easy-personal-rs $version'" >&2
            failed=1
        fi
    else
        printf '%s\n' "$got" | indent >&2
        echo "FAIL: $image: the install failed" >&2
        failed=1
    fi
done
[ "$failed" -eq 0 ] || exit 1
echo "PASS: $deb installs on: $images"
