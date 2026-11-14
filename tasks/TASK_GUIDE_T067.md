# TASK_GUIDE — T067: Close the 7 T061 parity findings (decide, document, align `/api/live` charset) and stop the runner leaking on SIGTERM
**Date**: 2026-11-14
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: qa-expert
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/qa.md`
5. Read `rust/docs/PARITY_T061.md` (sections "Findings", "Known limits"), `tasks/TASK_GUIDE_T061.md`, and `rust/parity/run_parity.py` (README rules section near "README differences", the probe expectation table near line 1477, the findings text near line 1600, the `finally` cleanup)
6. Read the "Decided for you" lists in `rust/README.md` for T047, T048, T051, T056, T058
7. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder; fakes and temp store copies only, as T061

---

## Requirement

**Original request** (owner, 2026-11-14): handoff items 2 and 6 ("the 7 parity findings … one small task that documents or aligns them, before T063"; "runner can leak a server when killed hard"); then "continue your work as parallel [admin]".

**Restated intent**:
> After this task, `run_parity.py` reports **differs new 0**: each of the 7 findings is either aligned in Rust (then `equal`) or named by a new numbered README item that the runner's rule checks (then `differs: README …`). The Supervisor decided each one for the owner (`[admin]`, ⚠ decided for you — mark each README line so):
>
> | Finding | Decision |
> |---|---|
> | `cli: --version` | keep Rust's `easy-personal-rs <version>`; README item |
> | `cli: mcp` (http Ctrl-C exit 0, empty stdout) | keep Rust; README item in the T051 list |
> | `cli: usage` (`--by week`, `--since banana` texts) | keep Rust's one-line texts; README item in the T058 list |
> | `mcp prompt: investigate` (unknown prompt `-32602` vs `0`) | keep Rust (`-32602` is JSON-RPC "invalid params"); README item in the T047 list |
> | `mcp protocol: initialize` (capabilities, unknown method `-32601`, no stderr request log) | keep Rust (Rust serves no resources; `-32601` is JSON-RPC "method not found"); README item(s) in the T047 list |
> | `route: GET /api/live` charset | **align**: Rust sends `text/event-stream; charset=utf-8`; update the T056 README text that states Rust's value |
> | `route: GET /api/workspaces` 404 vs 405 with page | keep Rust's 405; README item in the T048 list |
>
> And: a SIGTERM to the runner (and SIGHUP) runs the same cleanup as its `finally` blocks: every server it started is killed and the world folder is removed.

**Out of scope**: any other Rust behaviour change; the other "Known limits"; T062/T063.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (handoff list and "[admin]", 2026-11-14)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist (RS-001..RS-003 `rust/docs/SPEC.md`; check, fix if wrong)

---

## Dependencies & Reachability
**Depends on**: T061 (merged)
**Entry point**: `rust/parity/run_parity.py` (`--check`), `GET /api/live` in `rust/crates/app/src/` (find where the SSE content type is set)

## Dependency rule
No new crate or Python dependency.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | Rust `GET /api/live` content type is `text/event-stream; charset=utf-8`; an existing or new Rust test asserts it; the live oracle/tests still pass | charset row |
| 2 | Six new numbered README items exist, each marked "⚠ decided for you", and each of the six findings has a runner rule naming its item; the regenerated `rust/docs/PARITY_T061.md` shows `differs new 0`, `not run 0` | other rows |
| 3 | `run_parity.py --check` passes from the T067 worktree, run in foreground and in background (`&`), as the T061 lesson requires | all |
| 4 | Test: start the runner, send SIGTERM while a server is up; afterwards no child process of that run is alive and the world folder is gone. Same for SIGHUP. Remove the SIGTERM line from "Known limits" | SIGTERM |
| 5 | Rust suite, clippy, fmt pass; numbers pasted | all |

---

## Evaluation & Acceptance

### Verification Command
```bash
cd rust && ./parity/.venv/bin/python parity/run_parity.py --check && cargo test --workspace 2>&1 | tail -5
```
Note: the venv lives in the main checkout; from the worktree use `/home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp/rust/parity/.venv/bin/python` and `env -u CLAUDE_CONFIG_DIR` for the leak script (T061 lesson).

### Evidence (filled by reviewer at Stage 4/5)
| Item | Result | Evidence |
|---|---|---|
| New test(s) cover acceptance criteria | ☐ | |
| `--check` passes fg and bg | ☐ | |
| Rust suite passes | ☐ | |
| code-review | ☐ | |
| UI: visual regression | ☐ N/A | no UI |
| UI: design-system compliance | ☐ N/A | no UI |
| UI: responsiveness | ☐ N/A | no UI |

## Demonstration
See `tasks/TASK_REVIEW_T067.md`. BEFORE: counts line of the current report and a SIGTERM run leaving a process/folder behind.

## UI / Design Acceptance Criteria
N/A — no UI.

---

## Approach
Charset first (test-first in Rust). Then one README item + one runner rule per finding, following how T061 maps existing items (`readme_ok`, rule transformations, the probe expectation table, the findings text — drop a finding's text once it is no longer new). Regenerate the report with the runner. Then the signal handler: turn SIGTERM/SIGHUP into the same path as KeyboardInterrupt/`finally` (e.g. a handler that raises), with a test.

## Edge Case Checklist
- A rule must make the two outputs equal only for the named difference, not hide any other (mutate: change one other byte and see it go `new`)
- SIGTERM during server start-up, and during the store copy

## Files to Change (Predicted)
`rust/parity/run_parity.py`, its test, `rust/docs/PARITY_T061.md` (regenerated), `rust/README.md`, the Rust file that sets the SSE header and its test.

## Files Must NOT Touch
`src/easy_personal/**`, `rust/parity/gen_*.py`, any other Rust behaviour.

## Test Plan
Rust test for the header; Python test for the signal cleanup; `--check` for the classification.

## Completion Checklist
- [ ] AC 1-5 with pasted output
- [ ] Commits on branch `T067` only; final report with `Status:`
