# TASK_GUIDE — T039: Bump the Rust crate version to 0.2.0 for `rust-v0.2.0`
**Date**: 2026-11-11
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/Cargo.toml`, `docs/release/rust-v0.1.0.md` (how the version is stated and checked), and grep `rust/` (not `target/`) for every place the text `0.1.0` appears
6. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel" (the `rust-v0.2.0` release plan: ingest of the kit sources, the scheduler and the hardening tasks T033-T038).

**Restated intent**:
> `easy-personal-rs --version` prints `easy-personal-rs 0.2.0`, from the one workspace version, and nothing else changes.

**Out of scope**: any behaviour change; the Python version (`0.4.0`); tags, pushes, release notes (the Supervisor writes the plan).

**Requirement Refs**: the Release Log convention in `RUNBOOK.md` (crate version equals the tag version).

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T033, T034 (the content of the release)
**Entry point**: Standalone — N/A

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | `rust/Cargo.toml` `[workspace.package] version` is `0.2.0`; both crates inherit it (`version.workspace = true`); `rust/Cargo.lock` carries `0.2.0` for the two workspace packages and nothing else in the lock changes (show `git diff rust/Cargo.lock`); no dependency is added, removed or updated (no `cargo update`) | one version |
| 2 | A grep of `rust/` (excluding `target/`, `.venv`, `node_modules`, `*/golden*`, `*/fixtures*`) for the text `0.1.0` is pasted before and after; every remaining hit is explained (for example the release plan of 0.1.0 or a fixture that records a version Python printed) and any test or doc that pins the old Rust version is updated | no stale version |
| 3 | `cargo build` then `./target/debug/easy-personal-rs --version` prints `easy-personal-rs 0.2.0`; `cargo test --workspace` under `TZ` unset (paste the summary); clippy and fmt clean (also `--features window`); `git status` shows only the files you list; `git status -- src tests web` empty | quality |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T039.md`.

### Success Criteria

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the debug binary | `easy-personal-rs 0.2.0` | run it |
| 2 | `git diff --stat` | `Cargo.toml`, `Cargo.lock` and any pinned doc or test only | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && ./target/debug/easy-personal-rs --version && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: the version line in `rust/Cargo.toml`.
**Vital slice**: the bump, the lock, the grep, the tests.
**Cut list**: no changelog file, no tag, no release automation.

---

## Edge Case Checklist

- [ ] `cargo build` must not touch the network or update other lock entries (use the already-vendored lock; if cargo wants to change another entry, stop and report)
- [ ] A fixture that records Python's own version string (`0.4.0`) is untouched
- [ ] Stale references: `rust/README.md` and `rust/crates/app/tests/cli.rs` mention of the version text

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/Cargo.toml` | `version = "0.2.0"` |
| `rust/Cargo.lock` | the two workspace package versions |
| any test or README line that pins `0.1.0` for the Rust app | updated |
| `tasks/TASK_REVIEW_T039.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `docs/release/**`, `RUNBOOK.md`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

BEFORE (`--version` prints 0.1.0, the grep), the bump, AFTER (`--version`, grep, tests, lint). Paste every output.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T039.md` before the first change
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] The grep before and after and the `--version` line pasted
- [ ] Supervisor notified: task ready for Stage 4 review
