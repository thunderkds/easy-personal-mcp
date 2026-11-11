# TASK_GUIDE — T030: Leak suite over the Rust ingest — planted secrets never reach the store, the surfaces or the logs
**Date**: 2026-11-11
**Complexity Level**: C2
**Risk Level**: High
**Priority**: P0
**Assigned agent**: qa-automation
**Agent guide**: `agents/qa.md`

---

## Mandatory Startup (Do Not Skip)

Before writing anything:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the 2026-11-11 entry "Rust P2 ingest" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/qa.md`
5. Read the Python leak suite you reuse, **read-only**: `tests/leak/README.md`, `tests/leak/canaries.py`, `tests/leak/conftest.py` (`leak_run`), `tests/leak/harness.py`, the leak tests, and the fixtures `tests/fixtures/leak/session.jsonl.tmpl` and whatever else `leak_run` plants
6. Read the Rust surfaces it must scan: `rust/crates/app/src/{main.rs,api.rs,mcp.rs,server.rs}` (what `store-info`, `serve`, `mcp` and `ingest` print), `rust/README.md` (the `ingest` section), `rust/parity/README.md` (the oracle's venv)
7. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Everything runs in temp folders with `EASY_PERSONAL_HOME` and `EASY_PERSONAL_AGENT_ROOT` set explicitly

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): build the Rust ingest (P2a) with the Python leak suite as its privacy gate (NFR-002): planted secrets and text must never reach the store, the MCP, the CLI, the API or the logs.

**Restated intent**:
> The same planted-canary fixtures the Python leak suite uses are ingested by the **Rust** `ingest` command, and then every Rust surface (the ingest's own stdout and stderr, `store-info`, every `serve` API route, every `mcp` tool, the log output, the store file, its WAL and SHM) is scanned for every canary in every encoding the Python suite checks, with the same anti-vacuity guards, and the run fails on a single hit.

**Out of scope**: any change to `tests/**` (the Python suite stays as it is and keeps passing), to `src/**`, or to the Rust product code (if a leak is found: report it, do not fix it here); `kit_trace`, `ui`, `verifier`, `context`, `live` planted fields (Rust P2a does not read them: they are scanned too, because a leak through an unread source must still be impossible, but they are not ingested).

**Requirement Refs**: NFR-002 (privacy), RS-003 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T029 (the `ingest` command)
**Entry point**: `rust/parity/leak_rust.py` (run with the parity venv's Python)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New `rust/parity/leak_rust.py` imports the Python suite's `Canaries` and fixtures **read-only** (by path, `sys.path`, or by copying at run time into a temp folder; never editing `tests/**`), plants every canary exactly as `leak_run` does into a temp agent root and projects folder, and runs the **Rust** binary `rust/target/debug/easy-personal-rs ingest` with an env built from scratch (`EASY_PERSONAL_HOME`, `EASY_PERSONAL_AGENT_ROOT` temp). It refuses to run if any env var points at a real home (same guard idea as `gen_ingest.py`) | gate over the real binary |
| 2 | **Surfaces scanned** after the ingest, each with the Python suite's `Canaries.find` (raw, JSON, `\u`, URL, base64 at 3 alignments, hex): the ingest's stdout and stderr; `store-info` (text and `--json`); every HTTP route of `serve` (all nine `/api/*` routes with realistic query variants, plus the static route) over real HTTP with the token; every MCP tool of `mcp` over stdio (initialize, `tools/list`, one `tools/call` per tool, including error paths); every log/stderr stream; the **raw bytes** of `store.sqlite`, `store.sqlite-wal`, `store.sqlite-shm` and `sqlite3 .dump` (use Python's `sqlite3` module); the temp data dir listing (no stray files). A second `ingest` run is scanned too (idempotent re-read) | every surface |
| 3 | **Anti-vacuity** (the guard that makes zero hits meaningful), each its own check that fails the run if false: the ingest really ran (`events` rows exist, `new_events > 0`); the failed `tool_result` was counted but its text not stored (as `test_failed_tool_result_was_counted_and_its_text_not_stored`); a **control canary** that is allowed to be stored (the session id of a fixture line) **is** found in the raw store bytes, proving the scan can see inside the files; an intentionally planted "bad" check (a unit test of the scanner: a blob with a canary in each of the seven encodings is detected) | zero hits must mean something |
| 4 | **Coverage list**: the report prints one line per canary (the 42 patterns of `canaries.py`) with where it was planted and the number of surfaces scanned for it, and fails if any canary was scanned on fewer than all surfaces; and prints the total bytes scanned per surface | nothing skipped silently |
| 5 | **Result**: exit 0 and a final line `leak check: 0 hits over <N> canaries, <M> surfaces, <B> bytes` when clean; on a hit exit 1 listing canary, encoding, surface and a safe excerpt of **at most 8 characters around it with the canary itself masked** (the report must not become a leak). Run it twice with different random canaries (the Python suite randomises per run): both clean | repeatable |
| 6 | **Negative proof** (the gate bites): temporarily make the Rust product store one canary (for example a throwaway local patch to the parse layer that stores a message text, applied in a **copy** of the workspace under the scratchpad, never in the repo) and show `leak_rust.py` exits 1 naming that canary and surface; paste the red run and say how the copy was made and discarded (do not delete anything; leave the scratch copy) | the gate detects a leak |
| 7 | **Python suite unchanged and green**: `git status -- tests src` empty; `rust/parity/.venv/bin/python -m pytest tests/leak -q` still passes (paste the summary line) | Python untouched |
| 8 | **Docs**: an "Ingest leak check" section in `rust/parity/README.md` (how to build the binary, run the script, read the report, what a hit means); a line for the Supervisor on how to add it to the `/verify` recipe (do not edit the skill) | usable |
| 9 | `git status --short` shows changes only under `rust/parity/` (new `leak_rust.py`, README section) and `tasks/TASK_REVIEW_T030.md`; no change in `rust/crates/**` | scope |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T030.md`.
> **Locked oracle**: `tests/leak/**` (read-only) and the Rust product code (read-only).

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the Rust binary and the planted fixtures | `leak check: 0 hits ...`, exit 0 | Supervisor runs it twice |
| 2 | the scanner's own unit check | a canary in each of seven encodings is found | script self-test |
| 3 | a copy of the product patched to store a canary | exit 1 naming it | negative proof |
| 4 | `git status -- tests src rust/crates` | empty | run it |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain (built binary) and rust/parity/.venv
cd rust && . "$HOME/.cargo/env" && cargo build && ./parity/.venv/bin/python parity/leak_rust.py && ./parity/.venv/bin/python -m pytest ../tests/leak -q
```

---

## Approach

**Pattern reference**: `tests/leak/conftest.py` (`leak_run`) and `canaries.py`; `rust/parity/gen_ingest.py` (guard, temp folders, tokens).
**Vital slice**: plant + ingest + scan the store and ingest output (criteria 1, 2 partly, 3) → the API and MCP surfaces → coverage report → negative proof → docs.
**Cut list**: no new canary patterns, no fuzzing, no CI job, no Rust code.

---

## Edge Case Checklist

- [ ] The script never writes outside its temp folders and `rust/parity/`; it never reads `~`
- [ ] The report masks the canary; no canary text is ever printed
- [ ] The store's WAL and SHM are scanned **while the writer's connection is still open and after close** (WAL content may still be unmerged)
- [ ] `serve` and `mcp` child processes are stopped by PID and left no stray process (check by PID)
- [ ] The token for `serve` is read from its first output line and never printed
- [ ] Rust surfaces that need no data (`/api/version`) are still scanned
- [ ] Wrap every child process in `timeout`; a hang fails the run, never blocks it
- [ ] Stale references: `rust/parity/README.md` and `BRAINSTORMING_LOG_RUSTP2.md` wording about "leak suite" (README only)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/parity/leak_rust.py` (new) | the driver and scanner |
| `rust/parity/README.md` | the "Ingest leak check" section |
| `tasks/TASK_REVIEW_T030.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `tests/**`, `src/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app and its suite |
| `rust/crates/**`, `rust/spikes/**`, `rust/Cargo.*` | the Rust product (report leaks, do not fix) |
| `rust/parity/ingest/**`, `gen_ingest.py`, `gen_ingest_corpus.py`, the older goldens | the oracle |
| `skills/verify/SKILL.md`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Scanner self-test first (red/green), then the real run twice, then the negative proof in a scratch copy, then the Python leak suite. Paste every output.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T030.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Both clean runs and the red negative proof pasted
- [ ] Python leak suite summary line pasted
- [ ] Supervisor notified: task ready for Stage 4 review
