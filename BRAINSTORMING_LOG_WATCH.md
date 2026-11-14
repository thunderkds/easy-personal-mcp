# BRAINSTORMING_LOG_WATCH.md — Watch service, tray icon and long-term effectiveness

Tier: **Deep** (new long-running process, possible store migration, new data source, UI).
Date: 2026-11-14. Host board: `rust/PROJECT_KANBAN.md` (the Python app is being retired, T061–T063).

## The Problem Space

The owner wants a background service that watches and collects state, with a tray icon and desktop
alerts. Today (verified):

- Live state exists only while `dashboard` runs: Python `src/easy_personal/core/live.py`, Rust
  `rust/crates/app/src/live_feed.rs`. Attention states live in memory only.
- Board (`core/board.py`) and scheduled jobs (`core/jobs.py`) are read live, never stored.
- Plan limits: only the latest status-line snapshot `<account>/easy-personal-limits.json`.
- Scheduled ingest is a daily oneshot systemd timer (`data/systemd/easy-personal-ingest.timer`).
- Machine status: no code reads CPU, RAM, disk, load or battery.
- Usage events are stored forever (unless `ingest --retention-days N`) and carry `git_branch`,
  `session_id` and `is_sidechain` (`sources/claude_code.py:138-169`, `m0001_baseline.py:34`).
- Per-session "good result" = a commit in the session window AND tool-error rate ≤ threshold
  (`core/kitperf.py:230`).
- Rust has a Tauri v2 window behind `--features window` (`rust/crates/app/src/window.rs`, `tauri = "=2.12.5"`).

### Decisions locked in grilling (2026-11-14, owner)

| # | Decision |
|---|---|
| D1 | Host = easy-personal-rs + Tauri (tray + notifications). |
| D2 | Desktop alerts: waiting for you, plan limit high, failing / hung, machine pressure (disk/RAM/battery). Lifts FR-050's "no notifications" and the Out-of-Scope cut "attention states in … notifications". |
| D3 | Live state: agent state, task state, token watch (usage + 5h/7d plan limits), machine status (CPU, RAM, disk, load, battery, Claude processes). Short window only. |
| D4 | Long-term history: **only** tokens used and agent effectiveness. |
| D5 | Effectiveness = **tokens per good result** and **tokens per task done**. |
| D6 | Task attribution = **branch + session link**: events on the task branch, plus sidechain (sub-agent) events of a session whose main thread is on that branch. Work on `main` = "unattributed", shown as its own line. |
| D7 | Tray menu and dashboard view design: **the owner provides it**; ask at the start of the UI task. |

## Questions for the User

Resolved: D1–D7. Still open (asked at Stage 2, one at a time):

1. Task done time: git merge commit (`Merge branch 'T061'`) or the board's Done row? (Recommended: git merge — it is in git history forever; the board is read live and can be edited.)
2. Minimal-track branches (`fix-fake-keychain`, `fix-race-test`) carry no task ID. Map them through `tasks/MINIMAL_LOG.md`, or show them as "unattributed"? (Recommended: unattributed first, MINIMAL_LOG mapping on the cut list.)
3. Autostart: systemd user service (`Restart=on-failure`) or an XDG autostart `.desktop` entry? (Recommended: XDG autostart — a tray needs the graphical session; systemd user units start before it on some setups.)

## Alternative Paths

### Option A — Compute-on-read (Minimalist)
One process, `easy-personal-rs tray`. It runs the existing live tail and in-memory samplers (60 s,
24 h ring buffer for machine status and plan-limit %). It raises notifications. Effectiveness is a
**query** over stored events plus local git merges. **No migration, no new table.**
- Pros: smallest code; no history to migrate or prune; numbers recompute when the rule changes.
- Cons: short-window graphs restart empty after a reboot; "tokens per task" depends on git history still being local.

### Option B — Ledger tables (Scalable)
The same process, plus two tables: `samples` (60 s, kept 30 days) and `task_ledger` (one row per
task when it reaches Done: tokens, cost, good results, sessions).
- Pros: survives reboots; fast reads; a ledger row is frozen even if git history is rewritten.
- Cons: a migration (Rust and Python must agree while both run); frozen rows go stale when the attribution rule changes; pruning code.

### Option C — Split daemon + tray (Separated)
A headless `watch` systemd service collects and stores. A separate Tauri tray reads it over the
loopback API (token-guarded) and shows alerts.
- Pros: collection survives a GNOME crash or logout; the tray stays thin.
- Cons: two processes to install and version; IPC and auth between them; most code of the three.

## 50% Rule Check

Option A is the 50% path. Tokens are already stored forever, so effectiveness needs a query, not a
collector. The short-window live state needs no disk. One process with a tray and notifications
replaces a daemon, IPC and a tray.

## Adversarial Review

| Option | Why this might fail |
|---|---|
| A | `ingest --retention-days N` deletes the events that effectiveness reads → the trend silently shortens. Rebased or squash-merged branches lose the merge commit → a task shows no Done time. The 24 h buffer is lost on restart. |
| B | Two writers (Python and Rust) during the retirement window → migration ordering risk (NFR-009). A ledger row frozen with a wrong attribution rule stays wrong forever. |
| C | GNOME logout kills the tray but not the daemon → alerts go nowhere, silently. Version skew between daemon and tray. It doubles the install surface right when T062 changes install steps. |

## Recommended Path

**Option A**, with one escape hatch: if the owner later wants short-window graphs to survive
reboots, add Option B's `samples` table only (never the ledger). Effectiveness stays compute-on-read.

## Surgical Scope

Should touch:
- `rust/crates/app/src/` — new `tray` subcommand (behind `--features window`), notifier, sampler module.
- `rust/crates/core/` — machine-status reader (`/proc/stat`, `/proc/meminfo`, `/proc/loadavg`, `statvfs`, `/sys/class/power_supply`), effectiveness query.
- `rust/crates/app/src/api.rs` — read-only routes for the live machine/plan buffer and effectiveness.
- `PRD.md` — new FRs; amend FR-050 and the Out-of-Scope line (D2).
- `rust/PROJECT_KANBAN.md`, `rust/PROJECT_SPEC.md`, `tasks/TASK_GUIDE_*.md`.

Must not touch:
- The Python app `src/easy_personal/` (being retired) — except nothing.
- Store schema (Option A adds no migration).
- `live_feed.rs` read-position logic (shared with scheduled ingest; no double counting).
- Any Claude config, hooks or status line (NFR-003).

## Edge Case Checklist for TASK_GUIDE

- [ ] Tray and `dashboard` run at once: both tail; the shared read position prevents double storage (README "Two dashboards").
- [ ] GNOME without the AppIndicator extension: no tray area → say so once, keep alerts working.
- [ ] Alert flapping: a value hovering at a threshold → hysteresis (for example alert at 90 %, re-arm below 85 %) and one alert per state change.
- [ ] "Pause alerts" must not drop the state, only the popup.
- [ ] Plan-limit file missing, broken or `stale` (reset passed) → no plan alert, show "no data".
- [ ] No battery (desktop) → battery is "no data", never 0 %.
- [ ] Several Claude accounts → plan limits and alerts per account.
- [ ] Branch is `main`, `HEAD`, `agent-xxx` or a minimal-track `fix-*` → unattributed unless linked through its session (D6).
- [ ] A sidechain session whose parent's main thread is on `main` → unattributed.
- [ ] A task branch merged twice (fix rounds) → Done time = the first merge or the last? (Stage 2.)
- [ ] `--retention-days` set → the effectiveness view states the oldest stored day.
- [ ] Unknown model price → "unknown price", never $0 (existing rule).
- [ ] No message text, tool input or output in any notification (NFR-002): name the tool, project and session id only.
- [ ] Notifications go over the local D-Bus only (NFR-001).
- [ ] The tray process stays bounded in memory (24 h × 1 sample/min ring buffer).

## Next Actions

1. Owner selects a path (below).
2. Stage 2: amend `PRD.md` (new FRs + FR-050 + Out of Scope), update `rust/PROJECT_SPEC.md`, then `to-issues`. Likely slices:
   machine-status reader → in-memory sampler + API → notifier with hysteresis → effectiveness query + API/MCP → tray shell (owner's design) → dashboard view (owner's design).
3. Resolve the three open questions above at Stage 2.

## User Selection

_Pending — owner will choose in the next session (2026-11-14). Resume: ask path A/B/C, then Stage 2 open questions 1–3 one at a time._
