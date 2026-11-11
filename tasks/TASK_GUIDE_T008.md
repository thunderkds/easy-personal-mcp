# TASK_GUIDE — T008: P1 slice — MCP stdio server with `ping`, `usage_summary`, `usage_breakdown`
**Date**: 2026-11-10
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 "Rust port" entries in `memory/decisions.md` and all 2026-11-10 entries in `memory/learnings.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/docs/spike-T001.md`, `BRAINSTORMING_LOG_RUSTPORT.md` and `rust/docs/SPEC.md`
6. Read `src/easy_personal/surfaces/mcp.py` (tool definitions, descriptions, `create_app`), `rust/spikes/t001-crates/src/main.rs` (the rmcp stdio pattern) and `rust/crates/app/tests/server.rs` (spawn-the-binary tests)
7. Read the oracle: `rust/parity/golden/mcp__*.json` and their entries in `rust/parity/golden/manifest.json`; the store is `rust/parity/store-v11.sqlite`

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>` listing the files you actually read, and say which listed files you did not read.

---

## Requirement

**Original request** (owner, 2026-11-10): the Rust app runs all current features, including the MCP server (stdio by default).

**Restated intent**:
> `easy-personal-rs mcp` serves an MCP server on stdio with the tools `ping`, `usage_summary` and `usage_breakdown`, whose results equal the Python app's MCP goldens, and whose `initialize` reports the Rust app's own version. It is the first MCP slice; the other tools follow as their core functions land.

**Out of scope**: the other nine tools (`warnings` waits for T006, the rest later); Streamable HTTP (`--http`); the `investigate` prompt; `save_report` and any write tool; any change to the core crate; the window.

**Requirement Refs**: RS-001 (MCP server info), RS-003 in `rust/docs/SPEC.md`; Python `PRD.md` FR-076 is NOT in scope (write tool).

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T001 — `rmcp` =3.9.0 stdio proven in the spike; T005 — `metrics::usage_summary`, `usage_breakdown`; T003 — the `mcp__*` goldens
**Entry point**: `easy-personal-rs mcp` (CLI subcommand) and the MCP methods `initialize`, `tools/list`, `tools/call`

**Parallel tasks**: you own `rust/crates/app/src/mcp.rs`, `rust/crates/app/tests/mcp.rs`, and the new `Mcp` variant and `mod mcp;` line in `rust/crates/app/src/main.rs` (re-read it before editing; T004 already added `Serve`). Do NOT edit `rust/crates/core/**` (T005 agent and T006 are working there): call its public functions only. `rust/Cargo.toml` and `rust/crates/app/Cargo.toml` are shared: re-read before every edit, add only your own lines (`rmcp`, and what it needs).

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `easy-personal-rs mcp` reads JSON-RPC from stdin and writes only JSON-RPC to stdout (nothing else on stdout, ever); logs, if any, go to stderr and never contain a secret | RS-006 |
| 2 | `initialize` returns the Rust app's name and the Cargo package version as server info (Python's `FastMCP` reports the app version too: see `create_app` in `src/easy_personal/surfaces/mcp.py`); the protocol version is echoed for the revisions the spike proved | RS-001 |
| 3 | `tools/list` returns the three tools; for each, `name` and `description` equal the matching entry in `rust/parity/golden/mcp__tools_list.json` and `inputSchema` equals it parsed. If rmcp's generated schema cannot equal it, declare the schema by hand; if that is also impossible, stop and report the exact difference | RS-003 |
| 4 | `tools/call ping` returns the same result as `mcp__ping__all.json` | RS-003 |
| 5 | `tools/call usage_summary` (no params, and with the `since`/`until` window of `mcp__usage_summary__window`) and `usage_breakdown` (each `by` in the manifest) return results equal to the matching `mcp__usage_*` goldens, parsed, over `rust/parity/store-v11.sqlite` | RS-003 |
| 6 | The store path comes from the same override variable as `store-info`; the server opens the store only read-only, per call | RS-002 |
| 7 | An invalid `by` or `since` is returned as an MCP tool error carrying Python's message text (see `mcp__*` goldens and `mcp.py`), not a crash and not a JSON-RPC protocol error unless Python's is one | RS-003 |
| 8 | The server exits cleanly when stdin closes | RS-006 |
| 9 | `tools/call` of an unknown tool and malformed JSON are answered with a JSON-RPC error; the process keeps running | RS-006 |
| 10 | `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python MCP output in `rust/parity/golden/mcp__*.json` over `rust/parity/store-v11.sqlite`. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden/mcp__ping__all.json, rust/parity/golden/mcp__tools_list.json, rust/parity/golden/mcp__usage_summary__all.json, rust/parity/golden/mcp__usage_summary__window.json, rust/parity/golden/mcp__usage_breakdown__day.json, rust/parity/golden/mcp__usage_breakdown__model.json, rust/parity/golden/mcp__usage_breakdown__project.json, rust/parity/golden/mcp__usage_breakdown__tool.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | `initialize`, then `tools/list`, over stdin | server info with the Cargo version; 3 tools equal to the goldens | test spawning the real binary |
| 2 | `tools/call` for each golden entry | parsed result equals the golden | test |
| 3 | a bad `by` | tool error with Python's text | test |
| 4 | stdin closed | exit 0 | test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/spikes/t001-crates/src/main.rs` (the `mcp` subcommand: `ServerHandler`, `tool_router`, `serve(stdio())`) and `rust/crates/app/tests/server.rs` (tests that spawn the real binary and talk to it).
**Vital slice**: `initialize` + `tools/list` + `ping`, then `usage_summary`, then `usage_breakdown`.
**Cut list**: no HTTP transport, no prompts, no write tools, no other tools, no logging framework, no `lib.rs`.

Use `rmcp` `=3.9.0` behind one module (`mcp.rs`) so a later major version change touches one file. Tests speak newline-delimited JSON-RPC to the spawned binary. Pass the protocol versions the spike proved; do not enable the `2026-07-28` no-handshake mode.

---

## Edge Case Checklist

- [ ] Nothing but JSON-RPC on stdout (a stray `println!` would corrupt the protocol)
- [ ] Each call opens the store afresh read-only, so a store that appears later or is replaced between calls is handled
- [ ] A store newer than the build knows (`StoreError::Newer`) → a clear tool error, not a crash
- [ ] A missing store → the same "no data" answer Python gives, or a clear tool error if Python errors (check which)
- [ ] Large responses do not block on stdout buffering
- [ ] No absolute path, row text or secret in any result, error or log (NFR-002)
- [ ] The test never opens `~/.easy-personal`
- [ ] Not proven by T001 and not required here: a handshake against the real Claude Code client; list it in the report as an open item
- [ ] Stale references: `rust/README.md` lists the `mcp` command

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/Cargo.toml` | add `rmcp =3.9.0` (and only what it needs) to `[workspace.dependencies]` (re-read first) |
| `rust/crates/app/Cargo.toml` | use it |
| `rust/crates/app/src/mcp.rs` | new — the server and three tools |
| `rust/crates/app/src/main.rs` | `mod mcp;` and the `Mcp` subcommand only (re-read first) |
| `rust/crates/app/tests/mcp.rs` | new — tests that spawn the binary |
| `rust/README.md` | document `mcp` |
| Pre-existing pin tests | None — `cli.rs` and `server.rs` must still pass |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/core/**` | T005 fixes and T006 are working there |
| `rust/crates/app/src/server.rs`, `rust/crates/app/tests/server.rs` | T004, done and reviewed |
| `rust/parity/**` | the oracle; locked |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: write the tests that spawn `easy-personal-rs mcp` and compare with the goldens (they fail before `mcp` exists), then implement slice by slice. Finally flip a fixture value in a temp copy of the store and confirm the matching comparison fails.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T008.md`
- [ ] Ran the real binary over stdio by hand: `initialize`, `tools/list`, one `tools/call`
- [ ] Supervisor notified: task ready for Stage 4 review
