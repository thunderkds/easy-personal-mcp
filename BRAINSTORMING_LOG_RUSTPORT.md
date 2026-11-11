# BRAINSTORMING_LOG_RUSTPORT.md
**Generated**: 2026-11-10
**Task / Context**: Stage 0 — Rust version of easy-personal-mcp (owner-agreed Option B: staged port), with a plan to replace Python later. Platforms: Ubuntu first, macOS next, Windows last. Risk: High. Tier: Deep.
**Skill**: `Skill({ skill: "brainstorming" })`

---

## The Problem Space

The owner wants an **application version written in Rust** that runs *all* current features, and, where possible, to **replace the Python implementation over time**. Today the product is Python 3.11 (`src/easy_personal`, ~9.7k lines, 57 files) plus a React/Vite dashboard (`web/`, ~14.8k lines), with 94 FRs and 15 NFRs in `PRD.md`, and ~22.9k lines of Python tests.

Non-negotiable constraints (all verified in the repo):
- **Same data.** The store is SQLite at `~/.easy-personal/store.sqlite`, WAL, `foreign_keys=ON`, one writer, versioned by `PRAGMA user_version` with 11 numbered migrations (`core/migrations/m0001..m0011`). The Rust build must open the same file; it must never corrupt it or run a migration the Python side can't follow.
- **Same contracts.** 28 `/api/*` routes (Starlette), the MCP tools (`FastMCP`: usage_summary, usage_breakdown, warnings, kit_summary, kit_findings, save_report, …), the `easy-personal` CLI subcommands, and the `exec` launcher.
- **NFRs that bind a port**: NFR-001 loopback only; NFR-006 launcher < 200 ms and only the OS keychain as a dependency; NFR-005 ingest speed; NFR-013 `/api` reads < 2 s.
- **Today the Python code is Linux-only in practice.** No `sys.platform` branching exists; scheduling is `systemctl --user` (`core/schedule.py`, `data/systemd/*`); secrets use the Secret Service via `keyring`. macOS and Windows are real new work, not a recompile.
- **Pipeline rule (Gate 4)**: a different language and deployment target gets its own `PROJECT_KANBAN.md` and `PROJECT_SPEC.md`.
- The Rust toolchain is not installed on this machine yet.

## Questions for the User

Asked one at a time. Open:

1. ~~What is "the application"?~~ **Answered 2026-11-10: a desktop window (GUI).** The window shell is in scope; the CLI and the MCP server stay in the same binary.
2. ~~Share one store during the transition?~~ **Answered 2026-11-10: yes.** One store file, one writer at a time (the existing lock), Python migrations frozen, new migrations in Rust only.
3. Is the Python app retired for everyone at the end, or kept as a supported fallback for a release or two? *(default taken: deprecate for one release, then remove; owner can override)*
4. Who are the Windows users (soon, or only "later")? *(default taken: Windows is last, P5, as the owner said)*
5. ~~Where does the Rust project live?~~ **Answered 2026-11-10: first a sibling folder, then reversed by the owner the same day: "should stay in this repo".** Final: `rust/` in this repo, with its own `rust/PROJECT_KANBAN.md` and `rust/PROJECT_SPEC.md`; guides in the root `tasks/`.

---

## Alternative Paths

| Option | Name | Summary | Invasiveness | Code Volume | Regression Risk | Recommended? |
|--------|------|---------|-------------|------------|----------------|--------------|
| A | The Simple Path | Rust/Tauri shell around the existing React page; Python app runs as a hidden helper process | Low | ~2k lines Rust | Low | |
| B | The Scalable Path | Staged native port: Rust core replaces Python slice by slice, same store, same API and MCP contracts, Python kept as parity oracle, then retired | Medium (new project, Python untouched) | ~10–14k lines Rust | Medium, bounded by parity tests | ✅ Yes (owner agreed) |
| C | The Minimalist Path | Big-bang rewrite, switch once everything exists | High | ~10–14k lines Rust | High | |

### Option A — Tauri shell + Python helper
**Approach**: Rust owns only the window, the version, and starting the Python app.
**Pros**: Fastest; gives an installable app and a version early.
**Cons**: The logic is still Python, so it isn't a Rust version; the app then ships two runtimes; Windows packaging of Python is painful.
**Why it might fail**: It meets "an application" but not "written in Rust" or "replace Python", so the owner's real goal stays open.

### Option B — Staged native port (strangler)
**Approach**: A new Rust project under `rust/` delivers vertical slices. Each slice goes live only when it passes parity against Python on the same fixtures.
**Pros**: Always shippable; Python is the oracle; risk stays per slice; a single static binary is the end state.
**Cons**: Two implementations live at once; every contract detail (JSON shape, rounding, timestamps) must match.
**Why it might fail**: Subtle drift (float rounding, key order, local-day boundaries, abstain counts); two writers on one store; scope creep from porting the 14.8k-line UI (it should not be ported, only served).

### Option C — Big-bang rewrite
**Approach**: Build the whole thing, then switch.
**Pros**: No dual maintenance.
**Cons**: Nothing runnable for months; no early feedback; hardest to review.
**Why it might fail**: The 94 FRs drift while it is being built, and the first real run finds everything at once.

---

## 50% Rule Check

Same goal with half the code: do **not** port the dashboard (serve the existing built React bundle from the Rust binary, embedded), do not port the connector catalog logic twice (keep `catalog/*.yaml`, `prices.yaml`, `rules.yaml` as shared data files read by both), and port **read-only surfaces first** — usage, warnings, kit, memory graph, MCP read tools — since they are the bulk of day-to-day use and need no writer coordination. Writes (accounts, workspaces, connectors, sync, schedule) come after. This roughly halves the first release.

---

## Recommended Path

**Option B — staged native port, with Python kept as the parity oracle and a defined sunset.**

### Phases (each phase = one or more TASK_GUIDEs on the new board)
| Phase | Delivers | Gate to pass |
|-------|----------|--------------|
| P0 Foundation | Cargo workspace; CI on Ubuntu; `--version` (CLI, `/api`, MCP server info, dashboard footer); open the store read-only and report `user_version` and table counts | Builds on Ubuntu; refuses a store with a newer `user_version` than it knows |
| P1 Read core | usage, breakdown, outliers, warnings/rules, plan, kit, memory graph; `/api` GET routes; embedded React bundle; MCP stdio read tools | Golden-file parity vs Python on `tests/fixtures`; NFR-013 |
| P1b Window shell | Desktop window around the dashboard, version in the title bar and About; the window loads the page from the in-process server on `127.0.0.1` with the existing Host/Origin/token guard (no new trust boundary). Packages: `.deb` / AppImage on Ubuntu first | App opens on Ubuntu; NFR-001 and NFR-008 still hold; closing the window stops the server |
| P2 Ingest | `claude_code`, `kit_trace`, `git_local`, `context`, `verifier` sources; store writer behind the single-writer lock | NFR-005; leak suite (planted secrets never reach store, MCP, CLI or logs) |
| P3 Write surfaces | workspaces, accounts, connectors, sync (`claude mcp add-json --scope user`), keychain, `exec` launcher, in-binary scheduler, `save_report` | NFR-006 (< 200 ms); DDR-0001 write rules kept |
| P4 macOS | launchd scheduling; Keychain via the keyring crate; release binary | Same parity suite on macOS |
| P5 Windows | Task Scheduler; Credential Manager; paths | Same parity suite on Windows |
| P6 Python sunset | See below | Per-feature flip criteria met |

### Plan to replace Python (owner request)
0. **Forced replacement is allowed** (owner, 2026-11-10: "if forced, we can replace the python"). If running both becomes impractical (a contract that cannot be matched, a dependency that blocks one side, maintenance cost), Python may be replaced earlier than P6, one surface or all of it. It needs its own TASK_GUIDE naming what goes and why, and the owner's confirmation. The steps below are the default, not a limit.
1. **Flip per surface, not all at once.** A surface (CLI command, API route group, MCP tool, source, connector write) flips from Python to Rust once it passes parity and has run clean for one release.
2. **Migrations are frozen on the Python side** at the last numbered migration at P0. New migrations are made in Rust only, continuing the number sequence. Rollback relies on `bak-v<N>` backups taken before any migration.
3. **Single writer.** Only one implementation writes the store at a time, enforced with the existing lock (`core/lock.py`) and a per-surface switch.
4. **Python becomes maintenance-only** once P3 is complete and macOS is released: security fixes only, no new features.
5. **Deprecate for one release**, then remove the Python package (`src/`, `pyproject.toml`) and its tests from the main line. The Python test suite is archived as the parity corpus, not deleted.
6. **Shared data stays shared** (`catalog/*.yaml`, `data/*.yaml`, PRD) until the sunset; then it moves to the Rust repo.

### Stack candidates (to confirm in the P0 spike, not decided here)
Rust 2021+ on stable; a single async HTTP server for `/api` and static files; `rusqlite` (bundled SQLite, same WAL settings); a YAML crate that is still maintained (verify — the original `serde_yaml` is archived); the `keyring` crate; an MCP SDK for Rust (verify maturity and spec version against `FastMCP` behaviour before relying on it, with a hand-rolled stdio JSON-RPC as the fallback); `clap` for the CLI. For the window: Tauri v2 is the leading candidate (Rust core, system webview, reuses the React bundle); it uses the OS webview (WebKitGTK on Ubuntu, WKWebView on macOS, WebView2 on Windows) — the exact Ubuntu package names and minimum versions must be verified in the P0 spike. None of these have been verified yet.

---

## Surgical Scope

Files that **should** be touched (this repo, during planning only):
- `BRAINSTORMING_LOG_RUSTPORT.md` — this log
- `memory/decisions.md` — the decision record, once the owner locks the path (Supervisor writes)

New Rust project inside this repo (own board file, per Gate 4):
- `rust/PROJECT_SPEC.md`, `rust/PROJECT_KANBAN.md`, `rust/docs/`, the Cargo workspace under `rust/`; `tasks/TASK_GUIDE_T001.md`, `T002.md`

Files that **must not** be touched:
- `src/easy_personal/**` — the Python app is the reference; no behaviour change during the port
- `src/easy_personal/core/migrations/**` — frozen at the P0 number; Rust must not add or edit a Python migration
- `web/**` — the React bundle is served as-is, never ported
- Root `PROJECT_KANBAN.md` — a Linear cache; the Rust work has its own board at `rust/PROJECT_KANBAN.md` (Gate 4)

---

## Edge Case Checklist for TASK_GUIDE

- [ ] Rust opens a store whose `user_version` is **newer** than it knows → refuses with a clear message, writes nothing
- [ ] Python and Rust both running against the same store file → one writer only; the other reads or waits (`busy_timeout`), never corrupts
- [ ] JSON parity: float rounding, key order, null vs missing, integer vs float, UTC vs local-day boundaries, abstained counts
- [ ] Rule and price YAML read identically by both; a bad file fails closed, not silently
- [ ] Secrets never reach argv, logs, store or the page (leak suite ported as a Rust test)
- [ ] HTTP binds `127.0.0.1` only; Host/Origin/token guard on every `/api/*` route (NFR-001, NFR-008)
- [ ] `exec` launcher stays < 200 ms and only touches the keychain (NFR-006)
- [ ] No keychain backend (headless Ubuntu) → a clear error, never plaintext storage
- [ ] Path handling: home directory, `~/.claude*` account folders, separators and case sensitivity differ on macOS and Windows
- [ ] Scheduling differs per OS (systemd user timer, launchd, Task Scheduler) → one in-binary scheduler design, per-OS installer
- [ ] Large transcripts: ingest streams line by line with bounded memory (NFR-005); a corrupt line is counted as abstained, never a crash
- [ ] `claude` CLI missing or a different version → connector writes degrade with a message (DDR-0001)
- [ ] Window: the webview differs per OS (a missing or old WebKitGTK on Ubuntu, WebView2 absent on older Windows) → detect and show a clear message, never a blank window
- [ ] Window: the page loads from `127.0.0.1` on a random port with the token; another local process or browser tab must not be able to reuse it (NFR-001, NFR-008)
- [ ] Window: close, quit and crash must stop the server and release the store lock; a second launch focuses the first window instead of starting a second writer
- [ ] Window: headless or SSH Ubuntu has no display → the CLI and MCP stdio paths must still work without it
- [ ] Cross-compiling and signing: macOS notarization and Windows signing are deferred, but the CI matrix must not assume Linux paths
