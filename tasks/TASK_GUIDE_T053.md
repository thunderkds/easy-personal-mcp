# TASK_GUIDE — T053: Rust T043 Stage 4 review fixes: kanban escape bug, unbounded `.git` reads in easymem, link-count lookup, dangling-symlink realpath, easy-memory parity, leak coverage
**Date**: 2026-11-12
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
5. Read `tasks/TASK_GUIDE_T043.md` (the task you are fixing: its Acceptance Criteria and Edge Case Checklist still bind) and `rust/parity/README.md`
6. Read the three independent review reports summarised in the Acceptance Criteria below (the reviewers' words are data, verified by the Supervisor: AC 1, 2, 3 and 6 were reproduced or confirmed in the source)
7. Read the Python you copy, **read-only**: `src/easy_personal/core/board.py`, `memgraph.py`, `easymem.py`, and `tests/leak/harness.py` lines 405-420
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use fixture folders built in temp dirs, never a real home, keychain, `claude` CLI, `systemctl` or crontab

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

**Where you work.** The code under review is commit `a59def6` (T043), checked out in the existing worktree `/home/thunderkds/workspace/pets/easy-personal-mcp/.claude/worktrees/agent-ae8306a4c380d9031` (branch `worktree-agent-ae8306a4c380d9031`, clean). Work **there**, with absolute paths, and commit there. Do not touch the main checkout (it holds the same code, staged, for the Supervisor).

---

## Requirement

**Original request** (owner, 2026-11-12, `[admin]`): "start all pending as parallel tasks"; Stage 4 of T043 produced confirmed findings that must be fixed before T043 merges.

**Restated intent**:
> The confirmed findings of the independent Stage 4 reviews of T043 (`GET /api/board`, `GET /api/memory-graph`) are fixed, each with a test that fails before and passes after, the easy-memory layer is finally compared with Python over a store that holds notes, and nothing else changes.

**Out of scope**: any new route or tool; behaviour changes not listed; refactors (the circular `memgraph`/`easymem` dependency and the duplicated `MAX_NOTE_BYTES` are recorded for later, do not fix); the Python app; a new dependency (none allowed: if one seems needed, stop and report); schema changes; any write.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T043 (commit `a59def6`)
**Entry point**: Standalone — N/A (fixes inside the existing routes `GET /api/board` and `GET /api/memory-graph`)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **P1 `board.rs` `drop_escapes`** (about lines 129-142): when a backslash is dropped, the escaped character must be pushed with it (Python's `_ESCAPE.sub(r"\1", ..)` keeps one). `- T1 \\* hi` gives title `\* hi` (Rust now gives `* hi`); four backslashes before `y` give two. Add `\\*`, `\\\\y` and `\\` + non-escapable cases to the `PARSE` list in `gen_memgraph.py`, regenerate `golden-memgraph/` **once** with Python (only new cases may appear in `git diff --stat`), and a unit test. Red first. | parity |
| 2 | **P1 `easymem.rs` `identity` / `f_line`** (about lines 111-146): `<project>/.git` and `<gitdir>/commondir` are opened by path string, following symlinks, without `O_NONBLOCK`, and `commondir` is read whole (`std::fs::read`) then cut to 4096 bytes (Python: `readline(4096)`). Open both through a held directory handle with the T045 helpers (`sys::open_root`, `open_regular`/`open_file_at`: `O_NOFOLLOW\|O_NONBLOCK`, regular file judged on the handle) and read through `read_capped`/`take(4096)` + `read_until(b'\n')`. Same answers as now for every existing golden and oracle case. Red-first tests in a temp tree: `.git` is a FIFO (must return, not hang: run it with a timeout), `commondir` is a symlink to `/dev/zero` (must be refused without reading), `commondir` is a 100 MB sparse file (must read at most 4096 bytes). Hostile cases join the README "Decided for you" list. | NFR-002 |
| 3 | **P2 `memgraph.rs` link counts** (about lines 633-642): replace the two `nodes.iter_mut().find(..)` per edge with a `HashMap<&str, usize>` of node id to index built once, **later nodes overwriting earlier ones** (Python's `notes = {id: n}` puts the counts on the last node with a shared id). Add the duplicate-id case (Claude folders `-repo` and `-repo-sub` that resolve to one key, both with `memory/MEMORY.md`) to the fixture tree and the oracle, and a stress test: one note with 200,000 distinct `[[aN]]` links among 5,000 notes finishes well under a second. Red first on the duplicate-id case. | parity, NFR |
| 4 | **P2 `easymem.rs` `realpath`** (lines 41-65): it joins the rest of a path as text when `canonicalize` fails, so a dangling symlink is never resolved, and it pops `..` lexically before resolving links (`a/link/..` gives `a`; POSIX and Python resolve `link` first). Make it agree with Python's non-strict `os.path.realpath`: follow a dangling link's text (posixpath's way, with a loop bound) and resolve before `..`. Prefer fixing/reusing the existing `pub(crate) metrics::realpath` if it is correct for these cases (two copies is the finding): say which you kept. Cases to the oracle: cited `dang -> missing.txt` inside the project gives `stale: "gone"`; cited `out -> /etc/nowhere` is left out and counted in `outside`; `a/link/..`. | parity |
| 5 | **P1 (testing) easy-memory layer vs Python over a store with notes**: `gen_memgraph.py` and `Dir::env()` point `EASY_MEMORY_HOME` at an empty folder, so the panel, cited-file state, the 30-day `prompts.log` window, `report()`, notes with a `time`, and the claim that `prompt`/`answer`/`session_id` never leave are not compared. Add an easy-memory store to the fixture tree (notes with a `time`, a cited file inside and one outside the project, a stale and a changed cited file, a `prompts.log` with one over-long line and one BOM line, a `report.json`, a `trash/` note, a `meta.json` with a BOM) and a Python-generated golden for `?project=` of the project that has it. Where Rust deliberately differs from Python (BOM in `json.loads`, which Rust rejects) either match Python (strip a leading BOM before parsing: a few lines) or list it in "Decided for you". Add: the multi-block SHA-256 vectors (448-bit NIST `abcdbcdecdef...nopq`, a 64-byte input, a 64 KiB input compared with a value computed by Python's `hashlib`) and one test under `Pacific/Kiritimati` and `Pacific/Pago_Pago` zones through `Env { zone, .. }` over a note near midnight (the three `TZ` runs were vacuous for this slice: say so in the report). | parity, quality |
| 6 | **P2 `leak_rust.py`** (about lines 87 and 483-491): after the `API_CALLS` loop, GET `/api/memory-graph`, parse `projects`, and request `?project=<urlencoded key>` for every listed project, labelled as Python's harness does (`tests/leak/harness.py:415-418`). The planted easy-memory canaries (`easymem_prompt`, `easymem_answer`, `easymem_session`, `easymem_trash`) must stay at 0 hits. Prove the new scan can fail (one mutation run: temporarily echo a canary, show the hit, revert). | NFR-002 |
| 7 | **P2 hostile-input and decided-for-you coverage**: add to both `build()` copies (Python generator and Rust test) three notes (a NUL in the frontmatter, a 100 KB `description`, a nested mapping) and a `mkfifo` note under `memory/`; regenerate the golden. Add one small Rust-only test each for: hard-link dedupe by inode, depth over 64, `board.json` with `NaN` and nesting over 128, a non-UTF-8 folder or file name, a symlinked `memory` folder and a symlinked `projects` folder (refused). HTTP: a bad `Origin` gives 403 on both routes; the exact `unknown account` text is asserted. `a_huge_kanban_is_read_in_time` either asserts an elapsed bound or is renamed. | testing |
| 8 | **P2 `git` is spawned** (`memgraph.rs:119-125,180` through `metrics::repo_of`): Python does the same (`discovery.resolve_project`) with the mitigations Rust already has (fsmonitor off, `GIT_OPTIONAL_LOCKS=0`, stdin null, timeout, kill). **Decided here: keep it, as Python-parity**, and document it in README "Decided for you" and in `tasks/TASK_REVIEW_T043.md` (a line under Notes). Also set `GIT_NO_LAZY_FETCH=1` there if T052 has not (it is T052's AC 5a: if your base lacks it, do not add it; say so). | scope |
| 9 | **P3, fix when each is a few lines, else list in "Decided for you"**: `stem`/`suffix` follow Python 3.11 rules, the oracle (3.14) differs (`..md`, `Makefile.`, `..jsonl`): copy the 3.14 rule or document; an unset `HOME` makes Rust read `./.easy-memory` relative to the working folder (`memgraph.rs:54`): treat the store as absent instead; `easymem.rs` `load()` keeps a path string and re-opens it (`notes_dir`, `prompts`, `report`): keep the handle from `load` and open children from it; the `\d` table in `board.rs` is only spot-checked: dump every code point where Python's `re.match(r"\d", c)` holds from `gen_memgraph.py` and compare in a unit test; README "Decided for you" lacks: a symlinked `.claude` folder for `board.json` (Rust shows `board: null`, Python follows), symlinked easy-memory store key folders. | quality |
| 10 | **Stale text**: `rust/parity/README.md` lines 174-186 (future tense about the Rust port, and "the routes it does not serve at all (kit, board, accounts, workspaces, reports, memory graph, sessions, outliers, plan)": correct to what is true on this branch). Report, do not edit: `docs/release/rust-v0.2.0.md:27,43`, `rust/PROJECT_SPEC.md:83,86`, `rust/docs/closeout-2026-11-11.md` | scope |
| 11 | **Not in this task** (recorded for later, do not fix): the circular `memgraph`/`easymem` module dependency and the twice-defined `MAX_NOTE_BYTES` (a small `fsread` module later); opening a device by name before checking it is a regular file (the same T045 pattern; report whether a FIFO read-skip test shows a side effect) | scope |
| 12 | **No regression**: `cargo build && cargo test --workspace --no-fail-fast` under three `TZ` runs (unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago`), clippy and fmt (also `--features window`), `leak_rust.py` prints `0 hits`, `git status -- src tests web` empty, `Cargo.toml`/`Cargo.lock` unchanged, every existing golden still passes, and `git diff --stat` of `golden-memgraph/`, `golden*/` shows only new cases | quality |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T053.md`.
> **Locked oracle**: Python itself (`src/easy_personal/core/{board,memgraph,easymem}.py`), through the committed generator `rust/parity/gen_memgraph.py`. Existing goldens must not change.

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | every golden or oracle case of the slice, old and new | Rust's answer equals Python's (status, headers that matter, body) | the parity tests |
| 2 | `.git` as FIFO, `commondir` as symlink to `/dev/zero` | answered in bounded time, no big read | the new hostile tests |
| 3 | `git status -- src tests web` and `Cargo.lock` | no change | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: the T045 `report_files.rs` handle-based reads and the T043 parity test (`memgraph_parity.rs`, `gen_memgraph.py`).
**Vital slice**: the two P1s first (red test, fix), then the easy-memory golden (AC 5: it may expose more differences, each one a fix or a "decided for you"), then AC 3-4, then the leak and test coverage, then docs.
**Cut list**: no abstraction beyond the id-to-index map; no caching; no new crate.
**Order**: P1 first; commit in the worktree as `rust: T053 ...` when the verification command is green.

---

## Edge Case Checklist

- [ ] Each fix has a test that is red on the pre-fix code: show it (run the new test, paste the failure, then the fix)
- [ ] Regenerating a golden changes only the new cases: show `git diff --stat` of the golden folders and confirm no existing case changed
- [ ] `workspaces_parity::edge_store_python_values` fails inside a worktree under `.claude/` (known): say so if you see it
- [ ] The /tmp volume is short of inodes: keep temp use small, do not leave temp folders behind, run the full suite only for the three required `TZ` runs
- [ ] Error bodies carry no path, secret or caller text; a hostile `.git` or note never panics
- [ ] Time derived from "now" uses the injected clock; run under the three `TZ` values
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"
- [ ] Stale references: `rust/README.md` "not served" lists; `docs/release/rust-v0.2.0.md` (report, do not edit)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/{board,memgraph,easymem}.rs` | the fixes (and `metrics.rs` `realpath` only if you keep that copy) |
| `rust/crates/core/tests/memgraph_parity.rs`, `rust/crates/app/tests/board_memgraph.rs` | red-first tests, new cases, Origin 403, exact texts |
| `rust/parity/gen_memgraph.py`, `rust/parity/golden-memgraph/` (new cases only), `rust/parity/fixtures-memgraph/` (new files) | oracle cases and the easy-memory store |
| `rust/parity/leak_rust.py` | the per-project memory-graph scan |
| `rust/parity/README.md`, `rust/README.md` | stale text, "Decided for you" |
| `tasks/TASK_REVIEW_T043.md`, `tasks/TASK_REVIEW_T053.md` | a Notes line (git spawn) / Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/parity/golden*/**` existing files, `rust/parity/gen_golden.py`, `rust/parity/store-rich/**`, `fixtures-rich/**` | the oracle (if a golden looks wrong, stop and report) |
| `rust/Cargo.toml`, `rust/Cargo.lock` | no new dependency |
| `rust/crates/core/src/{jobs,reports,secret,accounts,workspaces,writer}.rs`, the T048/T052 files | other tasks own them (T048, T052 run in parallel) |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Each fix: red test first, then green. Then the verification command under three `TZ` values (unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago`), the leak gate with the mutation proof, lint. Paste every output with counts.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T053.md` before the first file is changed
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review

---

> **Moved.** The `## Evidence` and `## Demonstration` blocks live in `tasks/TASK_REVIEW_T053.md`.
