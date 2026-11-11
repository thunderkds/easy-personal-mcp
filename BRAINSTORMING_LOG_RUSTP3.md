# BRAINSTORMING_LOG — Rust P3: the remaining dashboard routes, MCP tools and write surfaces

**Date**: 2026-11-11 · **Tier**: Deep (security-sensitive surfaces, a second writer, new dependencies) · **Mode**: `[admin]` ("do all, you can spawn parallel"): the questions below were answered with the Supervisor's default and each one is flagged **⚠ decided for you — please review**.
**Inputs**: the read-only inventory of 2026-11-11 (a research agent; claims spot-checked by grep: the web page references 17 distinct GET paths that Rust answers 404 for, not the 12 of the P1b close-out), `rust/README.md`, `rust/docs/closeout-2026-11-11.md`, `BRAINSTORMING_LOG_RUSTP2.md`.

## 1. Problem space

Rust today (`rust-v0.1.0` plus T033-T038, crate to be 0.2.0): `store-info`, `ingest` (events, kit_trace, ui, verifier), `serve`, `mcp`, `app`, `schedule`; 7 `/api` GET routes + `/api/version` + static files; 6 MCP tools; the store is opened **read-only** by `serve` and `mcp`.

Python does more (`src/easy_personal/`):

| Group | What Python has that Rust lacks | Size (Python lines) |
|---|---|---|
| Read routes | `/api/{reports, reports/{id}, first-run, accounts, kit, kit/reports, kit/workspaces, kit/report-files, kit/report-file, board, memory-graph, jobs, dirs, connectors, connectors/legacy, sync}` and `/api/live` (SSE): **17 GET paths** | reports 237, accounts 348, kit 509, board 162, memgraph 318, easymem 336, jobs 296, folders 44, first_run 34 |
| MCP | 6 more tools (`kit_summary`, `kit_findings`, `connector_catalog`, `connector_status`, `connector_suggestions`, `save_report`), the `investigate` prompt, `--http` transport | suggestions 104, catalog 112, kitperf 316, reports 237 |
| Store-only writes | POST/PATCH/DELETE workspaces, accounts (confirm, ignore, workspaces) | workspaces 249, accounts 348 |
| Secrets and processes | keychain (`keyring`), launcher `exec`/`headers`, connector add/remove/move/alias, sync apply, legacy (`claude mcp ...` subprocesses) | launch 101, keychain 30, writer 241, sync 392, legacy 119 |
| Live | `/api/live` SSE and the 0.1 s tail thread (a second writer inside the dashboard) | live 718 |
| Other | `context` ingest (HTTP MCP client), dashboard start-up ingest | context 234 |

No schema change is needed: every table is already at v11. The hazards are secrets, command execution, path safety, and a second writer.

## 2. Alternatives

| Option | Idea | Pros | Cons |
|---|---|---|---|
| **A. Read-first, writes in risk order** (recommended) | P3a all read routes and read MCP tools, P3b `save_report`, P3c store-only writes, P3d live, P3e keychain and `claude` subprocess surfaces (owner gate) | Every wave ships a usable Rust dashboard; the dangerous surfaces come last and behind a decision; each slice has a Python oracle; Python stays the writer for config until the Rust one is proven | Slowest to "everything"; the route table (`api.rs`, `mcp.rs`) is shared, so waves must merge in order |
| B. Everything in dependency order | Port all 12 slices before any release | One cutover | Weeks without a release; keychain and subprocess work blocks the safe read work; the biggest blast radius lands together |
| C. Minimalist: read routes only, writes stay in Python for good | P3a only; the Rust app is a fast read-and-ingest companion | 50% less code; no secret handling in Rust | Not a replacement; the owner's end goal (Python sunset, P6) is not reachable |
| D. Rust proxies unported routes to a Python dashboard on loopback | `serve` forwards 404 routes | Tiny | Two servers, two tokens, Python must keep running: defeats the port; rejected |

**50% less code check**: option C is the half-size version; A reaches it after P3a and then decides on the rest with evidence, so A includes C as its first milestone.

## 3. Adversarial review of A

| Failure | Likelihood | Mitigation |
|---|---|---|
| A "read" route that writes in Python (`GET /api/accounts`, `/api/sync` call `accounts.detect`, which INSERTs) behaves differently in a read-only Rust `serve` | Certain | Rust keeps `serve` read-only; the detect step stays in `ingest` (already ported); a new account shows after the next ingest, not at page load. **⚠ decided for you** (Q3). The golden for these routes is made after a detect run |
| Path traversal and symlink reads in `kit/report-file`, `dirs`, `board`, `memory-graph` | High | One table of path cases per route, taken from Python's tests and `tests/leak/*`; `O_NOFOLLOW`, size caps, no content echo; security review at Medium/High |
| Secrets reach a log, the store, argv or the page when the keychain surface is ported | Medium | Defer to P3e behind an owner gate; the leak suite (`leak_rust.py`) is extended first with the connector canaries; no plaintext fallback (as Python) |
| Command execution (`claude mcp add-json`, `execvpe` of a keychain command) | Medium | Argument lists only, validated names, 60 s timeout, a fake `claude` in tests, never the real CLI; P3e only |
| A second writer racing Python (live tail batches, write routes without the ingest lock) | Medium | Copy Python's contract exactly (busy timeout 5 s, per-batch `ingest.lock` for the tail); the T031 coexistence harness is the template |
| Two waves edit `api.rs` / `mcp.rs` at once | High | Each wave's agents work in worktrees; the Supervisor merges in order and runs the full suite after each merge; a route registers itself in one line |
| A new dependency (`keyring`/`zbus`, an HTTP client) pulls a large tree and licences | Medium | Not before P3e / S12; ask the owner then (`cargo tree`, licence list) |
| SSE live tail (`Last-Event-ID`, `rolled_past`, pings) is hard to freeze as goldens | High | Event-sequence goldens from Python's own tests (`test_live.py`, `test_api_live.py`); P3d is C3 and last before secrets |
| Python's `save_report` stores `server_version` from its package version | Certain | The golden normalises that field; Rust stores its own version |

## 4. Recommended path and slices

Option A. Each slice is a task with a guide, an oracle (Python's real answers frozen as goldens, as in `rust/parity/`), a Stage 4 review, and the leak gate. Complexity C2 unless stated.

| Task | Slice | Delivers | Risk | Oracle | Needs |
|---|---|---|---|---|---|
| T040 | S1 | `/api/reports`, `/api/reports/{id}`, `/api/first-run` | Low | goldens exist (`api__reports__*`, `api__first_run__all`) | `schedule_cmd` unit dir |
| T041 | S2 | `GET /api/accounts`, `GET /api/sync` (read-only, no `detect`) | Medium | goldens `api__accounts__*`, `api__sync_plan__all` | T040 |
| T042 | S3 | `/api/kit`, `/api/kit/workspaces`, `/api/kit/reports`, MCP `kit_summary` | Medium | goldens `api__kit*`, `mcp__kit_summary__*` | none |
| T043 | S4 | `/api/board`, `/api/memory-graph` | Medium | goldens `api__board__*`, `api__memory_graph__*` (add a project-graph case) | T041 |
| T044 | S5 | `/api/dirs`, `/api/jobs`, MCP `kit_findings` | Medium | Python tests with fake tools; `mcp__kit_findings__*` | T042 |
| T045 | S6 | `/api/kit/report-files`, `/api/kit/report-file` | **High** (paths) | Python tests, `tests/leak/test_kit_report_files_leak.py` | T042 |
| T046 | S7 | connector catalog parser, MCP `connector_catalog` / `connector_status` / `connector_suggestions`, `GET /api/connectors` (no keychain: presence reported unknown, as Python does without one) | Medium | goldens `mcp__connector_*`, `api__connectors__all` | none |
| T047 | S8 | MCP `save_report`, prompt `investigate` | Medium | `mcp__prompt_investigate__*`; copy-store-then-compare for `save_report` | T040 |
| T048 | S9 | POST/PATCH/DELETE workspaces, accounts confirm/ignore/workspaces (store only; a change that needs `claude` is refused with Python's message until T050) | Medium-High | Python tests, same inputs on a store copy | T041 |
| T049 | S10 | `/api/live` SSE and the tail thread | **High**, C3 | event-sequence goldens from Python's live tests | T048 |
| T050 | S11 | keychain, launcher, connector add/remove/move/alias, sync apply, legacy | **High**, C3 | in-memory keyring and fake `claude` | T046, T048, **owner approval of the crate** |
| T051 | S12 | `context` ingest, MCP `--http`, dashboard start-up ingest | Medium | `tests/sources/test_context.py` with the fake MCP server | none, optional |

**Waves** (parallel inside a wave, each agent in its own worktree, merged in order): wave 1 = T040, T042, T046; wave 2 = T041, T044, T045, T047; wave 3 = T043, T048; wave 4 = T049; gate; wave 5 = T050, T051.
**Milestones for releases**: `rust-v0.3.0` after waves 1-3 (a read-complete dashboard and MCP, store-only writes); `rust-v0.4.0` after T049; `rust-v1.0.0` candidate after T050 and a P6 cutover decision.

## 5. Questions and decisions (defaults, **⚠ decided for you — please review**)

- **Q1 Order**: option A, read-first. Default yes.
- **Q2 Keychain crate (`keyring`/Secret Service) and an HTTP client**: not decided now; asked at the P3e gate with `cargo tree` and licences. Nothing in T040-T049 adds a dependency (each task must say so).
- **Q3 Read routes that insert in Python** (`detect`): Rust `serve` stays read-only; the accounts detect runs in `ingest`. Differences: a new account appears after the next ingest.
- **Q4 Write routes and the lock**: copy Python (no `ingest.lock`, SQLite busy timeout 5 s, one in-process write lock), same confirm protocol (`pending: true`, then `confirm: true`), same status codes. `serve` opens a write handle only for these routes, never at start-up.
- **Q5 Python-crash cases**: same rule as P2: Rust skips or refuses cleanly and lists each in the task review.
- **Q6 Live tail**: after the write routes, as its own C3 task; the start-up ingest stays out until T051.
- **Q7 macOS/Windows**: not in P3 (P4/P5).
- **Q8 Release rhythm**: a release after each milestone above; the Supervisor plans it, the owner approves and publishes.

## 6. Edge Case Checklist (goes into every P3 guide)

- [ ] Python is the oracle: freeze its answers first (goldens), then port; the golden of a route that detects or writes is made after the Python write step
- [ ] Every route has a 400/403/404/405/409 table copied from Python; error bodies carry no path, secret or caller text
- [ ] Path inputs: absolute-path rules, `..`, symlinks (`O_NOFOLLOW`), size caps, non-UTF-8 names, dotfiles
- [ ] No secret in argv, env of a child (beyond what Python passes), logs, the store or an error; the leak suite covers the new surface with its canaries
- [ ] A write route and an ingest at the same time: tested with the T031 harness
- [ ] The store stays v11; a stale or newer store is refused (exit 2); no migration
- [ ] `--features window` clippy and the three-`TZ` test run for every task; Python tree untouched
- [ ] Each task lists its "decided for you" items and its stale references (README, release notes)
- [ ] No test reaches the real systemd, keychain, `claude` CLI or home folders (`env_clear`, scratch `PATH`, fakes)

## 7. Surgical scope

**Touch**: `rust/**`, `tasks/**`, `memory/**` (Supervisor), docs. **Never**: `src/**`, `tests/**`, `web/**` (Python and the page), `pyproject.toml`, `uv.lock`, the owner's real data and keychain.
