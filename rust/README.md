# easy-personal-rs

Rust port of easy-personal, built in slices (see `PROJECT_SPEC.md`). Today it has five subcommands (`store-info`, `ingest`, `serve`, `mcp`, `app`; `app` opens a window only in a build with `--features window`); the Python app
stays the working one.

## Build and run

```bash
. "$HOME/.cargo/env"        # rustup was installed without editing the shell profile
cd rust
cargo build
./target/debug/easy-personal-rs --version        # easy-personal-rs <Cargo version>
./target/debug/easy-personal-rs store-info       # or --json
./target/debug/easy-personal-rs ingest           # copy new Claude Code transcript metadata into the store; see "Ingest (P2a)"
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
so far are `GET /api/version` -> `{"version": "<Cargo version>"}`, the six read routes, the two workspace routes and `GET /api/connectors` below. Every response carries the Python app's
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
- `GET /api/connectors` (T046) -> `{"connectors": [...], "workspace_connectors": [...]}`. `connectors` is the provider catalog (the five entries of
  Python's `catalog/*.yaml`), sorted by category, then the claude.ai-connector entries first, then provider; each is
  `{id, category, provider, claude_ai_connector, claude_ai_link, fields: [{env, label, secret}], configured}`. `workspace_connectors` are the
  stored rows `{id, alias, provider, workspace}`, by workspace name then alias. It takes no parameter (the whole query string is ignored), like
  the workspace routes. Errors: `403` without the token or with a bad Host/Origin, `405` for `POST`, `PATCH`, `PUT` and `DELETE` (the CSP still
  set; Python's `POST /api/connectors` is not served), `503` for a missing, newer, busy, unreadable or table-less store (the fixed texts above), `500
  {"error": "internal error"}` if the embedded catalog ever failed to parse (a test parses every file, so this cannot ship). Fields shown, and
  why they are safe: the catalog holds public names, labels and one fixed `https://claude.ai/settings/connectors` link (never a command line,
  which Python's route does not show either); a stored row shows only a connector id, an alias, a provider id and a workspace name (the secrets
  live in the keychain, which this build never reads). **`configured` is always `false`**: Python answers `true` only when the OS keychain
  holds the item *and* `claude mcp get` knows the entry, and answers `false` when it cannot ask (no keychain backend, no `claude`), which is
  the state of the goldens. This build asks neither (no keychain crate, no process; both arrive with T050 behind the owner's approval), so on a
  machine where Python says `true`, this says `false`. `GET /api/connectors/legacy` and `/api/connectors/<id>` are `404`; the connector write routes are not served (`POST /api/connectors` is `405`).
- `GET /api/kit`, `GET /api/kit/workspaces` and `GET /api/kit/reports` (T042, read-only; below, "Kit"). `/api/kit` takes the optional `workspace` and
  `account` as the usage routes do (same 400 texts); `/api/kit/workspaces` takes `since`, `until` and `account` (a `workspace` is ignored, as in Python);
  `/api/kit/reports` takes no parameter. A `since` or `until` that is not a date is `400` with an `error` key (the text is CPython's, ours says the
  same for the same inputs). `/api/kit/report-files` and `/api/kit/report-file` are T045 (below, "Report files"). Writes are 405.
- `GET /api/reports` -> `{"reports": [...]}` (T040: the newest 50 saved investigate reports, newest first; each `{id, saved_utc, workspace, account,
  fired_count, summary_first_line}`), `GET /api/reports/{id}` -> one report (`{id, saved_utc, server_version, workspace, account, summary, fixes,
  as_of_utc, now_as_of_utc, rules: [{rule_id, title, before, now}]}`: `before` is the saved snapshot, `now` is each rule measured on this request for the
  saved scope, with the system clock) and `GET /api/first-run` -> `{"steps": [{id, done, hint}]}` (first ingest, daily timer, a confirmed account whose
  folder exists, a workspace besides the orphan). All three are read-only, take no parameter (the whole query string is ignored) and show the report text
  (`summary`, `fixes`) as stored: it is the one text the store keeps, and it is neither logged nor put in any error. Errors:

  | Request | Answer |
  |---|---|
  | `GET /api/reports/{id}`, `{id}` not 1 to 18 ASCII digits (after `%XX` decoding), or no such report | `404 {"error": "no such report"}` (leading zeros are fine: `001` is 1) |
  | `GET /api/reports/{id}`, a saved snapshot or fixes value that is not what `save_report` writes | `500 {"error": "the saved report cannot be read"}` |
  | any of the three, no token, bad Host or Origin | `403 {"error": "forbidden"}` (before anything else) |
  | `POST`, `PUT`, `PATCH`, `DELETE` on any of the three | `405`, empty body, CSP set (`HEAD` answers like `GET`) |
  | no store, a store newer than this build, a busy, unreadable or non-file store | `503`, as for the other routes |

  Decided for you (T040; Rust differs from Python): a report whose snapshot Python cannot read is a fixed `500` here (Python: a traceback), and in the list it
  is kept with `fired_count` 0 (Python fails the whole list); the first-run hint for the timer is always `easy-personal schedule install` (Python asks
  `systemctl --user show-environment` per request and, on a machine with no systemd user session, answers a crontab line built from its own command path:
  the Rust server never starts `systemctl`; the install command prints that cron line itself); the timer is looked up under `XDG_CONFIG_HOME` (empty counts as unset)
  or `$HOME/.config` read per request, and with neither set it counts as not installed (Python asks the passwd entry); the report's `now` side uses the shipped
  rule pack like `/api/warnings` (the `config.yaml` threshold override is not ported); a `{id}` that decodes to text with a `/` (`/api/reports/1%2F2`) is the JSON
  `404 no such report` here (Python: Starlette's plain-text `Not Found`, no route matches), and `/api/reports/` with no id is the empty `404` (Python: the same plain text).
- `GET /api/accounts?by=account|provider` and `GET /api/sync?account=<id>` (T041; read-only). **No account detection**: Python's two handlers first run
  `accounts.detect`, which INSERTs the `.claude*` / `.codex*` folders it finds; the Rust server never does (the detect stays in `ingest`), so a new agent
  folder appears after the next `ingest`, not at page load. Nothing is written and no process is started (no `claude`, no keychain).
  - `/api/accounts` -> `{"by": "account", "accounts": [...]}` (`by` defaults to `account`) or `{"by": "provider", "providers": {"claude": [...], ...}}`. Each
    account is `{id, provider, label, folder, status, config_dir_env, is_default, workspaces, status_line}`: `status` is `missing` when the folder is gone and the
    account is not ignored; `workspaces` are the names it picked; `status_line` is `{email, plan, organization, last_model, last_active_utc, tokens_today,
    tokens_7d, limits}` (tokens: one row per message, since local midnight in `TZ` / the OS zone and in the last 7 x 24 h; the newest event with a model;
    the default account also owns events with no account). For a Claude account it reads exactly two small regular files of its folder, as Python does:
    three keys of `<folder>/.claude.json` (`oauthAccount`: `emailAddress`, `billingType`, `organizationName`; the default account's file may sit beside its folder)
    and `<folder>/easy-personal-limits.json` (`five_hour` / `seven_day` `used_percentage`, `resets_at`, `written_at`). Nothing else in a folder is opened or listed.
  - `/api/sync` -> `{"steps": [{account_id, account, action, alias, connector_id, line}]}`: what applying the store's picks would change in each confirmed Claude
    account (and each ignored one that still holds registrations); `?account=<id>` selects one (an empty value is no filter). The lines are data from the store
    (an account label or alias with a newline or quotes is JSON-escaped, never run). The apply (`POST /api/sync`) is `405`; the account writes
    (`POST /api/accounts`, `.../confirm`, `.../ignore`, `PUT .../workspaces`) are T048's: `405` on `/api/accounts`, `404` on the sub-paths.

  | Request | Answer |
  |---|---|
  | `by` not exactly `account` or `provider` (an empty `by=` too) | `400 {"error": "by must be account or provider"}` |
  | `account` not plain ASCII digits | `400 {"error": "account must be an account id"}` |
  | `account` with no account (`0`, `99`, 20 digits) | `404 {"error": "no such account"}` |
  | no token, bad Host or Origin | `403 {"error": "forbidden"}` (before anything else) |
  | `POST`, `PUT`, `PATCH`, `DELETE` on either route | `405`, empty body, CSP set (`HEAD` answers like `GET`) |
  | no store, a store newer than this build, a busy, unreadable or non-file store | `503`, as for the other routes |

  Decided for you (T041; Rust differs from Python): (1) the guide's checklist said Rust never opens an account folder, but Python's status line (and the `missing`
  status and the plan's `folder missing`) need the folder: Rust stats it and reads only the two files above, with Python's size caps (32 MiB, 64 KiB), because
  without them the email, plan, organization and limits of every real account would be empty; a folder, pipe or other non-regular file where a file should be
  is "not there" (Python: a folder gives the same, a pipe would hang it). (2) `sync` answers as Python does when `claude` is **not** on `PATH` (the state of the
  goldens): on a machine that has it, Python asks `claude mcp get` per entry and can also say `record`, `forget`, `refused`, "missing from Claude Code: re-add"
  and "does not point at the launcher"; Rust says only `add` (wanted, not registered), `remove` (registered, not wanted) and `skipped`; a registration the store
  still wants is left alone. (3) `account` is ASCII digits only: Python's `isdigit` also accepts `²` (then `int()` fails: a 500) and `٣` (read as 3); both are `400` here.
  (4) An `account` of 20 digits is `404 no such account` (Python: sqlite3 `OverflowError`, a 500). (5) A JSON file Python parses but `serde_json` does not
  (`NaN` / `Infinity` literals, nesting deeper than 128) reads as "no file"; an integer beyond 64 bits in `limits` is shown as a float. (6) With a missing store
  the answer is `503` even when `account` is also invalid (Python validates `account` first, then creates the store). (7) The agent root that decides
  `is_default` is `EASY_PERSONAL_AGENT_ROOT`, else `HOME`, read per request (empty counts as unset), as Python's `config.agent_root()`.
- Every other Python route is still absent (404; with `--static-dir`, a non-`/api/` path is looked up in the folder first).

```bash
./target/debug/easy-personal-rs serve        # prints the port and token
curl -H "x-dashboard-token: <token>" http://127.0.0.1:<port>/api/version
```

`mcp` serves the MCP protocol on stdio (newline-delimited JSON-RPC; stdout carries nothing else) with eleven tools: `ping`,
`usage_summary`, `usage_breakdown`, `session_outliers`, `warnings`, `improvement_plan`, `kit_summary` (T042: optional `workspace` and `account`, `null`, absent and `""` mean no filter; the projects whose folder is gone are listed with `missing_on_disk: true`, as Python's tool lists them), `kit_findings` (T044, below) and the three connector tools (T046, below). Their names, descriptions and schemas, and their results over
`parity/store-v11.sqlite`, equal the Python app's `mcp__*` goldens; `warnings` and `improvement_plan` read the clock, so their goldens
(default and rich store) are compared in-process at the manifest's `frozen_now`: the binary has no flag or variable that sets the clock.
`session_outliers` takes `since` and `until` only, as in Python (it reads no clock, so its goldens are compared through the binary; an extra argument is dropped, as Python drops it). `warnings` and `improvement_plan` take `workspace` and `account` as in Python; so do `usage_summary` and `usage_breakdown` here, which
Python's tools do not (Python drops an extra argument; this build honours it, so a scope is never ignored, and the schemas stay equal
to Python's). `null`, absent and `""` mean no filter; an unknown workspace or account is a tool error (`Error executing tool <name>:
unknown workspace`, Python's text); a value that is not a string is an argument error. `initialize` reports `easy-personal-rs` and the Cargo version; the revisions with the handshake
(2024-11-05 to 2025-11-25) are served, the 2026-07-28 no-handshake mode is not. The server ends with exit 0 when stdin closes. Each call
opens the store (same path rules as `store-info`) read-only; a missing store, or one newer than this build knows, is a tool error with
no path in it. Errors in a tool are results with `isError: true` and Python's text (`Error executing tool <name>: <message>`); an
unknown tool is a JSON-RPC error; a line that is not JSON is answered with `-32700` and a null id. Not ported yet: the other tool
(`save_report`), the `investigate` prompt and `--http`.

The connector tools (T046) take no argument (an extra one is dropped, as Python drops it) and only read: `connector_catalog` ->
`{"connectors": [{id, provider, category, claude_ai_connector, claude_ai_link, fields: [{label, secret}]}]}` in catalog-id order (no env names);
`connector_status` -> `{"connectors": [{id, configured}]}`, **`configured` always `false`** for the reason given for `GET /api/connectors` above;
`connector_suggestions` -> `{"suggestions": [{id, provider, webfetch_calls, mcp_calls}], "note": "..."}`: providers your stored usage points to
(WebFetch hosts in `tool_signals` matched against the provider domains, `mcp__<server>__<tool>` names in `events.tool_names` whose server name
contains a provider id; a `claude_ai_` server counts as already available and hides that provider), most calls first, ties in catalog order. The
first two do not open the store; `connector_suggestions` does, read-only, and a missing, newer or unreadable store is a tool error with no path in
it. Their names, descriptions and schemas equal Python's `tools/list` golden.

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

## Ported library functions (core crate)

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

`easy_personal_core::rules::evaluate` runs the shipped rule pack (`rules.yaml`, read by a hand-written line parser) and returns the `warnings` JSON (hits with causes and diagnosis, abstained rules) equal to the Python goldens; the clock is a parameter. `rules::evaluate_scoped(conn, now_micros, &rules, Scope)` limits every metric, the kernel-trace rule (through the scope's projects) and the easy-context checks to a workspace and/or account; an unknown one is `ScopedRulesError::UnknownScope` with Python's text, even when the store has too little history to measure. `rules::evaluate_checked_scoped` adds Python's `checked=True` list (every measured rule with `title`, `measured`, `threshold`, `fired`; T040) and `rules::left_out_ids` the ids of the rules left out because their kit part is not in use. Not ported yet: the `config.yaml` threshold override.

`easy_personal_core::reports::list_reports(conn)` and `get_report(conn, id, now_micros)` (T040; read-only, `reports.rs`) return the JSON of Python's `reports.list_reports` and `reports.get_report`: the saved snapshot is the `before` side of each rule, and `now` is `rules::evaluate_checked_scoped` over the saved workspace and account at `now_micros` (`not measured now` when the saved scope is gone or the rule is absent, `not in use` for a rule left out). `first_run::steps(conn, unit_dir)` is Python's `first_run.steps` with the unit folder passed in (`first_run::unit_dir()` reads `XDG_CONFIG_HOME` / `HOME`). `tests/reports_parity.rs` compares them with the eight goldens (`api__reports__all`, `api__reports_one__{found,missing,scoped}`, `api__first_run__all` of `parity/golden` and `parity/golden-rich`, counted from the manifests, under `frozen_now`) and `crates/app/tests/reports.rs` over the binary. The rich store's confirmed accounts name `/work/rich/...` folders that do not exist on a test machine (Python's golden run had them), so the tests point the copy's account rows at temp folders.

`easy_personal_core::plan::improvement_plan(conn, now_micros, &rules)` groups the fired rules by priority (items in `evaluate` order) and returns the `improvement_plan` JSON equal to the Python goldens in `parity/golden` and `parity/golden-rich`. `plan::improvement_plan_scoped(conn, now_micros, &rules, Scope)` is the same over `evaluate_scoped` (same error type).

`easy_personal_core::catalog` (T046) is the provider catalog: `entries()` parses the five files embedded from `crates/core/catalog/` (byte-for-byte
copies of Python's `src/easy_personal/catalog/*.yaml`; a test compares them) with a hand-written parser of Python's line format (no YAML crate);
`parse(id, text)` accepts what Python's `catalog.parse` accepts and refuses with its `CatalogError` texts. `easy_personal_core::suggestions` has the
read views `catalog_view`, `status`, `suggestions(conn)` and `connectors_list(conn)`; `configured_ids()` is the one place that will ask the keychain
(T050) and is empty today. `tests/connectors_parity.rs` compares them with the 8 goldens (4 per oracle), 495 parser cases and 3,000 `shlex` strings
answered by Python (`parity/gen_connectors_cases.py` makes `tests/connectors_cases.json`), and `crates/app/tests/connectors.rs` replays the route and
the tools over the binaries, with a fake `claude` first on `PATH` that must never run. Where this build differs from Python on purpose
(hostile input; all in the catalog parser, so only a file that is not the shipped one can reach them):
- A `command` line with an unclosed quote or a trailing backslash: Python raises an uncaught `ValueError` from `shlex`; here it is a `CatalogError`
  (`<id>: command has an unclosed quote or a trailing backslash`).
- A catalog file that is not UTF-8: Python's `read_text()` raises `UnicodeDecodeError`; here a `CatalogError` (`<id>: the catalog file is not valid UTF-8`).
- The pinned-version rule `\d+\.\d+\.\d+`: Python's `\d` also matches other Unicode decimal digits (`１.０.０`); here only ASCII digits pin a version.
- `load(id)` of Python (an id the caller names) is not ported: nothing in the read surface takes one, and Python's text echoes the id.
- The route and tools never print a command line, an env value or a path; an entry error names the entry id (and, for an unknown key, the key).

## Store writer (P2)

The read-only `store` module (`store-info`, `serve`, `mcp`) still never writes. Writing is a separate module, `easy_personal_core::writer`
(T027), with the lock in `ingest_lock`; the `ingest` command (T029, below) calls them. `writer::open_for_write(path)` creates a
new store from `src/schema_v11.sql` (a byte-equal copy of `parity/ingest/golden/schema-v11.sql`) when there is no file, an empty file or
no table, opens a `user_version` 11 store, and refuses every other store before writing a byte: newer -> the reader's "newer" text, older ->
`store is schema v<N>: run the Python app once to migrate it`, both exit code 2. It never migrates. Every connection gets
`busy_timeout=5000`, `foreign_keys=ON`, `journal_mode=WAL`; `meta.machine_id` is read first and only created when absent.
`Store::save_file` saves one file's events (Python's conditional upsert), the session-entrypoint back-fill, its tool signals and its offset in
one transaction and returns the rows added (`id > MAX(id)` as Python); `retire_files` and `prune(days, now_micros)` copy `store.py`.
`ingest_lock::ingest_lock(data_dir, timeout)` takes `flock(2)` on `<data_dir>/ingest.lock` (std `File::try_lock`), so it excludes the
Python ingest, and creates the data folder 0700. A Python app opens a store this module created without migrating it
(`tests/writer_python.rs`, ignored: needs `parity/.venv`).

## Ingest parse layer (P2a)

`easy_personal_core::ingest::claude_code::read_lines` (and the streaming `LineReader`) turn the bytes of one transcript file into the
events Python's `claude_code` source would store: line framing, `json.loads` leniencies, UTC timestamps, labels, the Bash first-word and
WebFetch-host signals. It keeps metadata only; no message text, thinking, tool input or output ever reaches an `Event`. It lives in
`crates/core/src/ingest/` and is proven against the Python oracle in `parity/ingest/` (goldens S01 and S06, and two differential
corpora, `corpus-time.json` and `corpus-label.json`, regenerated with `parity/gen_ingest_corpus.py --check`). Rust differs from Python
only where Python crashes, and abstains the record (or drops the timestamp) instead, then reads on:

- nesting deeper than 57,890 levels;
- an integer beyond int64 in a usage field;
- a list-valued (or object-valued) `type`;
- a timestamp whose UTC value leaves years 1-9999;
- a `cwd` holding a lone surrogate (Python raises `UnicodeEncodeError` binding the row).

## Ingest of folders (P2a)

`easy_personal_core::ingest::folders::ingest_accounts(store, opts)` runs Python's `accounts.detect`, gives untagged rows to the default
account, then reads every confirmed Claude account's `projects/` folder (or `opts.projects_override`). It finds every `*.jsonl`
(sorted by path components, folder symlinks not followed), resumes each file from its stored offset (back to 0 for a new inode or a
file shorter than the offset), resolves each `cwd` to its project through `git` (a worktree maps to its main repo, a relative `cwd`
is read from `opts.cwd_base`), and saves each file in one transaction. It returns `files` and `new_events` as Python prints them, plus
`skipped`. The `ingest` command (T029, below) calls it. Its tests rebuild the T026 oracle cases and match the goldens' tables
(`tests/ingest_folders_parity.rs`). A real run shows that Rust and Python share one store, and the second ingest adds 0
(`cargo test -p easy-personal-core --test ingest_folders_python -- --ignored`, needs `parity/.venv`). Where Python crashes and stays
stuck, Rust skips the entry, counts it in `skipped` and reads on: a folder named `x.jsonl`, an entry it cannot list, stat or read,
or a path that is not UTF-8.

## Ingest (P2a, P2b)

```bash
easy-personal-rs ingest [--source PATH] [--retention-days N]
```

Does what `easy-personal ingest` does, except `context`: takes the single-writer lock, opens the store, detects
agent accounts, reads every confirmed Claude account's transcripts from the stored offsets, optionally deletes events older than `N`
days, then reads the `kit_trace`, `ui` and `verifier` sources (T033), and prints one JSON object (`indent=2`, sorted keys, a trailing newline, as Python's `json.dumps(..., indent=2, sort_keys=True)`):

```json
{
  "files": 6,
  "kit_trace": {
    "files": 2,
    "new_events": 17
  },
  "new_events": 39,
  "not_ingested": [
    "context"
  ],
  "skipped": 0,
  "ui": {
    "sessions": 3,
    "skipped": 0,
    "status": "ok"
  },
  "verifier": {
    "projects": 1,
    "skipped": 0,
    "status": "ok"
  }
}
```

`pruned` (rows deleted) is added with `--retention-days`. `--retention-days` is a positive integer; `0` or a negative number is a usage error.
Stdout carries only the JSON; diagnostics go to stderr. No message holds a path, SQL or a row value.

**Environment** (same names and meaning as the Python app):

| Variable | Meaning | Default |
|---|---|---|
| `EASY_PERSONAL_HOME` | data folder (store `store.sqlite`, lock `ingest.lock`); made with mode 0700 when missing, an existing one keeps its mode | `~/.easy-personal` |
| `EASY_PERSONAL_AGENT_ROOT` | folder scanned for `.claude*` / `.codex*` agent accounts | `~` |
| `EASY_PERSONAL_CLAUDE_PROJECTS`, `--source` (the flag wins) | replace the default account's `projects` folder; a relative path is read from the current folder | unset |
| `EASY_PERSONAL_LOCK_TIMEOUT` | seconds to wait for the lock (a float) | `300` |
| `EASY_PERSONAL_UI_REPORTS` | easy-ui report folder (`session-*.json`); else `reports_dir` under `ui:` in `<data folder>/config.yaml`; unset means the UI checks have no data | unset |
| `EASY_PERSONAL_VERIFIER_REPORTS` | easy-verifier report container (`<repo folder name>/evidence-report-*.json`); else `reports_dir` under `verifier:` in `config.yaml` | `~/.easy-verifier-reports` |

**Order**: lock, then store, then accounts, then transcripts, then prune, then print. A busy lock never touches the store. The lock is released before the result is printed, like Python. A missing store is
created (schema v11).

**What is and is not ingested.** Ingested: the `claude_code` transcripts (events, tool signals, file offsets, accounts), then, in Python's order and each in
its own transaction, the three kit sources, with Python's rules and the same schema-v11 tables (T033; the oracle is `parity/gen_ingest_kit.py`):

- **`kit_trace`**: for each project that has a `claude_code` event and still exists as a folder, the files `<project>/memory/event-trace/*.jsonl` (sorted).
  Only `timestamp`, `tool_name` and `is_error` of a line are read (never `summary`); the file name is the kit task id (`_untagged` means none). A line that
  cannot be read is stored as an abstained row; a blank line is skipped; a last line without a newline waits. Same offset, replaced-file, shrunk-file and
  retire rules as the transcripts. Keys `files`, `new_events`.
- **`ui`**: `session-*.json` of the UI folder (files over 20 MB are skipped). Only `id`, `status`, `startedAt`, `endedAt` and the number of `actions` are kept.
  The rows are replaced (`INSERT OR REPLACE`, the later file wins for one id) and **never deleted**, except that an unset or missing folder deletes every row
  and answers `no data`; an existing empty folder does not. Keys `status`, `sessions`, `skipped`.
- **`verifier`**: per existing project the newest `evidence-report-*.json` (by the UTC stamp in the name) of `<project>/reports/` and, when the project's folder
  name is unique, of `<container>/<folder name>/`. Only `schema_version` 1 is read (another integer version gives `unknown`); a symlink, a non-regular file or
  a file over 1,000,000 bytes is skipped. The table is replaced in one transaction: any failure rolls back, keeps the old rows and answers
  `{"status": "failed", "reason": <error kind>}`. Keys `status`, `projects`, `skipped` (`no data` empties the table, as Python).

**Not ingested** (named in `not_ingested`, never printed with a made-up value): `context` (an HTTP client of easy-context; there is no `context` key). Not built: `live`,
the dashboard's start-up ingest (the daily timer is in "Scheduled ingest" below). `skipped` counts entries Rust could not read (a folder named `x.jsonl`, an unreadable entry, a path
that is not UTF-8, a trace entry that is not a regular file); the run goes on and a one-line note goes to stderr.

| Exit code | Meaning |
|---|---|
| 0 | done |
| 75 | another ingest holds the lock after the wait; stderr is exactly `another ingest is running; try again later` |
| 2 | a usage error, or a store that must not be touched: newer than v11 (the "newer" text) or older (`store is schema v<N>: run the Python app once to migrate it`); the store is not changed |
| 1 | any other failure, one line (a malformed `EASY_PERSONAL_LOCK_TIMEOUT`, an unusable data folder, a store that is not SQLite, a store busy for 5 s, a stdout that cannot be written) |

**Coexistence**: one writer at a time, enforced by the same `flock` on `ingest.lock` the Python app takes. Rust and Python share one v11 store:
after either ingests, the other adds 0 events (`cargo test -p easy-personal-rs --test ingest_cli -- --ignored`, needs `parity/.venv`). The Rust
ingest never migrates: run the Python app once to move an older store to v11.

**Coexistence tests** (T031, `crates/app/tests/ingest_coexist.rs`): real processes on one store. Two ingests started
at one barrier (Rust ‖ Rust; Rust ‖ Python and Python ‖ Python in the ignored half), 20 times each, end with the
golden S01 / S06 tables, `integrity_check` ok and no duplicate key, and one run always finds 0 new events (or exits 75
with a zero lock wait). A SIGKILL at fixed and seeded points (`KILL_SEEDS`) leaves whole files only, and a rerun by
either side with a zero lock wait equals a clean run. A SIGSTOPped holder makes the next ingest exit 75. `serve`,
`mcp` and `store-info` answer while an ingest runs. Copies (the inode in the key duplicates rows on both sides, as
golden S13 shows), v10 / v12 stores and odd data folders are covered too. A negative control bypasses the lock (two
data folders, one store via a symlink) and checks that the race test then sees two writers. Children run in their
own process group and are reaped, and their group is checked empty by PID. A guard refuses to start a child whose
`EASY_PERSONAL_HOME` or `EASY_PERSONAL_AGENT_ROOT` is unset or outside the temp folder. Keep `TMPDIR` short and on the
Linux filesystem (not `/mnt`).

```bash
cargo test -p easy-personal-rs --test ingest_coexist                   # Rust half
cargo test -p easy-personal-rs --test ingest_coexist -- --ignored      # Python half (needs parity/.venv)
```

Not covered: the Python dashboard's live tail (a continuous writer), a Python ingest killed inside its own migration.

**Differences from Python** (each on purpose):

| Python | Rust | Why |
|---|---|---|
| prints `kit_trace`, `context`, `ui`, `verifier` | prints `kit_trace`, `ui`, `verifier` as Python does, `not_ingested: ["context"]`, and `skipped` | `context` is not ported (T033); no invented numbers |
| a trace line nested past ~57,900 levels or with a time outside years 1-9999, a `ui` file with such a time: traceback, stuck on every later run | an abstained trace row / a skipped `ui` file | T033, same rule as T026/T028 |
| a FIFO named `*.jsonl`, `session-*.json`: blocks for ever | skipped and counted | T033 |
| `~user` in a report folder is expanded from the password database; a stamp with non-ASCII digits is a stamp | `~user` is left as written; not a stamp | T033 |
| a verifier report with a fractional score that is UTF-16/32, nested deeper than 128 levels or holds `NaN` beside it: read | skipped (the fraction is read by `serde_json` from the same bytes) | T033, listed in the `verifier.rs` header |
| a store newer than v11: traceback, exit 1 | the clean message, exit 2, nothing written | decided in T026/T027 |
| an older store: backs it up and migrates it | exit 2, `store is schema v<N>: ...`, nothing written | Rust never migrates (Python owns the schema) |
| a folder named `x.jsonl`, an entry it cannot read, an int beyond int64, a record whose `type` is a list: traceback, exit 1, stuck on every later run | skips the entry (counted in `skipped`) or abstains the record, reads on | see "Ingest parse layer" and "Ingest of folders" |
| a malformed `EASY_PERSONAL_LOCK_TIMEOUT`: traceback | one line, exit 1, before anything is made | no traceback |
| `--retention-days` larger than int64: Python integer | a usage error, exit 2 | clap parses an `i64` |

## Scheduled ingest (P2c)

```bash
easy-personal-rs schedule install [--retention-days N] [--dry-run]
easy-personal-rs schedule remove [--dry-run]
```

Runs the daily ingest on a **systemd user timer** (Linux), the way `easy-personal schedule` does: the same two unit files
(`easy-personal-ingest.service` and `.timer`, `OnCalendar=daily`, `Persistent=true`), written to `$XDG_CONFIG_HOME/systemd/user/`
(else `~/.config/systemd/user/`), then `systemctl --user daemon-reload` and `systemctl --user enable --now easy-personal-ingest.timer`.
`--dry-run` prints the units (or, for `remove`, what would go) and writes nothing. `--retention-days N` is passed on to the scheduled
`ingest`. The templates in `crates/app/units/` are a copy of `src/easy_personal/data/systemd/*`; a test compares the bytes.

- **The command line is the Rust binary**: `ExecStart=<absolute path of the running binary, shell-quoted> ingest [--retention-days N]`.
  Install from the binary you want to keep: if you move or delete it, the timer stops working until you install again.
- **Same unit names as Python**: installing from Rust replaces a Python-installed timer, and the reverse. This is deliberate: one
  scheduled ingest per machine. `schedule remove` from either one removes it.
- **No systemd user session** (no `systemctl`, or `systemctl --user show-environment` fails): nothing is written, the message and a
  cron line (`17 3 * * * <command> ingest`) are printed, exit 1. Cron is never installed.
- **`remove`** runs `disable --now` only when the timer file existed, deletes both files (a missing one is fine), then runs
  `daemon-reload` only if it had been installed.
- A failing `systemctl` call or a folder that cannot be written is exit 1 with one line that names no path and none of the tool's output
  (Python dies with a traceback there). Bad arguments are exit 2. A `--retention-days` above int64 is a usage error (Python takes any integer).
- Nothing here opens the store or the data folder. When neither `XDG_CONFIG_HOME` nor `HOME` is set it stops with a message (Python reads
  the passwd entry).

Test it without touching your machine: `cargo test -p easy-personal-rs --test schedule_cli` runs the binary in throwaway folders with a
fake `systemctl` first on a `PATH` of its own (see `parity/README.md`, "Schedule oracle").

## Kit (T042)

`easy_personal_core::kit` (read-only; `kit.rs`) ports `kit_summary`, `workspace_rollup` and `report_folders` of Python's `core/kit.py` with the parts they call
(`git_local`, `ui`, `verifier`, `context`). `kit_summary(conn, now_micros, Scope, hide_gone)` lists every project seen in usage (in the scope, a gone worktree
as its owner project) by folder name with its `kernel` (trace calls, errors, error rate rounded to 4 places, kit tasks, first and last time, abstained rows),
`git` (commits per OS-local day from `git rev-list --timestamp`, an argument list, 10 s limit: `abstained` when git is too slow, `no data` when it is not a
repo or fails), `context` (index time, latest commit, `stale` judged at the injected clock with the 60 s skew; `not in use` when no easy-context tool call
in the 7 days before `now`) and `verifier` parts, and the overall `ui` check summary; `hide_gone` (the dashboard route) drops the projects whose folder is
missing and adds `gone`. `workspace_rollup` gives per non-orphan workspace the verifier mean and lowest score (stored scores only; Python's `sum` is
reproduced, so the mean rounds alike) and the easy-ui sessions and calls of the period and account. `report_folders` lists per workspace repo the folders directly
inside it with `report` in their lowercase name (at most 8 per repo, by name, symlinks and non-directories skipped): the repo folder name, the report folder
name, a file count (regular files to 3 levels, 2000 at most, `files_capped` past it), the number of `session-*.json` files and the newest file time. It reads
names and `lstat` only: no file is opened, no symlink followed.

What an answer can show: the project folder name (last path component), the workspace name and colour, the repo folder name and the report folder name,
counts and times. Never a full path, a file name, a commit message or an author. No error body or log line carries a path, SQL or caller text.

Tests: `crates/core/tests/kit_parity.rs` replays the 17 goldens of `golden-rich` (7 `api__kit__*`, 3 `api__kit_workspaces__*`, `api__kit_reports__all`, 6
`mcp__kit_summary__*`) and the 4 of `golden`, over a temp copy of the rich store whose `/work/rich/kit` paths point at a temp copy of `fixtures-rich/kit`
(the temp folder must sort before `/work/rich`), and a hand-made store (`tests/kit_extra/extra.sql`, four git repos, report folders) against Python's answers
(`tests/kit_extra/expected.json`, made by `make_expected.py` under `TZ=UTC`; `commits_by_day` is checked apart under each `TZ` against the C `date`).
`crates/core/tests/kit_abstain.rs` (20 s: it waits for the 10 s limit twice) proves a hung `git` is `abstained`; `crates/app/tests/kit.rs` runs the three
routes and the tool over the real binaries (guard, 405, 400 texts, golden equality, `tools/list` text).

Where this port differs from Python (Python raises there, so its HTTP answer is a 500 or a crash):
- a stored score that is not a finite number is `null` (and a workspace mean that is not finite is `null`), a NULL `ts_utc` of an easy-ui event counts as the empty
  string, a stored score with a NULL `generated_at_utc` sorts before any time (Python compares `None` with text and raises), a `git rev-list` line that is not
  `<seconds> <hash>` is skipped, and a report or repo folder whose name is not UTF-8 is not listed;
- a score whole-valued and above 9e18 stays a float (Python prints a long integer); a file time is read through the platform's `SystemTime`, as
  `lstat`'s `st_mtime` float (`sec + nsec * 1e-9`, rounded half to even to the microsecond, then the second shown) and a time outside years 1..9999 is `null`, as Python's `_file_utc`;
- an unreadable store problem is the fixed 503 text of the other routes, not Python's traceback;
- a timeout is seen by `git rev-list` only (`abstained`); the `git log` of the index check keeps the other kit rules' behaviour (a slow git gives no commit time).

## Report files (T045)

`easy_personal_core::report_files` (read-only; `report_files.rs`) ports `report_files` and `report_file` of Python's `core/kit.py` (FR-090, NFR-002) and is served as
`GET /api/kit/report-files` and `GET /api/kit/report-file`. Both take `workspace` (a name), `repo_no` (the place of the repo in the workspace's repo list, as
`/api/kit/reports` shows it: plain ASCII digits, fewer than 9, else a miss) and `folder` (a report folder name of that repo, by the `/api/kit/reports` rule);
`report-file` also takes `path` (relative to the report folder, `/` separated, at most 3 parts and 512 characters, an extension of the table below). The last value of
a repeated key is used; a missing key is a miss. The orphan workspace has none.

- `report-files` -> `{"files": [{"path", "size", "openable", "mtime_utc"}], "truncated": bool}`: the regular files to 3 levels, newest first (then by path), 200 at most
  (2000 counted); `openable` is "extension in the table and size at most 10 MiB".
- `report-file` -> the bytes with `content-type` from the extension (`html`/`htm` `text/html; charset=utf-8`, `md` `text/markdown; charset=utf-8`, `txt`
  `text/plain; charset=utf-8`, `json` `application/json`, `png` `image/png`, `jpg`/`jpeg` `image/jpeg`, `pdf` `application/pdf`; never from the content), `x-content-type-options:
  nosniff`, `cache-control: no-store`, `content-disposition: inline`, and the CSP the server adds to every answer.
- Errors, fixed bodies that never echo the input: `404 {"error": "not found"}` for every miss and every refusal, `413 {"error": "too large"}` for a file over 10 MiB,
  `503` the store's fixed text, `403` the guard (before anything), `405` any method but `GET`/`HEAD` (`HEAD` is `GET` without a body).

**Path safety.** The repository folder comes from the store row, never from the request. The report folder, each sub-folder and the file are each opened **relative to the
handle of the level above** with `O_NOFOLLOW` (the std library has no `openat` and this task adds no crate, so a level is opened through `/proc/self/fd/<handle>/<name>`, which
the kernel resolves as `openat`): no level can be a symlink, whatever changes between the check and the open. The file is opened `O_NOFOLLOW | O_NONBLOCK` (a FIFO never
blocks), then judged on the open handle: a regular file with one link, size at most the cap, and at most cap + 1 bytes read from the handle (a file that grew is a 413). The listing
walks by handle too. A request path with `..`, a leading `/`, a NUL, a backslash, an empty part or a part that starts with a dot is a 404 before any file system call. The flags
are the Linux values for x86, x86-64, arm, aarch64 and riscv64; on any other target (macOS, Windows: later phases) the module compiles and every call is a 404.

Tests: `crates/core/tests/report_files_parity.rs` (the 9 rich goldens, the attack table of 53 cases against `rust/parity/golden-report-files/`, the race, the FIFO, the cap),
`crates/app/tests/report_files.rs` (9 goldens over the binary with headers, guard, 405, `HEAD`, query variants, encoded and double-encoded `..`, 413, 503), and `leak_rust.py`
(both routes registered). The oracle is made by `parity/gen_report_files.py` (Python's own functions over a temp tree; needs the parity venv and `PYTHONPATH=src`).

Attack table, case by case (Python's answer over the same tree / Rust's): symlink to a file outside 404 / 404; symlink to a folder outside 404 / 404; symlink inside to a file
inside 404 / 404 (Python refuses it too); FIFO 404 / 404, no block; folder named like a file 404 / 404; file of the cap 200 / 200; cap + 1 and a sparse 100 MB 413 / 413; a `..`,
absolute, backslash, NUL, empty, `.` or trailing-slash path 404 / 404; depth 4 404 / 404; upper-case extension 200 / 200; a name with a newline and a 255-byte name 200 / 200; `repo_no`
`-1`, `99999999999`, `1e3`, Arabic-Indic one 404 / 404, `000` 200 / 200; a file swapped for a symlink (or its folder, or the report folder) after the listing: refused (Rust-only test).
Equal in 48 of 53; the 5 below are decided.

Decided for you (Rust differs from Python on purpose):
- a file with **more than one hard link** is a 404 (Python serves it; a hard link to a file kept elsewhere is a name for outside content). The cost: a report file that something else
  also links to is listed as openable but not served. The case table has `hard.html` (to a file outside), `hard-in.html` and `linked.html` (both names of one file inside);
- a path **part that starts with a dot** is a 404 (Python serves `.hidden.html`; `.html` alone, with no extension, is a 404 in both). The listing is Python's, so `.hidden.html` is listed as openable;
- a **name that is not UTF-8** is left out of the listing and cannot be asked for (Python's JSON encoder fails on it: a 500 with a traceback). A report folder or repo whose name is not UTF-8 was already unlisted;
- `/proc` must be mounted (a container without it answers 404 to every call: fail closed);
- the repo folder is opened following symlinks (the owner put it there, as Python does), but the report folder under it, and every level below, may not be a symlink.

## Folder browser, scheduled jobs and kit findings (T044)

Three read-only surfaces, each equal to Python's answer (`parity/golden-jobs/` is made once by `parity/gen_dirs_jobs.py` from Python's own `list_dirs` and
`list_jobs` with the fake `systemctl`/`crontab` of `tests/fakes/jobs_fakes.py`; the four `mcp__kit_findings__*` goldens are Python's).

| Surface | Parameters | Answer / errors |
|---------|------------|-----------------|
| `GET /api/dirs?path=` | `path`: an absolute folder; absent or empty = the server's working folder (home if that was deleted) | `200 {path, parent, home, dirs: [{name, repo}], truncated?}`: sub-folder names (dot-names skipped, symlinks followed for the folder test only, sorted by case-folded name then name, 500 at most, then `truncated: true`); `repo` is true when `<dir>/.git` exists. `400 {"error": "Path must be absolute"}`, `400 {"error": "Not an existing folder"}` (also a file, a broken link, a link loop, a NUL byte), `403 {"error": "Cannot read this folder"}`. No store is opened. |
| `GET /api/jobs` | none | `200 {jobs, hidden, sources: {systemd, cron}}`; each job `{name, kind, command, schedule, next_run, last_run, last_exit}`; `command` is the first word (basename) only, and only the agent allow-list is listed (`claude`, `easy-personal`, `easy-personal-launch`, `easy-verifier`, `easy-context`, `easy-ui`, `easy-memory`); a kind that cannot be read is `"no data"`. No store is opened. |
| MCP `kit_findings` | `limit` (integer, default 3, clamped to 1..10), `since` (ISO date or datetime; no offset = local time) | the findings object of Python's tool; a store problem or a bad `since` is a tool error (`Invalid isoformat string: 'x'`). |

Privacy: `/api/dirs` shows the folder names and the absolute paths of the folder the owner browses, as Python does; it never shows a file name, a size, content or a
symlink target (a test lists a folder holding a file, a dangling link and a hidden folder and finds none of them). `/api/jobs` starts only `systemctl --user list-timers|show`
and `crontab -l` (the table `jobs::READ_ONLY`, checked on the whole argument list before any process starts), with an argument list (no shell), the caller's own environment plus
`LC_ALL=C`, a closed stdin, at most 8 MiB read and a 3 s limit (the process is killed and waited for). Unit names are validated like Python's `_UNIT` (never starting with `-`).
Every test runs `/bin/sh` fakes on a scratch `PATH`.

Decided for you (differences from Python, all on hostile or unusual input):

- `/api/dirs`: a folder name that is not UTF-8 is skipped (Python would fail the response). A path whose existing part is followed by a missing folder and `..`
  (`/missing/../tmp`) is `Not an existing folder` here (Python resolves the `..` in text and answers 200). With `HOME` unset, `home` is `/` (Python asks the account database).
  The NUL-byte text is `Not an existing folder` (Python's own text is CPython's `lstat: embedded null character in path`; status 400 and the `error` key are equal).
- `/api/jobs`: `config.yaml`'s `jobs: allow:` list is not read (the default allow-list is used); `systemctl`/`crontab` are found in `PATH` only (no `PATH` or an empty one is `no data`;
  Python falls back to `/bin:/usr/bin` when `PATH` is unset); more than 8 MiB of output, a hang past 3 s or a non-zero exit is `no data`; a JSON boolean or float in `next`/`last`
  is no time (Python would read `true` as 1 microsecond); a time beyond year 9999 is no time (Python raises).
- `kit_findings`: `limit` must be an integer (a float with no fraction is accepted; `null`, a string or a boolean is a tool error with a plain text, not pydantic's); `config.yaml`'s
  `thresholds: kit_error_rate` is not read (the default 0.04 is used).
- Python defect found: the comment in `folders.py` says a NUL byte makes `is_dir()` False; in fact `Path.resolve()` raises `ValueError` first. The route still answers 400 (with CPython's text).

Not served yet: the other Python routes (`/api/sync`, `/api/memory-graph`, `/api/live`, the accounts and connector writes, the report files). The release note
`docs/release/rust-v0.2.0.md` "not in this release" list still names these three; it is the Supervisor's to update.

## Test

```bash
cargo test && cargo clippy --all-targets -- -D warnings
```

Tests build their own temp stores and never open `~/.easy-personal`.
