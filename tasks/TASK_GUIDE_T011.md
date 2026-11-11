# TASK_GUIDE — T011: Generator hardening
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P2
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md`, the 2026-11-10 entries in `memory/decisions.md` and `memory/learnings.md` (T003 QA, T009, T007)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `rust/parity/gen_golden.py` (all of it: default, `--tz` and `--rich` modes), `rust/parity/README.md`, and the QA section in `tasks/TASK_REVIEW_T003.md`

> Read relevant files from `rust/docs/` before making any code changes. Your final report must contain a line `Startup reads: <paths>`.

---

## Requirement

**Original request** (owner, 2026-11-10): parity with Python before a surface flips; the oracle must be trustworthy.

**Restated intent**:
> Fix the cheap findings of the independent QA read of the generator (T003 P2/P3) without changing one byte of any committed golden, store or manifest.

**Hard constraint**: `rust/parity/golden/**`, `rust/parity/golden-tz/**`, `rust/parity/golden-rich/**`, `rust/parity/store-v11.sqlite`, `rust/parity/store-rich/**` and `rust/parity/fixtures-rich/**` stay **byte-identical** (prove with `diff -r` / `cmp` of fresh runs of all three modes). New checks and cases may only fail a run or print to stderr; new golden entries may not be added to any committed folder in this task.

**Out of scope**: new golden entries; changing the content of any golden; Rust code.

**Requirement Refs**: RS-003; QA findings in `tasks/TASK_REVIEW_T003.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T003, T007, T009 (all three modes exist)
**Entry point**: `python rust/parity/gen_golden.py` (all modes)
**Parallel task**: T010 works in `rust/crates/core/**`. Do not edit Rust. You own `rust/parity/gen_golden.py`, `rust/parity/README.md` and, if you need one, `rust/parity/check_lock.py`.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | **Clock patch is loud.** The generator asserts that every module it patches for the frozen clock was patched (a module whose `datetime` import style changed makes the run fail, naming the module). Prove with a test or a one-off mutation that the run fails when a listed module is not patched | QA P2 |
| 2 | **Enumeration is loud.** `list_prompts()` and `list_resources()` are compared with the known lists; a new prompt or resource fails the run, naming it (as GET routes and tools already do) | QA P2 |
| 3 | **`--out` is safer.** The generator deletes only what it wrote itself: it writes a marker file (`.gen_golden`) into each folder it creates and only removes a `golden*/` folder or `store-*.sqlite*` file that carries the marker or that it created in this run; a foreign `golden/` folder in `--out` makes the run stop with a clear message and delete nothing. The marker must not appear inside any committed oracle folder (keep it outside, e.g. next to the folders in `--out`, or in a `.gen_golden.json` at the `--out` root) | QA P2 |
| 4 | **Guards cover more.** Refuse `--out` and `--fixtures` equal to a real home folder (`$HOME`, the agent-root homes) and under a `.claude*` or `.easy-*` folder that does not exist yet. Re-run the existing refusals (`EASY_PERSONAL_HOME`, `EASY_PERSONAL_CLAUDE_PROJECTS`, `EASY_MEMORY_HOME`, `EASY_PERSONAL_UI_REPORTS`, `EASY_PERSONAL_VERIFIER_REPORTS`, `EASY_PERSONAL_AGENT_ROOT`, a symlink) against a fake `HOME` and paste the outputs. No run may touch the real home | QA P2, NFR-003 |
| 5 | **Install from the lock.** The README says how to install the venv from `uv.lock` (`uv sync --frozen`, or a constraints export) and why `pip install -e .` drifts (starlette 1.10.0 vs locked 1.9.0 and the like). The generator prints a one-line warning on stderr when an installed package version differs from `uv.lock` for starlette, anyio, sse-starlette or mcp; it never fails the run for it and the manifest does not change | QA P2 |
| 6 | **A fixture dependency is asserted.** The generator fails if the `kit_summary` fixture folder (`/work/demo-app`) exists on the machine, since the golden depends on it being absent | QA P3 |
| 7 | **Leak regex.** `sk-` is anchored so it no longer matches `task-`, `risk-` or `desk-` (word boundary or the real key shape); the run over the committed output still finds nothing and the committed goldens do not change | QA P3 |
| 8 | **Dead code and docs.** Remove the dead code the QA found (the no-op `re.sub`, the unreachable `/api/reports/{id}` special case, the unused `prompts = {}` and `- {"tools/list"}`, the unused `work` parameters, the stale "4-tuples" comment); use `lambda: []` for `suggestions.configured_ids`; fix the "Exit 2" docstring (it exits 1); note in the README that `pwd` and `time.tzset` make the generator POSIX-only. Then run `ruff format rust/parity/gen_golden.py` as the **last** edit and `ruff check rust/parity` | QA P3 |
| 9 | **Nothing changed in the output.** Fresh runs of the default, `--tz` and `--rich` modes (default environment and `PYTHONHASHSEED=3 LC_ALL=C TZ=America/New_York`) are byte-identical to the committed folders (`diff -r` and `cmp`); a combined sha256 of all oracle folders and both stores is identical before and after | RS-003 |
| 10 | `git status --short -- src web tests pyproject.toml uv.lock` is empty before and after; `cargo test` still passes (the Rust tests read the committed goldens) | Intent |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The oracle is the committed output itself: this task must not change it.
> **Locked oracle**: rust/parity/golden/manifest.json, rust/parity/golden-tz/manifest.json, rust/parity/golden-rich/manifest.json, rust/parity/store-v11.sqlite, rust/parity/store-rich/store-v11.sqlite

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | fresh runs of all three modes | byte-identical to the committed folders | `diff -r`, `cmp`, sha256 |
| 2 | a patched module whose patch missed | the run fails naming it | mutation |
| 3 | a foreign `golden/` folder in `--out` | the run stops, nothing deleted | command |
| 4 | `--out $HOME` and `--fixtures $HOME` (fake `HOME`) | refused, nothing opened | command |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the parity venv and the Rust toolchain on the owner's machine
cd rust/parity && PYTHONDONTWRITEBYTECODE=1 .venv/bin/python gen_golden.py --out "$S/d" && diff -r "$S/d/golden" golden && cd .. && . "$HOME/.cargo/env" && cargo test
```

---

## Approach

**Pattern reference**: the existing guards (`refuse`/`isolate` functions) and loud-failure checks in `gen_golden.py`.
**Vital slice**: criteria 1, 2 and 6 (loud failures) first; then 3 and 4 (safety); then 5 and 7; dead code, `ruff format` last.
**Cut list**: no package split; no new golden entry; no change to output; no new Python dependency.

Use only `PYTHONDONTWRITEBYTECODE=1` with the venv Python. Do every mutation on a copy, never on the committed files.

---

## Edge Case Checklist

- [ ] A mutation never leaves a changed committed file behind (compare the hash at the end)
- [ ] The refusal logic compares real paths (symlinks resolved) and a not-yet-existing parent
- [ ] The marker approach works on a second run into the same `--out` (it must still replace its own output)
- [ ] No absolute path, row text or secret in any output (NFR-002)
- [ ] Tests and probes never open `~/.easy-personal` or `~/.claude*`: use a fake `HOME`
- [ ] Stale references: `rust/parity/README.md` statements the changes make wrong

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/parity/gen_golden.py` | criteria 1–4, 6–8 |
| `rust/parity/README.md` | criteria 4, 5, 8 |
| `rust/parity/check_lock.py` | optional, if the lock check is easier outside the generator |
| Pre-existing pin tests | None |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `rust/parity/golden/**`, `golden-tz/**`, `golden-rich/**`, `fixtures-rich/**`, `store-v11.sqlite`, `store-rich/**` | committed oracles; byte-identical |
| `rust/crates/**` | T010 owns the core; no Rust change here |
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |

---

## Test Plan

Hash the oracles first. Make each change, re-run the three modes into a scratch `--out`, compare. Then the mutations (criterion 1), the foreign-folder case (3) and the fake-`HOME` refusals (4). End with `ruff format`, `ruff check`, a final three-mode comparison and the hash.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes
- [ ] Mutation and refusal outputs pasted into `tasks/TASK_REVIEW_T011.md`
- [ ] Combined hash identical before and after
- [ ] Supervisor notified: task ready for Stage 4 review
