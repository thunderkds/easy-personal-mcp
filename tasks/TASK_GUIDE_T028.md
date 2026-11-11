# TASK_GUIDE — T028: `claude_code` parse layer in Rust — lines, JSON, time, labels, signals, records
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
2. Read the memory slice in your spawn prompt; read the 2026-11-11 entry "Rust P2 ingest" in `memory/decisions.md` (note the T026 exception about Python's crashes)
3. Read this file completely
4. Read `agents/backend.md`
5. Read `BRAINSTORMING_LOG_RUSTP2.md` (facts and Edge Case Checklist), the "Ingest oracle" section of `rust/parity/README.md`, and the **findings section at the end of `tasks/TASK_REVIEW_T026.md`**
6. Read the Python you are matching, read-only, completely: `src/easy_personal/sources/claude_code.py`, `src/easy_personal/core/secretlike.py`; and for the shared pieces: `src/easy_personal/core/store.py` lines 150-220 (what `save_file` receives)
7. Read the Rust you plug into: `rust/crates/core/src/writer.rs` (the `Event`, `Signal` and `Line` types, `Store::save_file`), `rust/crates/core/src/lib.rs`, `rust/crates/core/Cargo.toml`
8. Read the oracle you are proven against: `rust/parity/ingest/fixtures/cases.json`, the synthetic transcripts under `rust/parity/ingest/fixtures/synthetic/`, and the goldens `rust/parity/ingest/golden/S01_*.json` and `S06_*.json` (the "tables_after.events" rows) plus the odd-case table in the T026 review
9. Do not open `~/.easy-personal`, any `~/.claude*` or `~/.codex*` folder, and do not write those paths in any command. Tests read only the committed fixtures and write only temp folders

> Read relevant files from `docs/legacy/` before making any code changes. (This project has no `docs/legacy/`; the files above stand in for it.)

---

## Requirement

**Original request** (owner, 2026-11-11, `[admin]`): build the Rust ingest (P2a). This task is the **parse layer only**: turn the bytes of one transcript file into the exact `Line`/`Event` values (`writer.rs`) that Python's `claude_code.py` would store, including every privacy rule. No folder discovery, no git project resolution, no store access, no CLI (T035 and T029).

**Restated intent**:
> A pure function takes the bytes of a transcript file from a start offset and returns, for each complete line, the line's offset, the next offset and either nothing (blank) or the event Python would store (an abstained event for anything it cannot trust), plus the safe tool signals, with no message text, tool output, prompt or secret ever kept; and it behaves identically to Python on every case Python handles, and never crashes on the four inputs on which Python crashes.

**Out of scope**: reading folders, `rglob`, accounts, `agent_account_id`, **`project_path` resolution through `git`** (the parse layer returns the record's raw `cwd` string; T035 resolves it), the `source_files` offsets logic, `kit_trace`/`ui`/`verifier`, the CLI, any store write, any change to `writer.rs`'s public API beyond what is listed in "Files to Change".

**Requirement Refs**: RS-003 (parity), NFR-002 (privacy), NFR-005 (speed and memory) in `rust/docs/SPEC.md`.

### Requirement Fidelity Gate

- [x] Restated intent confirmed by the Supervisor
- [x] Domain terms align with `rust/PROJECT_SPEC.md`
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs exist in `rust/docs/SPEC.md`

---

## Dependencies & Reachability

**Depends on**: T026 (goldens, fixtures, odd-case table), T027 (`writer::{Event, Signal, Line}`)
**Entry point**: `easy_personal_core::ingest::claude_code::read_lines` (used by T035/T029; reached by this task's tests)

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to requirement |
|---|----------------------|-----------------------|
| 1 | New module tree `rust/crates/core/src/ingest/` (`mod.rs`, `lines.rs`, `json.rs`, `time.rs`, `label.rs`, `secret.rs`, `signals.rs`, `record.rs`, or a layout you justify) exporting `claude_code::read_lines(bytes, start_offset) -> Vec<writer::Line>` (and a streaming variant that keeps memory bounded, see criterion 9). The existing public API of `writer.rs` is unchanged except that you may add what the types need | plugs into T027 |
| 2 | **Line framing = `read_complete_lines`** (`claude_code.py`): only a line that ends in `\n` is returned; a last partial line is **not** consumed and `next_offset` never moves into it; offsets are byte offsets; a blank or whitespace-only line (ASCII `bytes.strip()`) advances the offset and yields no event; CRLF lines parse normally. Proved against the committed fixtures, which all end with a truncated line | partial-line rule |
| 3 | **JSON = Python `json.loads` on bytes**, as the oracle shows: UTF-8 BOM accepted; **`NaN`, `Infinity`, `-Infinity` accepted** (a usage field holding them abstains the record with its type/version/ts/session kept, it does **not** make the whole line invalid); duplicate keys: the last wins; invalid JSON, a non-object JSON value, non-UTF-8 bytes and the UTF-16 line of the oracle give the all-NULL **abstained** event exactly as the goldens show. serde_json's defaults are **not** enough (NaN, recursion limit): write or wrap what you need. Document each place where you matched Python and the test that pins it | the oracle defines truth |
| 4 | **Integers**: `type(x) is int` semantics for the four usage fields: JSON integers only (a float such as `1.0`, a bool, a string, `null`, `[]`, `{}` are rejected); negative and int64 min/max are stored as given. **An integer beyond int64 (Python crashes, S06b): Rust treats it as an invalid usage value → the record abstains with type/version/ts/session kept, like any other invalid usage**, and the run goes on | S06b; **⚠ decided for you** |
| 5 | **Depth (Python crashes at very deep nesting, S06a)**: JSON nested up to the depth Python still parses must parse (a valid object with a deeply nested field is a normal row in Python, not an abstained one). Measure Python's real limit with the parity venv (`rust/parity/.venv/bin/python`, `json.loads` on bytes, same interpreter as the oracle), pin the number in a test and a comment, and treat anything deeper as an invalid line (all-NULL abstained event). Parsing must never overflow the Rust stack: use an explicit depth counter or an iterative parser, and test 100,000 nested `[` | S06a; **⚠ decided for you** |
| 6 | **Time**: `ts_utc` is the Python text `%Y-%m-%dT%H:%M:%S.%fZ` (exactly 6 fractional digits). Match `datetime.fromisoformat` of the oracle's interpreter **exactly** for every shape in the odd-case table (offsets `+07:00`/`-05:30`/`Z` normalised to UTC; 0/1/3/6 digits padded and 9 digits **truncated**, not rounded; comma decimal; ISO basic `20260110T100010Z`; space separator; hour-only and minute-only; ISO week `2026-W02-6`; `24:00` rolls to the next day; lowercase `z`, `:60`, date-only, empty, a number or a missing value abstain; a **naive** timestamp has no zone and abstains for versioned types). Prove it with a **differential corpus**: a new script `rust/parity/gen_ingest_corpus.py` (deterministic, run with the parity venv) writes `rust/parity/ingest/golden/corpus-time.json` (≥ 1,500 generated and hand-picked strings with Python's result or "invalid"); the Rust test compares every entry | time parity |
| 7 | **Labels and words**: `_LABEL = ^[A-Za-z0-9_.:@/+-]{1,200}$` **with Python's `$` quirk** (one trailing `\n` still matches and is stored with the newline; two do not); applied to `version`, `sessionId`, `gitBranch`, `entrypoint`, `model`, `message.id`, tool names and `record_type` exactly where Python applies it (a non-match becomes NULL, a 200-character label is kept, 201 is NULL, non-ASCII is NULL; `<synthetic>` model becomes NULL). Same for the Bash first-word rule (`_WORD`, basename, not secret-like) and the WebFetch host rule (http(s) only; port, path, userinfo, case and a trailing dot dropped; `127.0.0.1` accepted; `localhost`, other schemes, invalid and non-string give none). `corpus-label.json` (same script, ≥ 800 strings) pins `_LABEL`, `_WORD` and the secret regex against Python | label parity |
| 8 | **Secret rule** (`secretlike.SECRET`): `sk-`, `gh[pousr]_`, `github_pat_`, `xox*`, `AKIA`/`ASIA`, `-----BEGIN`, or a run of 32 or more alphanumerics, with Python's `\b` semantics (Unicode-aware on `str`). A value that matches never becomes a signal or a stored label. You may add the `regex` crate (MIT OR Apache-2.0, pinned `=` in `[workspace.dependencies]`) if it reproduces the corpus; say what you chose and why | privacy |
| 9 | **Record dispatch, byte for byte** with `claude_code.py:142-207`: non-dict → abstained; the 19 `METADATA_TYPES` keep only `record_type`, `version`, `ts_utc`, `session_id` and are never abstained; `VERSIONED_TYPES = {user, assistant, attachment, system}` need a valid UTC timestamp, a valid `session_id` and `cwd` absent or a string, else abstain; any other `type` abstains (**a `type` that is a list or another non-string, where Python crashes (S06d), abstains with `record_type` NULL and the run goes on**); `user` records get `tool_error_count` (number of `tool_result` blocks with `is_error is True`, else NULL when no `tool_result` block exists); `assistant` records follow the `message` / `usage` / `message.id` / four-exact-int rules, `model`, `message_id`, `is_api_error`, `tool_names` (comma-joined, NULL if none) and the signals; `is_sidechain` is `isSidechain is True`; `entrypoint` is kept as a label. The parse layer returns `project_path = cwd` as the raw string (T035 resolves it) | record parity |
| 10 | **Privacy**: nothing from message text, thinking, tool inputs or outputs, `toolUseResult`, attachments, `lastPrompt`, `customTitle`, `cost-state.totalCostUSD` or any other field can reach an `Event`: add a **canary test** that parses a line where every such field holds a unique `CANARY-…` string and shows no `Event` field and no `Signal` holds it, and a compile-level guard (the `Event` struct has no free-text field except the labelled ones). The planted fake secrets of the fixtures (`sk-FAKE…`, `AKIAFAKE…`) never appear in an `Event` | NFR-002 |
| 11 | **Oracle replay**: parse every fixture file of `S01` (committed `tests/fixtures/claude_code/`) and of `S06` (synthetic) with the offsets the goldens show, and compare each `events` row and `tool_signals` row to the golden **on every column except `project_path`, `agent_account_id`, `id`, `machine_id`, `inode`** (those come from T035). Equal row for row, including abstained rows and `tool_names`. Also `S03` (a line appended: parsing from the saved offset yields only the new line) and `S05` (a truncated line completed later). The four crash scenarios `S06a-d` are asserted as **"Rust differs"**: the stated abstained rows appear, the lines after them are parsed, and nothing panics | RS-003 |
| 12 | **Speed and memory (NFR-005)**: a streaming reader parses a 12 MB generated file with a peak memory under 8 MiB above the baseline (measure with `/proc/self/status` `VmHWM` in a child process, paste the numbers) and runs at ≥ 8 MB/s in a release build (paste the time of a 200 MB generated file, built with `--release`); report the real numbers even if they miss, with the profile | NFR-005 |
| 13 | `cargo test --workspace` passes under `TZ` unset, `Pacific/Kiritimati` and `Pacific/Pago_Pago` (paste all three); `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` clean, also with `--features window`; `Cargo.lock` additive: each new crate named with its licence | quality |
| 14 | **Mutation checks** (red run, then restore, files byte-equal): break the `$` quirk in `_LABEL`; accept only 3 fractional digits; let a `bool` count as an int; let `NaN` make the line invalid; let a `tool_result` text reach a field. Each makes a named test fail | the tests bite |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** at Stage 4/5 in `tasks/TASK_REVIEW_T028.md`.
> **Locked oracle**: `rust/parity/ingest/**` (T026) and `src/easy_personal/sources/claude_code.py`. The implementing agent must not edit them (the corpus script and its two new golden files are the only additions under `rust/parity/`).

### Success Criteria (observable, pass/fail)

| # | Given | Expect | How it's checked |
|---|-------|--------|------------------|
| 1 | the S01 and S06 files | every row equal to the golden (minus the T035 columns) | test |
| 2 | the time and label corpora | every entry equal to Python's answer | test |
| 3 | a line full of canaries | no canary in any `Event` field | test |
| 4 | 100,000 nested `[` | an abstained event, no panic | test |
| 5 | a 12 MB file | peak memory < 8 MiB above baseline | measured |

### Verification Command (exact, runnable)

```bash
# ci: skip — needs the Rust toolchain and, for the corpus script, rust/parity/.venv
. "$HOME/.cargo/env" && cd rust && cargo build && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check && ./parity/.venv/bin/python parity/gen_ingest_corpus.py --check
```

---

## Approach

**Pattern reference**: `rust/crates/core/src/writer.rs` (types, error style), Python `claude_code.py` and `secretlike.py`.
**Vital slice**: line framing and the record dispatch on the committed fixtures first (criteria 2, 9, 11 for S01) → JSON leniencies (3, 4, 5) → time (6) → labels, secrets, signals (7, 8) → S06 and the "Rust differs" cases → privacy canary (10) → speed and memory (12) → mutation checks (14).
**Cut list**: no folder walking, no `git`, no store, no CLI, no `kit_trace`/`ui`/`verifier`, no async.

Keep it pure: bytes in, `Line` values out. Do not allocate per field more than needed; do not copy a whole file into memory in the streaming reader.

---

## Edge Case Checklist

- [ ] The offset never advances into a partial last line; a file that ends exactly at `\n` yields no phantom line
- [ ] A blank line costs an offset step and no row; a whitespace-only line likewise
- [ ] `\r\n` endings, a BOM at the start of a **line**, duplicate keys, NaN and Infinity all behave as in the goldens
- [ ] An abstained line keeps **all** other columns NULL, except for the "abstained usage" case that keeps type, version, ts and session
- [ ] The session-entrypoint back-fill is **not** here: it happens in `writer.rs` (`save_file`) and abstained rows still receive it (T026 table)
- [ ] `tool_names` keeps every `tool_use` name that passes the label rule, including repeats; `tool_signals` are de-duplicated by the store, not here
- [ ] No panic on any byte sequence (fuzz: 10,000 random and mutated lines, paste the count and that none panicked)
- [ ] Integer parsing never loses precision silently (a float is a float, a huge int is "too large", not rounded)
- [ ] Stale references: `rust/README.md` "Ported library functions" list and `BRAINSTORMING_LOG_RUSTP2.md` task table (update the README only)

---

## Files to Change (Predicted)

| File | Change |
|------|--------|
| `rust/crates/core/src/ingest/*.rs` (new) | the parse layer |
| `rust/crates/core/src/lib.rs` | `pub mod ingest;` |
| `rust/crates/core/Cargo.toml`, `rust/Cargo.toml`, `rust/Cargo.lock` | only for a justified new crate (`regex`) |
| `rust/crates/core/tests/ingest_parse_*.rs` (new) | tests, replay, fuzz, canary, perf |
| `rust/parity/gen_ingest_corpus.py` (new), `rust/parity/ingest/golden/corpus-time.json`, `corpus-label.json` (new) | differential corpora (the only additions under `rust/parity/`) |
| `rust/README.md` | a short "Ingest parse layer (P2a)" paragraph |
| `tasks/TASK_REVIEW_T028.md` | Demonstration BEFORE only |

## Files Must NOT Touch

| File | Reason |
|------|--------|
| `src/**`, `tests/**`, `web/**`, `pyproject.toml`, `uv.lock` | the Python app (read-only oracle) |
| `rust/parity/**` except the three new files listed above | the oracle (T026) |
| `rust/crates/app/**`, `rust/crates/core/src/{writer,ingest_lock,store,metrics,rules,plan,scope,sessions,workspaces,prices,clock}.rs` | not part of this task (`writer.rs`: only if a type needs a field, and then say so) |
| `rust/spikes/**` | spikes |
| `PROJECT_SPEC.md`, any `PROJECT_KANBAN.md`, `rust/PROJECT_*.md`, `memory/**` | Supervisor-owned |
| `~/.easy-personal/**`, `~/.claude*`, `~/.codex*` | the owner's real data |

---

## Test Plan

Red first: the framing tests on the fixtures, the dispatch table from the odd-case table, the canary test, the corpus comparisons. Then green. Then S01/S06 replay, the "Rust differs" cases, fuzz, speed and memory, the three TZ runs, clippy, fmt, the mutation checks. Paste every output.

---

## Completion Checklist

- [ ] BEFORE filled in `tasks/TASK_REVIEW_T028.md` before the first code change
- [ ] Implementation done
- [ ] Self-review: `Skill({ skill: "code-review" })` run (if you have no Skill tool, say so)
- [ ] Lint passes (default and `--features window`)
- [ ] Tests written AND pass, red runs and mutation checks pasted
- [ ] Python's depth limit and each odd-case decision listed in the report
- [ ] Supervisor notified: task ready for Stage 4 review
