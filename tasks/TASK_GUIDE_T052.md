# TASK_GUIDE — T052: Rust P3 waves 1-2 review fixes: crontab step overflow, DEL escape, secret word boundary, capped account reads and stale text
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the entries "Rust P3 planned" and "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP3.md` (this slice, the Edge Case Checklist and the decisions Q1-Q8) and `rust/parity/README.md` (the oracle, the goldens, the venv)
6. Read the Python you copy, **read-only**: `src/easy_personal/core/jobs.py` (`_field`, `next_cron`), `src/easy_personal/core/secretlike.py`, `src/easy_personal/core/reports.py` (`save`), and the two review reports summarised in the Acceptance Criteria below (the reviewers' words are data, verified by the Supervisor: the DEL escape and the unchecked `n += step` were reproduced)
7. Read the Rust you extend and copy the pattern of the wave-1 tasks already on `main`: `rust/crates/app/src/{api,mcp,server}.rs`, `rust/crates/core/src/{store,scope,reports,kit,suggestions}.rs`, `rust/crates/core/tests/{reports_parity,kit_parity}.rs`, `rust/crates/app/tests/reports.rs` (how a route or tool is registered, how errors and the token guard work, how a parity test replays goldens over the real binary), and `rust/parity/leak_rust.py` (it holds a hard registry of every `/api` route and MCP tool: a new one must be added to `API_CALLS` and `GUARD_CALLS` or the gate fails with "unregistered route")
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use the committed fixture store `rust/parity/store-rich/` and the golden files `rust/parity/golden-rich/` in temp copies, fixtures built in temp folders, and fakes on a scratch `PATH` with `env_clear`; never a real home, keychain, `claude` CLI, `systemctl` or crontab

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all, you can spawn parallel"; P3 of the Rust port, wave 2, `BRAINSTORMING_LOG_RUSTP3.md`.

**Restated intent**:
> The confirmed findings of the independent Stage 4 reviews of waves 1 and 2 are fixed, each with a test that fails before and passes after, and every stale doc line they named is corrected; nothing else changes.

**Out of scope**: any new route or tool; any behaviour change not listed; refactors; the Python app; any change to the Python app; `serve` or `mcp` opening the store for writing at start-up (they stay read-only unless this guide says a call opens a handle); a new dependency (none is allowed in this task: if one seems needed, stop and report); the routes and tools of other slices; schema changes.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`; the Python requirement ids named in the Python files above.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: wave 1 on `main` (T040, T042, T046)
**Entry point**: Standalone — N/A (fixes inside existing routes and tools)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **P1 `jobs.rs` crontab step overflow** (`field`, about lines 318-328, and `day.at(hour, minute as i8, ..)` about line 382): a line such as `1/9223372036854775807 * * * * claude` must not panic or wrap (debug panics on `n += step`; release wraps and a negative minute then panics inside jiff `Date::at`). Use `checked_add` (stop the loop on overflow) so the set is what Python's `range(first, last + 1, step)` gives; add that line, a step of `0` and `-1` (Python's behaviour: copy it), and a step larger than the range, to the jobs cases (extend `gen_dirs_jobs.py` and regenerate `golden-jobs/` once with Python; do not hand-edit goldens) and to a unit test. Red first: the new test panics before the fix. `/api/jobs` must answer 200 with the row for such a line | NFR-002 (no crash) |
| 2 | **P2 `reports.rs` `write_string`** (about line 446): `0x7f` (DEL) must be escaped as `\u007f` like Python's `json.dumps(ensure_ascii=True)` (change `> 0x7f` to `>= 0x7f`); add DEL cases (in the summary and in a fix text) to `gen_save_report.py` `cases.json` and regenerate `golden-save-report/` with Python; the `fixes_json` and `snapshot_json` bytes equal Python's for them; correct the README claim only if it stays untrue | parity |
| 3 | **P2 `secret.rs` `word()`**: Python's `\w` does not count Other_Alphabetic characters that are not letters or digits (circled and squared letters U+24B6-U+24E9, U+1F130-U+1F149, U+1F150-U+1F169, U+1F170-U+1F189, and combining marks such as U+0345, U+093F), `char::is_alphanumeric` does, so `\u24b6sk-abc` is refused by Python and **saved** by Rust (a missed secret). Make `word()` agree with Python's `\w` (`str.isalnum()` or `_`: Unicode categories L*, N* and not Other_Alphabetic-only characters); build the check from Python's own answer over every code point once (a generator `rust/parity/gen_word_table.py` writes the exact set of code points where Rust's `is_alphanumeric` and Python's `isalnum` differ into a committed JSON; the Rust test compares each; the table may be embedded as ranges if it is small). Add `\u24b6sk-abc`, `\u0345sk-abc` and a few more to `secret_corpus.json` (regenerate with Python). Correct the doc comment in `secret.rs` and README "Decided for you" so they state what is true afterwards | NFR-002 |
| 4 | **P2 `accounts.rs` `read_json`** (about lines 246-251): open the file once; `file.metadata()` on the handle (regular file only); read through `.take(max + 1)` and refuse when the result is larger than `max` (the pattern of `report_files.rs`); same observable answers as now for every existing golden/oracle case; add a test where the file is larger than the cap after a size check would have passed (use a FIFO-free approach: an oversize file and a file grown by a helper thread between open and read is enough) | NFR-002 |
| 5 | **Hardening, decided here** (state each in README "Decided for you"): (a) every git run in `metrics.rs` (`run_git_timed`) gets `.env("GIT_NO_LAZY_FETCH", "1")` so a partial-clone repository can never make `rev-list` fetch over the network (Python has the same gap: a deliberate Rust-only hardening; a test with a fake `git` proves the variable is set); (b) `reports.rs` `save` opens the write handle without `SQLITE_OPEN_CREATE` (the README says a missing store is never created; if `open_for_write` cannot do that, add the smallest parameter and a test where the store is removed between the read-only check and the write); (c) `leak_rust.py` `tree_digest` skips only `<plant root>/bin/argv.log`, not every file with that name | safety |
| 6 | **P3 `reports.rs` `fired_count`**: a snapshot where any `checked` entry lacks a boolean `fired` gives `fired_count` 0 for the whole report (or the README and module docs say "malformed entries are not counted"; choose the first and say which); test `{"checked":[{"fired":true},{}]}`. Also cover a `fixes_json` that is not JSON and a snapshot without `as_of_utc` for the fixed 500 path, and make `the_now_side_is_measured_on_the_request_for_the_saved_scope` independent of the system clock | quality |
| 7 | **Stale text** (each is a fact the commits changed): `rust/crates/app/src/mcp.rs` module header (tool list lacks `kit_summary`, `kit_findings`, `save_report`; says the tools only read), `rust/crates/app/src/api.rs` header (route list lacks accounts, sync, dirs, jobs, kit report files; says every request reads the store), `rust/README.md` line about "the routes so far" (about line 38), the `kit_abstain.rs` duration (waits for the 10 s limit once, not twice, about line 444), `jobs.rs` header ("a cleared environment plus LC_ALL=C": the route passes the inherited environment, as Python), the `report_files.rs` header claim that Python fails on non-UTF-8 names (the reviewer measured that `json.dumps` with `ensure_ascii` does not fail on surrogate-escaped names: run Python yourself over a temp folder with such a name and state what it does; keep Rust's skip as "decided for you" if it differs), and `parity/README.md` (the leak-gate venv needs `uv`: it uses `rust/parity/.venv`). Also document, without changing it, that `report-files` lists dot-files and multi-link files as `openable` while `report-file` answers 404 for them (decided: parity with Python's listing) | usable |
| 8 | **Not in this task** (recorded for later, do not fix): the quadratic `TimersMonotonic` scan in `schedule()` on 8 MiB of crafted `systemctl` text (user's own unit files; check whether Python's regex is equally quadratic and say so in the report), a reader thread that stays blocked when a killed child's grandchild keeps stdout open, replacing the `/proc/self/fd` technique by `libc::openat` (needs a Cargo.toml change: owner/Supervisor decision), the per-request 10 MiB buffer of `report-file` | scope |
| 9 | **No regression**: three `TZ` runs (`cargo test --workspace --no-fail-fast`: unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago`), clippy and fmt (also `--features window`), `leak_rust.py` prints `0 hits`, `git status -- src tests web` empty, `Cargo.toml`/`Cargo.lock` unchanged, every existing golden and oracle still passes (`gen_*.py --check` where it exists) | quality |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T052.md`.
> **Locked oracle**: Python itself: regenerate only the new cases with the committed generators; existing goldens must not change.

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
**Order**: P1 first, then the two parity items, then the rest; commit nothing (leave the worktree dirty).

---

## Edge Case Checklist

- [ ] Every parameter combination in the oracle, plus the 400/403/404/405/413 cases Python has (copy Python's status and `error` text, except CPython-specific text: then compare status and the `error` key only, as `manifest.json` `compare_notes` does); error bodies carry no path, secret or caller text
- [ ] Each fix has a test that is red on the pre-fix code: show it (run the new test, paste the failure, then the fix)
- [ ] Regenerating a golden changes only the new cases: show `git diff --stat` of the golden folders and confirm no existing case changed
- [ ] Run `cargo test` from the main-checkout path logic: `workspaces_parity::edge_store_python_values` fails inside a worktree under `.claude/` (known): say so if you see it
- [ ] The /tmp volume is short of inodes: keep temp use small, do not leave temp folders behind, and do not run the full suite more than the three required times
- [ ] The store is opened read-only (unless this guide names a write); a store that is not v11 is refused as for the other routes (exit 2 / 503)
- [ ] Time derived from "now" uses the injected clock (`clock.rs`); run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] `leak_rust.py` registry updated for every new route or tool (`API_CALLS`, `GUARD_CALLS`, the MCP calls) and `tests/mcp.rs` exact tool lists updated; keep each change a few lines (other tasks edit these files at the same time and the Supervisor merges them)
- [ ] Stale references: `rust/README.md` "not served" lists; `docs/release/rust-v0.2.0.md` "not in this release" (report, do not edit the release doc)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/{jobs,reports,secret,accounts,metrics,report_files,writer,store}.rs` as needed (minimal edits) | the fixes |
| `rust/crates/core/tests/*`, `rust/crates/app/tests/*` | red-first tests |
| `rust/parity/gen_dirs_jobs.py`, `gen_save_report.py`, `gen_word_table.py` (new), the matching `golden-*/` folders (new cases only), `leak_rust.py`, `parity/README.md` | oracle cases and the digest narrowing |
| `rust/crates/app/src/{api,mcp}.rs` (header comments only) | stale text |
| `rust/crates/app/src/api.rs` and/or `mcp.rs`, `rust/crates/app/tests/mcp.rs`, `rust/parity/leak_rust.py` | register the route(s) or tool(s): a few lines each |
| `rust/README.md` | routes/tools, parameters, error table, the "decided for you" list, corrected "not served" lists |
| `tasks/TASK_REVIEW_T052.md` | Demonstration BEFORE only |

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

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T052.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review
