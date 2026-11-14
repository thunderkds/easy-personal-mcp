# TASK_GUIDE — T061: Retire Python 4 — parity run of every Python command, route and MCP tool against Rust
**Date**: 2026-11-14
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P3
**Assigned agent**: qa-expert
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/qa.md`
5. Read `rust/docs/PYTHON_RETIREMENT_PLAN.md` (T061 row, "Goldens" risk) and `rust/README.md` (the "Decided for you" lists of T048..T060: every difference already decided is named there)
6. Read the Python, **read-only**: `src/easy_personal/surfaces/cli.py` (every subcommand), `src/easy_personal/surfaces/api.py` (every route), the MCP server module (every tool and the prompt), and how `rust/parity/gen_*.py` run Python (they already know the fixtures, the fake keychain and the fake `claude`)
7. Read the Rust that serves the same surfaces: `rust/crates/app/src/{main,cli_cmds,connector_cmd,doctor_cmd,api,mcp}.rs`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder. Use temp copies of `rust/parity/store-rich` and the committed fixtures only. No real keychain item, no real `claude`, no real account folder: fakes only (as T050, T060)

---

## Requirement

**Original request** (owner, 2026-11-13): "approve all" and "auto approve to finish" for `rust/docs/PYTHON_RETIREMENT_PLAN.md`; 2026-11-14 "let start all parallel".

**Restated intent**:
> Before the Python app can be removed (T063), one run proves, surface by surface, that Rust does what Python does on the same store. The run lists **every** Python CLI subcommand, every API route and every MCP tool and prompt; runs it on Python and on Rust against copies of the same store; compares stdout, exit code and JSON body; and writes one table of results. Each difference is either **named in a README "Decided for you" list** (then the table cites the item) or it is **new** (then it is reported as a finding). No Rust or Python app code is changed by this task.

**Out of scope**: fixing any difference (a finding becomes its own task); macOS and Windows; the window app; a performance comparison; changing any `gen_*.py` oracle that already exists.

**Requirement Refs**: RS-001..RS-003 in `rust/docs/SPEC.md`; NFR-002 (`PRD.md`). Check the numbers; fix this line if one is wrong.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the owner ("approve all", "auto approve to finish", 2026-11-13; "let start all parallel", 2026-11-14)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T050, T058, T059, T060 (all merged on `main`)
**Entry point**: `rust/parity/run_parity.py` (new; runs both apps and writes the report) and its test `rust/crates/app/tests/parity_report.rs` or `rust/parity/test_parity_report.py` (new)

---

## Dependency rule

No Rust crate, feature or lockfile change. A Python dependency already in `rust/parity/.venv` may be used; adding one needs the Supervisor.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Complete inventory.** A script reads Python's argparse tree, its API route table and its MCP tool list and writes `rust/docs/PARITY_T061.md` with one row per CLI subcommand (and each connector/workspace/account action), per route (method + path), per MCP tool and per prompt. A test fails if Python gains a surface that the table lacks | RS-001 |
| 2 | **Same inputs.** Each row is run on Python and on Rust over fresh copies of the same store (`store-rich`), the same agent folder, the same clock (`TZ` and the frozen-time hooks the existing generators use), the fake keychain and the fake `claude`. No real data | RS-001, NFR-002 |
| 3 | **Compared.** CLI: stdout, stderr text for the user, exit code. Route: status, content type, JSON body (key order ignored only where the README already says so). MCP: the `tools/list` schema and the result of each tool call. Volatile fields (tokens, ports, paths of the temp folder, timestamps) are normalised by a stated rule, not by dropping the row | RS-001 |
| 4 | **Every difference classified.** Each row ends as `equal`, `differs: README <list> item <n>` (the item must exist and describe it), `differs: new` (a finding with a reproduction command) or `not run: <reason>` (only for a surface that cannot run offline, e.g. needs the real Secret Service). `differs: new` and `not run` are counted at the top of the report | RS-001 |
| 5 | **Findings are reported, not fixed.** For every `differs: new`, the report gives the exact command, both outputs (secrets and tokens cut) and a one-line proposed task (C, Risk). No `src/**` or `rust/crates/**` file changes | Surgical Changes |
| 6 | **No leak.** The report and every captured output are scanned with the repo canaries (`rust/parity/leak_rust.py` scanner, `Canaries.find`): 0 hits. Tokens and temp paths are not pasted | NFR-002 |
| 7 | **Repeatable.** `rust/parity/run_parity.py` runs from a clean checkout with the commands in the guide (one command), twice in a row with the same table. It cleans up its temp folders | RS-003 |

---

## Evaluation & Acceptance

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T061.md`.

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain and the parity venv
. "$HOME/.cargo/env" && cd rust && cargo build && env -u CLAUDE_CONFIG_DIR ./parity/.venv/bin/python parity/run_parity.py && ./parity/.venv/bin/python parity/test_parity_report.py && cargo test --workspace --no-fail-fast
```

**Reviewer note**: Medium risk; QA task, no app code. Stage 4 checks that the inventory is complete (spot-check five surfaces by hand), that each `differs: README item n` really matches that item, and that no `differs: new` was quietly turned into `equal` by normalising too much.

---

## Approach

**Vital slice**: first the CLI inventory and compare (fast, deterministic), then the API routes (start `dashboard` and `serve` on copies, same requests), then MCP tools. Reuse the existing generators' fixtures and fakes; do not copy them.
**Cut list**: no performance numbers, no fuzzing, no second store.
**Order**: red test first (the completeness test fails on an empty table), then the runner.

---

## Edge Case Checklist

- [ ] A surface that writes (workspace edit, connector add): run on a fresh copy each time, compare the store rows after, not only stdout
- [ ] A surface that needs a terminal prompt: stdin from a file or `/dev/null`, as T060's oracle does
- [ ] A route that needs the token or a Host header: same headers on both
- [ ] Two writers: never run Python and Rust at the same time on one store
- [ ] Time uses the frozen clock; three `TZ` values for the CLI rows that print times

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/parity/run_parity.py`, `rust/parity/test_parity_report.py` (new) | runner, completeness test |
| `rust/docs/PARITY_T061.md` (new) | the report |
| `tasks/TASK_REVIEW_T061.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/crates/**`, `rust/Cargo.toml`, `rust/Cargo.lock` | no app change: findings become tasks |
| existing `rust/parity/gen_*.py`, `rust/parity/golden*/**`, `store-rich/**`, `fixtures-rich/**` | the oracle |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**`, `rust/README.md` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*`, any real keychain item | the owner's real data |

---

## Test Plan

The completeness test (AC1) first, red on an empty table. Then the runner. Run it twice and diff the two reports (AC7). Leak scan of the report (AC6).

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T061.md` before the first file is created
- [ ] Runner and completeness test done, both pasted green
- [ ] Report counts (equal / differs README / differs new / not run) at the top, each `differs: new` with a reproduction
- [ ] Leak scan 0 hits, second run identical
- [ ] Supervisor notified: task ready for Stage 4 review
