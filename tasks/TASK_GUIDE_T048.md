# TASK_GUIDE — T048: Rust store-only write routes: workspaces (POST, PATCH, DELETE) and accounts (add, confirm, ignore, workspaces)
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the entries "Rust P3 planned" and "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP3.md` (this slice, the Edge Case Checklist and the decisions Q1-Q8) and `rust/parity/README.md` (the oracle, the goldens, the venv)
6. Read the Python you copy, **read-only**: `src/easy_personal/core/workspaces.py` (whole file: `add`, `edit`, `get`, `WorkspaceError`, `FIELDS`), `src/easy_personal/core/accounts.py` (`add`, `confirm`, `ignore`, `use`, `detect`, errors), `src/easy_personal/connectors/sync.py` (`rename_workspace`, `delete_workspace`: **only to see which steps need `claude`**), `src/easy_personal/connectors/writer.py` (`Out`, `Confirm`), and in `src/easy_personal/surfaces/api.py` the handlers `workspaces_add`, `workspaces_edit`, `workspaces_delete`, `_object_body`, `_json_body`, `_confirmed`, `_workspace_call`, `_workspace_id`, `_account_write`, `_run_writer`, `_WRITE_LOCK`, plus the Python tests for these routes (`tests/test_api_workspaces*.py`, `tests/test_api_accounts*.py`, `tests/leak/*` for writes)
7. Read the Rust you extend and copy the pattern of the wave-1 tasks already on `main`: `rust/crates/app/src/{api,mcp,server}.rs`, `rust/crates/core/src/{store,scope,reports,kit,suggestions}.rs`, `rust/crates/core/tests/{reports_parity,kit_parity}.rs`, `rust/crates/app/tests/reports.rs` (how a route or tool is registered, how errors and the token guard work, how a parity test replays goldens over the real binary), and `rust/parity/leak_rust.py` (it holds a hard registry of every `/api` route and MCP tool: a new one must be added to `API_CALLS` and `GUARD_CALLS` or the gate fails with "unregistered route")
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies, fixtures built in temp folders, and fakes on a scratch `PATH` with `env_clear`; never a real home, keychain, `claude` CLI, `systemctl` or crontab

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 2, `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> The dashboard's write actions on workspaces and accounts work on Rust with Python's contract (status codes, texts, the `confirm` protocol), changing **only the store**: a change that would need the `claude` CLI or the keychain (renaming or deleting a workspace that has connectors, registrations in an account) is **refused with a fixed text and changes nothing** until T050. `serve` opens a write handle only inside a write request, never at start-up.

**Out of scope**: the `claude` CLI, the keychain, connector writes (`/api/connectors*`, `/api/sync` POST), the live feed, schema changes, account detection at page load (it stays in `ingest`; a write route that Python precedes with `accounts.detect` does the same detect step here only if it is a plain INSERT already ported in `ingest`: say what you did); any change to the Python app; `serve` or `mcp` opening the store for writing at start-up (they stay read-only unless this guide says a call opens a handle); a new dependency (none is allowed in this task: if one seems needed, stop and report); the routes and tools of other slices; schema changes.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`; the Python requirement ids named in the Python files above.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: wave 1 on `main` (T040, T042, T046)
**Entry point**: `POST/PATCH/DELETE /api/workspaces[/{id}]` and `POST /api/accounts`, `POST /api/accounts/{id}/confirm`, `POST /api/accounts/{id}/ignore`, `PUT /api/accounts/{id}/workspaces` (`rust/crates/app/src/api.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | `workspaces.rs` (extend) and `accounts.rs` (extend) gain the write functions with Python's validation, field limits, uniqueness and ordering rules and **every error text and status** (`WorkspaceError.status`: 400/404/409/403 as Python), through one write path that opens the store for writing **per request** (`writer::open_for_write`: 5 s busy timeout, same pragmas, v11 required, never creates a store), one transaction per request, closed before the answer; one in-process write lock as Python's `_WRITE_LOCK`; no `ingest.lock` (Python takes none) | RS-003 |
| 2 | **HTTP contract**: `Content-Type` must be `application/json` (else 415 `content-type must be application/json`), body must be JSON (400 `body must be JSON`), an object (400 `body must be a JSON object`), `DELETE` may have an empty body; a body larger than a cap (Python has none: choose 1 MiB, answer 413 with a fixed text, list it as "decided for you"); `confirm` must be a boolean (400 `confirm must be true or false`); unknown field 400 `unknown field`; `{id}` not digits -> 404 (`unknown workspace` / `no such account`); the token, Host and Origin guard runs first as for every route (a cross-origin or token-less write is 403 and changes nothing: test it for every write route) | NFR-002, parity |
| 3 | **The confirm protocol**: an edit that renames, and a delete, first answer the plan with `pending: true` (copy Python's body) and apply only with `confirm: true`. Where Python's plan or apply needs `claude` or the keychain (the workspace has connectors with registrations), Rust answers the plan **as Python does when `claude` is not on PATH** for the read-only part, and with `confirm: true` refuses with a fixed text and 409, store unchanged (list it as "decided for you": until T050). A workspace with no connectors (store-only) is renamed/deleted fully; state exactly what Python does in that case and equal it | parity, safety |
| 4 | **Oracle (copy-store-then-compare)**: a generator `rust/parity/gen_writes.py` runs, for each case, Python's own handlers (through Starlette's `TestClient` or the core functions, as its tests do) on a copy of `store-rich` and the Rust server on another copy, then the test compares status, body and the **whole resulting database content** (all tables, normalising timestamps, autoincrement ids as Python makes them) -> `rust/parity/golden-writes/`; cases: every success, every refusal (duplicate name, bad colour/icon, too long, unknown field, bad repos, unknown id, orphan workspace is protected, confirm wrong type), accounts add (path inside a temp agent root), confirm with a label, ignore, use with workspaces list (unknown workspace, duplicates, empty); equal / total pasted; a refused request leaves the file bytes unchanged | locked oracle |
| 5 | **Concurrency**: the T031 coexistence harness (read `rust/crates/core/tests/` for the pattern) runs a write route while `ingest` runs on the same WAL store, and two write requests at once: both finish, rows consistent, `integrity_check` ok, no `database is locked` reaches a body (a fixed busy text after 5 s); `serve` start-up and every GET leave the store file bytes and mtime unchanged (test) | safety |
| 6 | **No secret or path in any answer**: error bodies are fixed texts that name none of the input; account `folder` paths are shown only as Python shows them; `leak_rust.py` registers every write route (`API_CALLS` with a body-bearing form if the harness supports it, else add a small helper; say what you did) and prints `0 hits`; tests never touch a real home or the real `claude` | NFR-002 |
| 7 | **No regression** (three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml`/`Cargo.lock` unchanged, existing goldens pass; `GET` routes still 405 on writes that are not ported) and **Docs** (`rust/README.md`: every route, body fields, status table, the refusal-until-T050 list, "decided for you"; the earlier statements "`serve` is read-only" and "writes are 405" corrected) | quality, usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T048.md`.
> **Locked oracle**: a new `golden-writes/` made once by Python over copies of `store-rich`, and the Python sources and tests (read-only).

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | every golden or oracle case of the slice | Rust's answer equals it (status, the headers that matter, body) | the parity test |
| 2 | the same routes over real HTTP without the token / with a bad Host / with a write method | 403 / 403 / 405 as for the other routes | server test |
| 3 | `git status -- src tests web` and `Cargo.lock` | no change | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

**Reviewer note**: High risk (the first Rust HTTP writes). Stage 4 includes the Supervisor's line-by-line read of the write path and an independent security review.

---

## Approach

**Pattern reference**: the wave-1 parity tests and how `api.rs` / `mcp.rs` register a route or tool.
**Vital slice**: the core module and its parity test against the oracle first (red first), then the route or tool wiring, then HTTP and MCP tests, then the leak registry and docs.
**Cut list**: no abstraction shared with other slices beyond what exists; no caching; no new crate.
**Order**: workspaces first (store only), then the accounts writes, then the refusal-until-T050 rule, then concurrency, then docs.

---

## Edge Case Checklist

- [ ] Every parameter combination in the oracle, plus the 400/403/404/405/413 cases Python has (copy Python's status and `error` text, except CPython-specific text: then compare status and the `error` key only, as `manifest.json` `compare_notes` does); error bodies carry no path, secret or caller text
- [ ] A body with duplicate keys, huge numbers, deep nesting (copy Python: `json.loads` accepts; `serde_json` limits depth at 128: Rust answers 400 `body must be JSON`, list it), non-UTF-8 bytes, a BOM
- [ ] A name with unicode, trailing spaces, 10,000 characters, NUL; `repos` with relative paths, `..`, a path that does not exist, duplicates, 1,000 entries: Python's rules and limits, copied
- [ ] A write while the store is a stale or newer version: refused as for every route (503) and nothing written
- [ ] The write handle is opened after the guard and body validation, so a bad request never opens the store for writing
- [ ] Double submit: the second identical POST gets Python's duplicate answer, not a second row
- [ ] The store is opened read-only (unless this guide names a write); a store that is not v11 is refused as for the other routes (exit 2 / 503)
- [ ] Time derived from "now" uses the injected clock (`clock.rs`); run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] `leak_rust.py` registry updated for every new route or tool (`API_CALLS`, `GUARD_CALLS`, the MCP calls) and `tests/mcp.rs` exact tool lists updated; keep each change a few lines (other tasks edit these files at the same time and the Supervisor merges them)
- [ ] Stale references: `rust/README.md` "not served" lists; `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/{workspaces,accounts,writer}.rs` (extend; `writer.rs` only if a parameter is missing) | the write functions |
| `rust/crates/app/src/{api,server}.rs` | the write routes, the body parsing, the lock (`server.rs` only for the method routing; keep the guard first) |
| `rust/crates/core/tests/writes_parity.rs`, `rust/crates/app/tests/writes.rs` (new) | oracle replay, HTTP, concurrency |
| `rust/parity/gen_writes.py` and `rust/parity/golden-writes/` (new) | the copy-store oracle |
| `rust/crates/app/src/api.rs` and/or `mcp.rs`, `rust/crates/app/tests/mcp.rs`, `rust/parity/leak_rust.py` | register the route(s) or tool(s): a few lines each |
| `rust/README.md` | routes/tools, parameters, error table, the "decided for you" list, corrected "not served" lists |
| `tasks/TASK_REVIEW_T048.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/parity/golden*/**` (existing files), `rust/parity/gen_golden.py`, `rust/parity/store-rich/**`, `fixtures-rich/**` | the oracle (if a golden looks wrong, stop and report) |
| `rust/Cargo.toml`, `rust/Cargo.lock` | no new dependency in this task (if one seems needed, stop and report) |
| other tasks' modules (see the wave list in `BRAINSTORMING_LOG_RUSTP3.md`) | another task owns them |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Oracle test red first (the route answers 404 / the tool is unknown), then green per case, then HTTP/MCP tests, leak gate, three `TZ` runs (`unset`, `Pacific/Kiritimati`, `Pacific/Pago_Pago`: `cargo test --workspace --no-fail-fast`; `workspaces_parity::edge_store_python_values` fails inside a worktree under `.claude/` and is green from the main checkout: say so if you see it), lint. Paste every output with counts.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T048.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review
