# Spike T016 — Tauri v2 window under WSLg (2026-11-10)

Throwaway code in `rust/spikes/t016-window/` (own `Cargo.lock`, empty `[workspace]` table, outside `rust/Cargo.toml`). Run: `cd rust/spikes/t016-window && timeout 60 cargo run`.
Machine: Ubuntu 26.04 on WSL2, WSLg (`DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`), Rust 1.99.1.

## Answers

1. **Does a Tauri window open here? Yes.** `tauri =2.12.5` (still the latest stable; 3.0.0 is alpha) with `tauri-build =2.7.4` builds with the apt packages the owner installed. The spike opens a window on `http://127.0.0.1:<port>/#<token>` and the page's own script posts a report back from inside the webview (`POST /report` with the token header). The Focused/Moved/Resized window events came from WSLg. The window was not looked at by eye, so "opened" rests on the in-webview report plus those events.
2. **Webview**: WebKitGTK 4.1, `libwebkit2gtk-4.1-0` 2.52.6, GTK 3.24.52 (`tao` 0.37.2, `wry` 0.57.2). The user agent reports `AppleWebKit/605.1.15 ... Version/60.5 Safari/605.1.15`. GTK picked the **Wayland** backend (`GdkWaylandDisplay`); `GDK_BACKEND=x11` also works (`GdkX11Display`).
3. **Fragment token and custom header under the dashboard CSP: both work.** The page read the 64-char token from `location.hash` (the fragment survives `WebviewUrl::External`). `fetch('/api/version', {headers:{'x-dashboard-token': t}})` gave 200 `{"version":"spike"}`; without the header 403; with a reversed token 403. Page and script load from the same origin (`/` and `/app.js`). The CSP is really enforced by this webview: an injected inline `<script>` was blocked (`script-src-elem`), a fetch to `http://localhost:<port>` (other origin) was blocked (`connect-src` falling back to `default-src 'self'`), and an inline `style` attribute was applied (`style-src 'unsafe-inline'`). The server saw only `GET /`, `GET /app.js`, 3 x `GET /api/version`, `POST /report`; same-origin GETs carry no `Origin`, the POST carries `Origin: http://127.0.0.1:<port>`.
4. **Does a remote origin get Tauri IPC?** `window.__TAURI_INTERNALS__` is injected into the remote page (`window.__TAURI__` is not, `withGlobalTauri` false), but without a capability every command is refused: `plugin:app|version` -> "app.version not allowed. Permissions associated with this command: core:app:allow-version", an app command `ping` -> "ping not allowed. Plugin not found". With a capability `{"windows":["main"], "remote":{"urls":["http://127.0.0.1:*"]}, "permissions":["core:app:allow-version"]}` (compiled in, `capabilities-remote/remote.json`) the plugin command returned `"0.0.0"`; the app command stayed refused (it has no permission entry; likely needs an app manifest in `build.rs`, not tried). The IPC `ipc://localhost/...` fetch is blocked by the dashboard CSP (`connect-src`, one violation logged); Tauri then falls back to postMessage and the calls still answered. **The dashboard needs no IPC** (it only calls the HTTP API), so P1b should ship **no capability file** and leave the CSP as it is.
5. **Env variables needed: none.** Plain run works. `GDK_BACKEND=x11`, `LIBGL_ALWAYS_SOFTWARE=1`, `WEBKIT_DISABLE_COMPOSITING_MODE=1` each also pass (not needed). Harmless stderr noise: `libEGL warning: failed to open /dev/dri/renderD128: Permission denied` and `MESA: ZINK` lines (no GPU access in WSL; software path).
6. **Failure modes** (`timeout` on every run):

| Run | Result |
|-----|--------|
| `DISPLAY` and `WAYLAND_DISPLAY` unset, `XDG_RUNTIME_DIR` kept | still opens, exit 0: GTK falls back to `$XDG_RUNTIME_DIR/wayland-0` |
| all three unset, no preflight | `tao` **panics** `Failed to initialize gtk backend!`, exit 101, 0.11 s (`Builder::build` panics, it does not return `Err`) |
| all three unset, with `gtk::init()` preflight | clear message, exit 5, 0.11 s (`gtk::init()` returns `Err`) |
| `DISPLAY=:99` (dead X server), `GDK_BACKEND=x11` | **hangs**: the main thread sits in `gtk::init` on a TCP connect to `127.0.0.1:6099` (SYN_SENT); killed by `timeout` at 40 s, exit 124 |
| page never reports (`T016_DROP_REPORT=1`) | message "no report from the webview within 20s", exit 3 at 20.13 s |

7. **Measurements** (real runs, WSLg): process start -> window created ~195 ms, page load finished ~311 ms, report ~423 ms (5 release runs: 421-430 ms; the very first debug run after the build was 939 ms); whole process 0.52 s wall. Process tree (spike, `WebKitNetworkProcess`, `WebKitWebProcess`, `bwrap`, `glycin-svg`: 6 processes) peaked at **518 MiB RSS summed, 335 MiB PSS** (sampled every 20 ms from `/proc`, 3 runs: 517.7-517.8 MiB). Release binary **11,145,832 bytes** (7,083,408 stripped), default profile, no LTO. `cargo build --release` 1 min 10 s; debug 50 s for the first full build, 1.5-5 s incremental (`target/` is 3.7 GB with both profiles).
8. **No traffic leaves the machine**: only the loopback listener (`ss`: the one INET socket of the tree at 4 s was `127.0.0.1:<port>` LISTEN); no updater, tray or network plugin in the config. Not a packet capture.

## What P1b must do

- The product server serves the static page **and its assets from the same origin** (`/`, `/assets/*.js`); inline scripts stay blocked, so the React bundle must stay external files (it is).
- The window opens exactly `http://127.0.0.1:<port>/#<token>` (`WebviewUrl::External`); the fragment is preserved and never sent to the server.
- Add the static-fallback test already required in `rust/PROJECT_SPEC.md` (encoded and dot-segment paths must not reach `/api` unguarded).
- No display must not break CLI and MCP (RS-006): keep Tauri out of those code paths, and before starting the window call `gtk::init()` (an `Err` means no display). Add a **timeout** around it or test the socket file first (`$WAYLAND_DISPLAY` / `/tmp/.X11-unix/X<n>`): a dead `DISPLAY` hangs instead of failing.
- Log page-load URLs without the fragment (the spike does; Tauri's payload URL includes the token).
- Runtime packages for the owner's Ubuntu: `libwebkit2gtk-4.1-0` and `libgtk-3-0t64` (wry wants WebKit2GTK 2.40 or newer, from its README).
- macOS (WKWebView) and Windows (WebView2, Edge Chromium) were not built; wry's README says so for both. WebView2 and WebKitGTK differ, so P4/P5 repeat this spike.

## Open items

- Window not inspected visually (no screenshot tool): confirm once by eye with `cargo run` and a longer timeout if wanted.
- App commands from a remote origin (manifest in `build.rs`) were not tried; not needed by the dashboard.
- Spike-only extras: `gtk = "=0.18.2"` (logs the GDK backend), a `T016_DROP_REPORT` test knob, `T016_REMOTE_IPC=1` build switch, `/dev/urandom` for the token (Linux only). `gen/` is generated by the build and git-ignored by the spike's own `.gitignore`.
