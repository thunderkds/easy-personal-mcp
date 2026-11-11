# easy-personal-rs

Rust port of easy-personal, built in slices (see `PROJECT_SPEC.md`). Today it has five commands (the fifth, `app`, opens a window only in a build with `--features window`); the Python app
stays the working one.

## Build and run

```bash
. "$HOME/.cargo/env"        # rustup was installed without editing the shell profile
cd rust
cargo build
./target/debug/easy-personal-rs --version        # easy-personal-rs <Cargo version>
./target/debug/easy-personal-rs store-info       # or --json
./target/debug/easy-personal-rs serve            # loopback API, Ctrl-C to stop
./target/debug/easy-personal-rs serve --static-dir <DIR>   # the same, plus the dashboard's files from <DIR>
./target/debug/easy-personal-rs mcp              # MCP server on stdio
cargo build --features window                    # adds the window (Tauri v2); see "Window" below
./target/debug/easy-personal-rs app --static-dir <DIR>     # the dashboard in a window, until the window closes
```

`store-info` opens the shared store (`$EASY_PERSONAL_HOME/store.sqlite`, else `~/.easy-personal/store.sqlite`, the same
variable as the Python app) read-only and prints the file name, `PRAGMA user_version`, the newest migration this build knows (11),
whether migrations are pending, and each table's row count. It never prints rows or the full path, and never writes.

| Result | Exit code |
|---|---|
| store read | 0 |
| no store file ("no data"; nothing is created) | 0 |
| store newer than this build knows | 2 |
| directory in place of the file, unreadable file, store busy for 5 s | 1 |

Reading a store in WAL mode makes SQLite create an empty `store.sqlite-wal` and a `store.sqlite-shm` next to it if none exist; the
store file itself is not changed.

`serve` binds `127.0.0.1` only, on a free port, and prints the same two lines as the Python app: `dashboard: http://127.0.0.1:<port>/#<token>`
and `Ctrl-C to stop.` (no browser is opened yet). The token is 32 random bytes, new per run, and only appears in that URL fragment. The routes
so far are `GET /api/version` -> `{"version": "<Cargo version>"}`, the six read routes and the two workspace routes below. Every response carries the Python app's
`Content-Security-Policy`.
The guard matches the Python app: `Host` must be `127.0.0.1:<port>`, an `Origin` if sent must be `http://127.0.0.1:<port>`, and every
`/api/` path needs the `x-dashboard-token` header (a token in the query string is not accepted). Failures get `403 {"error": "forbidden"}`.
A repeated `Host` or token header is refused (Python keeps the last one).

`serve --static-dir <DIR>` also serves the built dashboard's files from `<DIR>` on the same origin, behind the same guard and with the same
CSP on every answer, so a window (`app`, below) can open `http://127.0.0.1:<port>/#<token>`. Without the flag nothing changes (`GET /` is still an empty 404).
`<DIR>` is resolved once at start (it may be relative, end in `/` or be a symlink); a path that is not a directory stops `serve` with a
one-line message that names no path, before anything is bound or printed. Static rules, like Starlette's `StaticFiles(html=True)`:
`GET /` and `GET /<dir>/` serve that folder's `index.html`, a file is served with a content type by extension (`octet-stream` when unknown),
`HEAD` answers the headers only, any other method is `405`; a miss, a folder without `index.html` and a folder without a trailing slash are
a fixed `404` (no listing, no redirect, no path echoed). A non-`/api/` path needs **no token** (the page must load before it can read the
fragment) but Host and Origin are checked; `/api/*` needs the token exactly as before, and an `/api/` path no route serves is still the
empty `404`, never a file. Paths are read as raw text and decoded once per segment: `..`, `.`, empty segments, backslashes, NUL, `%2f`,
`%5c`, `%25` and `/%61pi/...` are all `404`. These differ from Python on purpose:
- **A name that starts with `.` is a `404`** (`.env`, `.web-stamp`; Python serves them), also when a symlink points at one.
- **A symlink is followed only while the resolved path stays under the folder's canonical path**; one that points outside is a `404`.
  A FIFO, socket or device is a `404` (never opened).
- **A file over 32 MiB is a `404`**; files are read per request (no cache, no `Range`, no conditional requests, no compression).

The read routes return the same JSON and status codes as the Python app, reading the store read-only (a new connection per request,
off the async workers, `TZ`/OS zone as in Python): `GET /api/usage/summary` and `GET /api/usage/breakdown?by=day|model|project|tool`
(both take `since` and `until`), `GET /api/usage/sessions` (the session timeline, 50 sessions with `points` and `gaps`) and `GET /api/usage/outliers`
(both take `since` and `until`), `GET /api/warnings` and `GET /api/plan`. All six take the optional `workspace` (a name) and `account`
(an id or a label, case-insensitive, stripped) and answer for that scope through the core's scoped functions; both given, both must match.
An unknown workspace or account is `400 {"error": "unknown workspace"}` or `{"error": "unknown account; valid labels: ..."}` (Python's text,
none of the input in it); an invalid `by` or `since` is reported first. An empty `workspace=` or `account=` is no filter. The query is
decoded like Starlette (`%XX`, `+` is a space, the last value of a repeated key wins; `since=` empty is an error). These differ from Python on purpose:
- No store file: `503 {"error": "no store yet: nothing has been ingested into the data folder"}`; Python creates an empty store. A store
  newer than this build: `503` with the core's text; busy, unreadable or not a file: `503` with one fixed text. No body carries a path.
- `GET /api/workspaces` -> `{"workspaces": [...]}` (orphan first, then by name; each `{id, name, color, icon, description, email, is_orphan, repos}`)
  and `GET /api/workspaces/unassigned` -> `{"repos": [...]}` (real projects with usage that no workspace repo covers, newest use first, each with
  its API-equivalent cost and abstained models) are read-only and ignore the whole query string, as Python's handlers do. The writes of
  Python's `/api/workspaces` (`POST`, `PATCH`, `DELETE`) are absent: those methods answer `405` (the CSP still set), and
  `/api/workspaces/<id>` is a `404`.
- Every other Python route is still absent (404; with `--static-dir`, a non-`/api/` path is looked up in the folder first).

```bash
./target/debug/easy-personal-rs serve        # prints the port and token
curl -H "x-dashboard-token: <token>" http://127.0.0.1:<port>/api/version
```

`mcp` serves the MCP protocol on stdio (newline-delimited JSON-RPC; stdout carries nothing else) with six tools: `ping`,
`usage_summary`, `usage_breakdown`, `session_outliers`, `warnings` and `improvement_plan`. Their names, descriptions and schemas, and their results over
`parity/store-v11.sqlite`, equal the Python app's `mcp__*` goldens; `warnings` and `improvement_plan` read the clock, so their goldens
(default and rich store) are compared in-process at the manifest's `frozen_now`: the binary has no flag or variable that sets the clock.
`session_outliers` takes `since` and `until` only, as in Python (it reads no clock, so its goldens are compared through the binary; an extra argument is dropped, as Python drops it). `warnings` and `improvement_plan` take `workspace` and `account` as in Python; so do `usage_summary` and `usage_breakdown` here, which
Python's tools do not (Python drops an extra argument; this build honours it, so a scope is never ignored, and the schemas stay equal
to Python's). `null`, absent and `""` mean no filter; an unknown workspace or account is a tool error (`Error executing tool <name>:
unknown workspace`, Python's text); a value that is not a string is an argument error. `initialize` reports `easy-personal-rs` and the Cargo version; the revisions with the handshake
(2024-11-05 to 2025-11-25) are served, the 2026-07-28 no-handshake mode is not. The server ends with exit 0 when stdin closes. Each call
opens the store (same path rules as `store-info`) read-only; a missing store, or one newer than this build knows, is a tool error with
no path in it. Errors in a tool are results with `isError: true` and Python's text (`Error executing tool <name>: <message>`); an
unknown tool is a JSON-RPC error; a line that is not JSON is answered with `-32700` and a null id. Not ported yet: the other six tools, the
`investigate` prompt and `--http`.

## Window (`app`, Ubuntu only so far)

The window is behind the Cargo feature `window`, **off by default**: a plain `cargo build` compiles no Tauri, GTK or WebKit crate
and links only libc, so `store-info`, `serve` and `mcp` run on a machine with no display and no desktop libraries. `cargo build
--features window` adds Tauri `=2.12.5` (`tauri-build =2.7.4`); building it needs the Ubuntu packages of `docs/spike-T001.md`
(`libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`), and
running it needs `libwebkit2gtk-4.1-0` and `libgtk-3-0t64`. A feature build links those libraries dynamically, so it needs them
installed even for `serve` and `mcp` (GTK is never started for those, and they need no display).

`app --static-dir <DIR>` (the flag is required) checks `<DIR>` as `serve` does, then checks for a display, then starts the
`serve` server in the same process and opens one window titled `easy-personal` (1280x800, resizable) on
`http://127.0.0.1:<port>/#<token>`. The token travels only in the fragment: never a query, an argument, stdout or a log line. stdout
stays empty; stderr gets one line when the window opens (the URL without the fragment) and one when it closes. Closing the window,
SIGINT or SIGTERM stops the server and exits 0, and no WebKit process is left behind. A build without the feature prints
`this build has no window: rebuild with --features window` and exits 4.

| Result of `app` | Exit code |
|---|---|
| the window closed, SIGINT/SIGTERM, or the self-test passed | 0 |
| `<DIR>` is not a readable directory, or the window could not be made | 1 |
| command-line error (for example no `--static-dir`) | 2 |
| `--self-test` saw no authenticated request within 20 s | 3 |
| a build without the `window` feature | 4 |
| no display, or the toolkit did not start within 10 s | 5 |

- **Display check**, before any Tauri or GTK code runs (a dead `DISPLAY` hangs GTK; no display at all makes Tauri panic): with
  neither `WAYLAND_DISPLAY` nor `DISPLAY` set (an empty value counts as unset), it fails at once. Otherwise it connects, with a
  2 s limit each, to `$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY` (or the absolute `WAYLAND_DISPLAY` path) and to the X server of
  `DISPLAY` (`:N` is `/tmp/.X11-unix/X<N>`, `host:N` is TCP port `6000+N`). One that answers is enough. When neither answers it
  exits 5 in at most 4 s with one line that names the variable but never a path or value. It only connects: it reads no file.
- **No IPC**: `tauri.conf.json` turns the global Tauri API off and names no plugin, and no capability file is compiled in (the
  compiled set is `{}`), so the page can call no Tauri command. Devtools are off. `security.csp` is null because the CSP comes from
  the server's header.
- **Navigation lock**: the webview may move only inside `http://127.0.0.1:<port>`. Every other URL (another port, `localhost`,
  `https`, `file:`, `data:`, `javascript:`, ...) and every new-window request is refused and logged by host only. A click on a link
  to another origin is also cancelled before WebKit connects to that host.
- **`--self-test`** (hidden from `--help`, for checks without eyes): prints `self-test: ok` and exits 0 once the page made one
  `/api/` request that passed the guard with the token, else exits 3 after 20 s. The fixture page is
  `crates/app/tests/fixtures/self-test`: `cargo run --features window -- app --self-test --static-dir crates/app/tests/fixtures/self-test`.
  The real-window tests are ignored by default; run them with a display:
  `cargo test --features window --test window -- --ignored --test-threads=1`.
- Not done: macOS and Windows (other webviews, no display check, Unix signals only), packaging, a tray or menu, embedding the
  dashboard in the binary, and focusing the first window when `app` runs twice (each run gets its own port and token). The real
  dashboard is not built here (`web/` needs `npm ci`).

## Ported library functions (core crate, no CLI/API yet)

`easy_personal_core::metrics::usage_summary` and `usage_breakdown` (by `day`, `model`, `project`, `tool`), with the `prices` line parser;
they return JSON equal to the Python goldens over `parity/store-v11.sqlite`. `usage_summary_scoped` and `usage_breakdown_scoped` take a
`scope::Scope { workspace, account }` too (T017): a workspace name keeps the events under its repo paths, an account (id or label,
case-insensitive) the events of that agent account, both given both must match, an empty string is no filter. An unknown workspace or
account is `MetricsError::Invalid` with Python's text (`unknown workspace`, `unknown account; valid labels: ...`); a workspace with no
matching project selects nothing. The plain functions are the scoped ones with no scope.
Like Python, the day key and an offset-less `since`/`until` use the machine's local zone, DST-aware: the `TZ` variable, else the OS zone
(`/etc/localtime`), read through `jiff` (`docs/spike-T009.md`). An unknown `TZ` reads as UTC (libc does the same); a bound with an offset
(`Z`, `+07:00`) ignores the zone. The zone is read once per call and `jiff` keeps it for 5 minutes, so a change of `TZ` inside a running process is
seen after that. Two odd `TZ` values differ from libc (`docs/spike-T009.md`): a POSIX string with no rules (`EST5EDT`, when the machine has no
`EST5EDT` zone file) reads as UTC, and a mis-cased name (`asia/bangkok`) is accepted.
Goldens for a non-UTC zone: `parity/golden-tz/` (`gen_golden.py --tz`).
`metrics::session_outliers` and `session_timeline` (also in `sessions.rs`; T018) are the per-session reads: `session_outliers(conn, since, until)` lists the sessions with at least 90 tool calls or a message context of at least 200,000 tokens (most calls first, ties by session id), and `session_timeline(conn, since, until, detail_sessions)` gives each session's start, end, models, token totals, one token point per message and the idle gaps of 30 minutes or more (newest session first). `detail_sessions` is `Some(50)` for the dashboard route: only the newest N sessions carry `points` and `gaps`, and the answer has a `detail_sessions` key only when it is given. `session_outliers_scoped` and `session_timeline_scoped` take the same `Scope` as above, with the same errors. `tests/sessions_parity.rs` compares them with the 5 goldens of `parity/golden` and the 18 of `parity/golden-rich` (`usage_outliers`, `usage_sessions`, `session_outliers`, workspace and account entries included), the non-UTC cases and a hand-made store with Python's answers. `GET /api/usage/sessions` (with `Some(50)`), `GET /api/usage/outliers` and the MCP tool `session_outliers` call them (T020).
A richer oracle (18 days, 3 models, 2 accounts, 6 of the 7 rules firing): `parity/golden-rich/` over `parity/store-rich/` (`gen_golden.py --rich`, T007). `tests/rich_parity.rs` compares `usage_*` and `rules::evaluate` on it; `tests/scope_parity.rs` compares the 41 `workspace`/`account` entries (summary, breakdown, warnings and plan, HTTP and MCP; four of them the unknown-scope errors) plus a store with real git repos for the kit rules.

`easy_personal_core::workspaces::list_all(conn)` and `unassigned(conn)` (T022; read-only, `workspaces.rs`) return the `{"workspaces": [...]}` and
`{"repos": [...]}` JSON of Python's `workspaces.list_all` and `workspaces.unassigned`: the workspaces ordered `is_orphan DESC, name` with their repos,
and the real projects (`metrics::project_map`, scratch folders skipped) with stored usage that no workspace repo equals or lies above (`scope::under`),
newest use first, costed once per API message with the price table of `usage_breakdown` (`usd` rounded to six places, `null` when nothing was priced and
a model abstained). `tests/workspaces_parity.rs` compares them with the four goldens (`api__workspaces__all` and `api__workspaces_unassigned__all` of
`parity/golden` and `parity/golden-rich`, counted from the manifests) and with Python's answers over a hand-made store (git repos under the build's
`target/tmp`, a gone worktree, a model missing from the price table, a tie). The one difference from Python: a stored message with a NULL
token column or a NULL timestamp makes Python raise a `TypeError`; here it is read as 0 / empty. `GET /api/workspaces` and
`GET /api/workspaces/unassigned` call them.

`easy_personal_core::rules::evaluate` runs the shipped rule pack (`rules.yaml`, read by a hand-written line parser) and returns the `warnings` JSON (hits with causes and diagnosis, abstained rules) equal to the Python goldens; the clock is a parameter. `rules::evaluate_scoped(conn, now_micros, &rules, Scope)` limits every metric, the kernel-trace rule (through the scope's projects) and the easy-context checks to a workspace and/or account; an unknown one is `ScopedRulesError::UnknownScope` with Python's text, even when the store has too little history to measure. Not ported yet: the `checked` list, the `config.yaml` threshold override.

`easy_personal_core::plan::improvement_plan(conn, now_micros, &rules)` groups the fired rules by priority (items in `evaluate` order) and returns the `improvement_plan` JSON equal to the Python goldens in `parity/golden` and `parity/golden-rich`. `plan::improvement_plan_scoped(conn, now_micros, &rules, Scope)` is the same over `evaluate_scoped` (same error type).

## Test

```bash
cargo test && cargo clippy --all-targets -- -D warnings
```

Tests build their own temp stores and never open `~/.easy-personal`.
