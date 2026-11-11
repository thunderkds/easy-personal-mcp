# TASK_GUIDE — T044: Rust `GET /api/dirs`, `GET /api/jobs` and the MCP tool `kit_findings` (read-only)
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
6. Read the Python you copy, **read-only**: `src/easy_personal/core/folders.py` (the whole file), `src/easy_personal/core/jobs.py` (the whole file) with its tests `tests/test_jobs*.py` and the fakes `tests/fakes/jobs_fakes.py`, `src/easy_personal/core/kitperf.py` (`kit_findings` and what it calls), the MCP tool `kit_findings` in `src/easy_personal/surfaces/mcp.py`, and the routes `/api/dirs`, `/api/jobs` in `src/easy_personal/surfaces/api.py`
7. Read the Rust you extend and copy the pattern of the wave-1 tasks already on `main`: `rust/crates/app/src/{api,mcp,server}.rs`, `rust/crates/core/src/{store,scope,reports,kit,suggestions}.rs`, `rust/crates/core/tests/{reports_parity,kit_parity}.rs`, `rust/crates/app/tests/reports.rs` (how a route or tool is registered, how errors and the token guard work, how a parity test replays goldens over the real binary), and `rust/parity/leak_rust.py` (it holds a hard registry of every `/api` route and MCP tool: a new one must be added to `API_CALLS` and `GUARD_CALLS` or the gate fails with "unregistered route")
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies, fixtures built in temp folders, and fakes on a scratch `PATH` with `env_clear`; never a real home, keychain, `claude` CLI, `systemctl` or crontab

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 2, `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> `GET /api/dirs?path=` lists sub-folder names for the dashboard's folder browser, `GET /api/jobs` lists the scheduled agent jobs (systemd user timers and crontab lines, command basename only), and the MCP tool `kit_findings` returns the recent kit findings, each exactly as Python answers, read-only.

**Out of scope**: any write; running any command other than the two allow-listed read-only ones below; the keychain; `claude`; any change to the Python app; `serve` or `mcp` opening the store for writing at start-up (they stay read-only unless this guide says a call opens a handle); a new dependency (none is allowed in this task: if one seems needed, stop and report); the routes and tools of other slices; schema changes.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`; the Python requirement ids named in the Python files above.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: wave 1 on `main` (T040, T042, T046)
**Entry point**: `GET /api/dirs`, `GET /api/jobs` (`rust/crates/app/src/api.rs`) and the `kit_findings` tool (`rust/crates/app/src/mcp.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | `folders.rs` (core): `list_dirs` as Python: absolute path only (400 `Path must be absolute`), `Not an existing folder` (also for a NUL byte), `Cannot read this folder` as 403, dot-names skipped, symlinks followed for the `is_dir` test only, sort `(casefold, name)`, cap 500 with `truncated: true`, `repo` flag by `lexists(<dir>/.git)`, `home` from the home variable; the path is never logged or echoed in an error. Names that are not UTF-8: skipped and listed as "decided for you" | RS-003 |
| 2 | `jobs.rs` (core): `list_jobs` as Python: `systemctl --user list-timers` / `show`, and `crontab -l`, **only** these (an allow-list table as `_READ_ONLY`), argument list never a shell, 3 s timeout per call, `LC_ALL=C`, stdin closed, unit names validated like `_UNIT` (never starting with `-`), only the command's first word (basename) leaves the module (no arguments, no `ExecStart`, no environment line), `no data` per kind that cannot be read, the cron `next` computation and the aliases (`@daily` ...) as Python | RS-003 |
| 3 | `kitperf.rs` (core) and MCP `kit_findings` (`limit`, `since`, Python's defaults and bounds): goldens `mcp__kit_findings__*` (4 files across `golden` and `golden-rich`) replayed and equal | locked oracle |
| 4 | Oracle for `dirs` and `jobs` (no committed goldens exist): a new generator `rust/parity/gen_dirs_jobs.py` runs Python's own `list_dirs` over a temp folder tree (spaces, unicode, dot-folders, a symlink to a folder, a broken symlink, a loop, a file, a `.git` folder, 501 sub-folders) and Python's `list_jobs` with the **fake `systemctl`/`crontab`** of `tests/fakes/jobs_fakes.py` (imported read-only, as `leak_rust.py` does) and writes `rust/parity/golden-jobs/`; the Rust tests replay both with their own fakes on a scratch `PATH`; equal / total pasted. Placeholders for temp paths are normalised in the generator, as `gen_ingest_kit.py` does | locked oracle |
| 5 | **Tests never run the real `systemctl` or `crontab`**: every process test sets `PATH` to a scratch folder holding only the fakes and uses `env_clear`; a test proves that with an empty `PATH` both kinds answer `no data` and the route still answers 200 | NFR-002 |
| 6 | Routes and tool over real HTTP / stdio: token guard, 405 on write methods, `/api/dirs` with no `path` uses the current working directory (Python: `os.getcwd()`, falling back to home if deleted) | surfaces |
| 7 | **No regression** (three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml`/`Cargo.lock` unchanged) and **Privacy**: `leak_rust.py` 0 hits with the routes registered; `/api/dirs` shows folder names and absolute paths of folders the owner browses (as Python does): name that, and prove no file name, size, content or symlink target is ever shown | quality, NFR-002 |
| 8 | **Docs**: `rust/README.md` routes, parameters, errors, and every "decided for you" item | usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T044.md`.
> **Locked oracle**: `rust/parity/golden*/mcp__kit_findings__*`, a new `golden-jobs/` made once by Python (with its test fakes) over temp trees, and the Python sources (read-only).

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
**Risk note**: this slice starts processes, so a reviewer will read the allow-list, the argv and the environment of every `Command` first. Use the process helper pattern of `schedule_cmd.rs` / `metrics.rs::run_git_timed` (timeout, argument list, no shell) rather than a new one.

---

## Edge Case Checklist

- [ ] Every parameter combination in the oracle, plus the 400/403/404/405/413 cases Python has (copy Python's status and `error` text, except CPython-specific text: then compare status and the `error` key only, as `manifest.json` `compare_notes` does); error bodies carry no path, secret or caller text
- [ ] `/api/dirs`: `path=/proc/self`, `/dev`, a path with `..`, a path with a NUL byte (`%00`), a 4,000-character path, a folder you cannot read (mode 000: 403), a path that is a file, a trailing slash; an unreadable entry is skipped, not an error
- [ ] `/api/jobs`: a crontab with 10,000 lines, a line with an unterminated quote, an env assignment before the command, a `sudo`/`env`/`bash -c` wrapper (copy Python's `_first_word` rules and its tests), a fake that prints 50 MB (cap the read), a fake that hangs (the 3 s timeout kills it; no zombie), a fake that exits non-zero
- [ ] A unit name that starts with `-` or holds a space is refused before any process starts (no option injection)
- [ ] `kit_findings`: `limit` 0, negative, huge, non-integer and `since` bad: Python's error texts or its clamp, copied
- [ ] The store is opened read-only (unless this guide names a write); a store that is not v11 is refused as for the other routes (exit 2 / 503)
- [ ] Time derived from "now" uses the injected clock (`clock.rs`); run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] `leak_rust.py` registry updated for every new route or tool (`API_CALLS`, `GUARD_CALLS`, the MCP calls) and `tests/mcp.rs` exact tool lists updated; keep each change a few lines (other tasks edit these files at the same time and the Supervisor merges them)
- [ ] Stale references: `rust/README.md` "not served" lists; `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/folders.rs`, `jobs.rs`, `kitperf.rs` (new), `lib.rs` | the read modules |
| `rust/crates/core/tests/{folders,jobs,kitperf}_parity.rs` (new), `rust/crates/app/tests/dirs_jobs.rs` (new) | oracle replay and HTTP/MCP tests |
| `rust/parity/gen_dirs_jobs.py` (new) and `rust/parity/golden-jobs/` (new) | the oracle for the routes that have no golden |
| `rust/crates/app/src/api.rs` and/or `mcp.rs`, `rust/crates/app/tests/mcp.rs`, `rust/parity/leak_rust.py` | register the route(s) or tool(s): a few lines each |
| `rust/README.md` | routes/tools, parameters, error table, the "decided for you" list, corrected "not served" lists |
| `tasks/TASK_REVIEW_T044.md` | Demonstration BEFORE only |

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

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T044.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review
