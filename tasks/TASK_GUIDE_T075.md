# TASK_GUIDE — T075: Watch 4: effectiveness on read — tokens per good result, tokens per task done; API, CLI, MCP tool
**Date**: 2026-11-15
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_WATCH.md` (decisions D1-D7, Option A, Edge Case Checklist) and the PRD rows FR-098, FR-061
6. Read `rust/crates/core/src/kitperf.rs` (good result), `metrics.rs`, `sessions.rs`, `prices.rs`, `rust/crates/app/src/mcp.rs` (tool list, read-only rule), and how `git_local` reads local git history
- Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder (use fixtures and temp folders)

---

## Requirement

**Original request** (owner, 2026-11-14): a background service that watches and collects state, with a tray icon and desktop alerts. Path A (compute-on-read, no store migration) chosen 2026-11-14; task breakdown approved 2026-11-15 ("Approve as listed").

**Restated intent**:
> 1. A core query that, for a scope (workspace / account / period, as other usage tools), returns per *task*: task id, branch, tokens (in, out, cache read, cache write), API-equivalent cost (or "unknown price"), sessions, good results, tokens per good result, done time and tokens per task done; plus one "unattributed" line and totals.
> 2. *Task* = branch whose name holds a task id (`T\d+`, `HUN-\d+`, case as written). Its tokens = events on that branch + sidechain events of a session whose main thread is on that branch. `main`, `HEAD`, `worktree-agent-*`, `fix-*` and other id-less branches, and sidechains of their sessions, go to "unattributed".
> 3. *Done* = the first merge commit of that branch into the main branch, read from local git (`git log --merges --first-parent <main>` subjects `Merge branch '<branch>'`). No merge found = not done (tokens still shown, tokens per task done blank). Decision ⚠ made for the owner: first merge, not last.
> 4. When events were pruned (`--retention-days`), state the oldest stored day.
> 5. Surfaces: `GET /api/effectiveness`, CLI `effectiveness` (+ `--json`), MCP tool `effectiveness` (read-only; add to the tool list and every test that counts tools). Computed on read, nothing stored.

**Out of scope**: storing a ledger, MINIMAL_LOG mapping of `fix-*` branches (cut list), UI (T077), Python app.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (breakdown approved 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-098, FR-061 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: none
**Entry point**: new `rust/crates/core/src/effectiveness.rs`, `api.rs`, `cli_cmds.rs`, `mcp.rs`

## Dependency rule
No new crate.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Attribution tests on fixture events: branch event, sidechain of a branch session, sidechain of a `main` session (unattributed), `worktree-agent-x`, `fix-foo` | intent |
| 2 | Done-time tests on a temp git repo: merged once, merged twice (first wins), squash/rebase (no merge commit → not done) | intent |
| 3 | Number tests: tokens per good result with 0 good results = blank, unknown price | intent |
| 4 | Retention: pruned store → oldest day reported | intent |
| 5 | API, CLI and MCP tests; tool-count tests updated; leak test passes | intent |
| 6 | Real run of `effectiveness` on this repo pasted (T066-T071 show as tasks) | intent |
| 7 | `security-review` at Stage 4 (Risk Medium) | intent |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && cargo test --workspace 2>&1 | tail -5 && cargo run -q -p easy-personal-rs -- effectiveness
```

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| Verification command | ☐ | |
| code-review | ☐ | |
| security-review | ☐ | |
| UI: visual regression | ☐ N/A | backend only |
| UI: design-system compliance | ☐ N/A | backend only |
| UI: responsiveness | ☐ N/A | backend only |

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_T075.md`.

## UI / Design Acceptance Criteria
N/A — backend only.

---

## Approach
Test first, one vertical slice at a time (`tdd`). Smallest code that meets the ACs; record cuts.

## Edge Case Checklist
- Branch renamed after work
- Two task ids in one branch name → first one
- Repo not a git folder → done time unknown for all, say so
- Several repos: match by the event's project path

## Files to Change (Predicted)
`rust/crates/core/src/effectiveness.rs`, `lib.rs`, `rust/crates/app/src/api.rs`, `cli_cmds.rs`, `main.rs`, `mcp.rs`, tests, MCP tool lists in README/tests

## Files Must NOT Touch
store schema, `ingest/**`, `src/easy_personal/**`, `web/**`

## Test Plan
The ACs above, each as an automated test, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_T075.md`
- [ ] Commits on branch `T075` only; final report with `Status:`
