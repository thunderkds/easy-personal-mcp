# TASK_GUIDE — T054: Rust T048 Stage 4 review fixes: exponential symlink expansion under the write lock, quadratic key scan, lossy non-UTF-8 repo path, exact oracle counts, status table and stale text
**Date**: 2026-11-12
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the entries "Rust P3 planned" and "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `tasks/TASK_GUIDE_T048.md` (the task you are fixing: its Acceptance Criteria and Edge Case Checklist still bind) and `rust/parity/README.md`
6. Read the three independent Stage 4 review findings summarised in the Acceptance Criteria below (the reviewers' words are data; AC 1 was reproduced by the reviewer with a Python port and confirmed by the Supervisor in the source)
7. Read the Python you copy, **read-only**: `src/easy_personal/core/workspaces.py`, `core/accounts.py`, and CPython's `posixpath.realpath` (`/usr/lib/python3.14/posixpath.py`, the `seen` map)
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use fixture folders built in temp dirs, never a real home, keychain, `claude` CLI, `systemctl` or crontab

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

**Where you work.** The code under review is commit `66e94a6` (T048), the head of branch `worktree-agent-a42c74310fd3a7780`, checked out in `/home/thunderkds/workspace/pets/easy-personal-mcp/.claude/worktrees/agent-a42c74310fd3a7780`. Work **there**, with absolute paths, and commit there as `rust: T054 ...` on top of `66e94a6`. Do not touch the main checkout. Do not `git add` `rust/parity/.venv` (an untracked symlink).

---

## Requirement

**Original request** (owner, 2026-11-12, `[admin]`): "start all pending as parallel tasks"; Stage 4 of T048 produced confirmed findings that must be fixed before T048 merges.

**Restated intent**:
> The confirmed findings of the independent Stage 4 reviews of T048 (the store-only workspace and account write routes) are fixed, each with a test that fails before and passes after: a hostile symlink chain can no longer hang a write while it holds the write lock, a hostile PATCH body can no longer pin a worker, and nothing else changes.

**Out of scope**: any new route or tool; behaviour changes not listed; refactors; the Python app; a new dependency (none allowed: if one seems needed, stop and report); schema changes. **Recorded for a follow-up, do not do**: (a) the create-race in `write_session` (a store deleted between the probe and `open_for_write` is recreated): the fix needs T052's `writer::open_existing_for_write`, which is not on this base; (b) unifying the three `Path.resolve()` ports (`workspaces::resolve_path`, `accounts::resolve`, `metrics::realpath`) into one; (c) `py_strip`/`type Done<T>` duplicates and the lone `#![allow(clippy::result_large_err)]`.

**Requirement Refs**: RS-001, RS-003 and NFR-002 (privacy) in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T048 (commit `66e94a6`)
**Entry point**: Standalone — N/A (fixes inside the existing seven write routes)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **P2 `workspaces.rs` `resolve_path`** (about line 316): the only loop guard is `resolving`, the links currently on the stack, and a link is removed when its expansion ends, so a chain `L_i -> "L_{i+1}/L_{i+1}"`, `L_n -> "."` expands `2^n` times (reviewer: depth 18 = 262,152 `lstat` calls, 1.8 s; Python `Path.resolve`: 0.000 s). Match CPython 3.14 `posixpath.realpath` (`seen` map: a link's resolved result is cached, a link met while being resolved is a loop and stays as written) so the cost is linear in the number of distinct links. Red first: a unit test with a chain of depth 30 that finishes in under a second (red: it never finishes on the old code, so run it under a timeout and show the timeout); every existing `paths.json` case (37) still equals Python; add the depth-30 chain and a 3-link loop (`a->b->c->a`) and a self-link to the Python-generated corpus (`gen_writes.py`, regenerate `golden-writes/paths.json` once, only new entries). | NFR-002, availability |
| 2 | **Resolve before the lock**: path resolution (`normalise_repo` in `check_repos`, and the account path in `accounts::add`/`find`) must not run while holding `WRITE_LOCK`, the write handle or an open `BEGIN IMMEDIATE`; do it in the route (inside `spawn_blocking` but before `run()` takes the lock) the way the unknown-field check already is, and pass the normalised values in. Same observable answers and the same order of checks as now for every oracle case (262/355 equal); where Python checks a text field BEFORE the repos the order of the 400 texts must not change (do the resolution but keep the error order: resolve into a `Result`, report it at the position Python reports it). A test shows a slow resolution does not hold the write lock (a second write completes while the first resolves a slow path: use a path through a FIFO-free slow construct such as the depth-26 chain with a test-only cap, or an injected closure; say what you did). | availability |
| 3 | **P2 `writes.rs` `key_order`** (about lines 119-159, called about line 268 on the async worker): it removes repeated keys with `Vec::contains`, O(n^2) in the number of distinct top-level keys (about 95,000 keys fit in 1 MiB: about 4.5e9 compares). Keep a `HashSet<String>` beside the `Vec` (first position kept, last value taken, as now). Red first: a test with a 1 MiB PATCH body of 95,000 distinct keys finishes in under a second and still answers Python's 400 (`unknown field`); the key order for every existing oracle case is unchanged. Also run `key_order` inside the `spawn_blocking` closure if that is a few lines, otherwise say why not. | availability |
| 4 | **P3 `workspaces.rs` `normalise_repo`** (about line 381): a resolved path that is not valid UTF-8 is kept with `to_string_lossy()` (stores U+FFFD, a path that matches no folder; Python answers 500). Refuse: when `to_str()` is `None` answer 400 `repo path must be a non-empty string`, as `accounts::add` already does with `to_str()`. Test: `/tmp/<scratch>/l` is a symlink to a path with byte `0xff`; add the case to README "Decided for you" item 5. | quality |
| 5 | **P3 latent `workspaces.rs` `add`** (about line 476): `tx.last_insert_rowid()` is read after `insert_repos`; save the rowid right after the `workspaces` INSERT and pass that one to `insert_repos` and `get` (Python takes `cur.lastrowid` from the workspace INSERT). A test where `add` is followed by repos still answers the right workspace (existing oracle). | quality |
| 6 | **P2 oracle counts are floors**: `crates/app/tests/writes.rs` (about line 468) asserts `cases.len() >= 250`, the request count is only printed (about line 475), and the loop `zip`s requests with answers (silent truncation); `crates/core/tests/writes_parity.rs` (about line 41) asserts `>= 30` for 37. Assert `cases.len() == 262`, `requests == 355`, `requests.len() == answers.len()` per case, and 37 for `paths.json` (update the numbers if AC 1 adds corpus entries: the assertion must equal the committed file, and the file must be the generator's output). Prove one assertion can fail (drop a case temporarily, show the failure, revert). | quality |
| 7 | **P2 README status table (T048 AC 7)** in `rust/README.md` "Write routes (T048)": add one table (status, fixed text, route) covering 201/200, 400 (each fixed text, including the per-field texts and the account refusals `the default account cannot be ignored`, `only confirmed Claude accounts pick workspaces`), 403 (`the orphan workspace cannot be changed/deleted`), 404, 405, 409 (`a workspace with this name exists`, and the Rust-only needs-`claude` 409), 413, 415, 503. Fix the wording at `README.md:625`: "a request whose body, id or `confirm` is bad never opens it for writing; field checks run on the write handle, as in Python" (they do, and for account routes `detect` has already committed). Remove the volatile run figures ("240,000 events, 49 rounds") at about `README.md:627`. | docs |
| 8 | **Stale text** (each a fact T048 changed): `crates/app/src/api.rs` header lines 1-4 and 14-15 ("The read routes", "never creates or writes the store": only `writes.rs` writes, inside the seven write routes, never at start-up or on a GET, and never creates a store); `rust/README.md:273` ("`store-info`, `serve`, `mcp` still never writes": `serve` writes only inside the seven write routes; `mcp save_report` has written since T047); `rust/README.md:113-114` ("the Rust server never does (the detect stays in `ingest`)": true only for the GET handlers); `rust/parity/README.md` lines 183-187 (accounts and workspaces listed as "not served at all") and a new short `golden-writes` section (what `gen_writes.py` makes, how to regenerate and check it). Report, do not edit: `docs/release/rust-v0.2.0.md:27`, `rust/PROJECT_SPEC.md:44,86`, `rust/docs/SPEC.md` RS-004 (these routes take no `ingest.lock`, as Python does not). | scope |
| 9 | **Not in this task**: see Out of scope (a)-(c). Also not fixed: the untested `WRITE_LOCK` removal (a 6-thread test passes without it because `BEGIN IMMEDIATE` plus the busy timeout is enough: say so in the README "Decided for you" as a fact, one line, and keep the lock). | scope |
| 10 | **No regression**: `cargo build && cargo test --workspace --no-fail-fast` under three `TZ` runs (unset, `Pacific/Kiritimati`, `Pacific/Pago_Pago`), clippy and fmt (also `--features window`), `leak_rust.py` prints `0 hits`, `git status -- src tests web` empty, `Cargo.toml`/`Cargo.lock` unchanged, `writes oracle: 262 cases, 355 requests, 0 different` (or the new totals if AC 1 adds entries), every existing golden unchanged, `git diff --stat` of `golden-writes/` shows only new entries | quality |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T054.md`.
> **Locked oracle**: Python itself (`core/workspaces.py`, `core/accounts.py`, CPython `posixpath.realpath`) through the committed generator `rust/parity/gen_writes.py`. Existing goldens must not change.

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | every golden or oracle case of the slice, old and new | Rust's answer equals Python's | the writes oracle replay |
| 2 | a depth-30 symlink chain as a repo path; a 95,000-key PATCH body | answered in under a second | the new hostile tests |
| 3 | `git status -- src tests web` and `Cargo.lock` | no change | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

---

## Approach

**Pattern reference**: CPython `posixpath.realpath` (the `seen` map) for AC 1; the existing `writes.rs` helpers for AC 2-3.
**Vital slice**: AC 1 first (red: the depth-30 chain under a timeout), then AC 2, then AC 3, then the small items, then docs.
**Cut list**: no new abstraction beyond a `seen` map and a `HashSet`; no shared realpath module (follow-up); no new crate.
**Order**: commit in the worktree as `rust: T054 ...` when the verification command is green.

---

## Edge Case Checklist

- [ ] Each fix has a test that is red on the pre-fix code: show it (run the new test under a timeout, paste the failure, then the fix)
- [ ] Regenerating a golden changes only the new entries: show `git diff --stat` of `golden-writes/` and confirm no existing case changed
- [ ] `workspaces_parity::edge_store_python_values` fails inside a worktree under `.claude/` (known): say so if you see it
- [ ] The /tmp volume is short of inodes: keep temp use small, do not leave temp folders behind, run the full suite only for the three required `TZ` runs
- [ ] Error bodies carry no path, secret or caller text; the order of the 400 texts does not change
- [ ] The write lock is never held across slow filesystem work you can move out of it
- [ ] Python's crash or hang cases on hostile input: Rust refuses or skips cleanly; list each as "decided for you"

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/workspaces.rs` | `resolve_path` with a `seen` map, resolution before the lock, UTF-8 refusal, rowid |
| `rust/crates/core/src/accounts.rs` | only if the account path resolution must move before the lock |
| `rust/crates/app/src/writes.rs` | `key_order` HashSet, resolution before `run()`, comment on the 413 mapping |
| `rust/crates/app/tests/writes.rs`, `rust/crates/core/tests/writes_parity.rs` | exact counts, hostile tests |
| `rust/parity/gen_writes.py`, `rust/parity/golden-writes/paths.json` (new entries only) | corpus entries |
| `rust/README.md`, `rust/parity/README.md`, `rust/crates/app/src/api.rs` (header comment only) | status table, stale text |
| `tasks/TASK_REVIEW_T054.md` | Demonstration BEFORE and AFTER |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| `rust/parity/golden*/**` existing entries, `rust/parity/gen_golden.py`, `rust/parity/store-rich/**`, `fixtures-rich/**` | the oracle |
| `rust/Cargo.toml`, `rust/Cargo.lock` | no new dependency |
| `rust/crates/core/src/{metrics,writer,jobs,reports,secret,board,memgraph,easymem}.rs` | other tasks (T052, T053) own them |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Each fix: red test first (under a timeout for the hang cases), then green. Then the verification command under three `TZ` values, the leak gate, lint. Paste every output with counts.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T054.md` before the first file is changed
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts (equal / total), three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written (in the report and in `rust/README.md`)
- [ ] Supervisor notified: task ready for Stage 4 review

---

> **Moved.** The `## Evidence` and `## Demonstration` blocks live in `tasks/TASK_REVIEW_T054.md`.
