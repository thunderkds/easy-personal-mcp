# TASK_GUIDE — T046: Rust connector reads: the catalog parser, MCP `connector_catalog` / `connector_status` / `connector_suggestions` and `GET /api/connectors` (no keychain)
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the entries "Rust P3 planned" and "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP3.md` (slice S7, the Edge Case Checklist and the decisions Q1-Q8) and `rust/parity/README.md` (the oracle, the goldens, the venv)
6. Read the Python you copy, **read-only**: `src/easy_personal/connectors/catalog.py` and the files in `src/easy_personal/catalog/`, `src/easy_personal/connectors/suggestions.py`, the read side of `src/easy_personal/connectors/` (`GET /api/connectors` in `src/easy_personal/surfaces/api.py`; do NOT read the keychain or `claude` code paths for porting, only for what they report when absent), and the three MCP tools in `src/easy_personal/surfaces/mcp.py`
7. Read the Rust you extend: `rust/crates/app/src/{api,mcp,server}.rs` (how a route or tool is registered and how errors and the token guard work), `rust/crates/core/src/{store,metrics,workspaces,scope}.rs` (how a read module opens the store and takes a `Scope`), and an existing parity test as the pattern: `rust/crates/core/tests/workspaces_parity.rs` and `rust/crates/app/src/api_tests.rs`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies; never a real home, keychain, `claude` CLI or systemd

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 1, slice S7 of `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> The connector catalog (a hand-written parser for the five catalog files), the three read-only MCP tools `connector_catalog`, `connector_status`, `connector_suggestions` and `GET /api/connectors` answer exactly what Python answers on a machine with **no keychain backend and no `claude` CLI** (the state of the golden): presence is reported as Python reports it when it cannot ask. The Rust code never reads a keychain and never starts a process in this task.

**Out of scope**: any write surface, any change to the Python app, `serve` or `mcp` opening the store for writing (they stay read-only), a new dependency (none is allowed in this task: if one seems needed, stop and report), the routes and tools of other slices, schema changes.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`; the Python PRD functional requirements named in the Python files above.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T004-T022 (the read core and the server)
**Entry point**: the three tools (`rust/crates/app/src/mcp.rs`) and `GET /api/connectors` (`rust/crates/app/src/api.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New core module `catalog.rs` (the parser of the catalog files: copy Python's accepted syntax and its error behaviour; the catalog files are embedded copies under `rust/crates/core/catalog/` with a test that compares them byte for byte with `src/easy_personal/catalog/*`, as T034 did for the unit files) and `suggestions.rs` | RS-003 |
| 2 | **Goldens** `mcp__connector_catalog__all`, `mcp__connector_status__all`, `mcp__connector_suggestions__all`, `api__connectors__all` are replayed and equal; equal / total pasted | locked oracle |
| 3 | `connector_status` and `GET /api/connectors` report presence exactly as the golden shows for "no keychain, no `claude`"; **no code path in this task calls a keychain, a `claude` process or the network**: a test or a `grep` in the report shows it; names are validated with Python's name rule | safety |
| 4 | The catalog parser: malformed files, duplicate ids, unknown keys, non-UTF-8 and very long lines behave as Python (or Rust refuses cleanly where Python crashes: list each) | parity |
| 5 | `suggestions` use the store (workspaces, usage) as Python does; scope arguments and the 400/404 cases equal Python's | parity |
| 6 | **No regression**: `cargo test --workspace` under `TZ` unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago` (paste all three); clippy and fmt clean (also `--features window`); `git status -- src tests web` empty; `Cargo.toml`/`Cargo.lock` unchanged (no new dependency); the existing goldens still pass | quality |
| 7 | **Privacy**: `rust/parity/leak_rust.py` still prints `leak check: 0 hits` (paste it); every new error body and log line carries no path, secret or caller text; where the new surface can show repository or account text, name which fields are shown and why they are safe | NFR-002 |
| 8 | **Docs**: `rust/README.md` lists the new routes/tools, their query parameters and error table; the P1b "not served" lists are corrected; every item "decided for you" (where Rust differs from Python) is listed in your report and in the README | usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T046.md`.
> **Locked oracle**: the golden files of `rust/parity/golden-rich/` (Python's real answers) and the Python sources (read-only).

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | every golden of the slice | Rust's answer equals it (status, headers that matter, body) | the parity test |
| 2 | the same routes over real HTTP without the token / with a bad Host | 403 as for the other routes | server test |
| 3 | `git status -- src tests web` and `Cargo.lock` | no change | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: the existing parity tests (`workspaces_parity.rs`, `sessions_parity.rs`) and how `api.rs` registers `/api/workspaces`.
**Vital slice**: the core module and its parity test against the goldens first, then the route or tool wiring, then HTTP and MCP tests, then docs.
**Cut list**: no abstraction shared with other slices beyond what already exists; no caching; no new crate.

---

## Edge Case Checklist

- [ ] Every parameter combination in the goldens, plus the 400/404/405 cases Python has (copy Python's status and `error` text, except text that is CPython-specific: then compare status and the `error` key only, as `manifest.json` `compare_notes` does)
- [ ] The store is opened read-only; a store that is not v11 is refused as for the other routes
- [ ] a connector entry holds `command`, `env` and `headers` in the keychain in Python: nothing like that is read or shown here; the catalog holds only public names and URLs: say which fields leave the process
- [ ] Time: anything derived from "now" uses the injected clock (`clock.rs`) so the goldens are reproducible; run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] Stale references: `rust/README.md` "not served" lists and `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/{catalog,suggestions}.rs` (new), `rust/crates/core/src/lib.rs`, `rust/crates/core/catalog/*` (new, embedded copies) | the read modules |
| `rust/crates/core/tests/connectors_parity.rs` (new) | golden replay and the catalog byte-compare |
| `rust/crates/app/src/api.rs` and/or `mcp.rs` | register the route(s) or tool(s) (one line each in the route table; keep the change minimal: other tasks edit these files at the same time) |
| `rust/README.md` | docs |
| `tasks/TASK_REVIEW_T046.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/parity/golden*/**`, `rust/parity/gen_golden.py`, `rust/parity/store-rich/**`, `fixtures-rich/**` | the oracle (if a golden looks wrong, stop and report) |
| `rust/Cargo.toml`, `rust/Cargo.lock`, other tasks' modules | no new dependency; another task owns them |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Parity test red first (the route answers 404 / the tool is unknown), then green per golden, then HTTP/MCP tests, leak gate, three `TZ` runs, lint. Paste every output with counts.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T046.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Golden counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written
- [ ] Supervisor notified: task ready for Stage 4 review
