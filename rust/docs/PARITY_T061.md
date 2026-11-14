# T061: parity of Python and Rust, surface by surface

Made by `rust/parity/run_parity.py`; checked by `rust/parity/test_parity_report.py`. Nothing in this file is written by hand.

Counts: equal 26 + equal except JSON key order 29, differs README 32, differs new 0, not run 0.

`equal except JSON key order`: the parsed bodies are equal and the key order of at least one body differs (Python sends insertion order, Rust sorted; rule N6).

86 surfaces were read from the Python app (argparse tree, route table, MCP tools and prompt); 1 protocol row is added. 181 probes ran, each on the Python app and then on the Rust app over the same world at the same path. Leak scan: 181 captured outputs and this report scanned with the repo canaries (`rust/parity/leak_rust.py` scanner, `Canaries.find`): 0 hits.

## How it ran

- A *probe* is one or more calls of a surface. Each probe builds a world (a copy of `store-rich`, the agent folders, a fake `claude`, a file keychain, fake `systemctl` and `crontab`), runs Python (`python -c "...cli.main()"` over this checkout's `src/`, or its `serve` and `mcp`), removes the world, builds the same world at the same path, and runs `target/debug/easy-personal-rs`. The two never run together on one store.
- The routes are asked over a real socket: Python's `serve` runs after `LiveTail().start()` (what `dashboard` does after its ingest), with the page's folder pointed at the world's; Rust's is `serve --no-ingest [--static-dir]`. `dashboard` itself is probed separately, with its ingest.
- JSON key order: Python sends insertion order, Rust sorted. The headline counts `equal except JSON key order` apart from `equal`. The rule is the project's (`rust/parity/README.md`, "Key order: which rule wins (T007)", and `rust/README.md` T059 item 6); see N6. MCP result key order is not tracked.
- Both children get an environment built from scratch: no `CLAUDE_CONFIG_DIR`, no display, a dead D-Bus address, `HOME`, `EASY_PERSONAL_HOME`, `EASY_PERSONAL_AGENT_ROOT` and the other folders inside the world, `TZ` (UTC; `Pacific/Kiritimati` and `Pacific/Pago_Pago` too for the rows that print a time). No owner folder, real keychain item or real `claude` is touched.
- Compared: CLI = exit code, stdout, stderr; route = status, content type, parsed JSON body (or the text); MCP = the `tools/list` entry and each call's result; and after every probe the store rows that changed, the keychain items, the `claude` calls and the fake `claude`'s state.
- A row is `differs: README <list> item <n>` only when that README item exists, holds the words the runner states for it, and a transformation that stands for the item makes the two outputs equal (the part the item does not describe stays, and the row is `differs: new`). The test checks the item and its words.
- The `<list>` names: `Tnnn` is the numbered "Decided for you (Tnnn ..." list of `rust/README.md` (T047's is the inline `(1) (2)` list; T040's is one paragraph, item 1). `Ingest` is the table "Differences from Python" of "Ingest (P2a, P2b)" (rows count from 1), `Schedule` the bullets of "Scheduled ingest (P2c)", `McpHttp` the table of "`mcp --transport http`", and `Static`, `ConnRoute`, `ConnTool`, `McpInit` single README paragraphs (item 1). These sections state a difference but are not "Decided for you" lists.

## Result per surface

| Surface | Probes | Result | Note |
|---|---|---|---|
| `cli: --version` | 1 | differs: README T058 item 13 |  |
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
| `cli: kit` | 6 | differs: README T058 item 8 | 1 of 6 probes differ: `bad-flag`; no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `cli: kit-findings` | 5 | differs: README T058 item 8 | 1 of 5 probes differ: `bad-limit`; no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `cli: mcp` | 3 | differs: README McpHttp item 1, T051 item 2, T051 item 8 | 2 of 3 probes differ: `http-handshake-and-ping`, `bad-host` |
| `cli: outliers` | 5 | equal | no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `cli: ping` | 1 | equal |  |
| `cli: plan` | 6 | differs: README T058 item 8 | 1 of 6 probes differ: `bad-flag`; no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `cli: schedule install` | 3 | differs: README Schedule item 1 | 3 of 3 probes differ: `dry-run`, `dry-run-retention`, `no-systemd` |
| `cli: schedule remove` | 1 | equal |  |
| `cli: store-check` | 4 | equal |  |
| `cli: usage` | 12 | differs: README T058 item 1, T058 item 14 | 3 of 12 probes differ: `bad-by`, `bad-date`, `no-store` |
| `cli: warnings` | 6 | differs: README T058 item 8 | 1 of 6 probes differ: `bad-flag`; no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `cli: workspace add` | 4 | equal |  |
| `cli: workspace delete` | 4 | equal |  |
| `cli: workspace edit` | 6 | equal |  |
| `cli: workspace list` | 2 | differs: README T058 item 1 | 1 of 2 probes differ: `no-store` |
| `mcp prompt: investigate` | 1 | differs: README T047 item 7 |  |
| `mcp protocol: initialize` | 5 | differs: README McpInit item 1, T047 item 8 | 3 of 5 probes differ: `handshake`, `unknown-method`, `stderr-log` |
| `mcp tool: connector_catalog` | 1 | equal |  |
| `mcp tool: connector_status` | 2 | differs: README ConnTool item 1 | 1 of 2 probes differ: `keychain-and-claude` |
| `mcp tool: connector_suggestions` | 1 | equal |  |
| `mcp tool: improvement_plan` | 1 | equal | no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `mcp tool: kit_findings` | 1 | equal | no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `mcp tool: kit_summary` | 1 | equal | no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `mcp tool: ping` | 1 | equal |  |
| `mcp tool: save_report` | 2 | differs: README T047 item 1, T047 item 6 | 2 of 2 probes differ: `valid-and-store`, `refusals` |
| `mcp tool: session_outliers` | 1 | equal | no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `mcp tool: usage_breakdown` | 1 | differs: README T047 item 1 |  |
| `mcp tool: usage_summary` | 1 | differs: README T047 item 1 |  |
| `mcp tool: warnings` | 1 | equal | no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `route: DELETE /api/connectors/{name}` | 2 | equal except JSON key order | key order of 3 bodies differs (N6) |
| `route: DELETE /api/workspaces/{id}` | 2 | equal except JSON key order | key order of 2 bodies differs (N6) |
| `route: GET / (static files of the page)` | 1 | differs: README Static item 1 |  |
| `route: GET /api/accounts` | 1 | equal except JSON key order | key order of 2 bodies differs (N6) |
| `route: GET /api/board` | 1 | equal except JSON key order | key order of 3 bodies differs (N6) |
| `route: GET /api/connectors` | 2 | differs: README ConnRoute item 1 | 1 of 2 probes differ: `configured`; key order of 1 body differs (N6) |
| `route: GET /api/connectors/legacy` | 1 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: GET /api/dirs` | 1 | equal except JSON key order | key order of 4 bodies differs (N6) |
| `route: GET /api/first-run` | 1 | differs: README T040 item 1 |  |
| `route: GET /api/jobs` | 1 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: GET /api/kit` | 1 | equal except JSON key order | key order of 3 bodies differs (N6); no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `route: GET /api/kit/report-file` | 1 | equal |  |
| `route: GET /api/kit/report-files` | 1 | equal |  |
| `route: GET /api/kit/reports` | 1 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: GET /api/kit/workspaces` | 1 | equal except JSON key order | key order of 3 bodies differs (N6) |
| `route: GET /api/live` | 2 | equal |  |
| `route: GET /api/memory-graph` | 1 | equal except JSON key order | key order of 2 bodies differs (N6) |
| `route: GET /api/plan` | 1 | equal | no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `route: GET /api/reports` | 1 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: GET /api/reports/{id}` | 1 | equal except JSON key order | key order of 2 bodies differs (N6) |
| `route: GET /api/store/health` | 1 | differs: README T059 item 6 | key order of 1 body differs (N6) |
| `route: GET /api/sync` | 1 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: GET /api/usage/breakdown` | 1 | equal except JSON key order | key order of 6 bodies differs (N6) |
| `route: GET /api/usage/outliers` | 1 | equal except JSON key order | key order of 3 bodies differs (N6); no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `route: GET /api/usage/sessions` | 1 | equal except JSON key order | key order of 4 bodies differs (N6) |
| `route: GET /api/usage/summary` | 1 | equal except JSON key order | key order of 5 bodies differs (N6) |
| `route: GET /api/warnings` | 1 | equal except JSON key order | key order of 3 bodies differs (N6); no rule fires at this clock (events are months old); rule firing is covered by the frozen-clock goldens |
| `route: GET /api/workspaces` | 3 | differs: README Static item 1, T048 item 15 | 2 of 3 probes differ: `guard`, `guard-with-page`; key order of 1 body differs (N6); with the page served (a real deployment): Python's static mount answers where its routes would say 405 |
| `route: GET /api/workspaces/unassigned` | 1 | equal |  |
| `route: PATCH /api/workspaces/{id}` | 3 | equal except JSON key order | key order of 3 bodies differs (N6) |
| `route: POST /api/accounts` | 1 | equal |  |
| `route: POST /api/accounts/{id}/confirm` | 1 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: POST /api/accounts/{id}/ignore` | 1 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: POST /api/connectors` | 2 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: POST /api/connectors/legacy` | 1 | equal except JSON key order | key order of 3 bodies differs (N6) |
| `route: POST /api/connectors/{id}/move` | 1 | equal except JSON key order | key order of 2 bodies differs (N6) |
| `route: POST /api/sync` | 2 | equal except JSON key order | key order of 3 bodies differs (N6) |
| `route: POST /api/workspaces` | 2 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: PUT /api/accounts/{id}/workspaces` | 1 | equal except JSON key order | key order of 1 body differs (N6) |
| `route: PUT /api/connectors/{id}/alias` | 1 | equal except JSON key order | key order of 2 bodies differs (N6) |

## Findings (`differs: new`)

None.

## Differences the README already names

| README item | What it says (start) | Rows |
|---|---|---|
| ConnRoute item 1 | **`configured` is always `false`**: Python answers `true` only when the OS keychain holds the item *and* `claude mcp get` knows the entry, and answers... | `route: GET /api/connectors` |
| ConnTool item 1 | `connector_status` -> `{"connectors": [{id, configured}]}`, **`configured` always `false`** for the reason given for `GET /api/connectors` above;... | `mcp tool: connector_status` |
| Ingest item 1 | / prints `kit_trace`, `context`, `ui`, `verifier` / prints the same four keys (T051 ported `context`) and `skipped` / one JSON, Python's keys /... | `cli: ingest` |
| McpHttp item 1 | a host other than `127.0.0.1` (`0.0.0.0`, `localhost`, `::1`, a name), with `stdio` too, as Python / exit 2 and one line, before any socket is bound /... | `cli: mcp` |
| McpInit item 1 | `initialize` reports `easy-personal-rs` and the Cargo version; the revisions with the handshake (2024-11-05 to 2025-11-25) are served, the 2026-07-28 ... | `mcp protocol: initialize` |
| Schedule item 1 | **The command line is the Rust binary**: `ExecStart=<absolute path of the running binary, shell-quoted> ingest [--retention-days N]`. Install from the... | `cli: schedule install` |
| Static item 1 | `serve --static-dir <DIR>` also serves the built dashboard's files from `<DIR>` on the same origin, behind the same guard and with the same CSP on eve... | `route: GET / (static files of the page)`, `route: GET /api/workspaces` |
| T040 item 1 | Decided for you (T040; Rust differs from Python): a report whose snapshot Python cannot read is a fixed `500` here (Python: a traceback), and in the l... | `route: GET /api/first-run` |
| T047 item 1 | A wrong argument type gets a short fixed text; Python's pydantic text echoes the offending value (`input_value=...`), which for a summary would put th... | `mcp tool: save_report`, `mcp tool: usage_breakdown`, `mcp tool: usage_summary` |
| T047 item 6 | `server_version` is this build's Cargo version (Python's is its package version).... | `mcp tool: save_report` |
| T047 item 7 | An unknown prompt (`prompts/get` of `nope`) is the JSON-RPC error `-32602` (invalid params) with the message `Unknown prompt: nope`; the message is Py... | `mcp prompt: investigate` |
| T047 item 8 | Three more differences in the protocol under the tools, all kept: (a) `initialize` capabilities are `prompts: {}` and `tools: {}` only; Python also se... | `mcp protocol: initialize` |
| T048 item 15 | (T067) **With the page served, a `GET` of a route that has only write methods is `405` here.** Python answers `404 Not Found` there: its static mount ... | `route: GET /api/workspaces` |
| T051 item 1 | `--no-ingest` exists on `serve` and `app` (Python's `dashboard` has no such flag). Python's `dashboard` stops with the ingest's exit code when the ing... | `cli: dashboard` |
| T051 item 2 | `mcp --transport http` is stateless and answers JSON, not SSE; `GET` and `DELETE` are 405; there is no `Mcp-Session-Id`. `--port` is a `u16` (Python: ... | `cli: mcp` |
| T051 item 8 | **Ctrl-C on `mcp --transport http` ends the server with exit 0 and an empty stdout.** Python's process ends by the SIGINT signal (exit code -2, shell ... | `cli: mcp` |
| T058 item 1 | **A missing store, or one newer than this build, is refused, and nothing is created or migrated.** Python's `store.opened` creates the store and migra... | `cli: usage`, `cli: workspace list` |
| T058 item 12 | **Not here**: nothing since T060 (`connector` and `doctor`). `exec` and `headers` are the separate `easy-personal-launch` program (T050). `store-check... | `cli: exec`, `cli: headers` |
| T058 item 13 | **`--version` prints `easy-personal-rs <version>`** (clap's text and this build's Cargo version, for example `easy-personal-rs 0.2.0`). Python prints ... | `cli: --version` |
| T058 item 14 | **Two usage errors have a one-line text, not argparse's.** `usage --by week` prints `easy-personal: by must be one of day, model, project, tool` (Pyth... | `cli: usage` |
| T058 item 8 | **Arguments.** `--limit` is read like Python's `int()` (a sign, spaces, `1_0`); digits of other scripts are refused, and a number past 64 bits is vali... | `cli: account list`, `cli: ingest`, `cli: kit`, `cli: kit-findings`, `cli: plan`, `cli: warnings` |
| T059 item 6 | **JSON key order in the route.** The route sends serde_json's sorted keys; Python's route sends keys in insertion order. The parsed objects are equal.... | `route: GET /api/store/health` |
| T060 item 1 | **The terminal.** Echo is turned off through the C library's `tcgetattr` / `tcsetattr` (no new crate; Linux's `termios` layout, so Linux only: elsewhe... | `cli: connector add` |
| T060 item 3 | **The question is asked with no store handle open** (as T058 #3) for `remove`, `sync`, `move`, `alias` and `adopt-legacy`: the plan is worked out agai... | `cli: connector adopt-legacy`, `cli: connector alias`, `cli: connector move`, `cli: connector remove`, `cli: connector sync` |
| T060 item 5 | **`doctor`'s keychain line names this build's backend**: `ok keychain: SecretService` (`FileKeychain` in a debug test run). Python prints its backend'... | `cli: doctor` |

## Rules that make two outputs comparable

- **N1** The world's folder, each app's launcher and binary and the Python interpreter are written as `<WORK>`, `<LAUNCHER>`, `<RUST_BIN>`, `<PYTHON>`. Both apps run at the same world path.
- **N2** A line that starts `easy-personal-rs: ` is read as `easy-personal: ` (README, T058 "Exit codes").
- **N3** The clock's own values (`as_of`, `as_of_utc`, `now_as_of_utc`, `saved_utc`, as a JSON key or inside a JSON text) and the bare `saved_utc` column of `investigate_reports` (the second the report was saved in: the two apps run a moment apart) are `<NOW>`. Rust has no clock hook: both apps read the machine's clock, and the rich store's events are months old, so every rule abstains (as in `gen_cli.py`).
- **N4** Ids the app makes at random: a new connector id (`c` and 8 hex digits) and `meta.machine_id`. Session and message ids of the store are never rewritten.
- **N5** The keychain is compared as its items; a world with no backend is `none` for both.
- **N6** A route body is compared as parsed JSON (key order ignored, list order kept). Python sends keys in insertion order, Rust sorted. The rule is the project's own: `rust/parity/README.md`, "Key order: which rule wins (T007)" (the Rust tests compare parsed JSON), and `rust/README.md` T059 item 6 for `/api/store/health`, which cites it. A row whose bodies are equal except for key order is counted apart in the headline (`equal except JSON key order`) and its Note gives the number of bodies; T059 item 6 is the only README item that names it, so that row is `differs: README`. MCP result key order is not tracked: tool results are compared as parsed JSON (the text content is parsed when it is JSON).
- **N7** Python's MCP SDK writes `Processing request of type <X>` on stderr for every request; Rust writes nothing. Those lines are dropped from every MCP probe's stderr, except in `mcp protocol: initialize / stderr-log`, which compares stderr as it is and carries the finding.

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

## Known limits

- The negative control (Rust on a store with one more workspace) proves the CLI stdout comparison only, not the state, keychain, `claude`-call, HTTP or MCP comparisons.
- Rule T058 item 1 (`r_missing_store`) checks Rust's exact refusal and, for Python, only that its step 0 exited 0 and that store rows were written.
- Rule T047 item 1 (`r_argument_error`) is applied to a missing argument as well as a wrong type; the item names only "a wrong argument type" (`save_report` without `summary`, `usage_breakdown` without `by`).
- Rule T047 item 6 sets column 2 (`server_version`) of every `investigate_reports` row to `<VERSION>` without checking it.
- Rule T060 item 3 drops every `claude mcp get` call, argv included: a wrong `get` argument would not show.
- Mask N4's `c` + 8 hex digits is applied to all text, not only to ids.
- Rule T058 item 13 matches `<digits>.<digits>.<digits>` against `easy-personal-rs <digits>.<digits>.<digits>`; it does not check that the number is the package's version.
- Rules T047 item 7 and item 8 match Python's stderr dump by its first line and its shape (a traceback, a list of pydantic errors), not line by line.
- A signal that arrives in the few instructions between a child's start and the runner's note of it can leave that child alive (SIGTERM and SIGHUP are handled like Ctrl-C).
- Python's `dashboard` command (probe `cli: dashboard`) cannot be told a static folder, so it serves the packaged bundle if the machine has one; the probe asks one API route, which the bundle does not change.
- The static fixture holds no dotfile and no symlink (the README's `Static` paragraph names both).
- Rules N3 and the clock caveat: the rows for warnings, plan, kit, outliers and kit-findings carry it in their Note.
