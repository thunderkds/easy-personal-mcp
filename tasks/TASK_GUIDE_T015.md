# TASK_GUIDE — T015: `warnings` and `improvement_plan` tools in the MCP server
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T008, T009, T010, T013)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/crates/app/src/mcp.rs` and `rust/crates/app/tests/mcp.rs`
6. Read the Python tools: `src/easy_personal/surfaces/mcp.py` (`warnings`, `improvement_plan`)
7. Read the oracle: `rust/parity/golden/mcp__tools_list.json` (the `warnings` and `improvement_plan` entries: description, `inputSchema`, `outputSchema`), `mcp__warnings__all.json`, `mcp__improvement_plan__all.json`, the rich ones in `golden-rich/`, and the manifests
8. Do not open `~/.easy-personal` or any `~/.claude*` folder, not even to check that it exists. Tests use `EASY_PERSONAL_HOME` pointing at a temp copy of a parity store

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10: "start the P1 wiring slice"): serve the ported read functions over MCP too.

**Restated intent**:
> The Rust `mcp` server lists and runs `warnings` and `improvement_plan` with the same names, descriptions, schemas and results as the Python server, next to the three tools it has.

**Out of scope**: the other seven tools; the `workspace` and `account` filters (criterion 4); the `investigate` prompt; HTTP routes (T014).

**Requirement Refs**: RS-003 (parity), NFR-002 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T008 (`mcp.rs`), T006 (rules), T010 (plan), T013 (`store::open_checked`, `clock::now_micros`)
**Entry point**: `easy-personal-rs mcp` (`tools/list` and `tools/call`)
**Parallel task**: T014 edits `rust/crates/app/src/server.rs` (and maybe `api.rs`), `tests/server.rs` and the app `Cargo.toml` dev-dependencies. Do not edit those. You own `mcp.rs` and `tests/mcp.rs`. Re-read `Cargo.toml` and `Cargo.lock` before editing; `cargo` may print "waiting for file lock".

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `tools/list` returns five tools; for each of the five, `name`, `description`, `inputSchema` and `outputSchema` equal the corresponding entry of `mcp__tools_list.json` (the test already does this for three) | RS-003 |
| 2 | `tools/call` for `warnings` and `improvement_plan` returns `structuredContent` and the parsed `content[0].text` equal to the goldens `mcp__warnings__all` and `mcp__improvement_plan__all` over the default store, and to the rich goldens over `rust/parity/store-rich/store-v11.sqlite`. Because both depend on the current time, the comparison is made **in-process**: `run_tool` takes the clock (UTC microseconds) as a parameter, `call_tool` passes `clock::now_micros()`, and a `#[cfg(test)]` unit test (or a test through the library part of the crate) passes the manifest's `frozen_now`. There is **no** environment variable or flag in the binary that changes the clock | RS-003 |
| 3 | Spawned-binary tests (clock-independent): both tools answer with a result that has the expected top-level keys (`hits` and `abstained`; `groups`) and `isError` false over the fixture store; the existing 15 tests pass unchanged except where they count tools (state each change) | Intent |
| 4 | `workspace` and `account` are accepted by the schema (nullable strings, as in the golden). When either is a non-empty string the tool answers a tool error `Error executing tool <name>: the workspace and account filters are not available in this build yet`; `null`, absent and empty string count as no filter. A wrong type is a tool error like the existing ones | Intent |
| 5 | Store states use `open_checked` as the other tools: no store, a newer store and a directory in its place are tool errors with no path in the text | RS-002, NFR-002 |
| 6 | The line transport, `-32700` handling, the unknown-tool error and the protocol-version limit are unchanged; the whole `rmcp` use stays in `mcp.rs` | Intent |
| 7 | The fixture bytes are unchanged after the tests (no `-wal`/`-shm`); `git status --short -- src web tests pyproject.toml uv.lock` is empty before and after; `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass | Intent |
| 8 | `rust/README.md`: the `mcp` paragraph lists five tools and the "not ported yet" sentence is corrected | stale-reference sweep |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the Python output in the goldens named above. The implementing agent must not edit `rust/parity/**`.
> **Locked oracle**: rust/parity/golden/mcp__tools_list.json, rust/parity/golden/mcp__warnings__all.json, rust/parity/golden/mcp__improvement_plan__all.json, rust/parity/golden-rich/mcp__warnings__all.json, rust/parity/golden-rich/mcp__improvement_plan__all.json

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | `tools/list` | five tools, each equal to the golden entry | spawn test |
| 2 | `run_tool` with the frozen clock | equals the four goldens | unit test |
| 3 | `workspace: "acme"` | the tool error of criterion 4 | spawn test |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: the three existing tools in `mcp.rs` (`tools()`, `run_tool`, `with_store`) and their tests.
**Vital slice**: the two tool definitions and `tools/list` equality first; then `run_tool` with the clock; then the in-process goldens; then the filter errors.
**Cut list**: no generated schemas (keep them hand-written like the others); no new dependency; no change to the transport.

---

## Edge Case Checklist

- [ ] Integers stay integers and floats stay floats; `structuredContent` equals the text body as parsed JSON
- [ ] Both tools answer an empty store (no hits) with an empty `hits` / `groups` list as Python does
- [ ] No absolute path, row text or secret in any result or error (NFR-002)
- [ ] The stdout channel carries only protocol lines; stderr stays empty
- [ ] Tests never open `~/.easy-personal`
- [ ] Stale references: `rust/README.md`; the module comment at the top of `mcp.rs` that says three tools

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/app/src/mcp.rs` | two tools, a clock parameter, the filter check, unit tests |
| `rust/crates/app/tests/mcp.rs` | the counts and new cases |
| `rust/README.md` | the `mcp` paragraph |
| Pre-existing pin tests | `tests/mcp.rs` assertions that say three tools: list each change in the report |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/crates/app/src/server.rs`, `rust/crates/app/tests/server.rs`, `rust/crates/app/src/api.rs` | T014 owns them |
| `rust/crates/core/**` | the core is done; if it lacks something, stop and report |
| `rust/parity/**` | the oracle |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Red first: extend the `tools/list` test to five tools and add the in-process golden tests (they fail while the tools are missing). Implement, then add the filter and store-state cases. Mutation: change a fixture value in a temp copy and see the golden comparison fail. Run the binary by hand once and paste the transcript.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T015.md`
- [ ] By-hand transcript pasted
- [ ] Supervisor notified: task ready for Stage 4 review
