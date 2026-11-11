# TASK_GUIDE — T037: Add the Rust `ingest` step and the leak-gate line to the `/verify` recipe
**Date**: 2026-11-11
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `skills/verify/SKILL.md` (the Rust step T024 added), `tasks/TASK_REVIEW_T029.md` and `tasks/TASK_REVIEW_T030.md` (the findings that ask for these lines), `rust/README.md` (the `ingest` section) and `rust/parity/README.md` ("Ingest leak check")
6. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "continue the next tasks as parallel"; open item 6 of the Rust close-out: the `/verify` recipe lacks the ingest step and the leak-gate line.

**Restated intent**:
> The Rust section of `skills/verify/SKILL.md` also runs the `ingest` command (twice, to show the second adds 0) and the ingest leak gate, so `/verify` covers the second writer and the privacy gate the way the review files ask. Additions only.

**Out of scope**: any change to the Python part of the recipe; any change outside `skills/verify/SKILL.md` and the review file; new tooling.

**Requirement Refs**: NFR-002 (privacy), RS-003 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T024, T029, T030
**Entry point**: Standalone — N/A

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | Under the existing Rust step, a block that builds the binary, ingests a **fixture** folder (use the committed `rust/parity/ingest/fixtures` or a generated one; never a real home) with `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT` set to temp folders, prints the JSON, then ingests again and shows `new_events: 0`, then `store-info` shows `schema v11` and `pending: no` | ingest covered |
| 2 | A line that runs `rust/parity/.venv/bin/python rust/parity/leak_rust.py` and says the pass condition (`leak check: 0 hits ...`, exit 0) and that the parity venv is built the way `rust/parity/README.md` says (no `uv` needed) | leak gate covered |
| 3 | **Every command you add is run by you, from the recipe text as written**, and its real output and exit code are pasted; the recipe uses no variable in command position after `env` or `timeout` (the auto-mode gate rejects it) and no `$` next to `git` | tested recipe |
| 4 | Additions only: `git diff skills/verify/SKILL.md` shows no removed line except a changed sentence that is now wrong (say which); the file stays under 150 lines or the Supervisor is told; the frontmatter `description` mentions Rust (open P3 item of T024), one clause | surgical |
| 5 | `git status` shows only `skills/verify/SKILL.md` and `tasks/TASK_REVIEW_T037.md`; `git status -- src tests web` empty | scope |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T037.md`.

### Success Criteria

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the added ingest block run as written | first run `new_events > 0`, second `new_events: 0` | pasted output |
| 2 | the added leak line run as written | `leak check: 0 hits`, exit 0 | pasted output |
| 3 | `git diff --stat` | only `skills/verify/SKILL.md` | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain and rust/parity/.venv
. "$HOME/.cargo/env" && cd rust && cargo build --release && ./parity/.venv/bin/python parity/leak_rust.py
```

---

## Approach

**Pattern reference**: the existing Rust step in `skills/verify/SKILL.md` and `docs/release/rust-v0.1.0.md` section 4 (the smoke test, already proven on the release binary).
**Vital slice**: the ingest block, then the leak line, then the description clause.
**Cut list**: no new script, no CI wiring.

---

## Edge Case Checklist

- [ ] The recipe cleans nothing it did not create and says where its temp folder is
- [ ] zsh does not split an unquoted multi-word variable: wrap loops in `bash -c '...'` as the recipe's gotchas say
- [ ] Stale references: any count or version in the Rust step that the new lines change

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `skills/verify/SKILL.md` | the added lines |
| `tasks/TASK_REVIEW_T037.md` | Demonstration BEFORE only (the verbatim current Rust step: this is a docs change) |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `rust/**` | everything else |
| `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Quote the current Rust step (BEFORE), add the two blocks, run each exactly as written, paste output and exit codes, show the diff.

---

## Completion Checklist

- [ ] BEFORE (verbatim prior text) in `tasks/TASK_REVIEW_T037.md`
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Both blocks run as written, output pasted
- [ ] Supervisor notified: task ready for Stage 4 review
