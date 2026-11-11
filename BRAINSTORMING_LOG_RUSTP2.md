# BRAINSTORMING_LOG_RUSTP2.md
**Generated**: 2026-11-11
**Task / Context**: Stage 0/2 — plan P2 (ingest) of the Rust port: the Rust app writes the store itself, so it no longer needs the Python app to fill it. Risk: High (first store writer, shared SQLite file, privacy rules). Tier: **Deep**.
**Skill**: `Skill({ skill: "brainstorming" })`
**Status**: LOCKED 2026-11-11 — owner chose the recommendation (Question 1: schema v11, Option A shaped as C). Q2-Q5 answered with Supervisor defaults in `[admin]` mode, flagged in `memory/decisions.md`.

---

## The Problem Space

Today the Rust app can only **read** a store that the Python app filled (P0/P1 closed 2026-11-11, see `rust/docs/closeout-2026-11-11.md`). P2 makes Rust able to **fill** it: read the Claude Code transcript files and write rows. Without P2 the Rust app cannot stand alone, and Python cannot be replaced.

What P2 has to reproduce (verified against the files, 2026-11-11):

- **One command**, `easy-personal ingest [--source PATH] [--retention-days N]` (`src/easy_personal/surfaces/cli.py:159-178`). Fixed order: `accounts.detect` → `claude_code.ingest_accounts` → `kit_trace` → `context` → `ui` → `verifier` → optional `prune`. Prints JSON (`indent=2, sort_keys=True`); a busy lock prints "another ingest is running; try again later" and exits **75**.
- **Single-writer lock**: `fcntl.flock(LOCK_EX|LOCK_NB)` on `<data_dir>/ingest.lock`, opened with mode `"a"`, 300 s default wait (`core/lock.py:20-36`). A Rust writer must use the same `flock` or the two implementations will not exclude each other.
- **Store writer** (`core/store.py`, 253 lines): `busy_timeout=5000`, `foreign_keys=ON`, `journal_mode=WAL`; **one transaction per file** (event inserts + `tool_signals` + the `source_files` offset together), so a kill resumes cleanly; the event upsert is `ON CONFLICT (machine_id, source, source_path, inode, byte_offset) DO UPDATE ... COALESCE ... WHERE ... IS NOT ...`.
- **Sources** (lines in Python): `claude_code` 303 (writes `events`, `tool_signals`, `source_files`), `kit_trace` 105, `ui` 85, `verifier` 162, `context` 234 (HTTP MCP client to easy-context), plus `accounts.detect` (~70 relevant lines). `git_local` is query-time only, not ingest.
- **Privacy rules** (the part that must never regress): never store message text, tool results, attachments, prompts or `totalCostUSD`; every string field must match `^[A-Za-z0-9_.:@/+-]{1,200}$` or become NULL; tool inputs are read only for the first word of a Bash command (not secret-like) and the host of a WebFetch URL; a `tool_result` contributes only an error count. The Python leak suite plants ~42 canaries and scans the store, WAL, CLI, MCP, API and logs.
- **Size**: about **1,740 Python lines** for the strict write path (four sources + store + 11 migrations + lock/schedule + config/discovery + accounts + CLI), about 2,700 with `context` and the continuous `live.py` writer (718 lines). About 3,300 lines of Python tests pin it.

### The trap the earlier plan did not see (verified)

`rust/PROJECT_SPEC.md` says "Python migrations frozen, new migrations in Rust only". But **Python refuses any store whose `user_version` is greater than 11, in every command** (`core/store.py:67-70`: `RuntimeError ... newer than this program`). So the **first Rust migration (v12) locks the Python app out of the store for good** (only the `<store>.bak-v11` backup brings it back). That is the real cutover moment, and it should be a decision, not a side effect of P2.

### Facts that shape the design

1. Python migrates on **every** open (reads, MCP, dashboard), not only ingest, and without the ingest lock.
2. The dedup key contains `inode`: a copied or restored transcript re-ingests as new rows (metrics survive because they dedupe by `message_id`; tool signals double count).
3. Sub-agent identity is **not stored** (`agent_id` appears 0 times in `src/`): only `is_sidechain`.
4. Project identity is `cwd` resolved through the `git` binary (`--git-common-dir`, `--show-toplevel`, 5 s timeout, `--path-format=absolute` needs git ≥ 2.31) and **baked into `events.project_path`**; any difference changes every project report.
5. Python-only semantics that Rust will not match by default: `$` in `_LABEL` also matches before a trailing newline; `fromisoformat` accepts more shapes than RFC 3339; `json.loads` accepts `NaN`/`Infinity` and auto-detects UTF-16/32; integers beyond int64 raise in the SQLite binding; `type(x) is int` rejects `bool`.
6. A **continuous second writer** exists: `core/live.py` (`LiveTail`, polls ~100 ms while the dashboard runs; holds the same lock with timeout 0 per batch and skips if busy).
7. Linux-only today: `flock`, systemd user timer, `st_ino`, 0700 data dir. macOS and Windows are new work (P4/P5).

---

## Questions for the User

**Question 1 (asked now, answer needed before any task guide):**
> Should P2 keep the store at schema **v11** (Rust writes into the existing tables, no new migration, so the Python app keeps working next to it), or may Rust start a **v12** (Rust owns the schema; Python can no longer open the store)?

*Recommendation:* keep **v11** for all of P2. It keeps the one-writer-at-a-time promise, keeps Python usable as the oracle and as a fallback, and turns the one-way cutover into its own later decision (P6).

**Queued (not asked yet; one question per turn; each has a default the owner may override):**
- Q2: Scope of first parity: events only (`claude_code` + `tool_signals` + accounts), or also `kit_trace`, `ui`, `verifier`? *Default:* events first (P2a), the rest as P2b. `context` (needs an HTTP/MCP client and `httpx`) and `live.py` are **out of P2**.
- Q3: May Rust **create** a brand-new store (embedding the final v11 schema), or must a store exist already (made once by Python)? *Default:* create it from an embedded v11 snapshot, never migrate an older store (refuse with "run the Python app once").
- Q4: Scheduler: a Rust `schedule install` (systemd user unit running `easy-personal-rs ingest`), or keep Python's unit for now? *Default:* later, P2c.
- Q5: Should Rust improve on Python's weak spots (inode in the key, no `agent_id`, same-size rewrite undetected)? *Default:* no; copy behaviour exactly, list the weak spots for P6.

---

## Alternative Paths

### Option A — Shadow writer, same v11 schema (**recommended**)
Rust adds `ingest` as a second writer of the **existing** v11 store: same lock file, same transaction shape, same upsert, same dedup key, same privacy rules, no migration. A brand-new store is created from an embedded v11 DDL snapshot.
- **Pros:** Python keeps working at any time; Python is the **oracle**: run both ingests over the same transcript folder and diff `events`, `tool_signals`, `source_files`, `kit_trace_events`, `ui_sessions`, `verifier_scores`; no cutover risk; rollback is "use the Python command".
- **Cons:** the Rust code must copy Python's quirks (the weak spots in "Facts" 2–5); cannot fix them until the cutover; two writers need the lock discipline and the coexistence tests.
- **Rough size:** 1,500–2,000 Rust lines plus tests (Python write path ≈ 1,740).

### Option B — Rust owns the schema (v12+, one-way)
Port the 11 migrations as Rust migrations and add new ones; Rust becomes the schema owner at the first run.
- **Pros:** room to fix the weak spots (add `agent_id`, drop `inode` from the key, content hash); one implementation to reason about.
- **Cons:** Python locked out at the first migration; no oracle after that; the backup copy is the only way back; the biggest blast radius; goes against "one writer at a time, Python stays usable".

### Option C — Minimalist: events-only ingest, first slice of A
Only `claude_code` → `events` + `tool_signals` + `source_files` and `accounts.detect`, plus `prune`. No `kit_trace`, `ui`, `verifier`, `context`, scheduler or live tail.
- **Pros:** about 40% of the code, covers the data that every report is built on (usage, warnings, plan, sessions); the smallest thing that lets the Rust app stand alone for usage.
- **Cons:** kit panel, UI sessions and verifier scores stay empty until the next slice; the Rust app is still not a full replacement.

Rejected: calling the Rust ingest from Python (pyo3) or the other way round: it keeps both runtimes in one process and contradicts "Rust standalone".

---

## 50% Rule Check

Same goal with half the code: **do not port `context` (234 lines + an HTTP/MCP client) or `live.py` (718 lines) in P2**; do not port the schema migrations (embed one v11 snapshot, refuse older stores); do not port `schedule` yet (the existing systemd unit can call the Rust binary later). That alone is about 55% less than the 2,700-line total, and it is Option C's shape. Reuse what exists: the six committed Python fixture files with their truncated last line, and the leak suite as the gate, instead of writing new oracles.

---

## Adversarial Review (why each path might fail)

| Option | Why it might fail |
|---|---|
| A | Two writers: a Rust writer that skips the lock, or uses `fcntl` (POSIX) locks that do **not** conflict with Python's `flock`, silently corrupts nothing but loses exclusion; a drift in `project_path`, UTC text format (exactly 6 fractional digits, text comparison in prune) or `_LABEL` semantics makes the two writers disagree on the same file; the inode in the key duplicates rows across the two writers if a file is copied. |
| B | The first migration is irreversible for Python users; any bug in the Rust migrator happens on the only copy of the owner's history (the backup is the single safety net); no oracle remains to prove parity. |
| C | Looks done but the Rust window shows empty kit/UI/verifier views; owners may think P2 is finished. Must be labelled P2a. |

---

## Recommended Path

**Option A, delivered in the shape of Option C first** (P2a), then the remaining sources (P2b), each slice gated by the Python oracle and the leak suite. Everything at schema v11; **no Rust migration in P2**; the cutover to v12 stays a separate owner decision at P6.

Proposed task order (drafts; guides are written only after Question 1 is answered):

| # | Task | Agent | Risk | Gate |
|---|---|---|---|---|
| T026 | Ingest oracle: Python generator runs `easy-personal ingest` over the committed fixtures plus new rich transcripts and dumps the tables (rows, not ids) as goldens under `rust/parity/` | QA | Medium | goldens equal Python's tables |
| T027 | Rust store writer: open for write (WAL, `busy_timeout=5000`, `foreign_keys=ON`), `flock` on `ingest.lock`, read `meta.machine_id`, one transaction per file, the exact conditional upsert, refuse `user_version != 11` | Backend | **High** | tests + coexistence with Python |
| T028 | `claude_code` parser: record dispatch, `_LABEL` with Python's `$` quirk, exact-int rule, UTC normalisation, tool signals, secret regex, git project resolution with the same flags and timeout | Backend | **High** | rows equal the goldens; unversioned/truncated/malformed lines abstain |
| T029 | `easy-personal-rs ingest` CLI: `accounts.detect`, `ingest_accounts`, JSON shape and key order, exit 75 on a busy lock, `--source`, `--retention-days` (text-compare prune) | Backend | Medium | output equals Python's JSON on the fixtures |
| T030 | Leak suite on the Rust ingest (as built: `rust/parity/leak_rust.py` imports the Python suite's canaries and fixtures read-only and drives the Rust binary; no change in `tests/`): canaries absent from store, WAL/SHM, output, logs | QA | **High** | zero canary hits, anti-vacuity guards pass |
| T031 | Coexistence tests: Rust and Python ingesting the same folder concurrently and one after the other; kill and resume; copied file; `integrity_check`; no duplicate keys | QA | **High** | identical tables, no corruption |
| T032 | Perf and memory gates: 8 MB/s on 200 MB, unchanged re-run < 2 s, < 8 MiB for a 12 MB file (NFR-005) | QA | Medium | numbers pasted |
| T033 (P2b) | `kit_trace`, `ui`, `verifier` sources (replace-all semantics, size and symlink limits) | Backend | Medium | goldens + leak canaries |
| T034 (P2c, optional) | `schedule install` for the Rust binary; dashboard start-up ingest | Infra | Medium | after owner Q4 |

`context` and `live.py` stay out until the owner asks.

---

## Surgical Scope

**Should change** (new code): `rust/crates/core/src/` (new `ingest/` module set, store writer), `rust/crates/app/src/main.rs` (one `Ingest` subcommand), `rust/parity/` (ingest generator and goldens), `rust/parity/leak_rust.py` (the leak gate; `tests/` is not changed), `rust/README.md`.

**Must not touch:** `src/easy_personal/**` (Python stays the oracle), `src/easy_personal/core/migrations/**` (frozen at 11, and no Rust migration in P2), `web/**`, the owner's real `~/.easy-personal` and `~/.claude*` (tests use temp folders and the committed fixtures only), `tests/**` other than the single leak-harness hook (needs the owner's nod).

---

## Edge Case Checklist for TASK_GUIDEs

- [ ] A second ingest (Python or Rust) while one runs: wait up to the timeout, then exit **75** with the same message; never two writers.
- [ ] Use `flock`, not POSIX `fcntl` locks; open `ingest.lock` for append and never delete it.
- [ ] Never run a migration; `user_version > 11` → refuse (exit 2 like the reader), `< 11` → refuse with "run the Python app once"; a brand-new empty store → create the v11 schema from the embedded snapshot **and set `user_version = 11`**.
- [ ] `machine_id` is read from `meta`, never regenerated; `source_path` is the un-resolved path string exactly as Python's `rglob` yields it; files processed in sorted order.
- [ ] One transaction per file: events, entrypoint back-fill, `tool_signals` and the `source_files` offset commit together; the offset only ever sits at a line start; a half-written last line is not consumed.
- [ ] Blank line advances the offset and stores nothing; invalid JSON or a non-object stores an **abstained** row, never a crash; a deeply nested line and an integer beyond int64 must not take the run down (Python behaviour for those is untested: decide and pin).
- [ ] UTC text exactly `%Y-%m-%dT%H:%M:%S.%fZ` (6 digits); a timestamp without a timezone abstains for versioned records; offsets like `+07:00` normalise to UTC.
- [ ] `_LABEL` parity including the trailing-newline quirk; `type(x) is int` (reject bool and float) for the four usage fields.
- [ ] Privacy: nothing from messages, tool results, attachments, prompts, `toolUseResult`, `totalCostUSD`; Bash first word only if not secret-like; WebFetch host only; the leak suite is the gate.
- [ ] Project resolution through the `git` binary with the same flags, 5 s timeout, `GIT_OPTIONAL_LOCKS=0`, cached per cwd per run; a missing folder keeps `cwd`.
- [ ] `agent_accounts` FK enforced: `foreign_keys=ON` on **every** connection; `INSERT OR IGNORE` for detected accounts; only `provider='claude' AND status='confirmed'` accounts are read; back-fill of `agent_account_id IS NULL` rows each run.
- [ ] Replace-all sources (P2b): a failed or empty run must not silently wipe the table unless Python does the same in the same case.
- [ ] Retention prune compares text; NULL `ts_utc` rows are kept; offsets are not touched.
- [ ] Output JSON equals Python's: indent 2, sorted keys, same key set.
- [ ] Tests never open `~/.easy-personal` or `~/.claude*`; the committed fixtures end with a truncated line on purpose.
- [ ] WSL `/mnt` drives: `flock` may not be reliable (unverified): tests use the Linux filesystem.

---

## Next Actions

1. Owner answers Question 1 (v11 or v12). If v11: the Supervisor asks Q2–Q5 one at a time, locks the path in `memory/decisions.md`, and writes TASK_GUIDE_T026–T032 (P2a) with the checklist above.
2. Add the P2 phase rows and the tasks to `rust/PROJECT_KANBAN.md` (Gate 4: Rust board only).
3. First spawn: T026 (the oracle) alone; nothing else may start before the goldens exist.
4. Decide at P6, not now: the v12 cutover, and fixes for the inode key and `agent_id`.

## User Selection

Owner, 2026-11-11: "as your recommend, let start and work for the final release [admin]" → **Option A shaped as Option C first; schema stays v11; no Rust migration in P2.** Q2 events first, P2b for the other sources, `context` and `live.py` out; Q3 Rust may create a new store from an embedded v11 snapshot and refuses older/newer stores; Q4 scheduler later (P2c); Q5 copy Python exactly. **⚠ Q2-Q5 decided for you — please review.**
