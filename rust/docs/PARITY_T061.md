# T061: parity of Python and Rust, surface by surface

Made by `rust/parity/run_parity.py`; checked by `rust/parity/test_parity_report.py`. Nothing in this file is written by hand.

Counts: equal 54, differs README 28, differs new 5, not run 0.

86 surfaces were read from the Python app (argparse tree, route table, MCP tools and prompt); 1 protocol row is added. 179 probes ran, each on the Python app and then on the Rust app over the same world at the same path. Leak scan: 179 captured outputs and this report scanned with the repo canaries (`rust/parity/leak_rust.py` scanner, `Canaries.find`): 0 hits.

## How it ran

- A *probe* is one or more calls of a surface. Each probe builds a world (a copy of `store-rich`, the agent folders, a fake `claude`, a file keychain, fake `systemctl` and `crontab`), runs Python (`python -c "...cli.main()"` over this checkout's `src/`, or its `serve` and `mcp`), removes the world, builds the same world at the same path, and runs `target/debug/easy-personal-rs`. The two never run together on one store.
- The routes are asked over a real socket: Python's `serve` runs after `LiveTail().start()` (what `dashboard` does after its ingest), with the page's folder pointed at the world's; Rust's is `serve --no-ingest [--static-dir]`. `dashboard` itself is probed separately, with its ingest.
- Both children get an environment built from scratch: no `CLAUDE_CONFIG_DIR`, no display, a dead D-Bus address, `HOME`, `EASY_PERSONAL_HOME`, `EASY_PERSONAL_AGENT_ROOT` and the other folders inside the world, `TZ` (UTC; `Pacific/Kiritimati` and `Pacific/Pago_Pago` too for the rows that print a time). No owner folder, real keychain item or real `claude` is touched.
- Compared: CLI = exit code, stdout, stderr; route = status, content type, parsed JSON body (or the text); MCP = the `tools/list` entry and each call's result; and after every probe the store rows that changed, the keychain items, the `claude` calls and the fake `claude`'s state.
- A row is `differs: README <list> item <n>` only when that README item exists, holds the words the runner states for it, and a transformation that stands for the item makes the two outputs equal (the part the item does not describe stays, and the row is `differs: new`). The test checks the item and its words.
- The `<list>` names: `Tnnn` is the numbered "Decided for you (Tnnn ..." list of `rust/README.md` (T047's is the inline `(1) (2)` list; T040's is one paragraph, item 1). `Ingest` is the table "Differences from Python" of "Ingest (P2a, P2b)" (rows count from 1), `Schedule` the bullets of "Scheduled ingest (P2c)", `McpHttp` the table of "`mcp --transport http`", and `Static`, `ConnRoute`, `ConnTool`, `McpInit` single README paragraphs (item 1). These sections state a difference but are not "Decided for you" lists.

## Result per surface

| Surface | Probes | Result | Note |
|---|---|---|---|
| `cli: --version` | 1 | differs: new |  |
| `cli: account add` | 2 | equal |  |
| `cli: account confirm` | 2 | equal |  |
| `cli: account ignore` | 2 | equal |  |
| `cli: account list` | 3 | differs: README T058 item 8 | 1 of 3 probes differ: `acct_bad_list_xby_colour` |
| `cli: account use` | 2 | equal |  |
| `cli: connector add` | 4 | differs: README T060 item 1 | 2 of 4 probes differ: `add_yes`, `add_ask_no` |
| `cli: connector adopt-legacy` | 2 | differs: README T060 item 3 | 1 of 2 probes differ: `legacy_adopt` |
| `cli: connector alias` | 2 | differs: README T060 item 3 | 1 of 2 probes differ: `alias_custom_then_default` |
| `cli: connector list` | 1 | equal |  |
| `cli: connector move` | 2 | differs: README T060 item 3 | 1 of 2 probes differ: `move_ask_yes` |
| `cli: connector remove` | 3 | differs: README T060 item 3 | 2 of 3 probes differ: `remove_stored_registered`, `remove_pre_workspace_ours` |
| `cli: connector sync` | 2 | differs: README T060 item 3 | 1 of 2 probes differ: `sync_every_step_kind` |
| `cli: dashboard` | 1 | differs: README T051 item 1 |  |
| `cli: doctor` | 3 | differs: README T060 item 5 | 3 of 3 probes differ: `doctor_clean`, `doctor_registrations`, `doctor_no_claude` |
| `cli: exec` | 3 | differs: README T058 item 12 | 1 of 3 probes differ: `in-the-main-binary`; Rust: the separate `easy-personal-launch` program; Rust: `easy-personal-rs exec` (no such command) |
| `cli: headers` | 3 | differs: README T058 item 12 | 1 of 3 probes differ: `in-the-main-binary`; Rust: the separate `easy-personal-launch` program; Rust: `easy-personal-rs headers` (no such command) |
| `cli: ingest` | 4 | differs: README Ingest item 1, T058 item 8 | 4 of 4 probes differ: `canary-tree`, `twice-then-usage`, `retention-zero`, `no-store` |
| `cli: kit` | 6 | differs: README T058 item 8 | 1 of 6 probes differ: `bad-flag` |
| `cli: kit-findings` | 5 | differs: README T058 item 8 | 1 of 5 probes differ: `bad-limit` |
| `cli: mcp` | 3 | differs: new | 2 of 3 probes differ: `http-handshake-and-ping`, `bad-host` |
| `cli: outliers` | 5 | equal |  |
| `cli: ping` | 1 | equal |  |
| `cli: plan` | 6 | differs: README T058 item 8 | 1 of 6 probes differ: `bad-flag` |
| `cli: schedule install` | 3 | differs: README Schedule item 1 | 3 of 3 probes differ: `dry-run`, `dry-run-retention`, `no-systemd` |
| `cli: schedule remove` | 1 | equal |  |
| `cli: store-check` | 4 | equal |  |
| `cli: usage` | 12 | differs: README T058 item 1, T058 item 7, T058 item 8 | 3 of 12 probes differ: `bad-by`, `bad-date`, `no-store`; `--by week`: Rust's text is the core's one line, not clap's (item 8 names clap's) |
| `cli: warnings` | 6 | differs: README T058 item 8 | 1 of 6 probes differ: `bad-flag` |
| `cli: workspace add` | 4 | equal |  |
| `cli: workspace delete` | 4 | equal |  |
| `cli: workspace edit` | 6 | equal |  |
| `cli: workspace list` | 2 | differs: README T058 item 1 | 1 of 2 probes differ: `no-store` |
| `mcp prompt: investigate` | 1 | differs: new |  |
| `mcp protocol: initialize` | 4 | differs: new | 2 of 4 probes differ: `handshake`, `unknown-method` |
| `mcp tool: connector_catalog` | 1 | equal |  |
| `mcp tool: connector_status` | 2 | differs: README ConnTool item 1 | 1 of 2 probes differ: `keychain-and-claude` |
| `mcp tool: connector_suggestions` | 1 | equal |  |
| `mcp tool: improvement_plan` | 1 | equal |  |
| `mcp tool: kit_findings` | 1 | equal |  |
| `mcp tool: kit_summary` | 1 | equal |  |
| `mcp tool: ping` | 1 | equal |  |
| `mcp tool: save_report` | 2 | differs: README T047 item 1, T047 item 6 | 2 of 2 probes differ: `valid-and-store`, `refusals` |
| `mcp tool: session_outliers` | 1 | equal |  |
| `mcp tool: usage_breakdown` | 1 | differs: README T047 item 1 |  |
| `mcp tool: usage_summary` | 1 | differs: README T047 item 1 |  |
| `mcp tool: warnings` | 1 | equal |  |
| `route: DELETE /api/connectors/{name}` | 2 | equal | key order of 3 bodies differs (N6) |
| `route: DELETE /api/workspaces/{id}` | 2 | equal | key order of 2 bodies differs (N6) |
| `route: GET / (static files of the page)` | 1 | differs: README Static item 1 |  |
| `route: GET /api/accounts` | 1 | equal | key order of 2 bodies differs (N6) |
| `route: GET /api/board` | 1 | equal | key order of 3 bodies differs (N6) |
| `route: GET /api/connectors` | 2 | differs: README ConnRoute item 1 | 1 of 2 probes differ: `configured`; key order of 1 body differs (N6) |
| `route: GET /api/connectors/legacy` | 1 | equal | key order of 1 body differs (N6) |
| `route: GET /api/dirs` | 1 | equal | key order of 4 bodies differs (N6) |
| `route: GET /api/first-run` | 1 | differs: README T040 item 1 |  |
| `route: GET /api/jobs` | 1 | equal | key order of 1 body differs (N6) |
| `route: GET /api/kit` | 1 | equal | key order of 3 bodies differs (N6) |
| `route: GET /api/kit/report-file` | 1 | equal |  |
| `route: GET /api/kit/report-files` | 1 | equal |  |
| `route: GET /api/kit/reports` | 1 | equal | key order of 1 body differs (N6) |
| `route: GET /api/kit/workspaces` | 1 | equal | key order of 3 bodies differs (N6) |
| `route: GET /api/live` | 2 | differs: new | 2 of 2 probes differ: `first-event`, `query-token` |
| `route: GET /api/memory-graph` | 1 | equal | key order of 2 bodies differs (N6) |
| `route: GET /api/plan` | 1 | equal |  |
| `route: GET /api/reports` | 1 | equal | key order of 1 body differs (N6) |
| `route: GET /api/reports/{id}` | 1 | equal | key order of 2 bodies differs (N6) |
| `route: GET /api/store/health` | 1 | differs: README T059 item 6 | key order of 1 body differs (N6) |
| `route: GET /api/sync` | 1 | equal | key order of 1 body differs (N6) |
| `route: GET /api/usage/breakdown` | 1 | equal | key order of 6 bodies differs (N6) |
| `route: GET /api/usage/outliers` | 1 | equal | key order of 3 bodies differs (N6) |
| `route: GET /api/usage/sessions` | 1 | equal | key order of 4 bodies differs (N6) |
| `route: GET /api/usage/summary` | 1 | equal | key order of 5 bodies differs (N6) |
| `route: GET /api/warnings` | 1 | equal | key order of 3 bodies differs (N6) |
| `route: GET /api/workspaces` | 2 | differs: README Static item 1 | 1 of 2 probes differ: `guard`; key order of 1 body differs (N6) |
| `route: GET /api/workspaces/unassigned` | 1 | equal |  |
| `route: PATCH /api/workspaces/{id}` | 3 | equal | key order of 3 bodies differs (N6) |
| `route: POST /api/accounts` | 1 | equal |  |
| `route: POST /api/accounts/{id}/confirm` | 1 | equal | key order of 1 body differs (N6) |
| `route: POST /api/accounts/{id}/ignore` | 1 | equal | key order of 1 body differs (N6) |
| `route: POST /api/connectors` | 2 | equal | key order of 1 body differs (N6) |
| `route: POST /api/connectors/legacy` | 1 | equal | key order of 3 bodies differs (N6) |
| `route: POST /api/connectors/{id}/move` | 1 | equal | key order of 2 bodies differs (N6) |
| `route: POST /api/sync` | 2 | equal | key order of 3 bodies differs (N6) |
| `route: POST /api/workspaces` | 2 | equal | key order of 1 body differs (N6) |
| `route: PUT /api/accounts/{id}/workspaces` | 1 | equal | key order of 1 body differs (N6) |
| `route: PUT /api/connectors/{id}/alias` | 1 | equal | key order of 2 bodies differs (N6) |

## Findings (`differs: new`)

Each is reported, not fixed (AC5). The proposed task names a Complexity and a Risk.

### Finding: `cli: --version`

Cause: Python prints the bare package version (`0.4.0`); Rust prints clap's `easy-personal-rs 0.2.0` (its Cargo version). The README's build section shows the Rust text but lists it in no "Decided for you" list. The numbers differ because the two packages are versioned apart.

Reproduce: `./parity/.venv/bin/python parity/run_parity.py --only 'cli: --version / version'` (probe `version`; world: store=none).
Run: `--version`

- at `steps[0]/stdout` (zone UTC):
  - Python: `"0.4.0\n"`
  - Rust: `"easy-personal-rs 0.2.0\n"`

Proposed task: C0, Risk Low: decide whether `--version` should print `easy-personal-rs <version>` on purpose (then add a line to a README list) or Python's bare form.

### Finding: `cli: mcp`

Cause: `mcp --transport http` stopped with Ctrl-C: Python's process ends by the SIGINT signal (exit -2, shell 130) after uvicorn's shutdown, and uvicorn prints one access-log line per request on stdout; Rust exits 0 with an empty stdout. The README ("`mcp --transport http`") states Rust's side ("stdout stays empty", "Ctrl-C stops the server with exit 0") and does not say Python differs. (The SSE body and the session id are named by T051 item 2 and are not part of this finding.)

Reproduce: `./parity/.venv/bin/python parity/run_parity.py --only 'cli: mcp / http-handshake-and-ping'` (probe `http-handshake-and-ping`; world: store=rich).
Run: probe `http-handshake-and-ping`

- at `steps[4]/code` (zone UTC):
  - Python: `-2`
  - Rust: `0`
- at `steps[4]/stdout` (zone UTC):
  - Python: `"INFO:     127.0.0.1:<PORT> - \"POST /mcp HTTP/1.1\" 200 OK\nINFO:     127.0.0.1:<PORT> - \"POST /mcp HTTP/1.1\" 202 Accepted\nINFO:     127.0.0.1:<PORT> - \"POST /mcp HTTP/1.1\" 200 OK\nINFO:     127.0.0.1:<PORT> - \"PO...`
  - Rust: `""`

Proposed task: C0, Risk Low: add one line to the T051 "Decided for you" list (Rust's exit 0 is the better one); nothing to change in the code.

### Finding: `mcp prompt: investigate`

Cause: `prompts/get` of an unknown prompt: Python's error code is `0`, Rust's `-32602`; the message `Unknown prompt: nope` is equal. The README says only "a JSON-RPC error".

Reproduce: `./parity/.venv/bin/python parity/run_parity.py --only 'mcp prompt: investigate / prompts'` (probe `prompts`; world: store=rich-disk).
Run: `prompts/list  ; prompts/get {"name": "investigate", "arguments": {}} ; prompts/get {"name": "investigate", "arguments": {"workspace": "demo"}} ; prompts/get {"name": "investigate", "arguments": {"workspace": "a\"b", "account": "c"}} ; prompts/get {"name": "nope", "arguments": {}}`

- at `steps[4]/answer/error/code` (zone UTC):
  - Python: `0`
  - Rust: `-32602`

Proposed task: C0, Risk Low: add the code to the T047 "Decided for you" list (Rust's `-32602` is the correct one).

### Finding: `mcp protocol: initialize`

Cause: Two differences no README item names. (1) `initialize` capabilities: Python also sends `experimental: {}`, `resources: {subscribe, listChanged: false}` and `listChanged: false` under `tools` and `prompts`; Rust sends `prompts: {}` and `tools: {}` only (the README names the `prompts` one, in the `investigate` paragraph; the server name and version are named by "McpInit" and are not part of this finding). (2) An unknown JSON-RPC method: Python answers `-32602` `Invalid request parameters`, Rust `-32601` with the method name (`-32601` is the JSON-RPC code for it).

Reproduce: `./parity/.venv/bin/python parity/run_parity.py --only 'mcp protocol: initialize / handshake'` (probe `handshake`; world: store=rich-disk).
Run: `initialize`

- at `steps[0]/answer/result/capabilities/experimental` (zone UTC):
  - Python: `{}`
  - Rust: `"<absent>"`
- at `steps[0]/answer/result/capabilities/prompts/listChanged` (zone UTC):
  - Python: `false`
  - Rust: `"<absent>"`
- at `steps[0]/answer/result/capabilities/resources` (zone UTC):
  - Python: `{"subscribe": false, "listChanged": false}`
  - Rust: `"<absent>"`
- at `steps[0]/answer/result/capabilities/tools/listChanged` (zone UTC):
  - Python: `false`
  - Rust: `"<absent>"`

Reproduce: `./parity/.venv/bin/python parity/run_parity.py --only 'mcp protocol: initialize / unknown-method'` (probe `unknown-method`; world: store=rich-disk).
Run: `nope/method `

- at `steps[0]/answer/error/code` (zone UTC):
  - Python: `-32602`
  - Rust: `-32601`
- at `steps[0]/answer/error/data` (zone UTC):
  - Python: `""`
  - Rust: `"<absent>"`
- at `steps[0]/answer/error/message` (zone UTC):
  - Python: `"Invalid request parameters"`
  - Rust: `"nope/method"`

Proposed task: C1, Risk Low: align the capabilities (a client that checks `resources` sees none) or list both differences in the T047 "Decided for you" list; keep `-32601` on purpose and say so.

### Finding: `route: GET /api/live`

Cause: The content type is `text/event-stream; charset=utf-8` in Python and `text/event-stream` in Rust. The README (T056 "The route") states Rust's value but not that Python's differs. A browser `EventSource` reads both.

Reproduce: `./parity/.venv/bin/python parity/run_parity.py --only 'route: GET /api/live / first-event'` (probe `first-event`; world: store=rich-disk).
Run: `GET /api/live`

- at `steps[0]/type` (zone UTC):
  - Python: `"text/event-stream; charset=utf-8"`
  - Rust: `"text/event-stream"`

Reproduce: `./parity/.venv/bin/python parity/run_parity.py --only 'route: GET /api/live / query-token'` (probe `query-token`; world: store=rich-disk).
Run: `GET /api/live ; GET /api/live`

- at `steps[0]/type` (zone UTC):
  - Python: `"text/event-stream; charset=utf-8"`
  - Rust: `"text/event-stream"`

Proposed task: C0, Risk Low: add `; charset=utf-8` to the Rust header (or add the difference to the T056 list).

## Differences the README already names

| README item | What it says (start) | Rows |
|---|---|---|
| ConnRoute item 1 | **`configured` is always `false`**: Python answers `true` only when the OS keychain holds the item *and* `claude mcp get` knows the entry, and answers... | `route: GET /api/connectors` |
| ConnTool item 1 | `connector_status` -> `{"connectors": [{id, configured}]}`, **`configured` always `false`** for the reason given for `GET /api/connectors` above;... | `mcp tool: connector_status` |
| Ingest item 1 | / prints `kit_trace`, `context`, `ui`, `verifier` / prints the same four keys (T051 ported `context`) and `skipped` / one JSON, Python's keys /... | `cli: ingest` |
| Schedule item 1 | **The command line is the Rust binary**: `ExecStart=<absolute path of the running binary, shell-quoted> ingest [--retention-days N]`. Install from the... | `cli: schedule install` |
| Static item 1 | `serve --static-dir <DIR>` also serves the built dashboard's files from `<DIR>` on the same origin, behind the same guard and with the same CSP on eve... | `route: GET / (static files of the page)`, `route: GET /api/workspaces` |
| T040 item 1 | Decided for you (T040; Rust differs from Python): a report whose snapshot Python cannot read is a fixed `500` here (Python: a traceback), and in the l... | `route: GET /api/first-run` |
| T047 item 1 | A wrong argument type gets a short fixed text; Python's pydantic text echoes the offending value (`input_value=...`), which for a summary would put th... | `mcp tool: save_report`, `mcp tool: usage_breakdown`, `mcp tool: usage_summary` |
| T047 item 6 | `server_version` is this build's Cargo version (Python's is its package version).... | `mcp tool: save_report` |
| T051 item 1 | `--no-ingest` exists on `serve` and `app` (Python's `dashboard` has no such flag). Python's `dashboard` stops with the ingest's exit code when the ing... | `cli: dashboard` |
| T058 item 1 | **A missing store, or one newer than this build, is refused, and nothing is created or migrated.** Python's `store.opened` creates the store and migra... | `cli: usage`, `cli: workspace list` |
| T058 item 12 | **Not here**: nothing since T060 (`connector` and `doctor`). `exec` and `headers` are the separate `easy-personal-launch` program (T050). `store-check... | `cli: exec`, `cli: headers` |
| T058 item 7 | **Which input is echoed.** An error text is the core's fixed text, which names none of the input (NFR-002). Two things name what the owner typed on th... | `cli: usage` |
| T058 item 8 | **Arguments.** `--limit` is read like Python's `int()` (a sign, spaces, `1_0`); digits of other scripts are refused, and a number past 64 bits is vali... | `cli: account list`, `cli: ingest`, `cli: kit`, `cli: kit-findings`, `cli: plan`, `cli: usage`, `cli: warnings` |
| T059 item 6 | **JSON key order in the route.** The route sends serde_json's sorted keys; Python's route sends keys in insertion order. The parsed objects are equal.... | `route: GET /api/store/health` |
| T060 item 1 | **The terminal.** Echo is turned off through the C library's `tcgetattr` / `tcsetattr` (no new crate; Linux's `termios` layout, so Linux only: elsewhe... | `cli: connector add` |
| T060 item 3 | **The question is asked with no store handle open** (as T058 #3) for `remove`, `sync`, `move`, `alias` and `adopt-legacy`: the plan is worked out agai... | `cli: connector adopt-legacy`, `cli: connector alias`, `cli: connector move`, `cli: connector remove`, `cli: connector sync` |
| T060 item 5 | **`doctor`'s keychain line names this build's backend**: `ok keychain: SecretService` (`FileKeychain` in a debug test run). Python prints its backend'... | `cli: doctor` |

## Rules that make two outputs comparable

- **N1** The world's folder, each app's launcher and binary and the Python interpreter are written as `<WORK>`, `<LAUNCHER>`, `<RUST_BIN>`, `<PYTHON>`. Both apps run at the same world path.
- **N2** A line that starts `easy-personal-rs: ` is read as `easy-personal: ` (README, T058 "Exit codes").
- **N3** The clock's own values (`as_of`, `as_of_utc`, `now_as_of_utc`, `saved_utc`) are `<NOW>`. Rust has no clock hook: both apps read the machine's clock, and the rich store's events are months old, so every rule abstains (as in `gen_cli.py`).
- **N4** Ids the app makes at random: a new connector id (`c` and 8 hex digits) and `meta.machine_id`. Session and message ids of the store are never rewritten.
- **N5** The keychain is compared as its items; a world with no backend is `none` for both.
- **N6** A route body is compared as parsed JSON (key order ignored, list order kept), as the project's own rule says (`rust/parity/README.md`, "Key order: which rule wins (T007)"). Python sends keys in insertion order, Rust sorted. The number of bodies whose order differs is in the Note column; the rows whose README item says it (T059 item 6) cite it. MCP results are parsed the same way (the text content is parsed when it is JSON).

## Rust-only surfaces (not in the Python inventory, not compared)

- `cli: app`
- `cli: serve`
- `cli: store-info`
- `route: GET /api/version`

## What this run does not reach

- The real keychain, the real `claude`, `systemctl`/`crontab` and the real browser: fakes only. `schedule install` is run with `--dry-run` and with no systemd (the cron path); the install itself is not.
- Python's live tail writes events to the store (README T049 item 1: Rust's never does); the world holds no transcript, so the tail has nothing to read and `GET /api/live` is compared on its first (empty) snapshot.
- The clock: see N3. Rules that fire, and the improvement plan over a store with recent events, are proven by the frozen-clock goldens, not here.
- Performance, fuzzing, a second store, macOS and Windows (cut list of the guide).
