# TASK_GUIDE — T062: Retire Python 5: cutover — Rust becomes the default (install, MCP registration, timer, connectors), steps the owner runs
**Date**: 2026-11-15
**Complexity Level**: C1
**Risk Level**: Medium
**Priority**: P3
**Assigned agent**: common-infrastructure
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Read `rust/docs/PYTHON_RETIREMENT_PLAN.md`, `rust/docs/PARITY_T061.md` ("differs README" items), `README.md` (`## Install`, `### Register the MCP server`, `## Schedule`, `## Connectors`, `## Upgrade and rollback`), `rust/README.md` (`## Build and run`, `## Window`, `## Scheduled ingest (P2c)`, `## Connector writes, keychain and launcher (T050)`)
6. Read `rust/crates/app/src/claude_cli.rs` `launcher()` (the launcher is found beside `std::env::current_exe()`, else on `PATH`)
- **Never run anything against the owner's real setup**: no `claude mcp add/remove` without a temp `CLAUDE_CONFIG_DIR`, no `systemctl --user`, no `schedule install/remove` without a temp `XDG_CONFIG_HOME`, no write under `~/.local`, `~/.cargo`, `~/.easy-personal`, `~/.claude*`, `~/.config`. Never run `real_keychain_items_cross_check` or any `#[ignore]` keychain test. Every real-run check uses temp folders.

---

## Requirement

**Original request** (owner, 2026-11-15): "as your recommend, let continue" — the go-ahead for T062, Ubuntu only (macOS and Windows stay not planned), recorded on `main` in `ad5d88b`.

**Restated intent**:
> 1. One owner-facing cutover document, `rust/docs/CUTOVER.md`, with numbered steps the owner runs, each a copy-paste command block and the line it should print. Order:
>    - **Step 0, checks before anything changes**: back up the store (`cp ~/.easy-personal/store.sqlite …`); `easy-personal-rs store-check`; run the keychain interop test `cd rust && cargo test -p easy-personal-rs --bin easy-personal-rs real_keychain_items_cross_check -- --ignored` (it reads the real Secret Service; the cutover stops if it fails, because the Rust launcher must read the secrets Python stored — `rust/docs/PYTHON_RETIREMENT_PLAN.md` "Keychain").
>    - **Step 1, install**: build the dashboard (`npm --prefix web ci && npm --prefix web run build`; Vite writes `src/easy_personal/dashboard/static`), then `cargo install --locked --path rust/crates/app --features window --root ~/.local/share/easy-personal-rs`, copy the built dashboard to `~/.local/share/easy-personal-rs/static`, and link only `easy-personal-rs` into `~/.local/bin` (it replaces the old 0.1.0 file there; `easy-personal-launch` in `~/.local/bin` stays Python's, so rollback keeps working). Explain: the Rust launcher is found beside the real binary (`current_exe()` follows the link).
>    - **Step 2, MCP server**: for each confirmed Claude account (default and each `CLAUDE_CONFIG_DIR`), `claude mcp remove easy-personal --scope user` then `claude mcp add --scope user easy-personal -- ~/.local/bin/easy-personal-rs mcp`; check with `claude mcp get easy-personal`.
>    - **Step 3, timer**: `easy-personal-rs schedule install --dry-run`, then `easy-personal-rs schedule install` (same unit names: it replaces the Python timer, `rust/README.md` "Same unit names as Python"); check `systemctl --user list-timers | grep easy-personal`.
>    - **Step 4, connectors**: `easy-personal-rs connector sync` shows the plan (aliases that "do not point at the launcher" are re-added pointing at the Rust launcher); confirm; then `easy-personal-rs doctor`.
>    - **Step 5, dashboard**: `easy-personal-rs dashboard --static-dir ~/.local/share/easy-personal-rs/static` (and `app --static-dir …` for the window).
>    - **The week**: what to watch (ingest runs daily, MCP tools answer, dashboard opens, connectors start) and where to note a problem; the T063 gate after it.
>    - **Rollback**: `uv tool install --reinstall .` is still there; `easy-personal schedule install`, the Python MCP registration, `easy-personal connector sync`; restore the store backup only if a migration happened (none is expected: both read schema 11).
>    - Known differences the owner will see (from `rust/docs/PARITY_T061.md`, link, do not copy): `--version` prints `easy-personal-rs 0.2.0`; command name `easy-personal-rs`.
> 2. `README.md`: a short `## Rust app (default from T062)` section right after `## Install` that says Rust is now the default on Ubuntu and points at `rust/docs/CUTOVER.md`; no other README change. Do not edit `rust/README.md` (another session's tasks add to it).
> 3. An automated guard that the document stays true: a test that reads `rust/docs/CUTOVER.md`, takes every `easy-personal-rs <subcommand …>` it shows, and checks each subcommand path exists (`<path> --help` exits 0 on the built binary). It runs offline, in temp folders, and touches nothing real.

**Out of scope**: running any step on the owner's machine (the owner runs them), macOS/Windows, T063 (removing Python), packaging (.deb, release builds in CI), changing any Rust behaviour.

### Requirement Fidelity Gate
- [x] Restated intent agreed: owner go-ahead 2026-11-15, Ubuntu only; scope agreed with the other supervisor (`rust-57`): README.md and `rust/docs/` only, `rust/README.md` untouched
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: `rust/docs/PYTHON_RETIREMENT_PLAN.md` row T062; NFR-001, NFR-006 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: T061 (merged)
**Entry point**: `rust/docs/CUTOVER.md`

## Dependency rule
No new crate.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `rust/docs/CUTOVER.md` has steps 0-5, the week, and rollback, each a command block plus its expected line | intent 1 |
| 2 | Step 0 runs the keychain interop test and says the cutover stops if it fails | intent 1 |
| 3 | Install keeps Python's `~/.local/bin/easy-personal-launch` untouched; a real run in temp folders shows the linked `easy-personal-rs` finds the launcher beside its real file (`doctor` or `connector sync --help`/plan in a temp `CLAUDE_CONFIG_DIR` with a fake `claude`) | intent 1 |
| 4 | The install command block itself works: run it with `--root <temp>` and the dashboard copy into `<temp>/static`; `easy-personal-rs --version` and `dashboard --static-dir <temp>/static` serve the page (`GET /` 200) | intent 1 |
| 5 | Test: every `easy-personal-rs …` subcommand in the document exists (`--help` exit 0); it fails when a made-up subcommand is added | intent 3 |
| 6 | `README.md` gains only the short section and the link | intent 2 |
| 7 | `cargo test --workspace`, clippy `-D warnings`, `cargo fmt --check` pass | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test -p easy-personal-rs --test cutover_doc && cargo test --workspace --no-fail-fast 2>&1 | grep -E '^test result|FAILED' | tail -5 && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

### Evidence (filled by reviewer at Stage 4/5)
> Moved to `tasks/TASK_REVIEW_T062.md`.

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T062.md`. BEFORE: the verbatim current `README.md` `## Install` section (docs change) and the absence of `rust/docs/CUTOVER.md`.

## UI / Design Acceptance Criteria
N/A — documentation and one test.

---

## Approach
Write the test (AC5) first against an empty document, then the document, then the temp-folder real runs (AC3, AC4). Smallest text that a person can follow; plain words (AGENTS.md Writing Principles apply: the owner reads it).

## Edge Case Checklist
- `~/.local/bin` already holds an old `easy-personal-rs` 0.1.0 file (a regular file, not a link): the step replaces it (`ln -sf`)
- Several Claude accounts: step 2 repeats per account with `CLAUDE_CONFIG_DIR=<folder>`
- No systemd user session: `schedule install` prints a cron line and exits 1 — say what to do
- `cargo` not on `PATH` (`. "$HOME/.cargo/env"`)

## Files to Change (Predicted)
`rust/docs/CUTOVER.md` (new), `README.md`, `rust/crates/app/tests/cutover_doc.rs` (new), `tasks/TASK_REVIEW_T062.md`

## Files Must NOT Touch
`rust/README.md`, `rust/crates/**/src/**`, `web/**`, `src/easy_personal/**`, `PRD.md`, any `PROJECT_KANBAN.md`, `memory/**`, `.claude/**`

## Test Plan
AC5 as an automated test; AC3 and AC4 as pasted temp-folder runs; the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T062.md`
- [ ] Commits on branch `t062-cutover` only; final report with `Status:`
