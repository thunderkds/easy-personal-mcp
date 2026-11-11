# TASK_GUIDE — T041: Rust `GET /api/accounts` and `GET /api/sync` (read-only; no account detection)
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: Medium
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
6. Read the Python you copy, **read-only**: `src/easy_personal/core/accounts.py` (`grouped`, `list_accounts`, `status_lines`, `group_by_provider`, `detect` only to know what it writes), `src/easy_personal/connectors/sync.py` (`plan`, `plan_all`, `Step.as_dict`; **not** `apply`), `src/easy_personal/connectors/writer.py` only where `plan` calls it, and the routes `/api/accounts` and `/api/sync` with `_sync_account` in `src/easy_personal/surfaces/api.py`
7. Read the Rust you extend and copy the pattern of the wave-1 tasks already on `main`: `rust/crates/app/src/{api,mcp,server}.rs`, `rust/crates/core/src/{store,scope,reports,kit,suggestions}.rs`, `rust/crates/core/tests/{reports_parity,kit_parity}.rs`, `rust/crates/app/tests/reports.rs` (how a route or tool is registered, how errors and the token guard work, how a parity test replays goldens over the real binary), and `rust/parity/leak_rust.py` (it holds a hard registry of every `/api` route and MCP tool: a new one must be added to `API_CALLS` and `GUARD_CALLS` or the gate fails with "unregistered route")
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies, fixtures built in temp folders, and fakes on a scratch `PATH` with `env_clear`; never a real home, keychain, `claude` CLI, `systemctl` or crontab

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 2, `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> `GET /api/accounts?by=account|provider` and `GET /api/sync?account=<id>` answer what Python answers over the same store, **without inserting anything**: Python's handlers first run `accounts.detect(conn)` (which INSERTs new account rows); the Rust `serve` never does (decision Q3: the detect stays in `ingest`, already ported), so a new account folder appears after the next `ingest`, not at page load.

**Out of scope**: `POST`/`PUT` on accounts and `POST /api/sync` (apply), any `claude` subprocess, the keychain, account detection in `serve`; any change to the Python app; `serve` or `mcp` opening the store for writing at start-up (they stay read-only unless this guide says a call opens a handle); a new dependency (none is allowed in this task: if one seems needed, stop and report); the routes and tools of other slices; schema changes.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`; the Python requirement ids named in the Python files above.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: wave 1 on `main` (T040, T042, T046)
**Entry point**: `GET /api/accounts` and `GET /api/sync` (`rust/crates/app/src/api.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New core module `accounts.rs` (read part) in `rust/crates/core/src/`: the grouped list with each account's status line, by `account` (flat list) or by `provider`; and `sync.rs` (plan part): `plan_all` over confirmed (and ignored-with-picks, as Python's SQL) Claude accounts, one `Step` list as `Step.as_dict()`; no write, no `detect` | RS-003 |
| 2 | **Goldens** `api__accounts__*` (3: `by_account`, `by_provider`, `bad_by`) and `api__sync_plan__all` replayed and equal (equal / total pasted). If a golden only differs because Python's `detect` inserted a row first, stop and report which row (do not edit the golden) | locked oracle |
| 3 | `sync.plan` in Python asks `shutil.which("claude")` and, when present, runs `claude mcp get`. Rust **never starts a process and never asks the keychain**: it answers as Python does when `claude` is not on `PATH` (the state of the goldens). The difference on a machine that has `claude` is listed under "decided for you", as T046 did for `configured` | NFR-002 |
| 4 | Errors as Python: `by` not `account`/`provider` -> 400 `by must be account or provider`; `account` that is not an integer id -> 400 `account must be an account id`; an unknown account id on `/api/sync` -> 404 (Python's `NotFound` text); a status line or label that holds user text is shown as stored and never put in an error or log | parity |
| 5 | A hand-made store (built in a temp folder by a small script under `rust/parity/`, like T042) covers: an account with a missing folder, an ignored account with workspace picks, a provider other than `claude` (no plan), two accounts and `?account=` selecting one, a label with unicode; Python's answers over it are the oracle (generate once with Python, commit the generated files under a new `rust/parity/golden-accounts/`) | parity |
| 6 | Routes work over real HTTP (token, Host, Origin guard; `POST /api/accounts` and `POST /api/sync` are 405 here; `PUT /api/accounts/<id>/workspaces` and the confirm/ignore routes are 404/405 and listed as T048's) | surfaces |
| 7 | **No regression**: three `TZ` runs, clippy and fmt (also `--features window`), `git status -- src tests web` empty, `Cargo.toml`/`Cargo.lock` unchanged, existing goldens still pass | quality |
| 8 | **Privacy**: `leak_rust.py` prints `leak check: 0 hits` (paste it) with the new routes registered; name the fields these routes show (folder paths, labels, connector aliases, step lines) and why each is safe or already shown by Python; no secret, key or argv ever appears | NFR-002 |
| 9 | **Docs**: `rust/README.md` lists the routes, parameters, error table, and every item "decided for you" | usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T041.md`.
> **Locked oracle**: the golden files of `rust/parity/golden-rich/` (`api__accounts__*`, `api__sync_plan__all`), a new `golden-accounts/` made once by Python over a hand-made store, and the Python sources (read-only).

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

---

## Approach

**Pattern reference**: the wave-1 parity tests and how `api.rs` / `mcp.rs` register a route or tool.
**Vital slice**: the core module and its parity test against the oracle first (red first), then the route or tool wiring, then HTTP and MCP tests, then the leak registry and docs.
**Cut list**: no abstraction shared with other slices beyond what exists; no caching; no new crate.

---

## Edge Case Checklist

- [ ] Every parameter combination in the oracle, plus the 400/403/404/405/413 cases Python has (copy Python's status and `error` text, except CPython-specific text: then compare status and the `error` key only, as `manifest.json` `compare_notes` does); error bodies carry no path, secret or caller text
- [ ] A step line or alias containing newlines, quotes, or text that looks like a command is shown as data (JSON-escaped), never run
- [ ] An account `folder` that is relative, not UTF-8, missing, or a symlink: shown or skipped as Python does; Rust never opens the folder (only the store row is read)
- [ ] `?account=` empty string equals absent (Python's `_sync_account`); `0`, `-1`, `1.5`, `true` and a 20-digit id are the same 400 / 404 as Python
- [ ] The store is opened read-only (unless this guide names a write); a store that is not v11 is refused as for the other routes (exit 2 / 503)
- [ ] Time derived from "now" uses the injected clock (`clock.rs`); run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] `leak_rust.py` registry updated for every new route or tool (`API_CALLS`, `GUARD_CALLS`, the MCP calls) and `tests/mcp.rs` exact tool lists updated; keep each change a few lines (other tasks edit these files at the same time and the Supervisor merges them)
- [ ] Stale references: `rust/README.md` "not served" lists; `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/accounts.rs` (new), `rust/crates/core/src/sync.rs` (new), `rust/crates/core/src/lib.rs` | the read modules |
| `rust/crates/core/tests/accounts_parity.rs` (new), `rust/crates/app/tests/accounts.rs` (new) | oracle replay and HTTP tests |
| `rust/parity/gen_accounts.py` (new) and `rust/parity/golden-accounts/` (new) | the hand-made-store oracle |
| `rust/crates/app/src/api.rs` and/or `mcp.rs`, `rust/crates/app/tests/mcp.rs`, `rust/parity/leak_rust.py` | register the route(s) or tool(s): a few lines each |
| `rust/README.md` | routes/tools, parameters, error table, the "decided for you" list, corrected "not served" lists |
| `tasks/TASK_REVIEW_T041.md` | Demonstration BEFORE only |

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

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T041.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review
