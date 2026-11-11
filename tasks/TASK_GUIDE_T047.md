# TASK_GUIDE — T047: Rust MCP tool `save_report` and the `investigate` prompt
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
6. Read the Python you copy, **read-only**: `src/easy_personal/core/reports.py` (`validate`, `save`, `snapshot` and the constants), `src/easy_personal/core/secretlike.py` (the `SECRET` pattern), the `save_report` tool, the `investigate` prompt and its two text constants (`INVESTIGATE_PROMPT`, `SAVE_REPORT_STEP`) in `src/easy_personal/surfaces/mcp.py`, and the `reports` table in `rust/crates/core/src/schema_v11.sql`
7. Read the Rust you extend and copy the pattern of the wave-1 tasks already on `main`: `rust/crates/app/src/{api,mcp,server}.rs`, `rust/crates/core/src/{store,scope,reports,kit,suggestions}.rs`, `rust/crates/core/tests/{reports_parity,kit_parity}.rs`, `rust/crates/app/tests/reports.rs` (how a route or tool is registered, how errors and the token guard work, how a parity test replays goldens over the real binary), and `rust/parity/leak_rust.py` (it holds a hard registry of every `/api` route and MCP tool: a new one must be added to `API_CALLS` and `GUARD_CALLS` or the gate fails with "unregistered route")
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies, fixtures built in temp folders, and fakes on a scratch `PATH` with `env_clear`; never a real home, keychain, `claude` CLI, `systemctl` or crontab

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 2, `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> An MCP client can call `save_report` on the Rust server and the report appears on the dashboard's Warnings & plan page (Python's and Rust's `GET /api/reports`), and `prompts/get investigate` returns Python's text with the scope line. This is the first Rust MCP call that **writes**: only to the store's `reports` table, and only through a write handle opened for that one call.

**Out of scope**: any other write; `serve` writing; keeping a write handle open between calls; a secret scanner other than Python's pattern; changing the schema; any change to the Python app; `serve` or `mcp` opening the store for writing at start-up (they stay read-only unless this guide says a call opens a handle); a new dependency (none is allowed in this task: if one seems needed, stop and report); the routes and tools of other slices; schema changes.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`; the Python requirement ids named in the Python files above.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: wave 1 on `main` (T040, T042, T046)
**Entry point**: the `save_report` tool and the `investigate` prompt (`rust/crates/app/src/mcp.rs`), listed by `tools/list` and `prompts/list`

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | Core `reports.rs` gains `validate` and `save` with Python's limits (summary <= 2,000 chars, <= 5 fixes, fix text <= 300, `tool`/`number` <= 100, keys only `text`/`tool`/`number`, `text` required, empty text refused) and **every refusal text byte-for-byte** (they name the field, never the value); the secret pattern is Python's `SECRET` ported with the same matches on a corpus: a test feeds >= 40 strings (keys, tokens, PEM headers, near misses, unicode) through both and compares (generate the corpus answers with Python once into `rust/parity/golden-save-report/`). The Rust `regex` crate may be used only if it is **already** in `Cargo.lock`; otherwise hand-write the matcher and prove it on the corpus | RS-003, NFR-002 |
| 2 | The snapshot stored is what Python stores: every rule measured now for the scope (`evaluate_checked_scoped`, `left_out_ids` from T040), in the same JSON shape, `as_of_utc` from the injected clock; `server_version` is Rust's own version (the **one** field the comparison normalises, as the plan says); an unknown workspace or account is refused with Python's text and nothing is stored; the scope is validated **before** the write | parity |
| 3 | **Copy-store-then-compare**: for each case (plain, scoped by workspace, by account, both, fixes with citations, 5 fixes, no fixes, unicode text) the oracle script `rust/parity/gen_save_report.py` runs Python's `reports.save` on a copy of `store-rich` and Rust's tool on another copy, and the test compares the new `reports` row (all columns, `saved_utc` and `server_version` normalised) and `GET /api/reports/<id>` of the Rust server over each copy; equal / total pasted. Refusal cases (over limit, secret-looking text, wrong types, unknown scope) store nothing: the row count is unchanged | locked oracle |
| 4 | **Write discipline**: the store is opened for writing **only inside the `save_report` call** (`store.rs` write-open if one exists from T031, else the smallest function that does what Python's `store.opened` does: same busy timeout 5 s, same pragmas, schema must be v11 else refuse); one transaction; closed before the answer is built; no `ingest.lock` taken (Python takes none: copy it); a test runs `save_report` while an `ingest` runs on the same store (the T031 coexistence harness is the pattern) and both finish with consistent rows; `mcp` start-up and every other tool still open read-only (a test asserts the file's mtime and bytes are unchanged after any read tool call) | safety |
| 5 | `prompts/list` and `prompts/get investigate` (with and without `workspace` / `account`, the `Scope: pass ...` line with `json.dumps(ensure_ascii=False)` quoting): the goldens `mcp__prompt_investigate__*` (2) replayed and equal; `initialize` advertises the prompts capability as Python does; `tools/list` now holds `save_report` with Python's description and input schema (compare to `mcp__tools_list.json`) | surfaces |
| 6 | An argument of the wrong type or a missing required argument is Python's MCP error (copy what the existing Rust tools already do); no tool result, error or log line ever echoes the summary or fix text | NFR-002 |
| 7 | **No regression** (three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml`/`Cargo.lock` unchanged) and `leak_rust.py` `0 hits` with the tool registered (canaries planted in the summary of a refused call must not appear on any surface) | quality, NFR-002 |
| 8 | **Docs**: `rust/README.md` (the tool, the prompt, the write discipline, the error table, every "decided for you") | usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T047.md`.
> **Locked oracle**: `rust/parity/golden*/mcp__prompt_investigate__*`, `mcp__tools_list.json`, a new `golden-save-report/` made once by Python over store copies, and the Python sources (read-only).

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
**Risk note**: the first write path of the Rust MCP server. A reviewer reads the open/close of the write handle and the order validate -> scope -> snapshot -> insert first.

---

## Edge Case Checklist

- [ ] Every parameter combination in the oracle, plus the 400/403/404/405/413 cases Python has (copy Python's status and `error` text, except CPython-specific text: then compare status and the `error` key only, as `manifest.json` `compare_notes` does); error bodies carry no path, secret or caller text
- [ ] Two `save_report` calls at once: two rows, two ids, no `database is locked` leaking to the client (the 5 s busy timeout, then a fixed error text that carries no SQL)
- [ ] A store that is read-only on disk (mode 0400): a fixed error, nothing stored, no path in the message
- [ ] A summary of exactly 2,000 characters, 2,001, and a multi-byte one (Python counts characters, not bytes); a fix with `number` given as a string or an int (copy Python's accepted types)
- [ ] The rules measured for the snapshot use the shipped rule pack as `/api/warnings` does (the `config.yaml` threshold override is still not ported: say so)
- [ ] The store is opened read-only (unless this guide names a write); a store that is not v11 is refused as for the other routes (exit 2 / 503)
- [ ] Time derived from "now" uses the injected clock (`clock.rs`); run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] `leak_rust.py` registry updated for every new route or tool (`API_CALLS`, `GUARD_CALLS`, the MCP calls) and `tests/mcp.rs` exact tool lists updated; keep each change a few lines (other tasks edit these files at the same time and the Supervisor merges them)
- [ ] Stale references: `rust/README.md` "not served" lists; `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/reports.rs` (extend), `rust/crates/core/src/secret.rs` (new, if a separate file is cleaner), `rust/crates/core/src/store.rs` (only if a write-open is missing) | validate, save, the secret pattern, the write handle |
| `rust/crates/core/tests/save_report_parity.rs` (new), `rust/crates/app/tests/save_report.rs` (new) | oracle replay, MCP tests, coexistence |
| `rust/parity/gen_save_report.py` (new) and `rust/parity/golden-save-report/` (new) | the copy-store oracle and the secret corpus |
| `rust/crates/app/src/api.rs` and/or `mcp.rs`, `rust/crates/app/tests/mcp.rs`, `rust/parity/leak_rust.py` | register the route(s) or tool(s): a few lines each |
| `rust/README.md` | routes/tools, parameters, error table, the "decided for you" list, corrected "not served" lists |
| `tasks/TASK_REVIEW_T047.md` | Demonstration BEFORE only |

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

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T047.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review
