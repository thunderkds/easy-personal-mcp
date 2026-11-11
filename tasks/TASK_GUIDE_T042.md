# TASK_GUIDE — T042: Rust `GET /api/kit`, `/api/kit/workspaces`, `/api/kit/reports` and the MCP tool `kit_summary` (read-only)
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
5. Read `BRAINSTORMING_LOG_RUSTP3.md` (slice S3, the Edge Case Checklist and the decisions Q1-Q8) and `rust/parity/README.md` (the oracle, the goldens, the venv)
6. Read the Python you copy, **read-only**: `src/easy_personal/core/kit.py` (`kit_summary`, the workspace kit, the reports list; lines 154-300 and the helpers they call), `src/easy_personal/core/kitperf.py` only where `kit.py` calls it, `src/easy_personal/core/rules.py` (kit rules already ported), `src/easy_personal/sources/git_local.py`, the MCP tool in `src/easy_personal/surfaces/mcp.py` (`kit_summary`) and the routes in `src/easy_personal/surfaces/api.py` (`/api/kit`, `/api/kit/workspaces`, `/api/kit/reports`)
7. Read the Rust you extend: `rust/crates/app/src/{api,mcp,server}.rs` (how a route or tool is registered and how errors and the token guard work), `rust/crates/core/src/{store,metrics,workspaces,scope}.rs` (how a read module opens the store and takes a `Scope`), and an existing parity test as the pattern: `rust/crates/core/tests/workspaces_parity.rs` and `rust/crates/app/src/api_tests.rs`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies; never a real home, keychain, `claude` CLI or systemd

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 1, slice S3 of `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> `GET /api/kit`, `GET /api/kit/workspaces`, `GET /api/kit/reports` and the MCP tool `kit_summary` answer exactly what Python answers over the same store and fixture repositories: per-project kit stage, trace counts, verifier and UI parts, report folder names, scoped by workspace and account, read-only.

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
**Entry point**: `GET /api/kit*` (`rust/crates/app/src/api.rs`) and the `kit_summary` tool (`rust/crates/app/src/mcp.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New core module `kit.rs` in `rust/crates/core/src/` that builds the kit summary from the store tables `kit_trace_events`, `ui_sessions`, `verifier_scores`, `context_index` and the project folders (existing `git` helper and the 10 s / 5 s limits of `metrics.rs`), with the `Scope` handling of the other modules; the report-folder listing reads only folder names and counts, never file contents | RS-003 |
| 2 | **Goldens** `api__kit__*` (7), `api__kit_workspaces__*` (3), `api__kit_reports__all` and `mcp__kit_summary__*` (6) are replayed and equal; equal / total pasted. The `context_index_stale` rule note in `manifest.json` `cannot_fire` is respected (do not make a test depend on a machine path) | locked oracle |
| 3 | Unknown workspace / account, bad `since`, and the 400/404 cases answer as Python; a project folder that no longer exists, a project with no traces, and an abstained trace row are in the fixture or a hand-made store and equal Python's answer | parity |
| 4 | MCP: `tools/list` shows `kit_summary` with Python's schema text; `tools/call` returns the same JSON as the golden; an invalid argument is the same error; routes and the tool work over real HTTP / stdio | surfaces |
| 5 | Report folder reading: names only, symlinks and non-directories as Python treats them; no file content or path in any answer beyond what the golden shows | privacy |
| 6 | **No regression**: `cargo test --workspace` under `TZ` unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago` (paste all three); clippy and fmt clean (also `--features window`); `git status -- src tests web` empty; `Cargo.toml`/`Cargo.lock` unchanged (no new dependency); the existing goldens still pass | quality |
| 7 | **Privacy**: `rust/parity/leak_rust.py` still prints `leak check: 0 hits` (paste it); every new error body and log line carries no path, secret or caller text; where the new surface can show repository or account text, name which fields are shown and why they are safe | NFR-002 |
| 8 | **Docs**: `rust/README.md` lists the new routes/tools, their query parameters and error table; the P1b "not served" lists are corrected; every item "decided for you" (where Rust differs from Python) is listed in your report and in the README | usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T042.md`.
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
- [ ] `git` is run with a timeout and an argument list (never a shell); a repository that is slow or missing gives Python's "unknown", not a hang
- [ ] Time: anything derived from "now" uses the injected clock (`clock.rs`) so the goldens are reproducible; run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] Stale references: `rust/README.md` "not served" lists and `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/kit.rs` (new), `rust/crates/core/src/lib.rs` | the read module |
| `rust/crates/core/tests/kit_parity.rs` (new) | golden replay |
| `rust/crates/app/src/api.rs` and/or `mcp.rs` | register the route(s) or tool(s) (one line each in the route table; keep the change minimal: other tasks edit these files at the same time) |
| `rust/README.md` | docs |
| `tasks/TASK_REVIEW_T042.md` | Demonstration BEFORE only |

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

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T042.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Golden counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written
- [ ] Supervisor notified: task ready for Stage 4 review
