# TASK_GUIDE — T033: Rust ingest of `kit_trace`, `ui` and `verifier` (P2b), table-for-table equal to Python
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing anything:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the 2026-11-11 entry "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP2.md` (Edge Case Checklist, the line "Replace-all sources (P2b)") and `rust/parity/README.md` (the oracle, its venv, how the T026 goldens are made)
6. Read the Python you copy, **read-only**: `src/easy_personal/sources/{kit_trace,ui,verifier}.py`, the `_label`, `_utc`, `read_complete_lines` they import from `claude_code.py`, `config.ui_reports_dir` / `verifier_reports_dir` in `src/easy_personal/core/config.py`, `store.retire_files` and `store.file_offsets`, and `_ingest` in `src/easy_personal/surfaces/cli.py`
7. Read the Rust you extend: `rust/crates/core/src/ingest/{mod,folders,claude_code,time,label,json}.rs`, `rust/crates/core/src/writer.rs`, `rust/crates/app/src/ingest_cmd.rs`, and the T026 / T035 tests that replay goldens (`rust/crates/core/tests/ingest_*`)
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Every run sets `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT` (and the two reports variables) to temp folders on the Linux filesystem

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): "do all the jobs for the final result"; P2b of the locked plan `BRAINSTORMING_LOG_RUSTP2.md`: after the events (P2a), the Rust `ingest` also fills `kit_trace_events`, `ui_sessions` and `verifier_scores`, so the kit, UI and verifier views of the dashboard and `kit_summary` are no longer empty after a Rust-only ingest.

**Restated intent**:
> `easy-personal-rs ingest` runs, after the events, the three Python sources in Python's order (`kit_trace`, `ui`, `verifier`) with Python's exact rules (what is read, what is kept, what is skipped, replace-all versus incremental), writes the same rows into the same schema-v11 tables, prints their counts under the same JSON keys, and a run of Rust and a run of Python over the same folders leave those three tables equal.

**Out of scope**: `context` (HTTP MCP client; stays out of P2), `live.py`, the scheduler (T034), reading these tables (the read side, `kit_summary`, is already ported), any schema change, any change to the Python app, any change to the old goldens.

**Requirement Refs**: RS-003, NFR-002 (privacy), NFR-005 in `rust/docs/SPEC.md`; FR-009, FR-010, FR-011, FR-013 in the Python spec.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T029 (the `ingest` command), T030 (leak gate), T035 (folders, offsets, retire)
**Entry point**: `easy-personal-rs ingest` (`rust/crates/app/src/ingest_cmd.rs` calls the new core module)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Oracle first.** New generator `rust/parity/gen_ingest_kit.py` (new file; `gen_ingest.py` and the old goldens are not edited) builds fixtures and goldens from the **real Python** ingest for scenarios K01.. covering: `kit_trace` (task file, `_untagged`, an abstained line, a blank line, a torn last line, a file replaced by another inode, a shrunk file, a project that no longer exists, two runs where the second adds 0, a deleted file that is retired), `ui` (ok session, bad JSON, wrong shape, `endedAt` bad, oversize file over 20 MB skipped, duplicate ids where the later file wins, unset folder, missing folder), `verifier` (scored report, abstained report, a newer file of unknown `schema_version`, symlink, oversize over 1 MB, non-regular file, container folder by repo name, two projects with one folder name, value out of 0..100, no readable JSON). The goldens are the table dumps (ids, machine_id and inodes excluded, as in T026) and the printed counts. `--check` shows 0 different | locked oracle |
| 2 | **`kit_trace`** in `rust/crates/core/src/ingest/kit_trace.rs` copies Python: projects are the distinct `project_path` of `events` rows of source `claude_code` that are existing directories, sorted; files `<project>/memory/event-trace/*.jsonl` sorted by name; only `timestamp`, `tool_name`, `is_error` are read (never `summary`); abstain on anything unreadable; file stem is the kit task id (`_untagged` means none); offsets, replaced-inode and shrunk-file restart; one transaction per file with `INSERT OR IGNORE`; `source_files` row for source `kit_trace`; retire the files no longer seen. Counts `files` and `new_events` | RS-003 |
| 3 | **`ui`** in `ingest/ui.rs`: `session-*.json` of the folder from `EASY_PERSONAL_UI_REPORTS`, else `ui.reports_dir` of `config.yaml` (same flat reader the Rust core already has for `thresholds:`; if it has none for another section, add the smallest one and test it); files over 20 MB skipped; the later sorted file wins for one id; replace semantics exactly as Python (unset or missing folder deletes all `ui_sessions` rows and returns `no data`; otherwise `INSERT OR REPLACE` only, **no delete**); counts `status`, `sessions`, `skipped` | FR-013 |
| 4 | **`verifier`** in `ingest/verifier.rs`: container from `EASY_PERSONAL_VERIFIER_REPORTS`, else `verifier.reports_dir`, else `~/.easy-verifier-reports` resolved from `HOME`; per project the newest `evidence-report-*.json` by the UTC stamp in the name, from `<project>/reports/` and, when the project folder name is unique, `<container>/<name>/`; parent-must-equal-folder check, `O_NOFOLLOW | O_NONBLOCK`, regular files only, 1,000,000 byte cap; only `schema_version` 1 is read, another integer version gives status `unknown`; `overall.value` 0..100 and the abstained rule exactly as Python; the table is replaced in **one transaction** (delete all, then insert), so any failure inside rolls back and leaves the old rows, and the result is `{"status":"failed","reason":<error kind>}` without the content; counts `status`, `projects`, `skipped` | FR-011 |
| 5 | **Parse rules shared, not copied twice**: use the existing `label`, `time` and `json` modules (`_label`, `_utc` equivalents); no new dependency, no new crate. Where Python crashes or hangs on hostile input (a deeply nested JSON line or file, an integer beyond int64, a timestamp outside years 1-9999) Rust abstains or skips like T026/T028 and the case is listed in your report as "decided for you", never copied | robustness |
| 6 | **Same data**: for every scenario of criterion 1, a Rust run and a Python run over the same folders give equal `kit_trace_events`, `ui_sessions`, `verifier_scores` and `source_files` (as the T026 replays compare); then Python after Rust and Rust after Python add 0 rows and leave the tables equal; a golden replay test in `rust/crates/core/tests/` runs on every `cargo test` (no Python needed) | single writer, two languages |
| 7 | **The command**: `ingest` prints, in addition to today's keys, `kit_trace`, `ui` and `verifier` objects with Python's keys; there is **no `context` key** (decided for you: Rust does not port `context`; the T029 README note says so); exit codes unchanged (0, 2 for a refused store, 75 busy); the three sources run under the same `ingest.lock` and each in its own transaction as Python does, after the events; a failure of `ui` or `verifier` never fails the run (Python never raises there) | CLI parity |
| 8 | **Privacy** (NFR-002): extend nothing in `tests/leak`; run `rust/parity/leak_rust.py` and show **0 hits** with the kit, ui and verifier canaries now actually ingested (the T030 report said they were scanned but not ingested; paste the new line and say if the anti-vacuity check for them can now be switched from "not ingested" to "ingested" — if that needs a change in `leak_rust.py`, make the smallest one and say so) | NFR-002 |
| 9 | **No regression**: `cargo test --workspace` under `TZ` unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago` (paste all three); the 13 default coexistence tests and the ignored real-Python ones that touch ingest still pass (run `-- --ignored` for `ingest_coexist` and paste the count); `rust/parity/bench_ingest.py` is **not** slower on the events path (the new sources add an `unchanged re-run` cost: report it, it must stay under 2 s on the 200 MB set); clippy and fmt clean (also `--features window`); `git status -- src tests web` empty; no new dependency | quality |
| 10 | **Docs**: `rust/README.md` ingest section (what the three sources read, the keys, no `context`), `rust/parity/README.md` (how to regenerate and check the K goldens); the "not included" line in `docs/release/rust-v0.1.0.md` is **not** edited (the Supervisor does it) | usable |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T033.md`.
> **Locked oracle**: the Python sources and the goldens made from them (read-only).

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | every K scenario | Rust tables equal the golden and a real Python run | golden replay and `--check` |
| 2 | Rust then Python (and the reverse) on one folder | the second adds 0 rows | real-Python test |
| 3 | `leak_rust.py` | `leak check: 0 hits` | Supervisor runs it |
| 4 | `git status -- src tests web` | empty | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain, git and rust/parity/.venv (real-Python half)
. "$HOME/.cargo/env" && cd rust && cargo build && ./parity/.venv/bin/python parity/gen_ingest_kit.py --check && cargo test --workspace && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/src/ingest/folders.rs` (offsets, retire, one transaction per file, `Store::save_file`), `ingest/accounts.rs`, T026's `gen_ingest.py` and its goldens, T035's coexistence test.
**Vital slice**: generator and K goldens for `kit_trace` → `kit_trace.rs` and replay → `ui` → `verifier` → the command and its keys → real-Python equality, leak gate, perf → docs.
**Cut list**: no `context`, no shared abstraction across the three sources beyond what two of them already need, no multi-threading, no schema or dedup-key change.

---

## Edge Case Checklist

- [ ] A failed or empty `verifier` run must not silently wipe the table unless Python does the same in the same case (one transaction; unknown/failed leaves old rows)
- [ ] `ui` with the folder unset or missing **does** wipe `ui_sessions` (Python does); with an existing but empty folder it does not
- [ ] `kit_trace` reads only projects that still exist as directories; a symlinked `event-trace` or trace file is handled as Python does (record what Python does, copy it)
- [ ] `summary` and every caller-authored field (`target`, `issues`, `actions` contents, `dimensions`) are never stored or logged; no path, value or content in an error message
- [ ] The Python crash cases are not copied (criterion 5) and each one is a test
- [ ] Time zones: the three timestamps go through the same UTC conversion as the events; run under the three `TZ` values
- [ ] Stale references: `rust/README.md` ingest section, `docs/release/rust-v0.1.0.md` "not included" (report, do not edit), `BRAINSTORMING_LOG_RUSTP2.md` task table (do not edit)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/ingest/{kit_trace,ui,verifier}.rs` (new), `ingest/mod.rs` | the three sources |
| `rust/crates/core/src/writer.rs`, `ingest/folders.rs` | only what `kit_trace` needs (a save for `kit_trace_events`), smallest change |
| `rust/crates/core/src/rules.rs` (or where the flat config reader lives) | only if `ui:` / `verifier:` sections need it |
| `rust/crates/app/src/ingest_cmd.rs` | call the sources, print their keys |
| `rust/crates/core/tests/ingest_kit_*.rs` (new) | the golden replays and robustness cases |
| `rust/parity/gen_ingest_kit.py`, `rust/parity/ingest/{fixtures,golden}-kit/**` (new) | the K oracle |
| `rust/parity/leak_rust.py` | only if criterion 8 needs it |
| `rust/README.md`, `rust/parity/README.md` | docs |
| `tasks/TASK_REVIEW_T033.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/parity/gen_ingest.py`, `gen_ingest_corpus.py`, `make_fixtures_ingest.py`, `rust/parity/ingest/{fixtures,golden}/**`, the older goldens | the oracle |
| `rust/crates/core/src/ingest/{claude_code,json,time,label,secret,signals,accounts}.rs` | reuse, do not change (if a change is unavoidable, stop and report) |
| `skills/verify/SKILL.md`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Generator and goldens first (red: Rust has no sources, replay fails), then each source until its scenarios are green, then the command, then real-Python equality and coexistence, leak gate, perf, three time zones. Paste every output with counts.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T033.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] `gen_ingest_kit.py --check` 0 different; three `TZ` runs, leak run, perf line pasted
- [ ] "Decided for you" list written (crash cases, no `context` key, others)
- [ ] Supervisor notified: task ready for Stage 4 review
