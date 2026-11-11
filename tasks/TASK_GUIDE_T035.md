# TASK_GUIDE — T035: Ingest of folders — discovery, accounts, project resolution, offsets, `save_file`
**Date**: 2026-11-11
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P0
**Assigned agent**: backend-implementer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the 2026-11-11 entry "Rust P2 ingest" in `memory/decisions.md` (including the T026 exception)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP2.md`, the "Ingest oracle" section of `rust/parity/README.md`, and the findings section at the end of `tasks/TASK_REVIEW_T026.md`
6. Read the Python you are matching, read-only, completely: `src/easy_personal/sources/claude_code.py` (the folder part: `ingest_accounts`, `account_roots`, the per-file loop), `src/easy_personal/core/accounts.py` (`detect`, `_insert`, `_provider_of`, `default_id`), `src/easy_personal/core/discovery.py` (`resolve_project`, `repo_of`), `src/easy_personal/core/config.py`
7. Read the Rust you plug into: `rust/crates/core/src/writer.rs` and `ingest_lock.rs` (T027), `rust/crates/core/src/ingest/` (T028: `read_lines`), and `rust/crates/core/src/metrics.rs` (`run_git`, its 5 s / 10 s limits and env, `pub(crate)`)
8. Read the oracle: goldens `rust/parity/ingest/golden/S01..S14*.json`, `schema-v11.json`, `rust/parity/ingest/fixtures/repos.json` and `cases.json`, and the odd-case table in `tasks/TASK_REVIEW_T026.md`
9. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use temp folders and the committed fixtures only

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): build the Rust ingest (P2a). T027 wrote the store writer and T028 the line parser. This task connects them to **real folders**: find the transcript files, decide which account each belongs to, resolve each project's identity, keep the read offsets, and save every file through `Store::save_file`, so that the tables equal Python's after the same ingest.

**Restated intent**:
> `ingest_accounts` walks the Claude Code projects folders exactly as Python does (same files, same order, same accounts, same project identity through `git`, same offset rules), feeds each file to the parse layer, saves it in one transaction, and returns the same `files` and `new_events` counts, so that for every scenario of the T026 oracle the `events`, `tool_signals`, `source_files` and `agent_accounts` tables equal the goldens; and one bad input (a directory named `x.jsonl`, a deeply nested line, a huge integer, a list-valued type) never stops the run or hides the files after it.

**Out of scope**: the `ingest` CLI, JSON printing, exit codes, env-var reading, data-dir creation, the lock call, `--retention-days` (T029); `kit_trace`, `ui`, `verifier`, `context`, `live` (T033 and later); the scheduler; any migration; changes to the parser's behaviour (T028) or to the writer's behaviour (T027).

**Requirement Refs**: RS-003 (parity), NFR-002 (privacy), RS-006 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T028 (parse layer), T027 (writer), T026 (goldens)
**Entry point**: `easy_personal_core::ingest::claude_code::ingest_accounts` and `easy_personal_core::ingest::accounts::detect` (used by T029; reached by this task's tests)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | Public API (parameters, no env reads, no globals): `accounts::detect(store, agent_root) -> Result<()>`; `claude_code::ingest_accounts(store, opts) -> Result<Summary>` with `opts = { agent_root, projects_override: Option<PathBuf>, cwd_base: PathBuf }` and `Summary { files, new_events }` whose meaning and numbers equal the `files` / `new_events` of the oracle's `stdout_json` (say in the code which Python line defines each) | T029 reads env and calls these |
| 2 | **`accounts::detect`** = Python `detect` (`accounts.py`): walks `agent_root` for entries named `.claude*` / `.codex*` (resolving symlinks, skipping files); `.claude` becomes the default account (`confirmed`, label "Claude default", `config_dir_env` NULL); other `.claude*` folders with `projects/` or `settings.json` are inserted `detected`; `.codex*` only listed; **`INSERT OR IGNORE`**: an existing row is never changed. Oracle: S07 (`agent_accounts` rows equal, including a `.claude-work` account that the oracle confirmed through the real CLI: your test applies the same final row state by SQL and says so) | S07 |
| 3 | **Roots** = Python `account_roots`: only `provider='claude' AND status='confirmed'` rows, each as `<folder>/projects`; `projects_override` replaces the **default** account's root; with no default row the override is read **untagged** (`agent_account_id` NULL); no `~/.claude` and no override → nothing is ingested (S10a, S10b). At the start of `ingest_accounts` run Python's `UPDATE events SET agent_account_id = <default> WHERE agent_account_id IS NULL` | S07, S10a, S10b |
| 4 | **Discovery** = `sorted(root.rglob("*.jsonl"))`: every `*.jsonl` regular file under a root, **including** `<session>/subagents/agent-*.jsonl`, sorted by **path components** (the T026 oracle shows `p/` before `p-q/`), directories are **not** followed through symlinks (S06: the symlinked folder gives no rows, 13 `source_files`), a file symlink is followed like Python's `stat()`. **A directory named `x.jsonl` or any non-regular file is skipped, not an error (S06c: Python crashes, Rust differs)**; an unreadable file is skipped and counted, never fatal; the files after a bad one are still read | S01, S06, S06c |
| 5 | **Offsets** = Python's rule and nothing else: `known_inode, offset = known.get(key, (st_ino, 0)); if known_inode != st_ino or offset > st_size: offset = 0`. No mtime, no hash. Add `Store::file_offsets(source) -> HashMap<path, (inode, offset)>` to `writer.rs` (read-only; the only allowed change there). S02 (same ingest twice: 0 new), S03 (a line appended), S04 (offsets reset: rows unchanged, `entrypoint`/`tool_error_count` back-filled), S05 (a truncated line completed later), S13 (file replaced: new inode, rows as in the golden), S14 (file deleted: `source_files` row retired, events kept) | S02-S05, S13, S14 |
| 6 | **`retire_files`** only for files that vanished **under a root that exists now** (Python `:254,268-269`); a missing root retires nothing | S14 |
| 7 | **Project identity** = `discovery.resolve_project(cwd)` byte for byte: a `cwd` that is not an existing directory is kept as is; otherwise `git rev-parse --path-format=absolute --git-common-dir` (parent of `.git` ⇒ a worktree maps to its main repo), else `--show-toplevel`, else `cwd`; the same `git` flags (`-c core.fsmonitor=false`), `GIT_OPTIONAL_LOCKS=0`, **5 s timeout**, result **cached per cwd per run**; a trailing slash, a symlink path and a relative `cwd` behave as the S06 goldens show (a relative `cwd` is resolved against `opts.cwd_base`, which stands for the Python process's cwd). Reuse `metrics.rs::run_git`; if you cannot, say why. The raw event's `project_path` (from the parser) is replaced by the resolved one before `save_file` | project identity is baked into the store |
| 8 | **Table equality**: for each scenario S01, S02, S03, S04, S05, S06, S07, S08a (36500 days: nothing pruned is T029's flag, but the ingest itself runs), S10a, S10b, S13, S14, the Rust ingest over the same inputs gives `events`, `tool_signals`, `source_files` and `agent_accounts` **equal to the golden, row for row, every column**, using the oracle's tokens. The Rust test recreates the work root exactly like the oracle (the work root is **padded to 240 characters**; the layout comes from `rust/parity/ingest/fixtures/repos.json`; fixed git author/date env). Machine id, ids and inodes are not compared (the golden says how they are tokenised) | RS-003 |
| 9 | **"Rust differs" cases** (Python crashes, S06a deep nesting, S06b huge int, S06c directory named `x.jsonl`, S06d list-valued type): the Rust run **succeeds**, the stated abstained rows (or the skipped entry) appear, the **files after them are ingested**, and a second run is clean (no stuck file). Each is a test with the expected rows spelled out and a comment "Python crashes here" | the Python crash must not stick Rust |
| 10 | **Coexistence with Python** (a real run, ignored test): ingest the S01 inputs with Rust into a fresh store, then run the real Python `ingest` on the same folder and the same store (parity venv): Python adds **0** new events and does not crash; then the reverse order (Python first, Rust second): Rust adds 0. Paste both runs. This is the proof that two implementations can share one store | second writer |
| 11 | **Privacy**: no path, project name or event content in any error or log text; a permission-denied folder gives a counted skip with a fixed message. The parse layer's guarantees (T028) are not re-opened, but a test shows that a transcript containing a `CANARY-…` in every free-text field leaves **no canary in the store file, WAL or SHM bytes** after `ingest_accounts` | NFR-002 |
| 12 | `cargo test --workspace` passes under `TZ` unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago` (paste all three); `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` clean, also with `--features window`; no new crate unless justified (name and licence); `Cargo.lock` additive | quality |
| 13 | **Mutation checks** (red run, then restore): sort files by string instead of path components; follow a directory symlink; drop the `offset > st_size` reset; resolve project with `--show-toplevel` only; skip the `NULL → default account` update. Each makes a named test fail | the tests bite |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T035.md`.
> **Locked oracle**: `rust/parity/ingest/**` (T026) and the Python sources named above. The implementing agent must not edit them.

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | S01, S06, S07 inputs | tables equal the goldens | test |
| 2 | S02, S03, S04, S05, S13, S14 | incremental behaviour equals the goldens | test |
| 3 | the four crash inputs | Rust succeeds, later files ingested | test |
| 4 | Rust then Python, Python then Rust on one store | the second adds 0, no crash | ignored real run |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain, git, and (ignored test) rust/parity/.venv
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && cargo test -p easy-personal-core --test ingest_folders_python -- --ignored && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: Python `claude_code.py` (`ingest_accounts` and the per-file loop), `accounts.py`, `discovery.py`; Rust `writer.rs`, `ingest/` (T028), `metrics.rs::run_git`.
**Vital slice**: `file_offsets` + discovery + the per-file loop on the committed fixtures (S01, S02) → incremental scenarios (S03-S05, S13, S14) → project resolution on `repos.json` (S06) → accounts and roots (S07, S10) → the "Rust differs" cases → coexistence run → privacy test → mutation checks.
**Cut list**: no CLI, no env reading, no lock call, no retention, no other sources, no parallelism.

---

## Edge Case Checklist

- [ ] Files are processed in the sorted order; the **event id order** the goldens imply (ids are not compared, but order of insertion decides `MAX(id)` counts) is respected
- [ ] The path stored in `source_files` and `events.source_path` is the **un-resolved** path string exactly as discovery yields it
- [ ] A file that shrinks (`offset > size`) restarts from 0 (the rows already there dedupe by key)
- [ ] A file replaced by another with the same size but a new inode is read from 0 (S13)
- [ ] A half-written last line is not consumed; the stored offset sits at a line start
- [ ] One failing file (unreadable, a directory, a parse that returns an error) never rolls back another file's transaction
- [ ] `git` missing or timing out gives the plain `cwd`, never an error
- [ ] Two cwds that resolve to the same repo trigger one `git` call each at most (cache)
- [ ] The default account row is found by `folder` / provider rules, not by id 1
- [ ] Stale references: `rust/README.md` "Ingest" wording; `BRAINSTORMING_LOG_RUSTP2.md` task table (README only)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/ingest/{accounts,discover,project,files,claude_code}.rs` (new, layout yours) | discovery, accounts, project resolution, the per-file loop |
| `rust/crates/core/src/ingest/mod.rs`, `rust/crates/core/src/lib.rs` | module lines only |
| `rust/crates/core/src/writer.rs` | add `Store::file_offsets` (read-only) and nothing else |
| `rust/crates/core/src/metrics.rs` | only a visibility change if `run_git` needs it (say so) |
| `rust/crates/core/tests/ingest_folders_*.rs` (new) | oracle replay, differs cases, privacy, the ignored Python run |
| `rust/README.md` | a short "Ingest of folders (P2a)" paragraph |
| `tasks/TASK_REVIEW_T035.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app (read-only oracle) |
| `rust/parity/**` | the oracle |
| `rust/crates/app/**`, `rust/crates/core/src/ingest/{lines,json,time,label,secret,signals,record}.rs` | T028's parse layer is not re-opened (report a needed change instead) |
| `rust/crates/core/src/{ingest_lock,store,rules,plan,scope,sessions,workspaces,prices,clock}.rs` | not part of this task |
| `rust/spikes/**` | spikes |
| `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Red first: S01/S02 on the fixtures, the incremental scenarios, the project cases, the accounts cases, the four "differs" cases. Then green. Then the ignored coexistence run, the privacy test, the three TZ runs, clippy, fmt, the mutation checks. Paste every output.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T035.md` before the first code change
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Tests written AND pass, red runs and mutation checks pasted
- [ ] Both coexistence runs pasted
- [ ] Supervisor notified: task ready for Stage 4 review
