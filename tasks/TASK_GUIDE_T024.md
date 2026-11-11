# TASK_GUIDE — T024: A Rust step in the `/verify` recipe
**Date**: 2026-11-11
**Complexity Level**: C1
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: common-infrastructure
**Agent guide**: `agents/common-infrastructure.md`

---

## Mandatory Startup (Do Not Skip)

Before changing anything:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/common-infrastructure.md`
5. Read `skills/verify/SKILL.md` completely (the file you change), `rust/README.md` (the commands of `easy-personal-rs`) and `tasks/TASK_REVIEW_T023.md` (the real runs that worked)
6. Do not open `~/.easy-personal` or any `~/.claude*` folder, and do not write those paths in any command. Use temp stores only

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files in step 1 and 5 stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, accepting the Supervisor's suggestion): the project's `/verify` recipe has no Rust step, so the Rust port is verified only by its own tests. Add one.

**Restated intent**:
> `skills/verify/SKILL.md` gains a short Rust section, so a reader who runs `/verify` after a Rust change can build the Rust binary, read a store, serve one request, and (where a display exists) run the window self-test, all isolated in a temp dir, with the same style as the Python sections.

**Out of scope**:
- Any change to `rust/**`, `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock`
- Any change to the existing Python sections of the recipe (add only; do not reword them)
- Installing anything on the machine; a CI job; packaging
- macOS and Windows steps (write one line saying they are not covered yet)

**Requirement Refs**: none (process tooling). Related: RS-006 (no display must not break CLI and MCP paths) in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] No Requirement Refs to cover (process tooling)

---

## Dependencies & Reachability

**Depends on**: T023 — the `app --self-test` command and the fixture folder `rust/crates/app/tests/fixtures/self-test` must exist (they do, commit 2dec625)
**Entry point**: `/verify` (the skill `verify`, file `skills/verify/SKILL.md`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | `skills/verify/SKILL.md` has a new `## Rust (easy-personal-rs)` section **after** the existing Python sections and before `## Probes worth repeating`; no existing line is changed or removed (`git diff` shows additions only) | add only |
| 2 | The section gives exact commands to: build the default binary; run `--version`; run `store-info` against a **temp copy** of `rust/parity/store-v11.sqlite` (never the owner's store); start `serve --static-dir` on the self-test fixture, make one authenticated request to `/api/version` with the token read from the printed line, and stop the server **by PID** (no `pkill -f` pattern); run `mcp` with an initialize, `tools/list` and one `tools/call usage_summary` over the temp store | read a store, serve one request |
| 3 | The section says the window step (`cargo build --features window` then `app --self-test --static-dir ...`) runs **only when `DISPLAY` or `WAYLAND_DISPLAY` is set** and the Tauri system packages are installed; otherwise it is skipped and the report says "window step skipped: no display" (never "pass") | window self-test where a display exists |
| 4 | The section says every command runs from `rust/`, that `. "$HOME/.cargo/env"` may be needed, and that the Bash tool runs zsh (wrap loops in `bash -c`), and has a Gotchas bullet for each surprise you meet while running the commands | same style as the Python sections |
| 5 | **Every command in the new section was run by you, as written, and the real output is pasted in your report** (default build, `--version`, `store-info`, `serve` + request + stop, `mcp` handshake + `usage_summary`, and the window step on this WSLg machine). A command that fails as written is fixed in the file, then re-run | the recipe works |
| 6 | After the recipe's server step, no `easy-personal-rs` process of yours is left (check by PID or `ps`, never `pkill -f` with a pattern that matches your own command line), and nothing was written inside the repo except `skills/verify/SKILL.md` and `tasks/TASK_REVIEW_T024.md` (`git status --short` pasted) | clean run |
| 7 | The new section names no path under `~` other than `$HOME/.cargo/env`, and no token value is printed in a pasted output (cut it) | privacy |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T024.md`.
> **Locked oracle**: none. The recipe is judged by running it.

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the new section, copied command by command into a fresh shell in `rust/` | each command works as written | Supervisor re-runs the section |
| 2 | `git diff skills/verify/SKILL.md` | additions only, in the right place | `git diff --stat` and a read |
| 3 | no display | the section tells the reader to skip the window step and how to say so | a read |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain; the window step also needs a display (WSLg)
git diff --stat -- skills/verify/SKILL.md && git diff -- skills/verify/SKILL.md | grep -c '^-[^-]'   # expect 0 removed lines
```

---

## Approach

**Pattern reference**: the existing sections of `skills/verify/SKILL.md` (a fenced shell block, then "Probes worth repeating" and "Gotchas").
**Vital slice**: write the section from the commands in `rust/README.md` and `tasks/TASK_REVIEW_T023.md`, run each, fix, paste.
**Cut list**: no new script, no helper file, no CI, no change to the Python sections.

Keep it short: the section should be about as long as the MCP stdio section plus a few bullets.

---

## Edge Case Checklist

- [ ] The recipe never points at `~/.easy-personal` (it uses a temp copy of the committed store)
- [ ] `serve` prints the token in its first lines: the recipe reads it into a variable and never echoes it
- [ ] The server is stopped by PID, and the PID is checked gone
- [ ] A machine with no `cargo` gets one line telling the reader to skip the section
- [ ] Stale references: `rust/README.md` or `rust/PROJECT_SPEC.md` mention of "verify has no Rust step" (none expected: say so)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `skills/verify/SKILL.md` | add the Rust section (additions only) |
| `tasks/TASK_REVIEW_T024.md` | Demonstration BEFORE (verbatim prior content of the skill file) only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/**`, `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the product; read-only for this task |
| the existing Python sections of `skills/verify/SKILL.md` | add only |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |

---

## Test Plan

Run each new command in order from `rust/` in a shell with a temp dir under the session scratchpad; paste real output (token cut). Then run the no-display variant of the window step (`env -u DISPLAY -u WAYLAND_DISPLAY`) only to show the skip wording is right, not to run the window.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Every command in the new section run, real output pasted
- [ ] `git diff` shows additions only; `git status --short` pasted
- [ ] Demonstration BEFORE filled in `tasks/TASK_REVIEW_T024.md` before the edit
- [ ] Supervisor notified: task ready for Stage 4 review
