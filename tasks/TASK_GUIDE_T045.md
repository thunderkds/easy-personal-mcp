# TASK_GUIDE — T045: Rust `GET /api/kit/report-files` and `GET /api/kit/report-file` (path safety)
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
6. Read the Python you copy, **read-only**: `src/easy_personal/core/kit.py` (`report_files`, `report_file`, `ReportNotFound`, `ReportTooLarge` and every helper they call: the whole report-folder part), `tests/leak/test_kit_report_files_leak.py` and any `tests/test_kit_report*.py` (the path cases), the routes `/api/kit/report-files`, `/api/kit/report-file` with `_report_args`, `_report_call`, `_report_file_response` in `src/easy_personal/surfaces/api.py`
7. Read the Rust you extend and copy the pattern of the wave-1 tasks already on `main`: `rust/crates/app/src/{api,mcp,server}.rs`, `rust/crates/core/src/{store,scope,reports,kit,suggestions}.rs`, `rust/crates/core/tests/{reports_parity,kit_parity}.rs`, `rust/crates/app/tests/reports.rs` (how a route or tool is registered, how errors and the token guard work, how a parity test replays goldens over the real binary), and `rust/parity/leak_rust.py` (it holds a hard registry of every `/api` route and MCP tool: a new one must be added to `API_CALLS` and `GUARD_CALLS` or the gate fails with "unregistered route")
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies, fixtures built in temp folders, and fakes on a scratch `PATH` with `env_clear`; never a real home, keychain, `claude` CLI, `systemctl` or crontab

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 2, `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> The dashboard's report-folder browser works on Rust: `report-files` lists the files of a workspace repository's report folder and `report-file` returns one file's bytes (a `.html`, `.json`, `.md` ...) with Python's content type and headers, and **nothing outside the report folder can ever be read**: not by `..`, an absolute path, a symlink, a hard-linked trick, a device or FIFO, an oversize file or a changed file between check and read.

**Out of scope**: any write; listing or reading anything beyond what Python allows; following symlinks anywhere Python does not; any change to the Python app; `serve` or `mcp` opening the store for writing at start-up (they stay read-only unless this guide says a call opens a handle); a new dependency (none is allowed in this task: if one seems needed, stop and report); the routes and tools of other slices; schema changes.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`; the Python requirement ids named in the Python files above.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: wave 1 on `main` (T040, T042, T046)
**Entry point**: `GET /api/kit/report-files` and `GET /api/kit/report-file` (`rust/crates/app/src/api.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New module `report_files.rs` in `rust/crates/core/src/` (not in `kit.rs`: T044 and T042 touch other kit code): `report_files(conn, workspace, repo_no, folder)` and `report_file(conn, workspace, repo_no, folder, path)` with the same allow-list of folders (the names `kit.py` allows), extension and content-type table, size cap (`ReportTooLarge` -> 413 `too large`), `ReportNotFound` -> 404 `not found`; the misses have fixed bodies that never echo the input (FR-090) | RS-003 |
| 2 | **Path safety, by construction, not by string checks alone**: resolve the repository root from the store row only (never from the request); open each directory component with `openat`-style `O_NOFOLLOW | O_DIRECTORY` (or the std equivalent) so no component can be a symlink; the final file is opened with `O_NOFOLLOW | O_NONBLOCK`, then `fstat` on the descriptor: regular file only, size checked on the descriptor (not on the path), read at most the cap + 1 bytes from the descriptor; a request path with `..`, a leading `/`, a NUL, a backslash, an empty part, or a dot-file is a 404 before any filesystem call. If the std library cannot do `openat`, use `libc` only if it is **already** a dependency of the workspace (`cargo tree`); otherwise stop and report (no new crate) | NFR-002 |
| 3 | **Goldens** `api__kit_report_files__*` (5) and `api__kit_report_file__*` (4) replayed and equal over the committed fixture (`fixtures-rich/kit/`); equal / total pasted; the `content-type`, `x-content-type-options: nosniff`, `cache-control: no-store`, `content-disposition: inline` headers equal Python's (and the CSP the server adds to every response is still set) | locked oracle |
| 4 | **Attack table**: a Rust test builds, in a temp folder, a repository whose report folder holds: a symlink to `/etc/passwd`-like file outside, a symlink to a folder outside, a symlink inside to a file inside (what does Python do? copy it), a hard link, a FIFO (must not block: `O_NONBLOCK` and a regular-file check), a directory named like a file, a file of cap, cap+1 and 100 MB (sparse), a file replaced by a symlink after listing (race: the open must still refuse), a non-UTF-8 name, a name with a newline, a dot-file, an upper-case extension, a very long name (255 bytes), and a `repo_no` of `0`, `-1`, `99999999999`, `1e3`, `١` (non-ASCII digit). Each case is compared with what Python answers over the same tree (oracle: a small `rust/parity/gen_report_files.py` that runs Python's own functions and writes `rust/parity/golden-report-files/`; where Python would follow a symlink out of the folder, Rust must **refuse** and the case is listed as "decided for you" with Python's behaviour stated) | NFR-002 |
| 5 | The canaries of `tests/leak/canaries.py`: a file outside the report folder holding a canary is never returned by any request in the table; the leak gate (`leak_rust.py`) gains the two routes and still prints `0 hits` | NFR-002 |
| 6 | Over real HTTP: token / Host / Origin guard first (403), 405 on write methods, `HEAD` as `GET` without a body, query parameters repeated or missing give Python's answer, percent-encoded `..%2f` and double-encoding are 404 | surfaces |
| 7 | **No regression** (three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml`/`Cargo.lock` unchanged) and **Docs** (`rust/README.md`: routes, parameters, error table, the attack table result, every "decided for you") | quality, usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T045.md`.
> **Locked oracle**: `rust/parity/golden-rich/api__kit_report_file*__*`, a new `golden-report-files/` made once by Python over a temp tree, `tests/leak/test_kit_report_files_leak.py` (read-only), and the Python sources.

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

**Reviewer note**: this task is High risk; Stage 4 includes a security review by the Supervisor and the attack table result is read line by line.

---

## Approach

**Pattern reference**: the wave-1 parity tests and how `api.rs` / `mcp.rs` register a route or tool.
**Vital slice**: the core module and its parity test against the oracle first (red first), then the route or tool wiring, then HTTP and MCP tests, then the leak registry and docs.
**Cut list**: no abstraction shared with other slices beyond what exists; no caching; no new crate.

---

## Edge Case Checklist

- [ ] Every parameter combination in the oracle, plus the 400/403/404/405/413 cases Python has (copy Python's status and `error` text, except CPython-specific text: then compare status and the `error` key only, as `manifest.json` `compare_notes` does); error bodies carry no path, secret or caller text
- [ ] A TOCTOU test: list, swap a file for a symlink, then read: refused; and read while the file grows past the cap: stops at cap + 1 and answers 413
- [ ] Unix only in this task; on other targets the module compiles but the routes answer 404 (macOS/Windows are P4/P5): the `O_NOFOLLOW`/`O_NONBLOCK` constants are right on x86/arm Linux and macOS only, so take them from the platform headers/crate that already exists, not from literals, or gate with `cfg` and say so
- [ ] Memory: the cap is enforced while reading; never `read_to_end` an unbounded source
- [ ] No log line or error carries a path, a file name, a byte of content or the request text
- [ ] The store is opened read-only (unless this guide names a write); a store that is not v11 is refused as for the other routes (exit 2 / 503)
- [ ] Time derived from "now" uses the injected clock (`clock.rs`); run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] `leak_rust.py` registry updated for every new route or tool (`API_CALLS`, `GUARD_CALLS`, the MCP calls) and `tests/mcp.rs` exact tool lists updated; keep each change a few lines (other tasks edit these files at the same time and the Supervisor merges them)
- [ ] Stale references: `rust/README.md` "not served" lists; `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/report_files.rs` (new), `rust/crates/core/src/lib.rs` | the module |
| `rust/crates/core/tests/report_files_parity.rs` (new), `rust/crates/app/tests/report_files.rs` (new) | goldens, the attack table, HTTP tests |
| `rust/parity/gen_report_files.py` (new) and `rust/parity/golden-report-files/` (new) | the attack-table oracle |
| `rust/crates/app/src/api.rs` and/or `mcp.rs`, `rust/crates/app/tests/mcp.rs`, `rust/parity/leak_rust.py` | register the route(s) or tool(s): a few lines each |
| `rust/README.md` | routes/tools, parameters, error table, the "decided for you" list, corrected "not served" lists |
| `tasks/TASK_REVIEW_T045.md` | Demonstration BEFORE only |

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

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T045.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review
