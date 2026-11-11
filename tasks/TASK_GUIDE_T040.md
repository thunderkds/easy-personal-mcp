# TASK_GUIDE — T040: Rust `GET /api/reports`, `/api/reports/{id}` and `/api/first-run` (read-only)
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the entries "Rust P3 planned" and "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP3.md` (slice S1, the Edge Case Checklist and the decisions Q1-Q8) and `rust/parity/README.md` (the oracle, the goldens, the venv)
6. Read the Python you copy, **read-only**: `src/easy_personal/core/reports.py` (the read side: list and one report, the before/now rule values), `src/easy_personal/core/first_run.py`, `src/easy_personal/core/schedule.py` (the timer check), and the routes in `src/easy_personal/surfaces/api.py` (search `reports`, `first-run`)
7. Read the Rust you extend: `rust/crates/app/src/{api,mcp,server}.rs` (how a route or tool is registered and how errors and the token guard work), `rust/crates/core/src/{store,metrics,workspaces,scope}.rs` (how a read module opens the store and takes a `Scope`), and an existing parity test as the pattern: `rust/crates/core/tests/workspaces_parity.rs` and `rust/crates/app/src/api_tests.rs`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies; never a real home, keychain, `claude` CLI or systemd

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 1, slice S1 of `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> `GET /api/reports`, `GET /api/reports/{id}` and `GET /api/first-run` answer exactly what the Python dashboard answers over the same store (the saved investigate reports with their numbers, one report with its before/now line per rule, and the first-run checklist including whether the daily timer is installed), read-only.

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
**Entry point**: `GET /api/reports`, `GET /api/reports/{id}`, `GET /api/first-run` (`rust/crates/app/src/api.rs`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New core modules `reports.rs` (list, one report, the current value of every rule for the before/now line, reusing the existing `rules.rs`/`metrics.rs`) and `first_run.rs` (the checklist; the timer check looks for the unit files exactly as Python does, `XDG_CONFIG_HOME`/`HOME` read per request) in `rust/crates/core/src/`, read-only | RS-003 |
| 2 | **Goldens** `api__reports__all`, `api__reports_one__{found,missing,scoped}` and `api__first_run__all` (and their request headers and statuses in `manifest.json`) are replayed by a test in `rust/crates/core/tests/` or `api_tests.rs` and equal; the number equal / total is pasted | locked oracle |
| 3 | `/api/reports/{id}` with a non-numeric id, an id that does not exist, and a scope that excludes the report answers as Python (400/404 and bodies); the report text is shown only as stored (it is the one text the store keeps) and never logged | privacy |
| 4 | Routes and the 403 guard work over real HTTP (a server test like the existing ones): token, Host, Origin; `405` for POST/PUT/DELETE on these paths is as Python (the write methods are not served in this task) | server |
| 5 | **No regression**: `cargo test --workspace` under `TZ` unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago` (paste all three); clippy and fmt clean (also `--features window`); `git status -- src tests web` empty; `Cargo.toml`/`Cargo.lock` unchanged (no new dependency); the existing goldens still pass | quality |
| 6 | **Privacy**: `rust/parity/leak_rust.py` still prints `leak check: 0 hits` (paste it); every new error body and log line carries no path, secret or caller text; where the new surface can show repository or account text, name which fields are shown and why they are safe | NFR-002 |
| 7 | **Docs**: `rust/README.md` lists the new routes/tools, their query parameters and error table; the P1b "not served" lists are corrected; every item "decided for you" (where Rust differs from Python) is listed in your report and in the README | usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T040.md`.
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
- [ ] `first-run` must not depend on the machine running the test: the timer check reads a temp `XDG_CONFIG_HOME` the test controls, never the real `~/.config`
- [ ] Time: anything derived from "now" uses the injected clock (`clock.rs`) so the goldens are reproducible; run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] Stale references: `rust/README.md` "not served" lists and `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/{reports,first_run}.rs` (new), `rust/crates/core/src/lib.rs` | the read modules |
| `rust/crates/core/tests/reports_parity.rs` (new) | golden replay |
| `rust/crates/app/src/api.rs` and/or `mcp.rs` | register the route(s) or tool(s) (one line each in the route table; keep the change minimal: other tasks edit these files at the same time) |
| `rust/README.md` | docs |
| `tasks/TASK_REVIEW_T040.md` | Demonstration BEFORE only |

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

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T040.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Golden counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written
- [ ] Supervisor notified: task ready for Stage 4 review
