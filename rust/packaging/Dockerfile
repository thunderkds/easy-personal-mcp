# T093: the build image for `build-deb.sh --docker`. Ubuntu 22.04, so the programs link against glibc 2.35 and
# `dpkg-shlibdeps` writes the Ubuntu 22.04 package names. The package then installs on 22.04, 24.04 and 26.04
# (the `t64` packages of 24.04 and later `Provide` the old names).
#
# The image holds the C toolchain, the dev libraries of the window build and Rust stable. It does not hold Node: the
# dashboard page is plain static files, so `build-deb.sh` builds it on the host and mounts it.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential ca-certificates curl dpkg-dev git pkg-config python3 \
        libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libgtk-3-dev \
        libayatana-appindicator3-dev libdbus-1-dev libssl-dev librsvg2-dev \
    && rm -rf /var/lib/apt/lists/*

# Rust is installed under /opt and made readable for every user: the build runs as the host user, not as root.
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable \
    && chmod -R a+rX /opt/rustup /opt/cargo
