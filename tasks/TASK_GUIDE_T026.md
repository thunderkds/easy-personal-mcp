# TASK_GUIDE — T026: Ingest oracle — Python `ingest` over fixtures, tables frozen as goldens
**Date**: 2026-11-11
**Complexity Level**: C3
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: qa-automation
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

Before writing anything:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt. Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read `agents/qa.md`
5. Read `BRAINSTORMING_LOG_RUSTP2.md` completely (the facts and the Edge Case Checklist are your coverage list), `rust/parity/README.md`, and `rust/parity/gen_golden.py` (the pattern to follow, including its real-home refusal)
6. Read the Python ingest you are freezing, read-only: `src/easy_personal/surfaces/cli.py` (lines 159-178), `src/easy_personal/core/{store,lock,config,discovery,secretlike,accounts}.py`, `src/easy_personal/sources/{claude_code,kit_trace}.py`, `src/easy_personal/core/migrations/`, and the fixtures `tests/fixtures/claude_code/`
7. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Everything runs in temp folders

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files in step 5 and 6 stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): build the Rust ingest (P2) as a second writer of the schema-v11 store, with the Python app as the oracle. Nothing in P2 may start before the goldens exist.

**Restated intent**:
> A repeatable generator runs the **real Python** `easy-personal ingest` over the committed fixtures and over a new invented, synthetic transcript set that exercises every edge case in `BRAINSTORMING_LOG_RUSTP2.md`, and freezes what Python wrote (the table rows, the CLI output, the exit codes) plus a snapshot of the fresh v11 schema, as golden files that the Rust ingest is later proven against, byte for byte.

**Out of scope**: any Rust code; any change to `src/`, `tests/`, `web/`, `rust/crates/**` or the existing goldens; `kit_trace`, `ui`, `verifier`, `context` and `live` goldens (P2b; but do not break them: the Python ingest runs them, record only that they ran); a Python bug fix (if Python does something odd, **record it**: the oracle defines the truth).

**Requirement Refs**: RS-003 (parity), NFR-002 (privacy), NFR-005 (ingest speed) in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: None (extends T003's parity tooling; the parity venv `rust/parity/.venv` exists)
**Entry point**: `rust/parity/gen_ingest.py` (run with the parity venv's Python)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | `rust/parity/gen_ingest.py` (new) runs with `rust/parity/.venv/bin/python`, imports the Python app from `src/` without changing it, and **refuses to run** when any env var points at a real home (same guard as `gen_golden.py`). Every run uses fresh temp folders for `EASY_PERSONAL_HOME`, the agent root and the work root | oracle is Python, no real data |
| 2 | **Inputs**: (a) the committed `tests/fixtures/claude_code/` (read only, copied to a temp folder); (b) a new synthetic set built by `rust/parity/make_fixtures_ingest.py` (new, deterministic, no real content) into `rust/parity/ingest/fixtures/`. The synthetic set covers **every** edge case in the Edge Case Checklist of `BRAINSTORMING_LOG_RUSTP2.md` that can be put in a transcript file: blank line; invalid JSON; valid JSON that is not an object; a truncated last line (no `\n`); naive timestamp (no zone); `+07:00` and `Z` offsets; a timestamp with 0, 3 and 6 fractional digits; `cwd` missing / not a string; usage with bool, float, string and negative ints; usage missing; a label with a trailing newline, with illegal characters and with 201 characters; model `<synthetic>`; a sidechain record and a `subagents/agent-*.jsonl` file; the same `message.id` on two records (split message, different usage); `tool_result` blocks with `is_error` true / false / absent; Bash tool inputs whose first word is plain, secret-like (`sk-...`, 32+ alphanumerics), a path (`/usr/bin/x`) and empty; a WebFetch URL with port, path and userinfo; every record type in `METADATA_TYPES` and an unknown type; an unversioned file; `entrypoint` present on some records of a session only; a deeply nested JSON line (about 100,000 `[`); an integer beyond int64 in a usage field; a non-UTF-8 byte; a file that is a directory named `x.jsonl`; a symlinked directory. If a case cannot be built, say which and why | the checklist is the coverage list |
| 3 | **Project resolution** cases use real folders under one work root and a real `git`: a plain folder, a git repo, a subfolder of a repo, a linked **worktree** of a repo, a folder that no longer exists, and a bare cwd string that is not absolute. The generator writes `repos.json` (the exact folders and git commands, with fixed author/date env so it is deterministic) so a Rust test can recreate the same layout. Paths in goldens use tokens: `<HOME>`, `<AGENT_ROOT>`, `<WORK>` | project identity is baked into `events.project_path` |
| 4 | **Scenarios**, each run in a fresh store unless it says otherwise, each recorded as `{scenario, exit_code, stdout_json, stderr_text_tokenised, tables_after}`: S1 cold ingest of the committed fixtures; S2 the same again (expect 0 new events); S3 a line appended to one file (incremental); S4 offsets reset to 0 (the back-fill path: no new rows, `entrypoint` and `tool_error_count` filled by the conditional upsert); S5 a file whose last line was truncated at the first run and completed at the second; S6 cold ingest of the synthetic set; S7 two accounts (default plus one `.claude-<name>` folder) with `agent_accounts` rows confirmed as the Python CLI would have them (say how you confirmed them); S8 `--retention-days 36500` (nothing pruned) and `--retention-days 1` (everything with a timestamp pruned, NULL-timestamp rows kept), both independent of today's date because every fixture date is in the past; S9 a busy lock: hold `flock` on `ingest.lock` from the generator and run ingest with `EASY_PERSONAL_LOCK_TIMEOUT=0.5` → exit **75** and the exact stderr text; S10 `--source PATH`; S11 a store with `user_version` 12 → the exact Python error and exit code (Python refuses); S12 an older store (`user_version` < 11 made by running only the first migrations, if the Python code allows building one; else say so) | scenarios are what Rust must reproduce |
| 5 | **Tables frozen**: `events`, `tool_signals`, `source_files`, `agent_accounts` (and the names and row counts of every other table), as JSON. Row ids, `machine_id`, absolute temp paths and inodes are not in the golden: rows are ordered by the table's key columns, `source_path` is tokenised, and the golden says for each table which columns it dropped and why. `events` rows keep **every** other column, including `abstained`, `tool_names`, `entrypoint`, `tool_error_count`, `agent_account_id` (as the account's folder, not its id) | rows equal Python's |
| 6 | **Schema snapshot** `rust/parity/ingest/golden/schema-v11.sql`: what a **fresh** Python store contains after `store.opened`: every `sqlite_master` row (`type`, `name`, `tbl_name`, `sql`) sorted by name, the seed rows (the orphan `workspaces` row), `PRAGMA user_version` (11), `journal_mode` (wal), `foreign_keys`, `busy_timeout`. Written so T027 can embed it and a test can prove a Rust-created store has the same `sqlite_master` content. `meta.machine_id` is noted as random and excluded | Rust creates a new store from it |
| 7 | **Determinism**: running the generator twice gives byte-identical goldens (`sha256` per file in `rust/parity/ingest/golden/manifest.json`, which also records the Python, SQLite and git versions and the scenario list). `gen_ingest.py --check` regenerates into a temp folder and compares with the committed goldens (exit 0 equal, exit 1 different, listing the files) | repeatable |
| 8 | **What Python really does** for the odd cases (naive timestamp, trailing-newline label, `<synthetic>` model, deep nesting, int beyond int64, bool/float usage, a directory named `x.jsonl`, a symlinked folder, non-UTF-8 bytes) is **recorded as it is**, including a crash. If Python crashes or aborts a file, the scenario records the exit code, the exception class and whether the other files were ingested. Your report has a table "case → what Python did", one line each, with the golden file that pins it | the oracle defines truth |
| 9 | **Nothing real**: a check in the generator (and a line in your report) shows the goldens and fixtures contain no home path, no email and no token-shaped text; the synthetic content is invented. Planted secret-like strings are the obvious fakes (`sk-FAKE...`, `AKIAFAKE...`) | NFR-002 |
| 10 | `git status --short` shows changes **only** under `rust/parity/` (new files, plus an appended section in `rust/parity/README.md`) and `tasks/TASK_REVIEW_T026.md`; `git status -- src tests web rust/crates` is empty; the Python tests you ran (`tests/core tests/sources tests/ops -q`, with the parity venv) still pass | Python untouched |
| 11 | `rust/parity/README.md` gets an "Ingest oracle" section: how to build the inputs, run the generator, run `--check`, what each file is, and the scenario list | usable by T027-T033 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T026.md`.
> **Locked oracle**: `src/easy_personal/**` (the Python app is the truth; do not edit it).

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the generator run twice | identical `sha256` for every golden | Supervisor reruns and compares |
| 2 | `gen_ingest.py --check` | exit 0 | Supervisor runs it |
| 3 | the S9 scenario | exit 75 and the "another ingest is running" line | read the golden and run it |
| 4 | `git status -- src tests web rust/crates` | empty | run it |
| 5 | grep of goldens/fixtures for `/home/`, `@`, `ghp_`, `sk-` (not `sk-FAKE`) | no match | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the parity venv (rust/parity/.venv) and git
cd rust/parity && ./.venv/bin/python gen_ingest.py && ./.venv/bin/python gen_ingest.py --check && git -C ../.. status --short -- src tests web rust/crates | wc -l
```

---

## Approach

**Pattern reference**: `rust/parity/gen_golden.py` (guard, temp folders, tokenising, manifest) and `rust/parity/make_fixtures_rich.py` (deterministic invented fixtures).
**Vital slice**: first S1 over the committed fixtures and the schema snapshot (criteria 1, 5, 6, 7); then the synthetic set and S6; then the other scenarios; then the odd-case table; then the README.
**Cut list**: no kit_trace/ui/verifier/context goldens; no Rust; no performance data (T032); do not rebuild the venv.

---

## Edge Case Checklist

- [ ] The generator never writes outside its temp folders and `rust/parity/ingest/`; it never reads `~`
- [ ] Python's ingest is run through its real CLI entry (subprocess or `main()`), not by calling internal functions, so exit codes and stdout are real
- [ ] `PYTHONHASHSEED`, locale, `TZ` and the git author env are fixed so output cannot vary
- [ ] No golden contains a temp path, an inode, a row id or `machine_id`
- [ ] Every synthetic file is tiny; the whole synthetic set stays under 200 KB; no real transcript text is copied
- [ ] The fixtures keep their truncated last line (do not "fix" them)
- [ ] The deep-nesting file does not make the generator itself crash or hang (use a timeout and record the result)
- [ ] Stale references: `rust/parity/README.md` and `rust/PROJECT_SPEC.md` still say "P2 ingest" without goldens: update only the README

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/parity/gen_ingest.py` (new) | the generator, `--check` |
| `rust/parity/make_fixtures_ingest.py` (new) | the deterministic synthetic set |
| `rust/parity/ingest/fixtures/**` (new) | generated synthetic transcripts and `repos.json` |
| `rust/parity/ingest/golden/**` (new) | `schema-v11.sql`, per-scenario JSON, `manifest.json` |
| `rust/parity/README.md` | append the "Ingest oracle" section |
| `tasks/TASK_REVIEW_T026.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app and its tests (read-only oracle) |
| `rust/crates/**`, `rust/spikes/**`, `rust/Cargo.*` | no Rust in this task |
| `rust/parity/golden/**`, `golden-rich/**`, `golden-tz/**`, `store-*`, `gen_golden.py`, `make_fixtures_rich.py` | the existing read oracle |
| `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Run the Verification Command; paste the two runs' manifests (hashes equal); paste `--check` exit 0; paste the S9 golden; paste the odd-case table; paste `git status` evidence; run `rust/parity/.venv/bin/python -m pytest tests/core tests/sources tests/ops -q` and paste the summary line. Negative check: change one byte in a committed golden, show `--check` exits 1 naming that file, restore it.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T026.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Tests/checks pass, output pasted
- [ ] Odd-case table ("case → what Python did") in the report
- [ ] Supervisor notified: task ready for Stage 4 review
