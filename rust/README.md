# easy-personal-rs

Rust port of easy-personal, built in slices (see `PROJECT_SPEC.md`). Today it has these subcommands: `store-info`, `ingest`, `schedule`, `serve` (also `dashboard`), `mcp`, `app` (opens a window only in a build with `--features window`) and, since T058, `ping`, `usage`, `outliers`, `warnings`, `plan`, `kit`, `kit-findings`, `workspace` and `account` (see "CLI commands (T058)") and, since T060, `connector` and `doctor` (see "Connector and doctor commands (T060)"); the Python app
stays the working one.

## Build and run

```bash
. "$HOME/.cargo/env"        # rustup was installed without editing the shell profile
cd rust
cargo build
./target/debug/easy-personal-rs --version        # easy-personal-rs <Cargo version>
./target/debug/easy-personal-rs store-info       # or --json
./target/debug/easy-personal-rs store-check      # read-only health check of the store; --json, --full; see "Store check (T059)"
./target/debug/easy-personal-rs ingest           # copy new Claude Code transcript metadata into the store; see "Ingest (P2a)"
./target/debug/easy-personal-rs serve            # ingest first (T051), then the loopback API, Ctrl-C to stop
./target/debug/easy-personal-rs serve --no-ingest          # the same without the start-up ingest
./target/debug/easy-personal-rs serve --static-dir <DIR>   # the same, plus the dashboard's files from <DIR>
./target/debug/easy-personal-rs dashboard        # the same as `serve` (Python's name for it)
./target/debug/easy-personal-rs usage [--by day|model|project|tool] [--since D] [--until D] [--workspace W] [--account A]   # T058: JSON as Python's CLI
./target/debug/easy-personal-rs workspace list   # also add (or create), edit, delete [--yes]; and `account list|confirm|ignore|add|use`
./target/debug/easy-personal-rs connector list   # T060: also add, remove, sync, move, alias, adopt-legacy [--yes]
./target/debug/easy-personal-rs doctor           # T060: keychain, `claude`, launcher and connector entries; reads only
./target/debug/easy-personal-rs mcp              # MCP server on stdio
./target/debug/easy-personal-rs mcp --transport http [--port N]   # the same tools over loopback HTTP, see "Start-up ingest, MCP over HTTP and the context source (T051)"
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
so far are `GET /api/version` -> `{"version": "<Cargo version>"}` and the read routes described in the sections below (usage, warnings and plan, workspaces, connectors, kit and
its report files, saved reports and first-run, accounts, sync, folders and scheduled jobs, and the live stream `GET /api/live`, T056); every one is a `GET`. Every response carries the Python app's
`Content-Security-Policy`.
The guard matches the Python app: `Host` must be `127.0.0.1:<port>`, an `Origin` if sent must be `http://127.0.0.1:<port>`, and every
`/api/` path needs the `x-dashboard-token` header. One exception (T056, as Python): `GET /api/live` with no token header may send the
token as the `token` query parameter, because a browser `EventSource` cannot set headers; on every other route and method a query token
counts for nothing. Failures get `403 {"error": "forbidden"}`.
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
  Python's `/api/workspaces` (`POST`, `PATCH`, `DELETE`) are served since T048 (below, "Write routes").
  `GET /api/workspaces/<id>` is a `405`.
- `GET /api/connectors` (T046) -> `{"connectors": [...], "workspace_connectors": [...]}`. `connectors` is the provider catalog (the five entries of
  Python's `catalog/*.yaml`), sorted by category, then the claude.ai-connector entries first, then provider; each is
  `{id, category, provider, claude_ai_connector, claude_ai_link, fields: [{env, label, secret}], configured}`. `workspace_connectors` are the
  stored rows `{id, alias, provider, workspace}`, by workspace name then alias. It takes no parameter (the whole query string is ignored), like
  the workspace routes. Errors: `403` without the token or with a bad Host/Origin, `405` for `PATCH`, `PUT` and `DELETE` (the CSP still
  set; `POST` is the add, T050, below "Connector writes"), `503` for a missing, newer, busy, unreadable or table-less store (the fixed texts above), `500
  {"error": "internal error"}` if the embedded catalog ever failed to parse (a test parses every file, so this cannot ship). Fields shown, and
  why they are safe: the catalog holds public names, labels and one fixed `https://claude.ai/settings/connectors` link (never a command line,
  which Python's route does not show either); a stored row shows only a connector id, an alias, a provider id and a workspace name (the secrets
  live in the keychain, which this route never reads). **`configured` is always `false`**: Python answers `true` only when the OS keychain
  holds the item *and* `claude mcp get` knows the entry, and answers `false` when it cannot ask (no keychain backend, no `claude`), which is
  the state of the goldens. This route asks neither (T050 left the T046 read side as it was), so on a machine where Python says `true`, this says
  `false`. The connector writes and `/api/connectors/legacy` are T050 (below, "Connector writes, keychain and launcher").
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
  `accounts.detect`, which INSERTs the `.claude*` / `.codex*` folders it finds; these two `GET` handlers never do (the detect stays in `ingest`, and runs in the T048 account *write* routes), so a new agent
  folder appears after the next `ingest` or account write, not at page load. Nothing is written and no process is started (no `claude`, no keychain).
  - `/api/accounts` -> `{"by": "account", "accounts": [...]}` (`by` defaults to `account`) or `{"by": "provider", "providers": {"claude": [...], ...}}`. Each
    account is `{id, provider, label, folder, status, config_dir_env, is_default, workspaces, status_line}`: `status` is `missing` when the folder is gone and the
    account is not ignored; `workspaces` are the names it picked; `status_line` is `{email, plan, organization, last_model, last_active_utc, tokens_today,
    tokens_7d, limits}` (tokens: one row per message, since local midnight in `TZ` / the OS zone and in the last 7 x 24 h; the newest event with a model;
    the default account also owns events with no account). For a Claude account it reads exactly two small regular files of its folder, as Python does:
    three keys of `<folder>/.claude.json` (`oauthAccount`: `emailAddress`, `billingType`, `organizationName`; the default account's file may sit beside its folder)
    and `<folder>/easy-personal-limits.json` (`five_hour` / `seven_day` `used_percentage`, `resets_at`, `written_at`). Nothing else in a folder is opened or listed.
  - `/api/sync` -> `{"steps": [{account_id, account, action, alias, connector_id, line}]}`: what applying the store's picks would change in each confirmed Claude
    account (and each ignored one that still holds registrations); `?account=<id>` selects one (an empty value is no filter). The lines are data from the store
    (an account label or alias with a newline or quotes is JSON-escaped, never run). Since T050 the plan asks `claude mcp get` per entry when `claude`
    is on `PATH`, as Python does (then a step can also be `record`, `forget` or `refused`); the apply `POST /api/sync` is T050 (below); the account writes
    (`POST /api/accounts`, `.../confirm`, `.../ignore`, `PUT .../workspaces`) are served since T048 (below, "Write routes").

  | Request | Answer |
  |---|---|
  | `by` not exactly `account` or `provider` (an empty `by=` too) | `400 {"error": "by must be account or provider"}` |
  | `account` not plain ASCII digits | `400 {"error": "account must be an account id"}` |
  | `account` with no account (`0`, `99`, 20 digits) | `404 {"error": "no such account"}` |
  | no token, bad Host or Origin | `403 {"error": "forbidden"}` (before anything else) |
  | `PUT`, `PATCH`, `DELETE` on either route | `405`, empty body, CSP set (`HEAD` answers like `GET`); `POST /api/accounts` (T048) and `POST /api/sync` (T050) are writes (below) |
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

`mcp` serves the MCP protocol on stdio (newline-delimited JSON-RPC; stdout carries nothing else) with twelve tools (the eleven below, and `save_report`, T047): `ping`,
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
unknown tool is a JSON-RPC error; a line that is not JSON is answered with `-32700` and a null id. Twelve tools are listed now: the eleven above are
read-only; `save_report` (T047, below) is the one that writes. The `investigate` prompt is served (T047). Not ported yet: `--http`.

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
and links only libc, so `store-info`, `serve` and `mcp` run on a machine with no display and no desktop libraries (the T050 keychain
talks D-Bus in pure Rust). `cargo build
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
| `--self-test` saw no authenticated request within 20 s, or its live check failed | 3 |
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
- **`--self-test`** (hidden from `--help`, for checks without eyes): waits until the page made one `/api/` request that passed
  the guard with the token (else exits 3 after 20 s), closes the window, then makes the **live check** (T057): the live feed was
  given to the server and its thread runs, and `GET /api/live?token=<token>` (the request the page's `EventSource` makes) answers
  `200 text/event-stream` with a first `snapshot` whose data has exactly `active_window_s`, `last_id`, `open_call_minutes` and
  `sessions` (about 5 s limit; a server that drips bytes can stretch it to about 10 s). It then prints `self-test: live snapshot ok` and `self-test: ok` and exits 0; a failed check prints
  `easy-personal-rs: self-test: live check: <reason>` (never the token) and exits 3. The same check runs without a display in
  `server.rs`'s unit tests (on `server::start` with a feed, and refused with no feed or a wrong token); the window test
  `the_self_test_reads_the_first_live_snapshot_with_the_token_in_the_query` runs when `DISPLAY` or `WAYLAND_DISPLAY` is set and
  prints `skipped: no display` otherwise. The check proves the server, the feed and the query-token route; it does not prove the
  webview's own `EventSource` (the fixture page makes one `fetch` only). The fixture page is
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

The read-only `store` module never writes. `store-info` never writes either; `serve` writes only inside the seven write routes of T048 (never at start-up, never on a `GET`) and `mcp` only in `save_report` (T047). Writing is a separate module, `easy_personal_core::writer`
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

Does what `easy-personal ingest` does: takes the single-writer lock, opens the store, detects
agent accounts, reads every confirmed Claude account's transcripts from the stored offsets, optionally deletes events older than `N`
days, then reads the `kit_trace` (T033), `context` (T051), `ui` and `verifier` sources, and prints one JSON object (`indent=2`, sorted keys, a trailing newline, as Python's `json.dumps(..., indent=2, sort_keys=True)`):

```json
{
  "context": {
    "indexes": 2,
    "status": "ok"
  },
  "files": 6,
  "kit_trace": {
    "files": 2,
    "new_events": 17
  },
  "new_events": 39,
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

- **`context`** (T051): easy-context's index freshness, read through its loopback MCP service; see "Start-up ingest, MCP over HTTP and the context source (T051)" below. Keys `status`
  (`ok`, `no data`, or `failed` when the store step fails), `indexes`.

**Not ingested**: nothing of Python's `ingest` list is left out (T051 ported `context`; the JSON has no `not_ingested` key any more). Not built: `live` (the dashboard's tail is the T056 feed,
which never writes the store). `serve` and `app` run this ingest before they start (T051, below); the daily timer is in "Scheduled ingest" below. `skipped` counts entries Rust could not read (a folder named `x.jsonl`, an unreadable entry, a path
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
| prints `kit_trace`, `context`, `ui`, `verifier` | prints the same four keys (T051 ported `context`) and `skipped` | one JSON, Python's keys |
| a trace line nested past ~57,900 levels or with a time outside years 1-9999, a `ui` file with such a time: traceback, stuck on every later run | an abstained trace row / a skipped `ui` file | T033, same rule as T026/T028 |
| a FIFO named `*.jsonl`, `session-*.json`: blocks for ever | skipped and counted | T033 |
| `~user` in a report folder is expanded from the password database; a stamp with non-ASCII digits is a stamp | `~user` is left as written; not a stamp | T033 |
| a verifier report with a fractional score that is UTF-16/32, nested deeper than 128 levels or holds `NaN` beside it: read | skipped (the fraction is read by `serde_json` from the same bytes) | T033, listed in the `verifier.rs` header |
| a store newer than v11: traceback, exit 1 | the clean message, exit 2, nothing written | decided in T026/T027 |
| an older store: backs it up and migrates it | exit 2, `store is schema v<N>: ...`, nothing written | Rust never migrates (Python owns the schema) |
| a folder named `x.jsonl`, an entry it cannot read, an int beyond int64, a record whose `type` is a list: traceback, exit 1, stuck on every later run | skips the entry (counted in `skipped`) or abstains the record, reads on | see "Ingest parse layer" and "Ingest of folders" |
| a malformed `EASY_PERSONAL_LOCK_TIMEOUT`: traceback | one line, exit 1, before anything is made | no traceback |
| `--retention-days` larger than int64: Python integer | a usage error, exit 2 | clap parses an `i64` |

## Start-up ingest, MCP over HTTP and the context source (T051)

### `serve` and `app` ingest first

`serve` and `app` (a `--features window` build) run the same ingest as the `ingest` command (no `--source`, no retention; the same lock, `EASY_PERSONAL_LOCK_TIMEOUT` or 300 s) before they bind a socket, as
Python's `dashboard` does. They print one line on stderr, `ingesting before start (waits if another ingest runs)...`, and nothing else on success (no JSON, no path, no row value). The live feed's poll thread
starts after the ingest. If the ingest fails (the lock stays busy for the whole wait, a store newer or older than v11, a malformed lock timeout, a store that cannot be opened), stderr gets one fixed line,
`ingest failed; starting anyway`, and the server still starts. `--no-ingest` skips the step (also on `app`). A build without `--features window` runs no ingest for `app`, because it starts nothing.

### `mcp --transport http`

```bash
easy-personal-rs mcp [--transport stdio|http] [--host 127.0.0.1] [--port 8765]
```

`http` serves the same 12 tools and the `investigate` prompt as stdio, at `POST http://127.0.0.1:<port>/mcp` (no token, as Python's FastMCP). `--port 0` picks a free port. stderr gets
`MCP over HTTP: http://127.0.0.1:<port>/mcp` and `Ctrl-C to stop.`; stdout stays empty. Ctrl-C stops the server with exit 0: open requests get 3 s, then the connections are dropped (a client stuck mid-request cannot keep it alive).

| Input | Result |
|---|---|
| a host other than `127.0.0.1` (`0.0.0.0`, `localhost`, `::1`, a name), with `stdio` too, as Python | exit 2 and one line, before any socket is bound |
| a bad `--port` (`abc`, 65536, -1) or `--transport` | a clap error, exit 2 |
| the port is in use | `easy-personal-rs: cannot bind 127.0.0.1:<port>: Address already in use (os error 98)`, exit 1 |
| any `Host` value that is not `127.0.0.1:<port>` or `localhost:<port>` (DNS rebinding; every duplicate value is checked) | 421 |
| any `Origin` value that is not `http://127.0.0.1[:port]` or `http://localhost[:port]` (any port, as Python) | 403 |
| a connection that has not sent its whole request (header and body) 10 s after it opened or after the last answer; an idle one | closed (the clock stands still while a tool exchange runs) |
| more than 64 connections open at once | the extra one is closed at once |
| a path other than `/mcp` | 404; `GET`, `DELETE`, `PUT` on `/mcp` are 405 (`Allow: POST`) |
| `Content-Type` other than `application/json` / `Accept` without `application/json` | 415 / 406 |
| a body over 1 MiB / not JSON / not a JSON-RPC message | 413 / 400 (`-32700`) / 400 (`-32600`) |

The transport is **stateless**: each POST is one JSON-RPC message with its own short-lived `rmcp` service (the handshake runs inside the request), so no session id is issued and a client can
call a tool without `initialize` first. A notification gets 202 with no body. The answer is `application/json` (Python's FastMCP answers with an SSE body; the spec allows both, and the Python
MCP client reads this server). `rmcp`'s own streamable-HTTP server was not used: its feature adds four packages the lock does not have (`rand`, `tokio-stream`, `sse-stream`, `async-trait`), so
`Cargo.toml` and `Cargo.lock` are unchanged. Tests: `tests/mcp_http.rs` (the answers equal stdio's, the guard table above, the refusals, Ctrl-C, port in use, listening on `127.0.0.1` only).

### The `context` source

`context` reads easy-context's index freshness (`ingest::context`, Python `sources/context.py`) and stores a project's `last_index_at` in `context_index` (a project that has a `claude_code` event, is listed by the
service with a `root` equal to its path and is not `root_exists: false`).

- **Read-only, allow-listed**: the client sends `list_indexes` and `get_stats` only; any other tool name is refused before a connection.
- **Loopback only**: the URL (`EASY_PERSONAL_CONTEXT_URL`, empty or unset means `http://127.0.0.1:8767/mcp`) must be `http` and host `127.0.0.1`. A name (`localhost`), `0.0.0.0`, an IPv6 literal, `https`, user info (any `@`),
  port 0 or above 65535, or a space, control or non-ASCII character is refused before a connection. Redirects are not followed. No TLS client exists.
- **No data, never a failure**: the service down, slow (3 s per socket operation and 6 s for the whole call, as Python's `wait_for`), a bad status, an unreadable, oversize or JSON-RPC-error reply, or a tool error on
  every root clears `context_index` and gives `{"status": "no data"}`; the run still exits 0. Up to 3 existing project folders are tried as the root header, as Python (the next one only after a tool error).
- **No index content is kept or printed**: only `root`, `last_index_at` and `root_exists` are read. A reply over **1 MiB** (1,048,576 bytes, pinned by a test) is "no data" (Python has no cap; our choice).
- **The client** is a small HTTP/1.1 client over `std::net::TcpStream` (`Connection: close`, about 450 lines, rustfmt-formatted, tests apart: the URL check, the exchange, chunked and SSE reading, the MCP calls): it does `initialize`, `notifications/initialized`, `tools/call` and a best-effort `DELETE` of
  the session, reads a JSON or SSE reply, plain, `Content-Length` or chunked. It needs no crate. A reqwest or hyper client would add a large tree (the rejected option; the owner can still ask for it).

Oracle: `parity/gen_context.py` runs Python's `context.ingest` against a scripted fake service for 41 cases (normal list, empty list, a root that does not exist, the service down, slow, 11 bad replies, a rejected root and the 3-tries limit,
SSE, chunked, a stateful session, 25 forms of `last_index_at`, 9 refused URLs) and 5 tool-name cases, and writes `parity/golden-context/golden.json`; the "normal" case must also give the same rows against the real
FastMCP fake of `tests/fakes/mcp_server.py`. `crates/core/tests/context_parity.rs` plays the same cases against a Rust fake and compares the stored rows, the tools and root headers the service saw and the sessions it was asked to end.
`crates/app/tests/context_ingest.rs` runs the `ingest` binary against the fake (down, refused URLs, a canary in the index).

### Decided for you (T051, for the owner's review)

1. `--no-ingest` exists on `serve` and `app` (Python's `dashboard` has no such flag). Python's `dashboard` stops with the ingest's exit code when the ingest fails (75 on a busy lock) and prints the ingest's JSON; Rust starts anyway and prints one progress line and, on a failure, one fixed line.
2. `mcp --transport http` is stateless and answers JSON, not SSE; `GET` and `DELETE` are 405; there is no `Mcp-Session-Id`. `--port` is a `u16` (Python: any integer, a crash above 65535). A `Host` with another port is refused (Python allows any port).
3. A `last_index_at` that Python cannot turn into a time (a number outside years 1-9999: Python's ingest raises and rolls back) is stored as an empty time, and a root listed twice keeps the later entry (Python raises on the duplicate). A reply that Python reads and Rust does not: a body over 1 MiB, a JSON nested more than 128 levels, `NaN`.
4. The URL rules are stricter in three ways than Python's (any `@`, any space/control/non-ASCII character, port 0 are refused). A project folder whose path is not printable ASCII cannot be sent as the root header: Python's header encoder raises and the whole `context` ingest is "no data"; Rust skips that folder only (it still uses one of the 3 tries) and tries the next. With one such project the result is the same as Python's.
5. The start-up ingest runs before the static folder is checked, so a bad `--static-dir` is reported after the ingest.
6. The HTTP server has a 10 s request-read limit and a cap of 64 open connections (Python's FastMCP/uvicorn has no such limit); the limit is a total per request, not per read, and does not run while a tool exchange is in flight. Writes to a slow reader have no timeout.
7. Every existing test that starts `serve` now passes `--no-ingest` (they test a read-only server over a fixed store); `startup_ingest.rs` tests the default.

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

## `save_report` and the `investigate` prompt (T047)

**`save_report`** `{summary, fixes, workspace?, account?}` -> `{"id": n, "saved_utc": "YYYY-MM-DDTHH:MM:SSZ"}`: the first MCP call of this build that
**writes**, and it writes one row of the store's `investigate_reports` table (what the dashboard's Warnings & plan page lists: `GET /api/reports`, in Python
and in this build). Python's limits and refusal texts, byte for byte (`core/reports.py`): `summary` 1 to 2,000 characters (characters, not bytes), at most 5
`fixes`, each an object with `text` (1 to 300) and optional `tool` and `number` (at most 100 characters when text; a number is stored as sent; `""` and `null`
count as absent; a bool, list or object is refused), no other key; text that looks like a secret is refused (`secret.rs`: Python's `SECRET` written by hand, no new
crate; compared with Python on 86 strings); an unknown `workspace` or `account` is refused. Every refusal names the field, never the value, and ends
`; nothing was saved`. The row also holds the **rule snapshot**: every rule of the shipped pack measured now for the scope (as `/api/warnings`; the `config.yaml`
threshold override is still not ported), in Python's JSON text (sorted keys, `ensure_ascii`, Python's float form), so `fixes_json` and `snapshot_json` equal the
rows Python writes (the clock, `server_version` and the store they ran on aside).

*Write discipline.* The order is fixed: validate -> open the store **read-only**, require schema v11, check the scope -> measure the rules -> only then open a
**write handle**, one `INSERT` in one `BEGIN IMMEDIATE` transaction (5 s busy timeout, the pragmas of Python's `store.connect`) -> close it -> answer. A refusal at
any earlier step leaves the file's bytes unchanged (a test compares them). `mcp` start-up and every other tool still open read-only (a test compares the file's
bytes and mtime after all of them). No `ingest.lock` is taken (Python takes none): a save during an `ingest` waits for the write lock like any writer (a test runs
25 saves against a running ingest). A missing store is never created. Not v11: `store is schema v10: run the Python app once to migrate it` (or the "newer" text).

| Case | Answer |
|------|--------|
| `summary` missing or not a string; `fixes` missing, not a list or holding a non-object; `workspace` / `account` not a string or null | tool error "argument `summary` is required and must be a string" (and the `fixes` and `workspace` forms), never the value |
| a limit, a secret, a wrong type inside a fix, an unknown scope | tool error with Python's text, e.g. `fixes[0].text: 301 characters, the limit is 300; nothing was saved` |
| the store busy for more than 5 s | `the store is busy (another process is writing); try again in a moment` |
| the store read-only on disk | `the store is read-only; nothing was saved` |
| no store, or a store newer than this build | the texts of the read tools |

**`investigate`** (`prompts/list`, `prompts/get`): Python's prompt text (equal to the goldens `mcp__prompt_investigate__{all,scoped}`), with the `Scope: pass
workspace="..." and account="..."` line when a non-empty `workspace` or `account` is given (quoted as `json.dumps(ensure_ascii=False)`); the same description
and two optional arguments; `initialize` advertises `prompts` (`{}`; Python adds `listChanged: false`). An unknown prompt, or an argument that is not a string, is a
JSON-RPC error (`Unknown prompt: nope`).

*Decided for you.* (1) A wrong argument type gets a short fixed text; Python's pydantic text echoes the offending value (`input_value=...`), which for a summary would
put the report text in an error. (2) The secret check's `\b` uses Python's `\w` (`str.isalnum()` or `_`), not Rust's wider `char::is_alphanumeric`: a table of the 273
ranges where the two differ (circled and squared letters such as U+24B6, combining marks such as U+0345 and U+093F) is carried in `secret.rs`, and a test compares
`word` with Python's answer for every code point (`golden-save-report/word_table.json`, made by `gen_word_table.py`); T052 closed the miss where `<circled letter>sk-abc` was saved
(corpus equal on all 105 strings). The table follows the Unicode version of the Python that made it (16.0.0). (3) Python crashes or stores oddly on a lone surrogate (`\ud800`) in the text and
on an integer above 64 bits or a float outside the double range: here the JSON line is a parse error (`-32700`) or the number is read as a double
(`1e+23` for `10**23`), never a crash. (4) The scope is checked on a read-only handle, and the write handle opens only after the rules are measured, so Python's
"measure then insert" has one extra step between them that no reader can see. (5) The store file's journal mode: a store still in `delete` mode (the test fixture
only) is switched to WAL by the save, as Python's `connect` does. (6) `server_version` is this build's Cargo version (Python's is its package version).

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
`crates/core/tests/kit_abstain.rs` (20 s, measured in T052 with a fake `git` that logs each start: two 5 s waits for the project's repository lookup, then the 10 s git-part limit once) proves a hung `git` is `abstained`; `crates/app/tests/kit.rs` runs the three
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
- a **name that is not UTF-8** is left out of the listing and cannot be asked for (Python's route fails on it: a 500 with a traceback. Measured in T052: `os.listdir` gives a surrogate-escaped name; plain `json.dumps` escapes it
  and does not fail, but Starlette's `JSONResponse` encodes with `ensure_ascii=False` and then to UTF-8 and raises `UnicodeEncodeError`). A report folder or repo whose name is not UTF-8 was already unlisted;
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

Not served yet: nothing of Python's routes, and (since T060) nothing of Python's CLI. The connector writes and the sync apply are served since T050, `/api/live` since T056, `GET /api/store/health` since T059, and the `connector` and `doctor` commands since T060. This list was stale before T043 (`/api/sync`, the report files, `/api/accounts` and
`/api/dirs` are served); T043 serves `/api/board` and `/api/memory-graph`, and T048 the store-only workspace and account writes. The release note `docs/release/rust-v0.2.0.md` "not in this release" list may still name some of these; it is the Supervisor's to update.

## Board and memory graph (T043)

`GET /api/board[?workspace=&account=]` and `GET /api/memory-graph[?project=]` answer what Python answers (`core/board.rs`, `core/memgraph.rs`, `core/easymem.rs`), read-only.
`board`: every project seen in usage (in the `workspace` / `account` scope, a `.claude/worktrees/...` path skipped, a project whose folder is gone counted in `gone`) with the
tasks of its `PROJECT_KANBAN.md` (`id`, `title` cut at 300 characters, `status`), `abstained` (rows with no id or in an unknown `###` subsection), and, only when
`.claude/board.json` names a board, `board` (40 characters) and `last_pull` (the file's time). A missing, over 5,000,000 bytes, non-UTF-8, not-regular or board-less file is `{"status": "no data"}`.
`memory-graph` without `project` (or with an empty one) lists the projects that have notes (`kit_notes`, `claude_notes`, `easy_memory_notes`); with one it gives that project's
`nodes`, `edges`, `counts` and the `easy_memory` panel. Errors: an unknown `workspace` / `account` or `project` is `400` with the core's fixed text (`unknown workspace`, `unknown account`,
`unknown project`: none names the input); a folder that cannot be read is skipped, never an error; the usual `403` (token, Host, Origin), `405` for a write method, `503` for the store states.
Shown, and why it is safe: the folder name, the task ids and titles, the board name, a note's file path, its frontmatter `name` and `description` (200 characters) and its `[[link]]`
targets: all the owner's own text, shown as Python shows it. A note body, an easy-memory `prompt`, `answer` or `session_id`, a file's content and any path outside the project never leave; no error,
log line or `Debug` output holds a title, a note text or a path. Every file is opened `O_NOFOLLOW | O_NONBLOCK` from a held folder handle (the repository folder and the account folder by
their own path), judged on the open handle (regular file) and read through `take(cap + 1)` (caps: kanban 5,000,000, board.json 5,000,000, note 1 MiB, 5000 notes per folder, 64 KB for meta/report,
10 MiB to hash a cited file). The Rust tests point `HOME`, `EASY_MEMORY_HOME` and the account folders at temp folders.

Oracle: `parity/golden-rich` and `golden` (`api__board__*` 8, `api__memory_graph__*` 4), and `parity/golden-memgraph/cases.json` (made by `parity/gen_memgraph.py`: Python's own functions over
the tree of `parity/fixtures-memgraph/` plus 600 notes, 5001 notes, a 1 MiB+1 note, a non-UTF-8 note, symlinks, claude memory, board files of every kind and 17 kanban texts); `crates/core/tests/memgraph_parity.rs`,
`crates/app/tests/board_memgraph.rs` (HTTP). T053 adds `parity/golden-memgraph/cases-t053.json` (same generator, a second tree under `<T>/t053`): hostile notes (a NUL, a 100 KB description, a nested mapping, a FIFO),
two Claude folders that resolve to one project (duplicate node ids), a worktree `.git` and a store path through `link/..`, and an **easy-memory store that holds notes** (`fixtures-memgraph/em-store/`: 14 note files of every kind, a cited
file inside and outside the project, a stale, a gone, a dangling-link and a FIFO one, 49 cited files against the 30 drawn, `prompts.log` with a BOM line and lines of 64 KiB + 1, + 2 and 200 KB, `report.json`, a `trash/` note, a `meta.json` with a BOM).
Python's "now" is frozen at 2026-01-20T00:00:00Z and its panel is made under four zones (`TZ` set before the run), so the Rust tests compare the layer under `UTC`, `America/Los_Angeles`, `Pacific/Kiritimati` and
`Pacific/Pago_Pago` through `Env { zone, .. }`: the three `TZ` runs of the whole suite never reached the layer (the zone is a field of `Env`, not read from `TZ`) and were vacuous for it. The same file holds every code point of
Python's `\d` (the unit test in `board.rs` compares the table over all 0x110000 points), and the planted canaries (`CANARY-*` in a note's `prompt`/`answer`/`session_id`, a log line, a `trash/` note) must not appear in any answer.

Decided for you (Rust differs from Python on purpose):
- A symlink is never followed: a symlinked `PROJECT_KANBAN.md` or `.claude/board.json` is no file; a symlinked note, a symlinked sub-folder and a symlinked `memory`, `projects`, `notes`, `prompts.log`, `report.json` or `meta.json` are skipped. Python follows a
  symlink that stays inside the memory folder (and a symlinked `memory` folder).
- A note with two names (a hard link) is listed once (by inode); Python dedupes by real path.
- A folder or file name that is not UTF-8 is skipped (Python would put surrogates in the JSON). A memory folder deeper than 64 levels is not entered (Python has no limit).
- `.claude/board.json` has Python's lack of a cap replaced by 5,000,000 bytes; a JSON nested deeper than 128 levels or with `NaN` is "no board" (Python crashes on the first, parses the second).
- A transcript is read for its `cwd` in 8 KiB chunks as Python's text reader does (a bad byte in the chunk of the first line skips the file), and at most 1 MiB; `prompts.log` at most 64 MiB; a cited file is hashed through a no-follow chain after Python's `realpath` rule decides it stays inside the project.
- easy-memory times are parsed as RFC 3339 / ISO 8601 by `jiff`; a time with no offset is UTC. Python's `fromisoformat` accepts a few more spellings. The easy-memory layer (`easymem.rs`: panel, cited files, `report`, the 30-day `prompts.log` window) is compared with Python's answers over a store that holds notes (`cases-t053.json`, above).
- T053 (review fixes). A UTF-8 byte order mark is taken in `meta.json` and in a `prompts.log` line (Python's `json.loads(bytes)` sniffs the codec) and refused in a note and `report.json` (Python decodes first, and `json.loads(str)` refuses a BOM): both as Python does. A JSON file in UTF-16 or UTF-32 (Python sniffs those too) is not read.
- T053. `<project>/.git` and `<gitdir>/commondir` are opened below a held folder handle with `O_NOFOLLOW | O_NONBLOCK`, judged on the handle and read for at most 4096 bytes (Python: `open` follows links and blocks on a FIFO; it reads `readline(4096)`). A `.git` FIFO, device or symlinked `.git` file is treated as no worktree file; a `commondir` that is a symlink, a FIFO, a device or not UTF-8 is treated as absent (the gitdir itself is the identity; Python: a symlink is followed, a bad byte raises and gives the same result). A lone `\r` ends a line in Python's text mode and not here (`.git`, `commondir`).
- T053. `realpath` is Python's `os.path.realpath` (a dangling link's text is followed, `..` after a link goes up from the link's target; since T055 there is no cap on links, see "Decided for you (T055)"). `metrics::resolve_path` is the one copy (the second one in `easymem.rs` is gone, and since T055 `workspaces` and `accounts` call it too); `metrics::realpath` is its text wrapper. It also serves `/api/dirs` and the scratch rule, where it is now closer to Python as well.
- T053. `Path(name).stem` follows Python 3.14, the oracle (a leading dot is no suffix: `..md` keeps its name; a trailing dot is one: `Makefile.` gives `Makefile`); Python 3.11 differs for those names. `Path(cited).name` drops empty parts and `.` (`dir/` is `dir`, `.` has no name).
- T053. With neither `EASY_MEMORY_HOME` nor `HOME` set (or `HOME` empty) the easy-memory store is absent; Rust never looks for `.easy-memory` below the working folder (Python asks the user database for the home folder).
- T053. `/api/board` shows `board: null` when `.claude` itself is a symlink (Python follows it); a symlinked easy-memory store key folder (`<store>/<key>`) is skipped (Python follows it). The easy-memory `load()` keeps each folder's path string and opens `notes/`, `prompts.log` and `report.json` by it later (not a held handle): a folder swapped for a symlink between the two calls is the one gap, left as it is (no new signature across four functions for a local, owner-owned store).
- T053. Rust spawns `git` (`metrics::repo_of`, through `memgraph::resolve_project`) exactly where Python does (`discovery.resolve_project`: a transcript's `cwd` that is a folder), with the mitigations Rust already has (`core.fsmonitor` off, `GIT_OPTIONAL_LOCKS=0`, stdin null, a timeout, a kill). Kept as Python parity (decided in T053). `GIT_NO_LAZY_FETCH=1` is T052's AC 5a and is not added here.
- The regular expressions of `board.py` are hand-coded (no new crate); `\d` is Python 3.14's Unicode 16.0 table of decimal digits, `casefold` is `scope::casefold` (an approximation outside everyday text).

## Review fixes of waves 1-2 (T052)

Each fix has a test that failed before it. Oracle cases were added by regenerating with Python (`gen_dirs_jobs.py`, `gen_save_report.py`, the new `gen_word_table.py`);
`git diff --stat` shows insertions only in the `golden-*` folders, no existing case changed.

- **Crontab steps** (`jobs.rs`): `1/9223372036854775807 * * * * claude` panicked in a debug build (`n += step`) and wrapped in a release build. The loop now stops on overflow, so the set is
  Python's `range(first, last + 1, step)`. A step beyond 64 bits (`1/99999999999999999999999`) is read as the nearest `i64` (a Python int has no limit; it gave "unknown" here before).
  A step of `0` or `-1` is refused as in Python. New oracle case `cron_steps` (12 lines) in `golden-jobs/jobs.json`; `/api/jobs` answers 200 with every row.
- **DEL** (`reports.rs`): `0x7f` is escaped as `\u007f` in `fixes_json` and `snapshot_json`, as `json.dumps(ensure_ascii=True)`. Cases `del_in_summary`, `del_in_fix`.
- **Secret word boundary** (`secret.rs`): see "Decided for you (2)" of the save section; the README claim there is corrected.
- **Account files** (`accounts.rs` `read_json`): the file is opened once (`O_NONBLOCK`, so a pipe never blocks), judged on that handle, and read through `take(max + 1)`; more than the cap is "no file",
  however the size changed since the open. A test reads `/proc/sys/kernel/pid_max` (size 0, more bytes than a cap of 1) and is skipped where there is no `/proc`.
- **Hardening, decided here (Rust-only)**: (a) every `git` run (`run_git_timed`) carries `GIT_NO_LAZY_FETCH=1`, so a partial-clone repository can never make `rev-list` fetch over the network
  (Python has the same gap; a test with a fake `git` reads the variable); (b) `save_report` opens its write handle with `open_existing_for_write`, without `SQLITE_OPEN_CREATE`: a store removed
  between the read-only check and the write is refused (`WriteError::Io`) and not created again (ingest's `open_for_write` still creates); (c) the leak gate's tree digest skips only
  `<plant root>/bin/argv.log`, not every file of that name (the gate's `--self-test` checks it).
- **`fired_count`** (`/api/reports` list): a snapshot where any `checked` entry has no boolean `fired` counts 0 for the whole report (a partial count would look like a healthy one).
  Tests: `{"checked":[{"fired":true},{}]}`, a `fixes_json` that is not JSON and a snapshot without `as_of_utc` (both a fixed 500 with none of the text).
  The "now side" test no longer depends on the system clock.
- **Stale text corrected**: the `mcp.rs` and `api.rs` headers, the routes list above, `jobs.rs` (the route passes this process's environment, as Python), `kit_abstain.rs` (the measured wait is
  2 x 5 s + 10 s, not 10 s twice), `report_files.rs` (Python's non-UTF-8 failure is Starlette's encoder, not `json.dumps`), `parity/README.md` (the venv and `uv`), and "Not served yet".
  `report-files` still lists a dot-file or a multi-link file as `openable` while `report-file` answers 404 for it (parity with Python's listing; documented in `report_files.rs`).
- **Recorded, not fixed here**: the `TimersMonotonic` scan in `schedule()` is quadratic on 8 MiB of crafted `systemctl` text (Python's `(On\w+Sec=...)` regex is quadratic too: 0.18 s, 0.73 s,
  3.06 s for 20, 40, 80 thousand characters of `On` repeated; the text is the user's own unit files); a reader thread that stays blocked when a killed child's grandchild keeps stdout open;
  replacing the `/proc/self/fd` technique with `openat` (a `Cargo.toml` change); the per-request 10 MiB buffer of `report-file`.

## Write routes (T048)

`serve` still opens **no** store at start-up and every `GET` still reads read-only (a test compares the store file's bytes and mtime after start-up and
eleven GETs). The first writes of the Rust app are seven routes, all **store only**: they change the SQLite store and nothing else. Code: `crates/app/src/writes.rs`
(routes, body rules, the lock) and, in the core, `workspaces.rs` (`add`, `edit`, `patch`, `delete`, `write_session`) and `accounts.rs` (`add`, `confirm`, `ignore`,
`use_workspaces`). Behind the guard of every route (token, Host, Origin: a write without them is `403` and changes nothing).

| Route | Body | Answer |
|---|---|---|
| `POST /api/workspaces` | `{name, color?, icon?, description?, email?, repos?}` (`name` `[a-z0-9][a-z0-9-]{0,39}`, not `orphan`; `color` `#rrggbb`; `icon` <= 32, `description` <= 500, `email` <= 254 characters; `repos` absolute or `~` paths, resolved, sorted, deduplicated) | `201 {"workspace": {...}}` |
| `PATCH /api/workspaces/{id}` | any of the fields above, and `confirm` (boolean) | `200 {"status", "lines", "pending", "workspace"}` |
| `DELETE /api/workspaces/{id}` | empty, `{}` or `{"confirm": bool}` | `200 {"status", "lines", "pending", "deleted"}` (`deleted` is the id, or `null` while only the plan was shown) |
| `POST /api/accounts` | `{path}`: an existing folder with `projects/`, `settings.json` (Claude) or `config.toml` (Codex) | `201`, the account row |
| `POST /api/accounts/{id}/confirm` | `{label}` (stripped, 1 to 60 characters) | `200`, the account row |
| `POST /api/accounts/{id}/ignore` | `{}` | `200`, the account row |
| `PUT /api/accounts/{id}/workspaces` | `{workspaces: [names]}` (replaces the picks; `[]` clears; confirmed Claude accounts only; not `orphan`) | `200`, the account row (`workspaces` sorted) |

Statuses and texts are Python's (`surfaces/api.py`, `core/workspaces.py`, `core/accounts.py`, `connectors/sync.py`); the oracle is `parity/golden-writes/` (`gen_writes.py` runs
Python's own handlers over copies of `store-rich`; `crates/app/tests/writes.rs` replays 262 cases / 355 requests over the real binary and compares each answer and **every
table**: 0 different; a case Python leaves unchanged leaves the store file byte for byte as it was). Order of a request: the guard; `Content-Type` `application/json` (else `415`
`content-type must be application/json`); the body (`400 body must be JSON`, `400 body must be a JSON object`; account routes: `400 body must be an object with boolean
confirm`; `413 request body too large` over 1 MiB); the id in the path (`404 unknown workspace` / `404 no such account`); `confirm` (`400 confirm must be true or false`); and only then
the store: **a request whose body, id or `confirm` is bad never opens it for writing**. The field checks that follow (names, texts, repos, labels, the account's state) run on the write handle, as in Python, and for the account routes `detect` has already committed by then (decided 4). The folder resolution of a repo or account path is done before the write lock is taken (T054). The store is opened inside the request (`writer::open_existing_for_write`: 5 s busy timeout, foreign keys on, schema 11 only,
never created), one transaction per change, and closed before the answer. One in-process lock lets one write run at a time; there is no `ingest.lock` (Python takes none), and
`writes_coexist.rs` runs the writes during a long ingest (`integrity_check` ok, no lock text in any body).

Every answer of the seven routes, by status. `{field}` is `color`, `icon`, `description` or `email`. Every body is `{"error": "<text>"}` and the text is fixed: it never names the caller's input (the one text that carries a stored id and alias is decided 8).

| Status | Text | Where |
|---|---|---|
| `201` | `{"workspace": {...}}` / the account row | `POST /api/workspaces` / `POST /api/accounts` |
| `200` | `{"status", "lines", "pending", ...}` / the account row | `PATCH`, `DELETE /api/workspaces/{id}` / the three `/api/accounts/{id}/...` routes |
| `400` | `body must be JSON` | every route |
| `400` | `body must be a JSON object` | the workspace routes |
| `400` | `body must be an object with boolean confirm` | the account routes |
| `400` | `confirm must be true or false` | `PATCH`, `DELETE /api/workspaces/{id}` |
| `400` | `unknown field` | `POST`, `PATCH` workspaces |
| `400` | `{field} must be text of at most {N} characters` (`color` 7, `icon` 32, `description` 500, `email` 254) | `POST`, `PATCH` workspaces |
| `400` | `color must look like #rrggbb` | `POST`, `PATCH` workspaces |
| `400` | `repos must be a list of paths` | `POST`, `PATCH` workspaces |
| `400` | `repo path must be a non-empty string` (also a path that resolves to non-UTF-8 bytes, T054) | `POST`, `PATCH` workspaces |
| `400` | `repo path must be absolute or start with ~` | `POST`, `PATCH` workspaces |
| `400` | `name must match [a-z0-9][a-z0-9-]{0,39}` | `POST`, `PATCH` workspaces |
| `400` | `the name 'orphan' is reserved` | `POST`, `PATCH` workspaces |
| `400` | `alias for connector <id> is too long or has invalid characters; it must match [a-z0-9][a-z0-9_-]{0,63}` | `PATCH` workspaces (rename) |
| `400` | `alias '<alias>' is in use by connector <id>; nothing was changed` | `PATCH` workspaces (rename) |
| `400` | `the store refused the change; nothing was changed` | any write a store rule stops (decided 5) |
| `400` | `not an agent folder: it must be an existing directory` | `POST /api/accounts` |
| `400` | `not an agent folder: no projects/, settings.json or config.toml` | `POST /api/accounts` |
| `400` | `label must be 1 to 60 characters` | `.../confirm` |
| `400` | `the default account cannot be ignored` | `.../ignore` |
| `400` | `only confirmed Claude accounts pick workspaces` | `PUT .../workspaces` |
| `400` | `workspaces must be a list of workspace names` | `PUT .../workspaces` |
| `400` | `unknown workspace` | `PUT .../workspaces` (a name that is no workspace) |
| `400` | `the orphan workspace cannot be picked` | `PUT .../workspaces` |
| `403` | `forbidden` | every route without the token, Host or Origin (before anything else) |
| `403` | `the orphan workspace cannot be changed` / `the orphan workspace cannot be deleted` | `PATCH` / `DELETE /api/workspaces/{id}` |
| `404` | `unknown workspace` | `PATCH`, `DELETE /api/workspaces/{id}` (and `/api/workspaces/unassigned`, decided 9) |
| `404` | `no such account` | the three `/api/accounts/{id}/...` routes |
| `405` | none (empty body) | every other method of these paths |
| `409` | `a workspace with this name exists` | `POST`, `PATCH` workspaces |
| `413` | `request body too large` | every route with a body over 1 MiB |
| `415` | `content-type must be application/json` | every route |
| `503` | `no store yet: ...`, the newer-store text, `store is schema v10: run the Python app once to migrate it`, `the store is busy (another process is writing); try again in a moment`, `the store cannot be written right now; try again in a moment` | every route, when the store cannot be opened for writing |

`PATCH` / `DELETE` and the confirm protocol: an edit without a new name is applied at once. A new name of a workspace that holds connectors, and every `DELETE`, first answer the plan
(`pending: true`, `status: 1`, the `lines` of Python's plan, nothing changed); `confirm: true` applies it, and the answer ends "nothing to change." (Python's `apply` with nothing to run).
A workspace with no connector is renamed or deleted fully. The account routes first run `ingest::accounts::detect` (an `INSERT OR IGNORE` of the agent folders), as Python's `_account_write` does.

Errors of the store: `503` `no store yet: ...` (missing, empty or table-less file: **nothing is created**), `503` the newer-store text (`user_version` 12), `503` `store is schema v10: run the
Python app once to migrate it`, `503` `the store is busy (another process is writing); try again in a moment` (after 5 s; no SQLite text), `503` `the store cannot be written right now; try again in a moment`.

**Since T050** a confirmed `PATCH` rename or `DELETE` whose plan changes an agent account's entries is committed and then applied through `claude`, as Python's `sync.apply`
does (with no `claude` on `PATH`: status 0 and the commands to run by hand). The T048 `409` is gone (below, "Connector writes, keychain and launcher").

### Decided for you (T048, for the owner's review)

1. One lock covers all seven write routes (Python locks only the plan-first routes). A write waits for the lock without a time limit, then up to 5 s for SQLite.
2. A body over 1 MiB is `413 request body too large` (Python has no cap). The content type is judged before the body is read.
3. (Replaced by T050.) The confirmed plan that needed `claude` was a `409`; it is now applied as Python applies it.
4. A refused account write can leave behind the agent folders `detect` inserted first (Python does the same); no other table changes. A write never creates a store.
5. Where Python crashes (500) or reads text this build does not: an id in non-ASCII digits (`%D9%A3`) or past 18 digits is `404`; a label that starts with NUL (the table's length check stops at the NUL)
   is `400 label must be 1 to 60 characters`; a NUL in an account path is `400 not an agent folder: ...`; `NaN`, `Infinity`, a number past a double, a lone surrogate, UTF-16/32 text and
   nesting past 128 levels are `400 body must be JSON` (a UTF-8 byte order mark is accepted, as Python does); a store rule that no check caught (UNIQUE, CHECK, trigger) is
   `400 the store refused the change; nothing was changed` (Python: 500).
6. When `HOME` is unset, `~` in a repo or account path is not expanded (Python asks the password database); `~name` is read from `/etc/passwd`. `Path.resolve()` is ported by hand
   (`metrics::resolve_path`, the one resolver since T055: CPython 3.14's `posixpath.realpath` with its `seen` map, so a chain that doubles at every hop costs one expansion per link, not 2^n; 53 inputs incl. dot segments, relative, dangling,
   chained, doubling and looping links and a self-link equal Python: `parity/golden-writes/paths.json`, `tests/writes_parity.rs`); a link met again while
   it is resolved (a loop) is kept as it is, as Python keeps it.
7. The checks of a `PATCH` run in the body's key order (Python's `dict`), so the first bad text field is the one named: `writes.rs` reads the key order from the body text, because `serde_json` here sorts keys.
8. The `400` of a rename whose default alias is taken or too long names the connector id and the alias (`alias 'firm-slack' is in use by connector other-slack; ...`): Python's text, built from the stored id and the
   workspace name that already passed the `[a-z0-9-]` check. It is the one answer that names something derived from the input.
9. `GET /api/workspaces/<id>` and the write routes' other methods are `405`; `PATCH`/`DELETE /api/workspaces/unassigned` are `404 unknown workspace` after the body checks (Python's routing falls through to `{id}`).

Added by T054 (the review fixes of T048):

10. A resolved repo path that is not valid UTF-8 (a link to a folder whose name holds the byte `0xff`) is `400 repo path must be a non-empty string`, as an account path already was: Python resolves it to a
    text with a lone surrogate and then fails at the store (500); T048 stored it with U+FFFD, a path that matches no folder.
11. The folder resolution of a repo path or an agent folder (and the second resolution `_find` does for the new account row) runs before the write lock and the write handle, not under them; the order of the
    400 texts is unchanged (the first refusal of the list is reported where Python reports it). The default account's own folder (`<agent root>/.claude`) is still resolved under the lock: it comes from the environment, not from the caller.
12. A body of 95,000 distinct keys (about 1 MiB) is scanned in linear time (`key_order`), and the scan runs in the blocking task, not on an async worker.
13. A fact, not a choice: the in-process `WRITE_LOCK` is not what makes the concurrent test pass (six writers also pass without it, because `BEGIN IMMEDIATE` and the 5 s busy timeout are enough); it is kept.
14. A `413` also answers a body whose read fails (a broken connection): no client is left to read it.

Stale reference (report only): `docs/release/rust-v0.2.0.md` "not in this release" should drop the workspace and account writes.

### Decided for you (T055, for the owner's review)

1. **One path resolver.** `metrics::resolve_path(&Path) -> PathBuf` is the only port of Python's non-strict `Path.resolve()` / `os.path.realpath` (CPython 3.14: a `seen` map, **no cap on links**, a link met while it is being resolved is a loop and stays as typed); `metrics::realpath(&str) -> String` wraps it (a name that is not UTF-8 comes back with U+FFFD). `workspaces::resolve_path`, `accounts::resolve` and the old 40-link `realpath` are gone. Compared with Python in `tests/resolver_parity.rs` over `golden-writes/paths.json` (53 inputs: chains of 41 and 60 links, loops of 2 and 3 links with a tail or a `..`, a `..` after a dangling link) and `paths-relative.json` (22 relative inputs, made with the working folder at the fixture).
2. **Caller-visible changes, all toward Python.** `metrics::realpath` (scratch rule, `/api/dirs`, easy-memory and memory-graph reads): a 3-link loop is kept as typed (it was walked round to the second link), and a chain of more than 40 links is followed (it stopped at the 40th). `accounts::resolve` (the `is_default` test of `GET /api/accounts`): a dangling link is followed by its text, a loop is kept as typed, a chain of more than 40 links resolves (`fs::canonicalize` gave up and the path was kept as typed), and `..` after a missing part is folded; an ordinary existing folder gives the same answer byte for byte. The default account's folder on the write side still passes through lossy text, as before.
3. **`write_session` cannot create a store.** Its write handle is `writer::open_existing_for_write`: a store removed between the read-only probe and the write is refused (`WriteFail::Write(WriteError::Io(NotFound))`, nothing created). The route's fixed answer for a missing store is unchanged (the probe still answers it first). Tested through a private `session(.., after_probe, ..)` hook that removes the store inside that window.
4. **One `py_strip` and one `Done<T>`.** Both live in `workspaces.rs` (`py_space`, `py_strip`, `pub(crate) type Done<T>`); `catalog.rs` and `accounts.rs` use them.
5. **`memgraph_parity` is independent of `TMPDIR`.** The cause was in the test: the golden project list was recorded by Python under `/tmp/...`, which sorts after `/nonexistent-t043/zz`; with a `TMPDIR` that sorts before it (for example under `/home`) Rust's list, sorted by project text as Python's `sorted()` does, has the same rows in another place. The test now sorts the golden `list.projects` by `project` text after putting this run's temp folder back; no expected value changed. A `TMPDIR` inside a git repo (a fixture project resolves to the enclosing repo) or under a hidden folder (`.claude/`) still changes other answers: that is a different cause and is not handled.
6. **`writes_coexist`** retries the designed `503` (`the store is busy (another process is writing); try again in a moment`) up to 6 times per request and counts the retries; any other status, a lock-error text, an exhausted bound or a failed ingest stops it, printing the ingest child's exit status, stdout and stderr. 10 of 10 runs passed on a quiet host and 10 of 10 under twelve `yes > /dev/null` loops (0 retries needed in either).
7. **Temp folders** made by `accounts_parity.rs`, the `ingest/config.rs` unit test, and the T055 tests are removed on drop (also when an assertion fails).
8. **A third semantic is left, on purpose.** `ingest/accounts.rs` (`detect`) and `ingest/verifier.rs` still use `fs::canonicalize` where Python uses `.resolve()`. It is accepted because `detect` checks `is_dir()` first (the path exists, so it is no loop or dangling link, where `canonicalize` and the one resolver agree) and `verifier.rs` only compares folders that exist and refuses when either cannot be resolved. The owner can overrule: pointing both at `metrics::resolve_path` is a few lines.

## CLI commands (T058)

The commands of Python's `easy-personal` that `easy-personal-rs` lacked, with Python's arguments and the text it prints (stdout and exit
code; `src/easy_personal/surfaces/cli.py` is the model). They call the code the API routes and MCP tools already run; `cli_cmds.rs` only reads the
arguments, prints, asks the question and maps a refusal to an exit code.

| Command | Prints | Runs |
|---|---|---|
| `ping` | `easy-personal core ok` | no store |
| `usage [--since D] [--until D] [--by day\|model\|project\|tool] [--workspace W] [--account A]` | JSON (`json.dumps(indent=2, sort_keys=True)`) | `usage_summary_scoped`, or `usage_breakdown_scoped` with `--by` |
| `outliers [--since D] [--until D] [--workspace W] [--account A]` | JSON | `session_outliers_scoped` |
| `warnings [--workspace W] [--account A]` | JSON (`as_of_utc`, `hits`, `abstained`) | `evaluate_scoped` with the shipped rules |
| `plan [--workspace W] [--account A]` | JSON `{"groups": [...]}` | `improvement_plan_scoped` |
| `kit [--workspace W] [--account A]` | JSON `{"projects": [...]}`, gone projects included | `kit_summary` (the MCP `kit_summary`) |
| `kit-findings [--json] [--limit N] [--since D]` | the findings and their habits as text, or JSON | `kit_findings_local` |
| `workspace list` | JSON `{"workspaces": [...]}` | `workspaces::list_all` |
| `workspace add NAME [--color C] [--icon I] [--description T] [--email E] [--repo P ...]` (`create` is the same) | the workspace, JSON | `workspaces::add` |
| `workspace edit NAME [--color\|--icon\|--description\|--email V (empty clears)] [--rename NEW] [--add-repo P ...] [--remove-repo P ...] [--yes]` | the workspace, JSON | `workspaces::patch` |
| `workspace delete NAME [--yes]` | the plan, then `nothing to change.` | `workspaces::delete` |
| `account list [--by account\|provider]` | JSON | detection over the agent root, then `accounts::grouped` |
| `account confirm ACCOUNT --label L`, `ignore ACCOUNT`, `add PATH`, `use ACCOUNT [WORKSPACE ...]` | the account, JSON | `accounts::confirm`, `ignore`, `add`, `use_workspaces` |
| `dashboard` | the two `serve` lines | `serve`: ingest first, then the loopback server until Ctrl-C; `--no-ingest` and `--static-dir` as `serve` |

`D` is an ISO date or datetime (no zone: local time, `TZ` read as the API does). An account is its id, its label (a filter) or its folder (a
write). The writes are the store-only writes of the T048 routes (same checks, same texts, same busy wait of 5 s); every one opens the store for
the call only and closes it. The account commands run Python's detection first (new `.claude*` / `.codex*` folders are inserted as `detected`)
and look only in `EASY_PERSONAL_AGENT_ROOT`, else `HOME`; `account add PATH` takes a folder anywhere, as Python's.

`workspace edit` with a new name for a workspace that holds connectors, and `workspace delete`, print the plan (`will change (...)`, the alias
lines, `keychain: untouched`), ask `Apply these changes? [y/N] ` on stdout and read one line from stdin (`y` or `yes`, any case, spaces
around, is yes; anything else, an empty line or end of input is no: `aborted; nothing was changed.`, exit 1). `--yes` skips the question. A
stdin that is not a terminal does not wait: an empty one is end of input.

Exit codes: 0 done; 1 aborted at the question, or a store that cannot be read or written (missing, busy, unreadable); 2 refused (the core's
text, Python's: `unknown workspace`, `name must match ...`, `the orphan workspace cannot be deleted`, ...), a store newer than this build, or a
usage error. Errors go to stderr as `easy-personal-rs: <text>`; Python's is `easy-personal: <text>`.

**Oracle.** `parity/gen_cli.py` runs Python's own `main()` over copies of `parity/store-rich` and over an empty store (342 cases, 597 step runs,
`TZ` = UTC, Pacific/Kiritimati and Pacific/Pago_Pago for the cases that read a date, a clock or a time) and writes `parity/golden-cli/cases.json`
(`gen_cli.py --check` regenerates and compares; run it with `CLAUDE_CONFIG_DIR` unset). `tests/cli_parity.rs` builds each case's work folder,
runs the real binary and requires the same exit code, the same stdout and the same stderr (Python's `easy-personal: ` prefix read as ours). The
clock's values in an answer, `as_of` of `kit-findings --json` and `as_of_utc` of `warnings`, are `<NOW>` on both sides. Because the binary's clock is the machine's, the cases of `warnings`, `plan` and `kit` read a store whose events are months old: every rule abstains and the plan is empty (the rules and the plan are proven at a frozen clock by the scoped goldens of `rules_parity`, `plan` and `kit`). `tests/cli_cmds.rs` holds what Python cannot be
asked: the refusals below, `dashboard`, and account detection. `parity/leak_rust.py` runs every command with a planted key.

### Decided for you (T058, for the owner's review)

1. **A missing store, or one newer than this build, is refused, and nothing is created or migrated.** Python's `store.opened` creates the store
   and migrates an older one. Here a missing store is `no store yet: nothing has been ingested into the data folder` (exit 1, no file made), a
   newer one is the API's text (exit 2), a busy or unreadable one is the API's fixed text. `ingest` (or Python) makes the store.
2. **No difference since T050.** A confirmed `workspace edit --rename` or `workspace delete` runs the plan through the `claude` CLI as Python does, or prints the commands to run by hand when `claude` is not on `PATH`. (T058 first refused such a change, exit 2; T050 removed that refusal.)
3. **The question is asked with no store handle open.** Python holds its write transaction while it waits for the answer (any other writer, an
   ingest, waits for it). Here the plan is worked out, printed, and the question asked with nothing locked; after a yes the plan is worked out
   once more and **refused if it is not the one that was shown** (`the plan changed while the question was open; nothing was changed`, exit 2).
   A few microseconds remain between that check and the commit.
4. **`workspace create`** is `workspace add` (the guide's word; Python has `add` only). Both work.
5. **`dashboard` is `serve`**: it does not open a browser (Python does), it takes `--static-dir`, and `--no-ingest` stays. Its ingest policy is
   T051's (one progress line; on a failure one fixed line and it starts anyway); Python's `dashboard` stops with the ingest's exit code.
6. **`--workspace ""` and `--account ""` are an unknown workspace / account**, as Python's CLI (the API reads an empty value as no filter).
7. **Which input is echoed.** An error text is the core's fixed text, which names none of the input (NFR-002). Two things name what the owner typed on
   the command line, as Python does: a bad `--since` / `--until` (`Invalid isoformat string: '<value>'`), and clap's own usage errors
   (`invalid value 'x' for '--limit <LIMIT>'`, an unknown subcommand). Both go to the owner's own terminal. The leak check does not type the canary there.
8. **Arguments.** `--limit` is read like Python's `int()` (a sign, spaces, `1_0`); digits of other scripts are refused, and a number past 64 bits is
   valid (the core caps a limit at 10). clap does not take Python's abbreviations of an option (`--desc`). A usage error prints clap's text on stderr
   (Python's is argparse's); stdout is empty and the exit code is 2 in both. A date that Python accepts at `argparse` and then crashes on (a
   local day on the first or last day of its range) is the core's answer here, exit 2 or 0, not a traceback.
9. **`account list` needs a store at schema v11**, because it runs detection through the write handle (Python migrates first). With nothing new to
   detect it changes no row.
10. **`account use` ends with Python's stderr note** (`run \`easy-personal connector sync\` ...`). Since T060 the command it names is here
    too, as `easy-personal-rs connector sync`; the note keeps Python's text (as every other line that names `easy-personal`).
11. **`warnings`, `plan`, `kit`** (added after the first T058 report; the guide's list did not name them): `kit` lists projects whose folder is gone, as Python's CLI does (the dashboard route hides them). The owner's `config.yaml` threshold override is not read by this build (`rules.rs`, `kitperf.rs` say so), so the shipped thresholds apply and Python's exit 2 for a bad one does not exist here.
12. **Not here**: nothing since T060 (`connector` and `doctor`). `exec` and `headers` are the separate `easy-personal-launch` program (T050). `store-check` is here since T059. Every other subcommand of Python's parser, with the same flags, is here (`schedule install|remove` already were). Rust has three more: `store-info`, `serve` and `app`.

## Connector writes, keychain and launcher (T050)

The Rust app does what Python's connector code does: secrets in the OS keychain, the launcher program, the connector writes, the
sync apply and the adoption of pre-workspace connectors. Oracle: `parity/gen_connectors.py` -> `parity/golden-connectors/` (40 cases,
102 requests, 175 `claude` calls, 14 launcher runs), replayed over the binaries by `crates/app/tests/connector_parity.rs`; the cases Python
cannot answer are in `tests/connector_writes.rs`, `tests/launcher.rs` and the unit tests of `keychain.rs`, `claude_cli.rs`, `sync_apply.rs`.

**Dependency (Step 0).** One crate, owner gate passed 2026-11-13 (`docs/T050_CRATE_OPTIONS.md`, option A), app crate only:
`keyring = { version = "=3.6.3", default-features = false, features = ["async-secret-service", "tokio", "crypto-rust"] }` (Supervisor,
after the first measurement with `sync-secret-service` showed a `libdbus-1` link). 76 new lock entries against `main`, all MIT /
Apache-2.0 / BSD (the record is in `tasks/TASK_REVIEW_T050.md`); the default build has 175 packages (104 before). D-Bus is pure-Rust
`zbus`: `ldd` of both binaries shows only libc, libm, libgcc_s and the loader. `zbus` blocks on a private tokio runtime of its own, which
panics on a thread inside a runtime, so every real-backend call runs on a fresh plain thread (`keychain::off_runtime`; the routes already
run on the blocking pool, and the launcher has no runtime). No other crate; the `claude` calls use `std::process::Command`.

**Keychain** (`crates/app/src/keychain.rs`, behind the `Keychain` trait). One item per connector: service `easy-personal/<name>`, user
`connector`, secret = `{"command": [...], "env": {...}, "headers": {...}}` written byte for byte as Python's `json.dumps` writes it, so Python
and Rust read each other's items (`keyring` finds an item Python wrote, which has no `target` attribute, by its fallback search of the default
collection). Name rule `[a-z0-9][a-z0-9_-]{0,63}`, checked before any backend call (`invalid connector name`). No usable backend, a locked
one or any backend error: `no usable OS keychain (Secret Service); unlock or install one, e.g. gnome-keyring. Secrets are never stored in
plaintext`, nothing written. Removing an absent item is not an error. Linux Secret Service only: on another OS there is no backend (every
write refused; nothing kept in memory or a file). **Debug builds only**: `EASY_PERSONAL_TEST_KEYCHAIN=<file>` swaps in a file-backed fake
(`unusable` = no backend; a file holding `locked` = a backend that refuses), which the tests and the leak gate use; a release build has no
such switch. The cross-check with the real keychain is an ignored test the owner runs once by hand:
`cargo test -p easy-personal-rs --bin easy-personal-rs real_keychain -- --ignored` (it writes and deletes two `easy-personal/t050-crosscheck-*`
items and needs `parity/.venv` with the Python app).

**Launcher** `easy-personal-launch exec|headers <connector>` (second binary of the app crate, `src/launch.rs`; it uses only `keychain.rs` and the core's `pyjson` text helpers: no server, no store).
`exec` reads the item and `exec`s the provider command with the item's `env` added to its own environment: the secret is in the provider's
environment, never its argv; stdin, stdout, stderr, signals and the exit code are the provider's own (it replaces the launcher process, as
`os.execvpe` does). `headers` prints the item's `headers` as one JSON line. The command comes from the keychain item only. Python's fixed texts
on stderr with `easy-personal: ` and exit 1: `connector '<name>' is not configured`, `... has an unreadable keychain entry`, `... has no provider
command`, `cannot start provider command for '<name>'`, `invalid connector name`, the no-backend text; bad usage is exit 2. It adds about
1 ms before the provider runs (NFR-006: < 200 ms; `tests/launcher.rs` measures it).

**`claude` subprocess** (`crates/app/src/claude_cli.rs`, the core's `Claude` trait; argument lists in `crates/core/src/sync.rs`): `claude` is
found on `PATH` (an executable regular file), run with an argument list (never a shell), this process's environment minus
`CLAUDE_CONFIG_DIR`, plus `CLAUDE_CONFIG_DIR=<folder>` for an account that is not the default one (the folder comes from the account's row,
never the request), stdin closed, a 60 s timeout (then killed; the call answers 127, as Python does). stdout is kept up to 1 MiB and only parsed
(`Command:` / `Args:` of `claude mcp get`); stderr is read and dropped. Calls: `mcp get <alias>`, `mcp add-json <alias> {"type": "stdio",
"command": "<launcher>", "args": ["exec", "<id>"]} --scope user`, `mcp remove <alias> --scope user`. The launcher path is the
`easy-personal-launch` beside the running binary, else on `PATH`.

**Routes** (guard, `application/json`, the 1 MiB cap and the one write lock as the T048 writes; answers are Python's `{"status", "lines",
"pending"}` with HTTP 200, or `{"error"}`):

| Route | What |
|---|---|
| `POST /api/connectors` `{provider, workspace, values: {ENV: value}, alias?, confirm?}` | add to a workspace: keychain item `easy-personal/c<8 hex>` and one row; no `claude` call (`sync` registers it). Store refuses the row: the item is deleted again |
| `DELETE /api/connectors/{id}` `{confirm?}` | a stored connector: unregister it from every account through `claude`, then delete item and row once no account holds it; a catalog id with no row: its own entry in the default account (only if it points at the launcher) and its item; anything else `404 unknown connector` with no keychain or `claude` call (HUN-72) |
| `POST /api/connectors/{id}/move` `{workspace, confirm?}` | move (orphan included); the plan, then commit and apply |
| `PUT /api/connectors/{id}/alias` `{alias: text \| null, confirm?}` | custom alias, or `null` back to `<workspace>-<provider>` |
| `GET /api/connectors/legacy` | `{"legacy": [{id, keychain, claude, foreign}]}`, `503` with the keychain text when there is no backend (the store is only read) |
| `POST /api/connectors/legacy` `{confirm?}` | adopt into orphan, remove our own entry from the default account; a foreign entry is left |
| `POST /api/sync` `{account?, confirm?}` | the plan of every confirmed Claude account (or one), applied step by step with `confirm: true`; the first failing `claude` call stops the run (status 1) |

Texts are Python's (`unknown provider; see GET /api/connectors`, `workspace is required (a workspace name)`, `alias must be a string`, `values
must map field names to strings`, `unknown connector`, `account must be an account id`, `no such account`, ...); none names a secret, and a
`values` secret goes to the keychain only.

### Decided for you (T050, for the owner's review)

1. **Never shown: `claude`'s stderr.** Python adds it as a line after a failed `claude` call; Rust shows the exit code only (the task: the raw
   output is never shown). The oracle's failing cases print nothing on stderr, so the answers are equal; the leak gate plants a canary there.
2. **The debug-build test keychain** `EASY_PERSONAL_TEST_KEYCHAIN` (a plaintext JSON file). `cargo build` (this README's build) is a debug build,
   so a debug binary honours it when the variable is set; nothing sets it but the tests. Whoever can set it can also set `PATH`. A release build
   (`cargo build --release`) ignores it. The alternative (a Cargo feature) was a dependency-rule change.
3. **`headers` output is in key order** (`serde_json` without `preserve_order`; Python keeps the stored order). Python's own writes always store
   `headers: {}`; only an item written by hand with two or more headers shows a different order.
4. **One failure stops the whole sync run**, as Python's `sync.apply` does; a *skipped* account (e.g. a missing folder) does not stop the
   others. The guide's AC6 wording ("one account failing does not stop the others") differs; the oracle wins, and that AC6 sentence is
   superseded (Supervisor, 2026-11-13).
5. **The launcher-not-found text is Python's** (`cannot find easy-personal-launch; reinstall easy-personal (uv tool install) so it is on
   disk`), kept for parity although `uv` does not install the Rust build.
6. **`GET /api/sync` asks `claude`** when it is on `PATH`, as Python does; it still detects no accounts and writes nothing.
7. Where Python crashes (500) and Rust answers: a keychain that raises on read is the no-backend text (Python: a traceback); an empty
   `PATH` entry is not searched for `claude` (Python's `shutil.which` looks in the working folder); a provider env name with `=` or an
   empty one is `cannot start provider command` (Python: the same, from `execvpe`'s `ValueError`).
8. **The provider gets the default `SIGPIPE`** (Rust's `exec` restores it; Python's `execvpe` passes on Python's ignored `SIGPIPE`).
9. `POST`/`DELETE` on `/api/connectors/legacy` follow Python's routing: `DELETE` is the remove route (`404 unknown connector`).
10. The account folder in an account row is used as it is (no "outside its root" or symlink refusal): Python refuses neither. It only ever
    reaches `claude` as the value of `CLAUDE_CONFIG_DIR` (a test proves a folder named with `$(...)` and backticks runs nothing).
11. (Superseded by the Supervisor, 2026-11-13: the feature set is now `async-secret-service` + `tokio`; no `libdbus-1` link.)
12. **A `claude` that could not run is not "no entry".** When `claude mcp get` does not start or is killed at its 60 s timeout, Python
    takes the entry as absent (its plan then `forget`s the registration, and its pre-workspace remove deletes the keychain item). Rust
    changes nothing for that account: the plan step is ``skipped (`claude` could not run)``, and the pre-workspace remove answers status 1,
    ``the `claude` CLI could not run (it did not start or timed out); nothing was removed.`` A `claude` that ran and exited non-zero
    (127 included) is still "no entry", as in Python.

Not built (cut list): macOS and Windows keychains, retries, a new catalog entry, `configured` in `GET /api/connectors` (T046 read side
unchanged). The `connector ...` and `doctor` CLI commands are T060's (next section).

## Connector and doctor commands (T060)

Python's `easy-personal connector ...` and `easy-personal doctor` (`surfaces/cli.py`: `_connector`, `_connector_list`, `_sync`, `doctor`),
with Python's arguments, texts and exit codes. `crates/app/src/connector_cmd.rs` and `doctor_cmd.rs` read the arguments, ask and print;
everything else is T050's code (`connector_writes.rs`, `legacy.rs`, `keychain.rs`, `claude_cli.rs`, the core's `sync` and `workspaces`)
and T058's command frame (`cli_cmds.rs`: the store sessions, the question, the exit codes). No new store, keychain or subprocess logic.

| Command | What |
|---|---|
| `connector list` | the catalog by category (claude.ai connectors first), each with its fields; then `configured (id alias provider workspace)` |
| `connector add PROVIDER --workspace W [--alias A] [--yes]` | asks each field, shows the plan (`ENV=***`), asks, then the keychain item `easy-personal/c<8 hex>` and one row; no `claude` call |
| `connector remove NAME [--yes]` | a stored connector: unregister it from every account through `claude`, then delete item and row; a catalog id with no row: its own entry in the default account and its item; anything else `unknown connector; nothing was removed` (exit 2) before any keychain or `claude` call |
| `connector sync [--account ID\|FOLDER] [--yes]` | detects the agent folders, prints each account's plan, asks, applies it step by step through `claude` |
| `connector move ID WORKSPACE [--yes]`, `connector alias ID (ALIAS \| --default) [--yes]` | the plan, the question, then the store change and its `claude` steps |
| `connector adopt-legacy [--yes]` | pre-workspace connectors into orphan; our own entry leaves the default account |
| `doctor` | `ok`/`FAIL` lines for the keychain, `claude` on `PATH`, the launcher, each pre-workspace connector, a `note` for legacy ones, each stored connector's item and each account registration; exit 1 on any `FAIL` |

**Secrets** take Python's one route: the prompt. A secret field (`Bot token`, `API key`) is read as `getpass` reads it: from the
controlling terminal (`/dev/tty`) with echo off, the prompt written there. With no terminal it is Python's fallback: `Warning: Password
input may be echoed.` and the prompt on stderr, the line from stdin. A plain field (`Tenant ID`, the Gmail keys file) is `input()`: the
prompt on stdout, the line from stdin, stripped. No flag and no environment variable carries a value. The value is in the keychain item
only: never in a line (the plan shows `ENV=***`), an error, the store or a `claude` argument (`claude` gets the launcher command and the
connector id). An empty value is `<label> is empty; nothing was written.` (exit 2); end of input at a prompt is `input ended at the
prompt; nothing was written` (exit 2). Nothing is written before the yes.

**Questions** (`Apply these changes? [y/N] `) read one line of stdin: `y` or `yes` (any case, spaces around) is yes; anything else, an
empty line or end of input is no (`aborted; nothing was written.` / `removed.` / `changed.`, Python's line for that command, exit 1).
`--yes` skips the question. A stdin that is not a terminal never waits.

Exit codes: 0 done; 1 aborted, a step that failed (`claude` exit, a skipped account), or Python's `LaunchError` (no usable keychain, no
launcher: its text on stderr); 2 refused (Python's text: `unknown workspace`, `no such account`, `give either a new alias or --default`,
...) or a usage error. Errors go to stderr as `easy-personal-rs: <text>`.

**Oracle.** `parity/gen_connector_cli.py` runs Python's own `main()` with no controlling terminal (it forks and calls `setsid`) over the
T050 world: a temp copy of `store-rich`, the in-memory keychain of Python's tests, the fake `claude` of `golden-connectors/fake-claude.sh`
(per-account state, one failing call), `CLAUDE_CONFIG_DIR=/runner/dir` that the default account must drop, a pinned launcher path (or
none). It writes `parity/golden-connector-cli/cases.json` (95 cases, 116 steps, 295 `claude` calls; `--check` regenerates and compares).
`tests/connector_cli_parity.rs` builds the same world for the real binary (the debug-build fake keychain, the same fake `claude`, no
terminal) and requires per step the same exit code, stdout and stderr, and per case the same keychain items, the same `claude` writes
in the same order, the same fake state and the same rows. `tests/connector_cli.rs` and `tests/doctor_cli.rs` hold what Python cannot be
asked: the terminal (a pseudo-terminal from util-linux `script`: the secret is not echoed; Ctrl-C), no hang without a terminal, the
secret in no argument or output through add and sync, a `claude` that cannot run, names the rules refuse, doctor reads only.
`parity/leak_rust.py` (`connector_cli`) runs every action and `doctor` with the planted key typed as a secret and given where a refusal
could echo it, a canary on `claude`'s stderr and in an account path: 0 hits.

### Decided for you (T060, for the owner's review)

1. **The terminal.** Echo is turned off through the C library's `tcgetattr` / `tcsetattr` (no new crate; Linux's `termios` layout, so
   Linux only: elsewhere the stdin fallback is used). Python's own `GetPassWarning` line (it names `getpass.py`) is not printed; its
   `Warning: Password input may be echoed.` line is. Ctrl-C at a secret prompt restores echo, prints Python's `input ended at the
   prompt; nothing was written` and exits 2, as Python. Ctrl-C at a plain-field prompt or at the question ends the process (exit 130,
   nothing written); Python prints `input ended ...` (exit 2) or `aborted ...` (exit 1).
2. **Never shown: `claude`'s stderr** (as T050 #1). Python prints it as a line after a failed `claude mcp add-json` or `remove`.
3. **The question is asked with no store handle open** (as T058 #3) for `remove`, `sync`, `move`, `alias` and `adopt-legacy`: the plan is
   worked out again after a yes and refused if it changed. So the `claude mcp get` reads run up to three times (the oracle compares the
   writes). `add` asks inside its one store session with no transaction open, as Python does: the new id and the typed values belong to
   that run, so its plan cannot be worked out again.
4. **A missing store is refused** for every `connector` action and `doctor` (`no store yet: ...`, exit 1, nothing created; as T058 #1).
   Python creates the store and migrates it.
5. **`doctor`'s keychain line names this build's backend**: `ok keychain: SecretService` (`FileKeychain` in a debug test run). Python
   prints its backend's class name (`Keyring`, or another keyring backend's).
6. **A `claude` that could not run** (it did not start, or ran past 60 s) is never "no entry" (T050 #12). `doctor` prints
   ``FAIL <name>: `claude mcp get` could not run (it did not start or timed out); not checked``, where Python reports a missing entry or
   says nothing. `remove`, `sync`, `move` and `alias` skip that account (`skipped (`claude` could not run)`): nothing is removed or
   forgotten.
7. **Not changed: legacy detection still reads "could not run" as "no entry"** (`legacy.rs`, Python's behaviour, as T050 left it). It
   reaches `connector adopt-legacy` and `doctor`'s `note` line: with a `claude` that cannot run, adoption stores the row in orphan but
   does not remove the default account's entry (that entry still points at the launcher and still works), and the note can miss a
   connector that has only an entry. Nothing is deleted. Left for the owner to decide.
8. **A keychain that answers but refuses a read** is a `FAIL <name>: no usable OS keychain ...` line in `doctor` (Python crashes).
9. **`workspace edit --rename` and `workspace delete` now exit with the apply's status** (T058's shared `change`, widened here): a failed
   `claude` step exits 1 and `edit` prints no JSON, as Python. T058 printed the JSON and exited 0.
10. **Input lines**: `\r\n` ends a line as `\n`; a lone `\r` does not (in Python's `input()` and in universal-newline reading it does). One line holds at
    most 64 KiB of a value (4 KiB of an answer); bytes that are not UTF-8 are end of input (Python crashes). **A secret read from piped stdin
    differs from real Python**: Python's `getpass` fallback keeps the `\r` (`printf 'tok\r\n' |` stores `tok\r`, and a bare `\r\n` line is not empty);
    Rust stores `tok` and calls a bare `\r\n` line empty. The oracle reads its stdin with universal newlines, so its cases do not show this.
11. **Which input is echoed**: `connector add <provider>` with an unknown provider names it (`unknown provider 'x'; available: ...`), as
    Python's does, on the owner's own terminal; the leak check does not type the canary there. Every other refusal is a fixed text.
12. **Usage errors** print clap's text on stderr (Python's is argparse's); stdout is empty and the exit code is 2 in both.

## Test

```bash
cargo test && cargo clippy --all-targets -- -D warnings
```

Tests build their own temp stores and never open `~/.easy-personal`.

## Live tail engine (T049)

`easy_personal_core::live` is the engine of Python's live tail (`src/easy_personal/core/live.py`, FR-043..FR-049,
FR-071, FR-081..FR-083, FR-086): `LiveTail::poll_once(roots, now, store)` (or `poll_store(store_path, projects_override,
now)`, which opens the store with `store::open_checked` for that one step) walks each account's `projects/` tree (main and
`subagents/` files), reads only complete new lines from a per-file offset, and keeps the same metadata events (a buffer of
2,000 with ids from 1), per-session state (totals, kit stage, open calls, failing / waiting / needs attention, sub-agent
types) and snapshots as Python. `events_since`, `rolled_past`, `snapshot`, `last_id` and `state_version` mirror Python's
methods. There is no thread and no clock inside: `now` (microseconds, UTC) is an input.

**Ported**: `_item`, `_safe`, `_bash_label`, `_call_label`, `_meta_agent_type`, `_jsonl_files`, `_cursor`, `_scan`, `_warm`,
`_follow` (its publish path), `_publish`, `_agent_of`, `_set_stage`, `_pair`, `_evaluate`, `_attention`, `_count_usage`,
`_append`, `snapshot`. The parser is the existing `ingest::claude_code::parse_record`; projects go through
`ingest::folders::resolve_project` (widened to `pub(crate)`, no change), costs through `prices.rs`.

**Not ported here**: the polling thread, `GET /api/live`, SSE framing and `Last-Event-ID` (in `crates/app`, T056, section
"Live feed" below); the `config.yaml` thresholds (`live_failing_streak` 3 and `live_open_call_minutes` 10 are fixed).

**Decided for you** (where Rust differs from Python on purpose):

1. Publish-only: the engine never writes the store and never creates or takes `ingest.lock` (the owner's Path 1). Every
   line is published from the tail's own offset, which is Python's FR-071 path. Python as shipped stores what it reads, so
   after a file shrinks in place (same inode) its cursor is its own; here the cursor is what the scheduled ingest stored. A
   file rewritten shorter but still at least as long as the stored cursor restarts at that cursor (Python: at 0 once it
   had stored past the new size). The oracle does not cover that case.
   **What "read-only" means for the store**: it is opened with `store::open_checked` (SQLite read-only flag,
   `query_only`). The store is WAL; when its `-wal` / `-shm` files are absent (no other connection open), SQLite creates
   them for the read and may leave them. The store file's bytes and mtime never change and no WAL frame is written;
   no `ingest.lock` and no `-journal` appear.
2. Hostile files: every `*.jsonl` and `*.meta.json` is opened `O_NOFOLLOW | O_NONBLOCK` and judged on the open handle (a
   regular file, the scanned inode). A symlinked transcript is not followed (Python follows it); a file whose open fails
   for any reason is skipped for this step (Python's poll stops at any error but `FileNotFoundError`).
3. One line is held to at most 32 MiB (`MAX_LINE_BYTES`): a longer line is skipped once its newline arrives (no event;
   Python parses it); until then the file waits without holding more than the bound.
4. `MAX_BATCH_LINES` (500) is the publish batch, as in Python, which reads every new line of a file in one poll: the
   engine does the same, so batch boundaries (and `state_version`) equal Python's.
5. The roots and the account labels are read on every step (Python refreshes them once a second). A step whose store
   is missing, busy, newer or broken, or whose account rows, cursors or labels cannot be read, **skips its scan**: no
   offset is dropped, the first scan is not used up (the warm start still runs on the first good step), and only the
   time-based states move; no error text. (Python raises before it prunes or marks the first scan done, so it never
   reaches this state; T049 fix round, item 2.)
6. A lone surrogate in a Bash command reaches `jobs::command_name` as U+FFFD (Python passes the surrogate); the first word
   must then pass the ASCII label pattern, so the label is `None` either way.
7. Python's one-time warning for a block without an id is not logged (the core has no logger); the block is not tracked,
   as in Python.
8. Token sums in the snapshot and a message's growth (`new - old`) saturate at the `i64` bounds (Python's ints are
   exact); a hostile value never panics or wraps. `rolled_past(i64::MAX)` is `false` (no overflow on a client's
   `Last-Event-ID`). Rust-only test, no golden (Python's exact answer cannot be held in an `i64`).
9. A warm start bumps `state_version` even when the file's open fails (as Python's `finally` does when `_warm` raises
   after the bump point); the file is then skipped for that step. Difference: Python opens a file over 64 MiB (to find
   the first line start) before its `try`, so a failed open there does not bump `state_version`; Rust always bumps.
10. A transcript whose file name is not valid UTF-8 is skipped (Python yields it through `surrogateescape`).
11. A `cwd` holding a lone surrogate makes the whole record abstain (the shared parser's rule, `ingest/claude_code.rs`;
    Python crashes binding it).
12. A file replaced (new inode) between the scan and the open is skipped for that step and picked up by the next.
13. Root and override path strings are used as given (`Path::join`), not normalised the way Python's `Path` drops repeated
    or trailing separators; a cursor row is matched on the path text the scan produces.
14. AC 6's "at most 500 lines per file per step" is implemented as Python's 500-line batches, looped until the file's
    complete lines are read in that step (item 4): equality with Python wins over a per-step cap. The Supervisor accepts
    or rejects this explicitly.
15. The open flags come from the crate's one per-arch table (`report_files::sys::{O_NOFOLLOW, O_NONBLOCK}`, widened to
    `pub(crate)`): Linux on the listed arches, and macOS on purpose (its `0x4` / `0x100` in the fallback `sys`, so the
    core still compiles there as `rust/PROJECT_SPEC.md` plans). Any other target fails to compile, also on purpose. No
    darwin target is installed here: the macOS branch was checked by a cfg proxy in a scratch copy (fallback `sys` made
    active on Linux, `cargo check -p easy-personal-core --all-targets` finished), not by a real macOS build.
16. Same as Python, accepted unless the owner overrules: (a) when a root's files disappear for one step and come back
    (the project folder unreadable for a step, or an account flipped confirmed -> ignored -> confirmed), their offsets
    are pruned and the files are read again from byte 0, so that history is published again; (b) a store with zero
    confirmed accounts (and no override) still completes the first scan, so the first account confirmed later has its
    whole history published, with no warm start. Python prunes offsets and sets `_primed` the same way.

**Tests** (`crates/core/tests/live_parity.rs`, 15 tests, oracle `parity/golden-live/` from `parity/gen_live.py`). Fix
round: `scripts-2.json` (6 scripts, 39 steps: accounts with a confirmed `Work` row and a detected NULL-label row, all 19
mapped skills one per step, and the boundaries `META_WAIT_S`, `FORGET_AFTER_S`, `OPEN_CALL_MAX_S`, `MAX_BATCH_LINES`),
`labels-2.json` (`COMMAND_READ_CHARS` at 4096/4097/4098); Rust-only: a failed store step and a failed first step, huge
token values, 100,000 calls paired, the 64 MiB warm-start trim. 14 mutants of `live.rs` (stage values, the five bounds)
each turn a test red. First round: 28 lines
against Python's `_item` (AC 1); 100 label inputs against `_safe` / `_bash_label` / `_call_label` (AC 2); 10 scripts, 72
steps, events and snapshot equal to Python after every step (pairing, failing, waiting, stage, streams with sub-agent meta
files, usage, files with offsets / partial lines / truncation / replacement / 1,203 lines, warm start, a 2,001-event buffer
with `events_since` / `rolled_past` probes, idle sessions and a clock going back) (AC 4-7); hostile files under a 60 s
timeout (AC 8); over a WAL store: the store file, transcripts and folders byte- and mtime-unchanged, an empty `-wal` if
any (`-wal` / `-shm` may appear), no lock or journal file (AC 9); canaries in
text, thinking, tool content, tool inputs and a meta `description` absent from every event, snapshot and `Debug` line
(AC 10).

## Live feed and `GET /api/live` (T056)

`crates/app/src/live_feed.rs` ports Python's live-tail thread (`core/live.py` `start` / `stop` / `_run`) and the stream
(`surfaces/api.py` `live_stream`, `_sse`, `_snapshot_data`, `_resume_id`, `_EMPTY_SNAPSHOT`).

**The thread.** `serve` and `app` (each in its own process, only while it runs) make one `LiveFeed`: one `LiveTail` (T049) behind a
lock, and one thread `easy-personal-live-tail` that every 0.1 s calls `LiveTail::poll_store(<store path>, None, now)` with
`clock::now_micros`. It starts before the server binds and is stopped and joined last when the server stops (Ctrl-C, SIGTERM in
`app`, the window closed), within 10 s. `store-info`, `mcp`, `ingest` and `schedule` make no feed: no thread, no store opened, no file
read. A step whose store is missing, not v11, busy or broken, or whose root cannot be read, is the engine's own skip (the stream then
shows an empty snapshot and picks the data up when the store appears). A step that panics is caught and skipped; the thread goes on and
prints one fixed line once per process (`easy-personal-rs: live tail: a poll step failed and was skipped`), never the panic's text.
Publish-only: the tail opens the store read-only, never takes `ingest.lock` and never writes the store file (`store.sqlite`
keeps its bytes and mtime); the scheduled ingest stays the only writer. It is not "no file at all": the store is WAL, and when no
other connection has it open, SQLite creates empty `store.sqlite-wal` / `store.sqlite-shm` files next to it for the read-only
open and may leave them (T049 README item 1; confirmed at Stage 4).

**The route.** `GET /api/live` answers `200`, `content-type: text/event-stream; charset=utf-8`, `cache-control: no-store`, `x-accel-buffering: no`
(Python's headers) and the CSP. Messages, in Python's order:

```
event: snapshot
data: {"active_window_s":300,"last_id":<n>,"open_call_minutes":10,"sessions":[{<_SESSION_KEYS>}...]}

event: live
id: <event id>
data: {<_EVENT_KEYS>}

: ping
```

First a `snapshot` (unless `Last-Event-ID` names an id the buffer still holds: then the events after it, then a snapshot); then
one `live` message per new event; when the attention state changes, the events up to the new snapshot's `last_id`, then that
snapshot; a `: ping` comment after 15 s without one. Session keys: `session_id`, `account`, `project`, `last_ts`, `tool_calls`,
`tokens_in`, `tokens_out`, `cost_usd`, `attention`, `stage`, `stage_skill`. Event keys: `id`, `session_id`, `account`, `project`,
`ts`, `kind`, `tool`, `is_error`, `is_sidechain`, `label`, `agent`, `tokens_in`, `tokens_out`, `cost_usd`. No cache counts, no text.
`Last-Event-ID` resumes only as plain digits (spaces around are dropped); a number above the tail's last id, an id older than the
buffer, a value that is not a number, or no header starts with a snapshot. A stream ends when the client goes away (at the next
write) and when the server stops (within 0.1 s, before the server's own 1 s grace), so Ctrl-C never waits on it.

**Token in the query** (only this route, only `GET`): exactly one `token=` pair, compared in constant time as sent. A token
header, if present, decides alone. `HEAD` or `POST` with a query token, or a query token on any other route, is `403`. The token
is never echoed in a body, header or log line; there is no access log and the route sends no redirect.

**Kit items and the window (T057).** Python has no live kit-trace feed: its kernel traces (`memory/event-trace/*.jsonl`) are
read only by the scheduled ingest (`sources/kit_trace.py`) into the store, and shown from there on the Kit page, never on the Live
tab. The only kit item in the live stream is the kit stage (FR-083: `stage`, `stage_skill` on each snapshot session, from `Skill`
calls in transcripts), and this stream already sends it (T049 engine, T056 route). So nothing else is fed here. `app` starts the
same `LiveFeed` as `serve` and gives it to the same server (`window.rs`); the page opens `/api/live?token=<token>`. The
`app --self-test` live check (section "Window") proves that path.

**Oracle**: `parity/golden-live-stream/cases.json`, made once by `parity/gen_live_stream.py` from Python's own `live_stream`
(one stream pass per step, the stream's sleep replaced by a wait the generator releases). 10 cases, 36 steps: an empty tail with
and without a tail running, a new session, a tool call and its result, an error streak, a sub-agent file with its meta file, a
good resume, a resume above the last id, a resume older than the 2,000-event buffer, and a resume that is not a number. The test
(`live_feed.rs`, `the_stream_equals_python_live_stream_for_every_oracle_case`) replays each case through the same `LiveStream`
the route serves: 36 / 36 steps equal. A mutant that drops the "fresh snapshot on an attention change" branch makes 3 steps red.
HTTP tests: `crates/app/tests/live.rs` (the real binary: a line arrives within 2 s, the guard, resume, SIGINT with two streams
open, ten streams and one client that never reads, publish-only with the scheduled ingest run beside it, a store that appears
later, and a client with a 4 KiB receive buffer that never reads while more than 5 MiB is offered: it holds about 33 KB, the
other stream gets every event, SIGINT exits 0 in about 1 s). Unit tests of the body: a stream ends at the next tick once the
server is stopping (removing that check turns the test red), and `: ping` follows a quiet interval (the interval is a field,
15 s by default, 250 ms in the test). The leak suite (`parity/leak_rust.py`) reads the first message of the stream three ways
(query token, header token, resume).

### Decided for you (T056, for the owner's review)

1. `cache-control: no-store` and `x-accel-buffering: no`, as Python sends. This supersedes the guide's `no-cache` (AC 2), which the
   Supervisor accepted and is correcting in the guide.
2. A repeated `Last-Event-ID` header is `403 {"error": "forbidden"}`, as a repeated Host or token header (Python keeps the last).
3. Query token: no percent-decoding (the token is URL-safe base64, so `encodeURIComponent` leaves it as is); exactly one `token=`
   pair (Python takes the first of several); a token header present decides alone, even an empty one (Python falls back to the
   query when the header is empty).
4. A `Last-Event-ID` above `i64::MAX` is "no id": a snapshot first, the same outcome as Python's "above the last id".
5. A stream reads the tail with `try_lock`: while a poll step runs, the stream waits one more tick (0.1 s) instead of blocking a
   runtime worker. A client that does not read is not polled (back-pressure), so nothing grows on its side; it holds its
   connection until it goes away or the server stops. There is no write timeout.
6. JSON is serde_json's: UTF-8 text where Python writes `\uXXXX`, keys in sorted order. The parsed data is equal.
7. The oracle runs in-process over `LiveStream` (one pass per step, a frozen clock), not over a socket: a socket would add the
   poll thread's timing and make pass boundaries random. The HTTP tests check the same route end to end without a golden.
8. Python's tail stops for good when its store cannot be opened at start; the Rust thread keeps polling and picks a store up
   when it appears (AC 1's edge case).
9. A process-wide panic hook prints the fixed line for this thread only; every other thread keeps the hook it had.
10. A poll step that runs longer than 10 s (a very large first read) makes `stop` give up waiting and leave the thread behind,
    as Python's `join(timeout=10)`; the process then exits. A single line is at most 32 MiB (T049's bound), so a step over one
    file is bounded, but a step over many large new files is not capped in time.
11. `project` is cut to 255 bytes (on a UTF-8 character boundary) in what the stream sends, in snapshot sessions and in `live`
    events (`live_feed.rs`, `PROJECT_MAX_BYTES`). Python does not cap it. It is the one free-text field of the stream with no
    bound from the parser (a session id is at most 200 bytes, a label 64): a transcript with a 1 MB `cwd` made a ~100 MB
    snapshot before the cap. The engine and its golden are unchanged; the oracle cases have short projects.
12. Fix round, core `live.rs` (the one change there, one line plus a test hook): `poll_once` clears the replay flag first. A
    panic inside a warm replay (caught by the feed's thread) used to leave it set, and every later event stayed out of the
    buffer. Test: `live::tests::a_panic_inside_the_warm_replay_does_not_silence_the_tail` (a `#[cfg(test)]` flag makes the
    replay panic); the 15 `live_parity` tests are unchanged and green.

## Store check (T059)

`store-check [--json] [--full]` and `GET /api/store/health` answer what Python's `store-check` (HUN-244, FR-091) answers, from one function
(`core/store_health.rs`, `check`). Read only: no migration, no `VACUUM`, no backup, no row written, and no file made beside the store.
The answer holds counts, versions, sizes, times and file names only: never a row, an absolute path or the text of an error.

```bash
./target/debug/easy-personal-rs store-check            # eight lines: file, schema, integrity_check, foreign_key_check, tables, sources, backups, verdict
./target/debug/easy-personal-rs store-check --json     # one JSON object, Python's `json.dumps(indent=2, sort_keys=True)` byte for byte
./target/debug/easy-personal-rs store-check --full     # the full integrity_check with no time limit (can take minutes)
```

The JSON fields are Python's: `store_file`, `exists`, `readable`, `locked`, `size_bytes`, `wal_bytes`, `user_version`, `latest_migration`,
`migration_pending`, `newer_than_code`, `integrity_check` and `foreign_key_check` (`{ok, problems[, mode]}`), `tables` (name to row count, read from the
schema), `sources` (newest record time and abstained count per source, or `null` for "no data"), `backups` (the `<store>.bak-v<N>` names, lowest version first),
`incomplete`, `unreadable` and `healthy`. `healthy` is `true`, `false` (a problem was found; it always wins) or `null` (no problem found, but a part
did not finish: "not fully checked").

| Result | Exit code |
|---|---|
| healthy | 0 |
| a problem was found, no store, or a file that is not a database | 1 |
| not fully checked: the 1.5 s budget ran out, or the store is locked | 3 |
| Ctrl-C (`stopped` on stderr, no partial answer) | 130 |

- **Speed.** The four reads (`integrity_check`, `foreign_key_check`, table counts, sources) run in parallel threads, each on its own read-only connection,
  and stop at one deadline of 1.5 s. A store over 32 MB gets `quick_check` (`mode: "quick"`) in a default run. `--full` has no deadline and always runs
  the full `integrity_check`; it prints `full check: this can take minutes; Ctrl-C stops it` on stderr first, so `--json` stdout stays pure JSON. The route never does this.
- **Open mode, decided once.** No `-wal`: `immutable=1` (no lock, no side file; on a store in rollback-journal mode this ignores locks, as in Python). `-wal` and `-shm`: `mode=ro`;
  the check creates nothing, but SQLite itself touches an existing `-shm` of a live WAL store, so the hash and mtime of the `-shm` can change (the store and the `-wal` do not).
  `-wal` and no `-shm`: SQLite would create the `-shm`, so the store and its `-wal` are copied to a private temp folder (folder mode 0700, files 0600, removed after the check) and read there.
  A symlinked store reads the `-wal` beside its target. A `-wal` or `-shm` that is a symlink, a FIFO or anything but a regular file is never opened or copied: the answer is `unreadable: ["file"]`,
  the verdict is `NOT healthy` and the exit code is 1 (and no size is read through the link, `wal_bytes` is 0). A SIGKILL or an OOM kill can leave a private copy in the temp folder;
  every check removes the `easy-personal-check-*` folders that are older than 10 minutes, owned by this user, mode 0700 and not links.
- **Stopping.** The deadline and Ctrl-C stop a running statement through SQLite's progress handler, which looks only inside a statement of 1000 or more VM steps; the loops of short
  statements (the table counts, the sources) look at the cancel flag every round and at the deadline every 64th round. Ctrl-C and SIGTERM stop the check the same way (exit 130, the private
  copy is removed); a second signal ends the process at once with 130.
- **Caps.** At most 200 tables, 200 sources and 200 backup names are listed. The rest is counted: `tables_more`, `sources_more` and `backups_more` appear in the JSON only when above 0, and the text
  ends the line with `(N more not shown)`. The rows of a table past the cap are not counted. Python has no cap: below it the answers are equal, above it Rust differs.
- **Route.** `GET /api/store/health` is behind the same guard as every `/api/*` route (a missing or wrong token, a wrong Host or a foreign Origin is `403`; the
  token is not read from the query). `POST`, `PUT`, `PATCH` and `DELETE` are `405`. `HEAD` answers headers only and runs no check. One check runs at a time (a request that
  comes meanwhile waits and gets its answer) and an answer is reused for 5 s per store path. A missing, empty or damaged store is a `200` answer that says so;
  only "no data folder" is a `503`. The body equals `store-check --json`.
- **Oracle.** `parity/gen_store_health.py` runs Python's `store_health.check` over 27 cases (clean, empty, one migration behind, newer than the code, a corrupt page with
  and without `--full`, a damaged index, a WAL without SHM, a locked store, hostile table and source names, a BLOB and an integer time, a missing store, a missing
  store with a `-wal`, a zero-byte file, not a database, backups, a foreign-key violation, a later table, 150 tables, sources and backups (long lists under the cap), a table name that is not UTF-8, no optional table, the budget
  forced to 0 with and without a pending migration, `quick_check` forced, `--full` with a 0 budget). The golden is `parity/golden-store-health/` (fixture stores, `<case>.json`, `<case>.txt`).
  The Rust JSON and lines equal it for every case (`crates/core/tests/store_health_parity.rs`), and the real command's stdout equals it byte for byte for every plain case
  (`crates/app/tests/store_check.rs`). The `locked` case repeats its setup in the test: a writer in exclusive WAL mode over a store that has a stale `-wal` and `-shm`.

### Decided for you (T059, for the owner's review)

1. **One `unsafe` block.** The deadline and Ctrl-C stop a running SQL statement through SQLite's progress handler, as Python does (every 1000 VM steps). rusqlite's wrapper for it
   sits behind its `hooks` feature, which the workspace does not enable, and enabling it would change `Cargo.toml`. So `store_health.rs` calls `rusqlite::ffi::sqlite3_progress_handler`
   itself (three `unsafe` sites in `store_health.rs`: the callback `stop_now` that reads its context, and the two calls in `Reader` that set and clear the handler; each has a safety note, and the callback is removed
   before the connection closes). Nothing else in the workspace is `unsafe`. If the owner prefers the feature, the change is one line in `Cargo.toml` and the three sites become safe code.
2. **`isprintable` is approximated.** Python's `str.isprintable()` uses its Unicode table; Rust has no such function. For ASCII it is exact. Beyond ASCII a character is replaced by `?` when it is a
   control, separator (`is_whitespace`), format or private-use character or a noncharacter. A code point that is unassigned counts as printable here and as not printable in Python. The oracle's
   hostile names (ANSI escape, newline, zero-width space, no-break space, a long name, an accent and a symbol) are equal.
3. **`\d` in a backup name.** Python's `\d` also matches non-ASCII digits (`bak-v٣`); Rust lists only ASCII digits. A numeric tie (`bak-v7` and `bak-v007`) sorts by name, as Python does.
4. **A store that is a symlink to a missing target, or a path that cannot be resolved**: the path is used as it is. Python's `resolve()` would do the same.
5. **Ctrl-C in a default run** (no `--full`) also ends with exit 130 and `store-check: stopped`, like Python (its `KeyboardInterrupt` is caught the same way).
6. **JSON key order in the route.** The route sends serde_json's sorted keys; Python's route sends keys in insertion order. The parsed objects are equal. The CLI's `--json` is byte for byte Python's.
7. **Per-source queries.** A table that is missing in an old schema is "no data" (as Python). A time that is text but not UTF-8 in `context_index`, `ui_sessions` or `verifier_scores` is "no data" too (Python catches
   the decode error of that query the same way); in `events` it makes `sources` unreadable.
8. **The leak gate and the "table name" canary.** A table name, a source name and a time are shown by design (they come from the schema and the store, capped at 64 characters), so a canary in the first 64
   characters of one would always be a hit. The gate plants the canaries in the *rows* of a table whose name is hostile (an escape sequence and a newline), in a source path, in `meta`, in the names of files beside the store,
   in a source name and a time that are text with the canary past the 64th character (cut off), and in the same two as BLOBs (unreadable, so not shown). It also covers a WAL with no SHM, the error answers (no store, not a database,
   locked, a symlinked store, no data folder), a second (cached) route answer, and that no `easy-personal-check-*` folder is left in the temp folder. 0 hits are required.
9. **Not built (cut list):** repair, `VACUUM`, `REINDEX`, backup, a page, an MCP tool, a history of checks, a locked-store test that does not need two stale files (a plain lock cannot be made: a store with no `-wal` is read with `immutable=1`, which ignores locks).
