# TASK_GUIDE — T003: Parity oracle — golden JSON from the Python app
**Date**: 2026-11-10
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P0
**Assigned agent**: backend
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:
1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read `memory/MEMORY.md` and the 2026-11-10 "Rust port" entry in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTPORT.md` (parity edge cases) and `rust/docs/SPEC.md` (RS-003)

> Read relevant files from `rust/docs/` before making any code changes.

---

## Requirement

**Original request** (owner, 2026-11-10): a Rust version that runs all features; Python replaced later. Run independent tasks in parallel.

**Restated intent**:
> Capture what the Python app answers today, for every read-only `/api/*` route and MCP read tool, over the existing test fixtures, as frozen golden JSON files plus a generator script. The Rust build is later proven against these files (RS-003). Python is only read, never changed.

**Out of scope**:
- Any Rust code (needs T001/T002).
- Any change to `src/`, `web/`, `tests/`, `pyproject.toml`.
- Write routes (POST/PUT/PATCH/DELETE), the SSE route `/api/live`, and anything that touches the real machine (real `~/.claude*`, real keychain, `claude` CLI, network). Each skipped route is listed with its reason.
- Real data: only the repo's fixtures under `tests/fixtures/claude_code`.

**Requirement Refs**: RS-003 (`rust/docs/SPEC.md`); Python `PRD.md` NFR-013 (read shapes), NFR-003 (never open the real store).

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: None
**Entry point**: `rust/parity/gen_golden.py` (run with the venv's Python)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|----------------------|-----------|
| 1 | A venv at `rust/parity/.venv` (gitignored) runs the Python app's own tests: `pytest tests/surfaces/test_api.py -q` passes there. No global `pip install`, no `sudo` | Intent |
| 2 | `rust/parity/gen_golden.py` builds a temp store by running the Python ingest over `tests/fixtures/claude_code`, then writes one JSON file per read-only `/api/*` GET route and per MCP read tool to `rust/parity/golden/` | RS-003 |
| 3 | `rust/parity/golden/manifest.json` lists each golden file with: route or tool, request parameters, HTTP status, the Python and dependency versions, the store `user_version`, and a SHA-256 of the fixture tree. Skipped routes and tools appear with a reason | RS-003 |
| 4 | Output is deterministic: running the generator twice gives byte-identical files (sorted keys, 2-space indent, trailing newline). Any field that varies per run (clock, absolute path, random id) is listed in the manifest under `volatile` and normalised to a fixed token, never silently dropped | RS-003 |
| 5 | The generated store is also saved as `rust/parity/store-v11.sqlite` (the number is read from the migrations list, not hard-coded), small enough to commit (< 5 MB), for the Rust tests to open read-only | RS-002 |
| 6 | No absolute path, secret-like text, or real home path appears in any golden file (grep check in the verification command) | NFR-002 |
| 7 | Negative: the generator refuses to run if `EASY_PERSONAL_HOME` points at the real `~/.easy-personal`, and never opens it | NFR-003 |
| 8 | `git status --short -- src web tests pyproject.toml` is identical before and after | Critical Constraints |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | generator run twice | `diff -r` of the two output trees is empty | command |
| 2 | manifest | every `/api` GET route in `src/easy_personal/surfaces/api.py` is either a golden entry or a skipped entry with a reason | script check |
| 3 | `grep -rE '/home/|/Users/|sk-|ghp_' rust/parity/golden` | no match | command |
| 4 | `EASY_PERSONAL_HOME=$HOME/.easy-personal` | the generator exits non-zero without opening it | command |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs network for the venv and a local Python
cd rust/parity && .venv/bin/python gen_golden.py --out /tmp/t003-a >/dev/null \
  && .venv/bin/python gen_golden.py --out /tmp/t003-b >/dev/null \
  && diff -r /tmp/t003-a /tmp/t003-b \
  && ! grep -rE '/home/|/Users/|sk-|ghp_' golden \
  && .venv/bin/python -m pytest ../../tests/surfaces/test_api.py -q
```

---

## Approach

**Pattern reference**: `tests/surfaces/test_api.py` (the `home` fixture: `EASY_PERSONAL_HOME`, `EASY_PERSONAL_CLAUDE_PROJECTS`, `claude_code.ingest`, `TestClient(api.create_app(TOKEN, PORT, ...))`) and the existing golden files `tests/surfaces/hun186_golden.json`, `scope_golden.json` with their fixtures — imitate how they pin output. Also `tests/conftest.py` (the guards that stop a test opening the real store).
**Vital slice**: the usage routes (`/api/usage/summary`, `breakdown`, `outliers`, `sessions`), `/api/warnings`, `/api/plan`, `/api/kit` and the MCP tools `usage_summary`, `warnings`, `kit_summary`, plus the manifest. Add the rest of the read-only routes after the slice works.
**Cut list**: no Rust comparison harness (a later P1 task); no CI wiring; no write routes; no SSE; no new fixtures.

Build the generator as a plain script that imports `easy_personal` from `src/` (no change to the app). Pin time with explicit `since`/`until` parameters, and patch the clock only if a route has no parameter for it. Send the token header and `Host: 127.0.0.1:<port>` exactly as the tests do.

---

## Edge Case Checklist

- [ ] Floats: keep Python's exact output (no re-rounding); record the raw JSON text, not a re-serialised float
- [ ] Key order and `null` vs missing are preserved as the app sends them; only the file-level formatting is normalised
- [ ] A route that needs machine state (`/api/dirs`, `/api/sync`, `/api/connectors`, `/api/jobs`) → skip with a reason, or run it only with the fixtures and monkeypatched env
- [ ] Local-day boundaries depend on the machine timezone → fix `TZ=UTC` in the generator and record it in the manifest
- [ ] Python 3.14 on this machine vs the project's `>=3.11`: record the version; if a dependency has no wheel, report it, do not downgrade silently
- [ ] The venv and `target/` dirs are gitignored; golden files and the store are not
- [ ] Stale references: after adding `rust/parity/`, check whether any Python test scans the repo tree and counts files (report it, do not edit)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/parity/gen_golden.py` | new — the generator |
| `rust/parity/README.md` | new — how to build the venv and run it, one screen |
| `rust/parity/golden/*.json`, `rust/parity/golden/manifest.json` | new — generated output |
| `rust/parity/store-v11.sqlite` | new — generated fixture store |
| `.gitignore` | add `rust/parity/.venv/` |
| Pre-existing pin tests | None expected — report any test that scans the repo tree |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `web/**`, `tests/**`, `pyproject.toml`, `uv.lock` | the Python app is the oracle; read-only |
| `~/.easy-personal/**`, `~/.claude*` | the owner's real data (NFR-003) |
| `PROJECT_SPEC.md`, `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `rust/docs/spike-T001.md`, `rust/spikes/**` | T001 is running in parallel |

---

## Test Plan

Run the Verification Command. Then mutate one fixture value in a temp copy and confirm the generator's output changes in the matching golden file (proof the golden files are not constant). Finally run `git status --short -- src web tests pyproject.toml` and compare it with the value recorded first.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run
- [ ] Lint passes (`ruff check rust/parity` in the venv)
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_T003.md`
- [ ] Run the generator and confirm output files exist
- [ ] `memory/MEMORY.md` updated (Supervisor)
- [ ] Supervisor notified: task ready for Stage 4 review
