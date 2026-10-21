---
name: verify
description: How to build, launch and drive easy-personal-mcp for runtime verification (CLI, MCP stdio, MCP HTTP). Read by /verify before a cold start.
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

## Probes worth repeating

- `--host 0.0.0.0`, `localhost`, `::1` → refused, exit 2 (NFR-001).
- Bad `--port` / `--transport` → argparse error, exit 2.
- Planted-canary fixtures (once sources exist): grep every surface's output for the canary (NFR-002).

## Gotchas

- The Bash tool runs zsh: `$B $args` does not word-split a string. Wrap probe loops in `bash -c '...'`.
- `serverInfo.version` in `initialize` is the MCP SDK version, not the package version.
- The MCP server logs `Processing request of type …` to stderr on every request; stdout carries protocol only.
- Never point verification at the owner's real `~/.claude/projects/` with content-printing commands; use the fixtures under `tests/fixtures/`.
