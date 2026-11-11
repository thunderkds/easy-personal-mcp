# Main flow

How the Rust app sits next to the Python app during the transition, and how a request flows once it is running.

```mermaid
flowchart LR
  subgraph Sources["Local sources (read only)"]
    T[Claude Code transcripts]
    G[git repos]
    M[easy-memory store]
  end
  subgraph Rust["easy-personal-rs (one binary)"]
    ING[Ingest P2]
    CORE[Core: metrics, rules, kit, memory graph]
    API["/api + MCP on 127.0.0.1 (token guard)"]
    CLI[CLI]
    WIN[Desktop window P1b]
  end
  DB[("~/.easy-personal/store.sqlite\nWAL, user_version, one writer")]
  PY[Python app: parity oracle, then sunset]
  UI[React bundle served as-is]

  T --> ING
  G --> ING
  M --> CORE
  ING --> DB
  DB --> CORE
  CORE --> API
  CORE --> CLI
  API --> WIN
  UI --> WIN
  PY -. same file, one writer at a time .- DB
```

## Request flow (P1+)
1. The window (or the CLI, or an MCP client) calls `/api/*` or an MCP tool on `127.0.0.1`.
2. The Host/Origin/token guard checks the request.
3. The core reads the store (read-only connection, `busy_timeout`) and answers the same JSON shape as Python.
4. Writes (P2+) go through the single-writer lock; a newer `user_version` than the code knows is refused.
