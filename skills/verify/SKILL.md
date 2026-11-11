---
name: verify
description: How to build, launch and drive easy-personal-mcp for runtime verification (CLI, MCP stdio, MCP HTTP), and its Rust port easy-personal-rs. Read by /verify before a cold start.
---

# Verify recipe — easy-personal-mcp

Run from the task's worktree. Isolate everything in a temp dir so the owner's real install and data stay untouched.

```bash
T=$(mktemp -d)
export EASY_PERSONAL_HOME=$T/home UV_TOOL_DIR=$T/tools UV_TOOL_BIN_DIR=$T/bin
uv tool install -q .
B=$T/bin/easy-personal
$B --version            # package version
$B ping                 # core reachable from the CLI
```

## MCP stdio (the default surface)

Pipe a real handshake; keep stdin open a moment so responses flush:

```bash
printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"verify","version":"0"}}}' \
 '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
 '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
 '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ping","arguments":{}}}' \
 | (cat; sleep 2) | timeout 10 $B mcp 2>$T/err
```

## MCP HTTP (opt-in)

```bash
($B mcp --transport http --port 18765 >$T/http.log 2>&1 &); sleep 3
ss -ltnp | grep ':18765 '          # must show 127.0.0.1 only
curl -s -X POST http://127.0.0.1:18765/mcp -H 'content-type: application/json' \
  -H 'accept: application/json, text/event-stream' -d '<initialize JSON as above>'
pkill -f "$B mcp --transport http --port 18765"
```

## Rust (easy-personal-rs)

For a change under `rust/`. No `cargo` on the machine: skip this section and say so in the report. Every command runs from `rust/`. If `cargo` is not found, run `. "$HOME/.cargo/env"` first (rustup installed without editing the shell profile). The temp dir keeps the owner's store untouched: the recipe reads a copy of the committed synthetic store, never `EASY_PERSONAL_HOME`'s default.

```bash
T=$(mktemp -d); mkdir $T/home
export EASY_PERSONAL_HOME=$T/home
cp parity/store-v11.sqlite $T/home/store.sqlite
cargo build                                       # default build: no window, no display needed
B=$PWD/target/debug/easy-personal-rs
$B --version                                      # easy-personal-rs <Cargo version>
$B store-info                                     # user_version 11, row counts, no rows, no path
```

`serve` on the fixture page; read the token into a variable and never echo it. Stop by PID:

```bash
$B serve --static-dir crates/app/tests/fixtures/self-test >$T/serve.out 2>$T/serve.err & P=$!
for i in 1 2 3 4 5; do grep -q '^dashboard: ' $T/serve.out && break; sleep 1; done
U=$(grep -m1 '^dashboard: ' $T/serve.out); U=${U#dashboard: }
TOKEN=${U##*#}; BASE=${U%%/#*}                    # URL is http://127.0.0.1:<port>/#<token>
curl -s -w '\n' -H "x-dashboard-token: $TOKEN" $BASE/api/version    # {"version":"<Cargo version>"}
curl -s -o /dev/null -w '%{http_code}\n' $BASE/api/version  # no token: 403
kill $P; wait $P 2>/dev/null; kill -0 $P 2>/dev/null && echo "STILL RUNNING $P" || echo "stopped $P"
```

`mcp` over the temp store; the server ends when stdin closes:

```bash
printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"verify","version":"0"}}}' \
 '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
 '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
 '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"usage_summary","arguments":{}}}' \
 | (cat; sleep 2) | timeout 10 $B mcp 2>$T/mcp.err | cut -c1-160
```

`ingest` into a fresh store, twice, on a committed fixture folder. The data folder and the agent folder are temp folders under `$T/ingest` (the recipe removes nothing; `$T` is yours to delete), so no real account is detected and nothing real is read. The subshell keeps the exports out of the later steps:

```bash
cargo build                                       # the default binary again, in case a window build ran before
B=$PWD/target/debug/easy-personal-rs
I=$T/ingest; mkdir -p $I/home $I/agents
( export EASY_PERSONAL_HOME=$I/home EASY_PERSONAL_AGENT_ROOT=$I/agents
  $B ingest --source parity/ingest/fixtures/s3; echo "exit=$?"   # files 2, new_events 3, skipped 0, exit 0
  $B ingest --source parity/ingest/fixtures/s3; echo "exit=$?"   # new_events 0, exit 0
  $B store-info; echo "exit=$?" )                                # user_version: 11, pending: no, events: 3, exit 0
```

Ingest leak gate (NFR-002, RS-003): plants canary secrets, runs the Rust binary over them and scans every surface. Needs the default build above (it runs `target/debug/easy-personal-rs`) and the parity venv, built once as `rust/parity/README.md` says ("Build the venv"; `uv` is only used there to build it from the lock, running the gate needs none). The venv is git-ignored, so a worktree has none: call the main checkout's `rust/parity/.venv/bin/python` with this worktree's script. Pass: the last line is `leak check: 0 hits over 42 canaries, 116 surfaces, ...` and the exit code is 0 (1 is a hit or a failed check). It takes about 2 seconds once the binary is built.

```bash
./parity/.venv/bin/python parity/leak_rust.py; echo "exit=$?"   # leak check: 0 hits ..., exit 0
```

Window step, last (it replaces the default binary). Run it only when `DISPLAY` or `WAYLAND_DISPLAY` is set and the Tauri system packages (`rust/docs/spike-T001.md`) are installed; otherwise skip it and write "window step skipped: no display", never "pass":

```bash
if [ -n "$DISPLAY$WAYLAND_DISPLAY" ]; then
  cargo build --features window
  timeout 60 $B app --self-test --static-dir crates/app/tests/fixtures/self-test; echo "exit=$?"   # self-test: ok, exit 0
else echo "window step skipped: no display"; fi
```

macOS and Windows are not covered yet (the window and signal handling are Ubuntu only).

## Probes worth repeating

- `--host 0.0.0.0`, `localhost`, `::1` → refused, exit 2 (NFR-001).
- Bad `--port` / `--transport` → argparse error, exit 2.
- Planted-canary fixtures (once sources exist): grep every surface's output for the canary (NFR-002).

## Gotchas

- The Bash tool runs zsh: `$B $args` does not word-split a string. Wrap probe loops in `bash -c '...'`.
- Rust: the Bash tool runs zsh here too; the recipe above uses only POSIX shell syntax and runs in both zsh and bash. Wrap any loop over `$B` arguments in `bash -c '...'`. Do not run the recipe under `set -e`: `wait $P` returns 143 after `kill $P` and ends the shell before the PID check.
- Rust: the window step rebuilds `target/debug/easy-personal-rs` with the Tauri libraries linked in, so run it last (a later plain `cargo build` puts the default binary back). That binary needs `libwebkit2gtk-4.1-0` and `libgtk-3-0t64` even for `serve` and `mcp`; check CLI and MCP with the default build only.
- Rust: on WSLg the window step prints `libEGL` and `MESA` warnings on stderr (`failed to open /dev/dri/...: Permission denied`). They come from the graphics stack, not the app; the result is the `self-test: ok` line and `exit=0`.
- Rust: `serve` prints the token in the `dashboard:` line, so `$T/serve.out` holds it. Cut it from anything you paste into a report. `store-info` makes an empty `store.sqlite-wal` and `-shm` next to the temp copy; that is expected.
- Rust: a cold `target/` makes `cargo build` (and the first `--features window` build) slow; give the Bash call a long timeout. `rust/target/` is the only thing the recipe writes inside the repo, and it is git-ignored.
- `serverInfo.version` in `initialize` is the MCP SDK version, not the package version.
- The MCP server logs `Processing request of type …` to stderr on every request; stdout carries protocol only.
- Never point verification at the owner's real `~/.claude/projects/` with content-printing commands; use the fixtures under `tests/fixtures/`.
