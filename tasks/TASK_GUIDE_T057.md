# TASK_GUIDE — T057: Rust live feed, part 2: kit-trace feed and the window app showing the live view
**Date**: 2026-11-13
**Complexity Level**: C3
**Risk Level**: High
**Priority**: P1
**Assigned agent**: backend-developer
**Agent guide**: `agents/backend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md` and `rust/PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt; read the entry "Rust live feed: Path 1" in `memory/decisions.md`
3. Read this file completely
4. Read `agents/backend.md`
5. Read `tasks/TASK_GUIDE_T056.md` and `tasks/TASK_REVIEW_T056.md` (what T056 built: `LiveFeed`, `GET /api/live`, the guard carve-out) and `rust/README.md` sections "Live tail engine (T049)" and the T056 section
6. Read the Python you copy, **read-only**: `src/easy_personal/sources/kit_trace.py`, `src/easy_personal/core/kit.py` (what a kit trace row holds), `src/easy_personal/core/live.py` (the kit stage, `STAGE_MAP`, FR-083) and `web/src/LiveView.jsx` (what the Live tab shows today)
7. Read the Rust you extend: `rust/crates/app/src/{live_feed,server,api,main,window}.rs`, `rust/crates/core/src/{live,kit,ingest/kit_trace}.rs` (whatever exists), `rust/crates/app/tests/live.rs`
8. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests use temp folders and fixtures you create there

**Step 0 (stop rule).** Before any code, write in `tasks/TASK_REVIEW_T057.md` (Demonstration BEFORE) what "kit-trace feed" means in Python today: which file produces it, which route or page shows it, which fields. The decision text says "task id, tool name, ok/error, time; never command text". If Python has no live kit-trace feed to copy (the kit stage in the snapshot already comes from the T049 engine), **stop and report** with a one-paragraph proposal; do not invent a contract. If it exists, copy it.

---

## Requirement

**Original request** (owner, 2026-11-13): "the live from the window application does not show anything", then "run 56 and 57 now". Direction locked 2026-11-12: Path 1, publish-only; T057 = kit-trace feed plus app wiring, depends on T056.

**Restated intent**:
> With the window app (`easy-personal-rs app`) open, the Live tab shows what the Claude Code sessions are doing now, including the kit-trace steps as Python shows them, from the same feed as `serve`. Nothing in the feed carries command text, message text or tool output.

**Out of scope**: the T056 stream contract (change it only if Step 0 proves it must, and then stop and report), an always-on collector, store writes, `config.yaml` thresholds, web page changes, Python files.

**Requirement Refs**: FR-043..FR-049, FR-083, FR-086 (`PRD.md`); DDR-0004; RS-001, RS-003, NFR-001, NFR-002 in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist

---

## Dependencies & Reachability

**Depends on**: T056 (merged on `main` before this task starts)
**Entry point**: the `app` command (`rust/crates/app/src/window.rs`, `main.rs`) and `GET /api/live`

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | **Step 0 recorded** in `tasks/TASK_REVIEW_T057.md` before the first code file (see above) | — |
| 2 | **Kit-trace feed**: the Rust feed publishes the kit-trace items exactly as Python does (same fields, same order, same bounds). Each item holds a task id, a tool name, ok or error, and a time; never command text, message text or tool output (a test plants a canary in every text field of the fixture trace and scans the stream: 0 hits) | FR-083, DDR-0004 |
| 3 | **App wiring**: `app` starts the `LiveFeed` exactly as `serve` does and stops it when the window closes (exit within 10 s, code 0, thread joined). The window's page opens `/api/live` with the token in the query (T056's carve-out) and receives a `snapshot` then `live` events. Test without a display: the `app --self-test` path (existing) is extended to open `/api/live` and read the first `snapshot`; the display test runs only when `DISPLAY` or `WAYLAND_DISPLAY` is set and says "skipped: no display" otherwise | FR-043 |
| 4 | **Oracle**: a fixture kit-trace tree in a temp folder is fed to Python's code and to Rust; the message sequences for the kit items are equal (cases: a run with ok calls, a run with an error, an empty trace, a trace file that shrinks, a trace with a hostile name). Golden in `rust/parity/golden-live-kit/` made once by a generator `rust/parity/gen_live_kit.py` | FR-083 |
| 5 | **Publish-only and bounded**: no store write, no lock, trace files read with the same size and symlink limits as the T033 `kit_trace` ingest; a trace file over the limit is skipped, not read | NFR-002 |
| 6 | **Leak and regression**: `parity/leak_rust.py` covers the kit items (0 hits); three `TZ` runs, clippy x2, fmt, `git status -- src tests web` empty, `Cargo.toml` and `Cargo.lock` unchanged, existing tests and goldens pass | RS-003 |
| 7 | **End-to-end check by the Supervisor** (not an agent): with a window build on the owner's machine, a running Claude Code session shows in the Live tab. The agent states "not checked, needs a display" if it cannot | — |
| 8 | **Docs**: `rust/README.md` (kit-trace feed, the app wiring, "decided for you", "not served yet" lists) | — |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T057.md`.
> **Locked oracle**: `golden-live-kit/` made once by Python.

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | each oracle case | Rust kit items equal Python's | the parity test |
| 2 | `app --self-test` (window build) | opens `/api/live`, reads the first `snapshot`, exit 0 | the self-test |
| 3 | a canary in every text field of the fixtures | 0 hits on the stream | the leak run |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace --no-fail-fast && ./parity/.venv/bin/python parity/leak_rust.py && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --features window -- -D warnings && cargo fmt --check
```

**Reviewer note**: High risk (data from agent traces reaches a browser stream). Stage 4 includes an independent security review focused on what reaches the stream.

---

## Approach

**Vital slice**: Step 0, then the kit item in the engine's output with a red test, then the `app` wiring and self-test, then the oracle, leak and docs.
**Cut list**: no new route, no store write, no page change, no history beyond the engine's buffer.

---

## Edge Case Checklist

- [ ] A trace with a very long tool name, unicode, NUL, a name that looks like a path: bounded and plain, as Python
- [ ] A trace file that is a symlink out of its root, a FIFO, a file over the limit: skipped, never opened
- [ ] The window closes while a stream is open: exit within 10 s
- [ ] `app` started with no store: empty snapshot, no crash
- [ ] Time uses the injected clock; three `TZ` values
- [ ] `leak_rust.py` and `tests/mcp.rs` lists updated by a few lines only

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/live.rs` or a new `live_kit.rs`, `rust/crates/app/src/{live_feed,window,main}.rs` | the kit items; the `app` wiring |
| `rust/crates/app/tests/live.rs` (extend), `rust/crates/core/tests/live_kit_parity.rs` (new) | oracle, wiring, self-test |
| `rust/parity/gen_live_kit.py`, `rust/parity/golden-live-kit/` (new); `rust/parity/leak_rust.py` | the oracle, the leak registry |
| `rust/README.md` | docs |
| `tasks/TASK_REVIEW_T057.md` | Demonstration BEFORE (Step 0) |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app |
| existing `rust/parity/golden*/**`, `store-rich/**`, `fixtures-rich/**` | the oracle |
| `rust/Cargo.toml`, `rust/Cargo.lock` | no new dependency without a stop-and-report |
| `skills/**`, `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**`, `docs/release/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Red test per criterion, then green; oracle; leak gate; three `TZ` runs (`unset`, `Pacific/Kiritimati`, `Pacific/Pago_Pago`: `cargo test --workspace --no-fail-fast`).

---

## Completion Checklist

- [ ] Step 0 and BEFORE filled in `tasks/TASK_REVIEW_T057.md` before the first file is created
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Oracle counts, three `TZ` runs and the leak line pasted
- [ ] "Decided for you" list written
- [ ] Supervisor notified: task ready for Stage 4 review
