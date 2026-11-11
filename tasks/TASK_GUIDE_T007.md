# TASK_GUIDE — T007: A richer parity oracle
**Date**: 2026-11-10
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (all of them: T003 QA, T006, T009)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/parity/README.md`, `rust/parity/gen_golden.py` (all of it, incl. the `--tz` mode from T009), `rust/parity/golden/manifest.json`, and the QA findings in `tasks/TASK_REVIEW_T003.md`
6. Read the Python code that produces the data: `src/easy_personal/core/rules.py`, `core/metrics.py`, `core/kit.py`, `core/workspaces.py` (or the nearest module), `core/reports.py`, `core/accounts.py`, `surfaces/api.py` (routes and their parameters), and the Python tests under `tests/` that already build such data (`tests/surfaces/test_api.py`, scope tests) as the pattern for calling Python's own write functions
7. Read the Rust tests you will mirror: `rust/crates/core/tests/usage_parity.rs`, `warnings_parity.rs`, `usage_tz_parity.rs`

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): run all current features in Rust; parity with Python before a surface flips.

**Restated intent**:
> The T003 goldens come from one day of data, so they prove the shape of most answers and not their logic: warnings and plan only abstain; outliers, kit, board, accounts, workspaces and reports are empty. Add a second, invented, **richer** store and goldens, plus goldens for the filter parameters, so a wrong Rust port of those paths can no longer pass.

**Out of scope**: porting any new function to Rust; editing `rules.rs`, `metrics.rs` or any `src/` file; the generator hardening list (T011); regenerating or changing `rust/parity/golden/**` or `rust/parity/store-v11.sqlite`.

**Requirement Refs**: RS-003 (parity) in `rust/docs/SPEC.md`; QA findings P1 in `tasks/TASK_REVIEW_T003.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T003 (generator and goldens), T006 and T009 (the Rust slices the new goldens will test)
**Entry point**: `python rust/parity/gen_golden.py --rich` (opt-in mode, like `--tz`) and the new test `rust/crates/core/tests/rich_parity.rs`
**Parallel task**: T012 refactors `rust/crates/core/src/**` and `usage_parity.rs`. You must not edit those. You own `rust/parity/**` and `rust/crates/core/tests/rich_parity.rs`. If a Rust slice does not match a rich golden, **do not fix `src/`**: first check whether the golden or the port is wrong (re-run Python over the same store). If the port is wrong, keep that comparison as its own test marked `#[ignore = "<which function or rule differs>"]` so the suite stays green, and list it in your report as a finding for a later task. If you cannot tell, stop and report.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | `gen_golden.py --rich` builds an invented store `rust/parity/store-rich/store-v11.sqlite` (v11) from invented transcripts committed under `rust/parity/fixtures-rich/` (all text fictional; no real path, name or key), by calling Python's own ingest and write functions in an isolated home (the same guards as the default mode: refuse a real home, `~/.easy-personal`, `~/.claude*`, and patch the keychain, `claude` CLI and systemd). The default run (no flag) stays byte-identical to the committed `golden/` and store (`diff -r`, `cmp`) | RS-003 |
| 2 | The rich store has, at least: more than 14 days of history in the window that `rules.rs` reads (so the 7-day abstentions turn into measurements); sessions that make **each of the 7 rules fire in some entry** (`cache_hit_low`, `session_huge_no_compact`, `tokens_jump`, `project_dominates`, `tool_errors_high`, `context_index_stale` if reachable without a network, `records_unreadable`) — and at least one entry per rule where it measures and does not fire; at least one session over 170,000 context tokens with no compaction; tool errors; at least 3 models of which 2 are in `prices.yaml` (costed) and 1 is not (abstained); a non-empty outliers list and a timeline with at least one gap; two accounts (a `.claude` and a `.claude-work` agent root, ingested the way the real CLI does, through `ingest_accounts`); at least 2 workspaces with sessions assigned and 1 unassigned; at least 1 saved report; and at least 1 kit project with findings, board rows and a memory graph. Say in the README which of the rules cannot be made to fire in isolation and why | RS-003 |
| 3 | Goldens in `rust/parity/golden-rich/` (with a manifest listing versions, the frozen clock, the TZ, the fixture hash) for every route and MCP tool the default mode covers, over the rich store, **plus** the filter parameters: `workspace` and `account` on usage, warnings, plan, kit and board (API and the matching MCP tool arguments), and the report-folder parameters `repo_no` and `folder`. Every GET route or MCP tool that is neither golden nor skipped fails the run loudly, as in the default mode | RS-003 |
| 4 | **Key order**: the manifest of `golden-rich` records for each entry `raw_sha256` of the raw response body. The existing `golden/manifest.json` is **not** changed. State in `rust/parity/README.md` which rule wins (sorted keys for the golden files; `raw_sha256` for the order the app emitted) | T003 QA P1 |
| 5 | The `bad_since` style entries keep only status and the `error` key in the comparison notes (the message is CPython-specific); the manifest says so | T003 QA P2 |
| 6 | Output is deterministic: two runs give a byte-identical `golden-rich/` and store (`diff -r`, `cmp`), also under `PYTHONHASHSEED=1..3`, `LC_ALL=C` and `TZ=America/New_York` for the generator process | RS-003 |
| 7 | Rust tests `rust/crates/core/tests/rich_parity.rs` compare, as parsed JSON, the slices that exist: `usage_summary` and `usage_breakdown` (all `by`, with and without a window) and `rules::evaluate` against the matching rich goldens. The `workspace` / `account` entries are not ported in Rust: list them in the report as not testable yet. A deliberate change to one fixture value in a temp copy makes a comparison fail (mutation check) | RS-003 |
| 8 | Leak check: `grep -rE '/home/|/Users/|sk-|ghp_'` over `golden-rich/`, `fixtures-rich/` and the manifest finds nothing the generator did not already allow; the generator fails the run on a match like the default mode. The default mode's safety checks (real-home refusal) are unchanged and re-run | NFR-002, NFR-003 |
| 9 | `git status --short -- src web tests pyproject.toml uv.lock` is empty before and after; `rust/parity/golden/**`, `rust/parity/golden-tz/**` and `store-v11.sqlite` are byte-identical before and after; `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass; `ruff check rust/parity` passes | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is Python's own output. You must not edit `rust/parity/golden/**`, `rust/parity/golden-tz/**` or `rust/parity/store-v11.sqlite`.
> **Locked oracle**: rust/parity/golden/manifest.json, rust/parity/golden-tz/manifest.json, rust/parity/store-v11.sqlite

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | a default generator run | `diff -r` against `rust/parity/golden` empty; `cmp` of the store identical | command |
| 2 | `--rich` run twice | identical output | command |
| 3 | the rich warnings golden | at least one hit per rule (see criterion 2), not only abstentions | a test or script that counts them |
| 4 | the Rust core over the rich store | usage and warnings equal the goldens, or each difference is reported with the rule or function that differs | `cargo test` |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain and the parity venv on the owner's machine
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: the `--tz` mode in `rust/parity/gen_golden.py` (opt-in, own folder, own manifest, temp copy) and the scope tests in `tests/` that build workspaces and accounts with Python's own functions.
**Vital slice**: first the rich store and the warnings golden with every rule firing (it is the hardest and the most valuable); then usage and outliers; then the filter parameters; then kit, board, reports, accounts; then `raw_sha256` and the Rust test.
**Cut list**: no new Python dependency; no change to the default mode; no change to `src/`; no real data; no network.

Fixtures are plain `.jsonl` transcript files under `rust/parity/fixtures-rich/`, written once by a small committed script or by hand, with fixed timestamps relative to the frozen clock 2026-01-20T00:00:00Z. Keep the store small (a few hundred events at most).

---

## Edge Case Checklist

- [ ] Each rule at its boundary as well as clearly over it (the 36 synthetic cases in `warnings_parity.rs` show the boundaries Python has)
- [ ] A session whose records straddle the window start
- [ ] Two accounts with the same project folder
- [ ] A workspace with a gone folder and one with a live git repo (the generator makes the repo in a temp dir; the golden must not depend on it existing on another machine — use the abstain or gone path, and say which)
- [ ] The frozen clock still applies; the TZ is UTC for this oracle
- [ ] No absolute path, row text or secret in any golden (NFR-002)
- [ ] Tests never open `~/.easy-personal`
- [ ] Stale references: `rust/parity/README.md` mentions of "one day of data", `rust/README.md` if it lists the oracle

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/parity/gen_golden.py` | opt-in `--rich` mode (new functions; `main()` gets a few lines) |
| `rust/parity/README.md` | the new mode, the key-order rule, which rules cannot fire in isolation |
| `rust/parity/fixtures-rich/**` | new, invented transcripts |
| `rust/parity/store-rich/store-v11.sqlite` | new |
| `rust/parity/golden-rich/**` | new goldens and manifest |
| `rust/crates/core/tests/rich_parity.rs` | new |
| Pre-existing pin tests | None — all current suites stay green and unchanged |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/parity/golden/**`, `rust/parity/golden-tz/**`, `rust/parity/store-v11.sqlite` | committed oracles |
| `rust/crates/core/src/**`, `rust/crates/core/tests/usage_parity.rs`, `warnings_parity.rs`, `usage_tz_parity.rs` | T012 owns the source; the tests are pinned |
| `rust/crates/app/**` | not part of this task |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Build the generator mode, run it, inspect the warnings golden for one hit per rule, run it again for determinism, then write `rich_parity.rs` (red against any real difference, with the difference reported). Finally mutate a fixture value in a temp copy and confirm a comparison fails; run the default generator and `diff -r`.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T007.md`
- [ ] Ran the determinism and mutation checks
- [ ] Default generator run is byte-identical to the committed goldens
- [ ] Supervisor notified: task ready for Stage 4 review
