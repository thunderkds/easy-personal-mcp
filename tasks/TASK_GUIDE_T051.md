# TASK_GUIDE — T051: Rust start-up ingest, MCP `--http` transport, and `context` ingest (loopback MCP client)
**Date**: 2026-11-13
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P3
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the entries "Rust P3 planned", "Rust P2 ingest" and "Rust live feed: Path 1" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP3.md` (slice S12 and the dependency risk row) and `rust/parity/README.md`
6. Read the Python you copy, **read-only**: `src/easy_personal/sources/context.py` (whole file: the allow-list, the loopback-only URL rule, `ingest`, "no data" on every failure), `tests/sources/test_context.py` and its fake MCP server, `src/easy_personal/surfaces/mcp.py` (`serve`, the `http` transport), `src/easy_personal/surfaces/cli.py` (`_ingest` and the `dashboard` command's "ingesting before start" step), `src/easy_personal/core/lock.py`
7. Read the Rust you extend: `rust/crates/app/src/{main,mcp,ingest_cmd,server,window}.rs`, `rust/crates/core/src/ingest/` (the existing sources and `ingest_lock.rs`), `rust/crates/app/tests/` (how a test starts `mcp` and `serve`)
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use temp folders and fixtures. Never connect to a real easy-context service: tests use a fake loopback server you start on a free port

---

## Requirement

**Original request** (owner, 2026-11-13): "start all open tasks, by parallel if possible". Plan: `BRAINSTORMING_LOG_RUSTP3.md` slice S12, optional, "the start-up ingest stays out until T051" (Q6).

**Restated intent**:
> Three gaps between the Rust app and the Python app close. (1) `serve` and `app` run an ingest before they start, as Python's `dashboard` does. (2) `easy-personal-rs mcp --transport http` serves the MCP tools over loopback HTTP like Python. (3) `ingest` reads easy-context's index freshness through its loopback MCP service and stores it as Python does, and a failure is "no data", never a failed run.

**Out of scope**: keychain, launcher, connector writes (T050), the live feed (T056/T057), any non-loopback network access, a TLS client, any change to Python files.

**Requirement Refs**: FR-012, FR-025, FR-080 (context), NFR-001, NFR-002 (`PRD.md`); DDR-0002 (loopback only); RS-001..RS-003 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T033 (kit_trace, ui, verifier sources), T056 (merged; do not change the live feed)
**Entry point**: `serve` / `app` start (`rust/crates/app/src/{server,window,main}.rs`), `mcp --transport http` (`mcp.rs`, `main.rs`), `ingest` (`ingest_cmd.rs`, `crates/core/src/ingest/`)

---

## Dependency rule (read before any code)

The plan lists "a new dependency pulls a large tree and licences" as a risk and says to ask the owner. **Do not add or change any crate, feature flag or lockfile line without stopping and reporting first**, with `cargo tree` before and after, the added packages and their licences. The order below keeps the work that needs no crate first. If part 2 (rmcp's HTTP server feature) or part 3 (an HTTP client) needs a new package, stop that part, finish the others, and report the options: for part 3 the zero-crate option is a small hand-written HTTP/1.1 client over `std::net::TcpStream` restricted to `127.0.0.1` (the Python client is loopback only, no TLS, two allowed tools); judge its size honestly and report it.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Start-up ingest.** `serve` and `app` run the same ingest as the `ingest` command (all sources that exist, the same lock with Python's wait rule: "waits if another ingest runs") before they bind, print a one-line progress message like Python's ("ingesting before start...") with no path or row value, and still start when the ingest fails or finds no data (the failure is a fixed line, not a stop). A flag `--no-ingest` skips it (document it; Python has none, so list it as "decided for you"). The poll thread of T056 starts after the ingest | FR-012 |
| 2 | **MCP `--http`.** `mcp --transport http [--host 127.0.0.1] [--port N]` serves the same tool list and prompt as stdio over streamable HTTP on loopback, as Python. A host other than `127.0.0.1` (`0.0.0.0`, `localhost`, `::1`, a name) is refused with exit 2 before any socket is bound (NFR-001, as the verify recipe's probes). A bad `--port` or `--transport` is a clap error, exit 2. The server stops on Ctrl-C with exit 0. If no crate is needed, do it; otherwise stop and report per the dependency rule | NFR-001 |
| 3 | **`context` ingest.** The `context` source reads `list_indexes` and `get_stats` only (an allow-list: any other tool name is refused before a call), only from a `127.0.0.1` URL (`EASY_PERSONAL_CONTEXT_URL` or the default `http://127.0.0.1:8767/mcp`; any other host refused before a connection), with a 3 s timeout, and keeps only a project's root, `last_index_at` and `root_exists`, exactly as Python's `context.ingest`. Every failure (service down, slow, malformed reply, oversize reply) is "no data": the ingest run still exits 0. `ingest_cmd.rs` stops listing `context` as "not ingested". Replies are size-capped (choose 1 MiB, say so) and no index content is stored or printed | FR-012, FR-025, DDR-0002, NFR-002 |
| 4 | **Oracle.** Python's `tests/sources/test_context.py` fake MCP server (or an equivalent fake you write once in `rust/parity/gen_context.py`) feeds both Python's `context.ingest` and Rust's; the resulting store rows are equal for: a normal list, an empty list, a root that does not exist, a service that is down, a slow service (timeout), a bad reply, a refused host, and a refused tool name. Golden in `rust/parity/golden-context/`, made once by Python | FR-012 |
| 5 | **Leak, coexistence, regression.** `parity/leak_rust.py` covers the new surfaces (the `--http` server and the context source over a canary index: 0 hits over all surfaces); the start-up ingest and a concurrent `ingest` do not corrupt the store (reuse the T031 coexistence pattern for one case); three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, existing tests and goldens pass. A `Cargo.toml` or `Cargo.lock` change only after the dependency rule | RS-003, NFR-002 |
| 6 | **Docs.** `rust/README.md`: the new flags and defaults, the context rules, the "decided for you" list, corrected "not ingested" and "not served" lists. `rust/PROJECT_*.md` and `memory/**` are not edited (Supervisor) | — |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T051.md`.
> **Locked oracle**: `golden-context/` made once by Python, plus the Python sources and tests (read-only).

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | each oracle case | Rust store rows equal Python's | the parity test |
| 2 | `serve` over a temp store with new transcripts | events are in the store before the dashboard line is printed | the process test |
| 3 | `mcp --transport http --host 0.0.0.0` / `localhost` / `::1` | exit 2, no socket bound | the process test |
| 4 | `git status -- src tests web` and `Cargo.lock` | no change (Cargo.lock only as the dependency rule allows) | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

**Reviewer note**: Medium risk; the new network surface is loopback only. Stage 4 includes a code review and a check that no non-loopback connection or bind is possible.

---

## Approach

**Vital slice**: (1) start-up ingest (no crate); (2) `context` source with the hand-written loopback client if it stays small, else report; (3) `--http` last, because it is the one most likely to need a crate.
**Cut list**: no retry, no connection pool, no TLS, no non-loopback host, no new tool in the context allow-list, no change to the live feed.
**Order**: red test first for each part.

---

## Edge Case Checklist

- [ ] A URL with a userinfo part, a port 0, an IPv6 literal, a redirect, a hostname that resolves to 127.0.0.1: refused or "no data" as Python does (read `_check_url`-style code and copy)
- [ ] A reply that never ends, a chunked reply, a reply over the cap, a reply with an SSE body instead of JSON (the MCP streamable-HTTP reply may be either: handle both as Python's client does)
- [ ] The service answers after the 3 s timeout: "no data", and the thread or socket is closed, not leaked
- [ ] Start-up ingest while another ingest holds the lock: waits (Python's wait), then runs; a lock that never frees: the same bounded wait as Python, then start anyway
- [ ] `serve --no-ingest` and `app` with no store file: starts, empty
- [ ] `mcp --transport http` with the port already in use: a one-line error, exit 1, no path
- [ ] `leak_rust.py`, `tests/mcp.rs` and the tool lists updated by a few lines only
- [ ] Time uses the injected clock; three `TZ` values

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/ingest/context.rs` (new) and `ingest/mod.rs` (register) | the context source and its loopback client |
| `rust/crates/app/src/{ingest_cmd,main,mcp,server,window}.rs` | start-up ingest, `--http`, "not ingested" list (keep edits to `main.rs` and `mcp.rs` small: T057 works in parallel on `window.rs` and `main.rs`) |
| `rust/crates/core/tests/context_parity.rs`, `rust/crates/app/tests/{startup_ingest,mcp_http}.rs` (new) | oracle, process tests |
| `rust/parity/gen_context.py`, `rust/parity/golden-context/` (new); `rust/parity/leak_rust.py` | oracle, leak registry |
| `rust/README.md` | docs |
| `tasks/TASK_REVIEW_T051.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/crates/app/src/live_feed.rs`, `rust/crates/core/src/live.rs` | T056/T057 own them |
| existing `rust/parity/golden*/**`, `store-rich/**`, `fixtures-rich/**` | the oracle |
| `rust/Cargo.toml`, `rust/Cargo.lock`, `crates/*/Cargo.toml` | only after the dependency rule |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Red test per criterion, then green; the oracle; the leak gate; three `TZ` runs (`unset`, `Pacific/Kiritimati`, `Pacific/Pago_Pago`: `cargo test --workspace --no-fail-fast`).

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T051.md` before the first file is created
- [ ] Implementation done (or the parts blocked by the dependency rule reported, with options)
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts, three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written
- [ ] Supervisor notified: task ready for Stage 4 review
