# TASK_GUIDE — T043: Rust `GET /api/board` and `GET /api/memory-graph` (read-only, bounded file reads)
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
5. Read `BRAINSTORMING_LOG_RUSTP3.md` (this slice, the Edge Case Checklist and the decisions Q1-Q8) and `rust/parity/README.md` (the oracle, the goldens, the venv)
6. Read the Python you copy, **read-only**: `src/easy_personal/core/board.py` (whole file), `src/easy_personal/core/memgraph.py` (whole file), `src/easy_personal/core/easymem.py` (what `memgraph.py` calls), the routes `/api/board` and `/api/memory-graph` with `_memory_graph` in `src/easy_personal/surfaces/api.py`, and their Python tests (`tests/test_board*.py`, `tests/test_memgraph*.py`) for the file cases
7. Read the Rust you extend and copy the pattern of the wave-1 tasks already on `main`: `rust/crates/app/src/{api,mcp,server}.rs`, `rust/crates/core/src/{store,scope,reports,kit,suggestions}.rs`, `rust/crates/core/tests/{reports_parity,kit_parity}.rs`, `rust/crates/app/tests/reports.rs` (how a route or tool is registered, how errors and the token guard work, how a parity test replays goldens over the real binary), and `rust/parity/leak_rust.py` (it holds a hard registry of every `/api` route and MCP tool: a new one must be added to `API_CALLS` and `GUARD_CALLS` or the gate fails with "unregistered route")
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies, fixtures built in temp folders, and fakes on a scratch `PATH` with `env_clear`; never a real home, keychain, `claude` CLI, `systemctl` or crontab

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 2, `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> `GET /api/board?workspace=&account=` and `GET /api/memory-graph[?project=]` answer what Python answers over the same store and fixture folders: the kanban board of each project folder (from `PROJECT_KANBAN.md`, or the board file Python reads) and the memory-note graph (note names, links, frontmatter fields Python shows), read-only, with every file read bounded and non-following.

**Out of scope**: writing any file; reading any file Python does not read; following symlinks where Python does not; calling `claude` or any process; any change to the Python app; `serve` or `mcp` opening the store for writing at start-up (they stay read-only unless this guide says a call opens a handle); a new dependency (none is allowed in this task: if one seems needed, stop and report); the routes and tools of other slices; schema changes.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`; the Python requirement ids named in the Python files above.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: wave 1 on `main` (T040, T042, T046)
**Entry point**: `GET /api/board` and `GET /api/memory-graph` (`rust/crates/app/src/api.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New core modules `board.rs` and `memgraph.rs` (and `easymem.rs` only for what `memgraph` calls) in `rust/crates/core/src/`: `board(conn, scope)` over the store's projects (`scope.rs` handling as the other modules; worktree paths skipped as Python's `_is_worktree`), `project_list` and `project_graph` | RS-003 |
| 2 | **Goldens** `api__board__*` (7: all, account default/unknown/work, workspace acme/personal, both) and `api__memory_graph__*` (list, project, unknown_project) in `golden-rich`, plus the `golden` ones, replayed and equal; equal / total pasted. Add a **project-graph case with a real note tree** (a small fixture under a new `rust/parity/fixtures-memgraph/` plus a generator `gen_memgraph.py` that runs Python's own `project_graph` over it and writes `rust/parity/golden-memgraph/`): notes with frontmatter, links between notes, a link to a missing note, a note over the size cap, a symlinked note, a non-UTF-8 note, a note outside the memory folder, 600 notes (Python's caps) | locked oracle |
| 3 | **Bounded, non-following reads (the T045 rules)**: every file is opened `O_NOFOLLOW | O_NONBLOCK` from a held directory handle (reuse the `sys` helpers of `report_files.rs` by making them `pub(crate)`: a one-word visibility edit, no move of code, because T052 edits that file), judged on the open handle (regular file, size), read through `take(cap + 1)`; the caps are Python's (`board.py` / `memgraph.py` constants); a symlink, FIFO, device or oversize file is skipped exactly as Python skips it (copy Python's rule; where Python follows a symlink out of the project, Rust refuses and the case is listed as "decided for you") | NFR-002 |
| 4 | **Privacy**: board titles and note names are the owner's own text, shown as Python shows them and never put in an error body, log line or `Debug` output; name the fields shown and why safe; the code reads memory notes under the account roots (`projects/*/memory`) **only inside temp fixtures in tests** (`EASY_PERSONAL_AGENT_ROOT` and `HOME` pointed at temp folders; never the real home) | NFR-002 |
| 5 | Errors as Python: unknown `workspace`/`account` -> 400 with the core's fixed text; `project` unknown -> Python's answer (golden `unknown_project`); an unreadable project folder is skipped, not an error; routes work over real HTTP (token, Host, Origin, 405 on write methods) and `leak_rust.py` has both routes registered and prints `0 hits` | parity, surfaces |
| 6 | **No regression** (three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml`/`Cargo.lock` unchanged, existing goldens pass) and **Docs** (`rust/README.md`: routes, parameters, errors, "decided for you") | quality, usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T043.md`.
> **Locked oracle**: `rust/parity/golden*/api__board__*` and `api__memory_graph__*`, a new `golden-memgraph/` made once by Python over a fixture tree, and the Python sources and tests (read-only).

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
- [ ] A kanban file with 100,000 lines, a line with 1 MB, CRLF, a BOM, tabs, a table row with unbalanced pipes, unicode ids, duplicate ids: Python's behaviour (copy `board.py`'s caps and parser; its tests are the cases)
- [ ] A project folder that is deleted, is a file, is a symlink, or whose path is relative or non-UTF-8: Python's answer or a clean skip
- [ ] Memory-note frontmatter with huge values, binary data, NUL bytes, nested YAML: shown as Python shows or skipped; never a panic
- [ ] Time-dependent fields use the injected clock; run under the three TZ values
- [ ] The store is opened read-only (unless this guide names a write); a store that is not v11 is refused as for the other routes (exit 2 / 503)
- [ ] Time derived from "now" uses the injected clock (`clock.rs`); run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] `leak_rust.py` registry updated for every new route or tool (`API_CALLS`, `GUARD_CALLS`, the MCP calls) and `tests/mcp.rs` exact tool lists updated; keep each change a few lines (other tasks edit these files at the same time and the Supervisor merges them)
- [ ] Stale references: `rust/README.md` "not served" lists; `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/{board,memgraph,easymem}.rs` (new), `lib.rs`; `report_files.rs` (visibility of `sys` only) | the read modules |
| `rust/crates/core/tests/{board,memgraph}_parity.rs` (new), `rust/crates/app/tests/board_memgraph.rs` (new) | oracle replay and HTTP tests |
| `rust/parity/gen_memgraph.py`, `rust/parity/fixtures-memgraph/`, `rust/parity/golden-memgraph/` (all new) | the note-tree oracle |
| `rust/crates/app/src/api.rs` and/or `mcp.rs`, `rust/crates/app/tests/mcp.rs`, `rust/parity/leak_rust.py` | register the route(s) or tool(s): a few lines each |
| `rust/README.md` | routes/tools, parameters, error table, the "decided for you" list, corrected "not served" lists |
| `tasks/TASK_REVIEW_T043.md` | Demonstration BEFORE only |

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

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T043.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review
