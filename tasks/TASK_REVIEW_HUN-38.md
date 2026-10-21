# TASK_REVIEW — HUN-38: 

> Sibling of `tasks/TASK_GUIDE_HUN-38.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | New: `tests/ops/test_lock.py` (busy → LockBusy, waiter acquires after release, crashed holder does not block, 2 concurrent real `ingest` processes → `PRAGMA integrity_check` ok + no duplicate rows, CLI busy message), `tests/ops/test_retention.py` (unset keeps all, 30 d removes only older, CLI flag end to end), `tests/ops/test_schedule.py` (render, install, reinstall, remove, no systemd → cron snippet, dry-run; `systemctl` stubbed, temp `XDG_CONFIG_HOME`), `tests/ops/test_ingest_memory.py` (carried HUN-34 P2: >10 MB transcript, tracemalloc peak < 8 MB) |
| Verification command run | ☒ pass / ☐ fail | 2026-10-21T11:07Z Supervisor: `uv run pytest -q` → `113 passed, 2 skipped in 26.23s`; `uv run ruff check` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | Lock held by another process + `EASY_PERSONAL_LOCK_TIMEOUT=1` → stderr `another ingest is running; try again later`, exit 75; `--retention-days 0` → `invalid positive_int value: '0'`, exit 2; no systemd session → nothing written, cron snippet, exit 1 (test); `schedule remove` with nothing installed → exit 0 (test) |
| verify | ☒ pass / ☐ fail / ☐ N/A | 2026-10-21T11:07Z Supervisor, verify recipe (temp `EASY_PERSONAL_HOME`, `XDG_CONFIG_HOME`, fixtures): fresh-home `ingest` → files 5, new_events 31 (lock file created in data dir); busy lock → exit 75, after release → exit 0; `ingest --retention-days 3650` → `pruned: 0`, `--retention-days 1` → `pruned: 24`, `usage` messages 0 after; `schedule install --retention-days 30 --dry-run` prints service (`ExecStart=<abs path> ingest --retention-days 30`) + timer (`OnCalendar=daily`, `Persistent=true`), `schedule remove --dry-run` prints plan, temp XDG dir not created (nothing written). Real systemd never touched — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | 12 files in `main...HUN-38` + callers of `store`/`cli.main`. lane_check: `C1, 12 files, 510 lines, EXCEEDS (files 12 > 6; lines 510 > 250)` — under-labelled, reviewed at C2 depth. code-review 0 P0/P1, 2 P2 (systemctl failure → traceback; re-ingest of a replaced file restores pruned rows until next prune), 1 P3 (`%` in ExecStart path) — `reports/code-review_HUN-38_20261021T180830.html`; security (inline) pass — `reports/security-review_HUN-38_20261021T180830.html` (main checkout) |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | 2026-10-21T11:07Z `uv run pytest -q` → `113 passed, 2 skipped`; `uv run pytest tests/leak -q` green with `ingest --retention-days`, `schedule install|remove --dry-run` registered |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-21T11:08Z, server at 127.0.0.1:8766 called directly over HTTP, `score` repo=HUN-38 worktree, scope=changes, ref=`e8581ae...f16159c`: overall **75** (7 of 13: architecture 100, requirement-fidelity 67, code-quality 87, security 30, test-strategy 75, blast-radius 65, reliability 100); abstained: solution-fit, supply-chain, api, data, operations, infrastructure. Security 30 = redaction false positives (lockfile pack; a stray empty `ingest.lock` from the Supervisor's own first lock probe, since deleted). Report `~/.easy-verifier-reports/easy-personal-mcp-HUN-38/evidence-report-changes-20261021T110810-275625Z.html`. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend/ops task (no UI component) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend/ops task (no UI component) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend/ops task (no UI component) |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-21T11:03:37Z (temp EASY_PERSONAL_HOME / XDG_CONFIG_HOME, nothing installed):

```
$ uv run easy-personal schedule install
usage: easy-personal [-h] [--version] {ping,mcp,ingest,usage,outliers} ...
easy-personal: error: argument command: invalid choice: 'schedule' (choose from ping, mcp, ingest, usage, outliers)
rc=2
$ uv run easy-personal ingest --retention-days 30
usage: easy-personal [-h] [--version] {ping,mcp,ingest,usage,outliers} ...
easy-personal: error: unrecognized arguments: --retention-days 30
```

**AFTER**: 2026-10-21T11:06:11Z (temp dirs; `--dry-run`, nothing written or enabled):

```
$ easy-personal schedule install --dry-run --retention-days 90
# easy-personal-ingest.service
[Service]
Type=oneshot
ExecStart=<venv>/bin/easy-personal ingest --retention-days 90
# easy-personal-ingest.timer
[Timer]
OnCalendar=daily
Persistent=true
[Install]
WantedBy=timers.target
```

`uv run pytest tests/ops tests/leak -q` -> `34 passed`; full suite `113 passed, 2 skipped` (before leak registration run).

**DELTA**: A user can install a daily systemd user timer for ingest with one command, and ingest is safe to run twice at once and can prune old rows.

**WITNESS**: Supervisor, 2026-10-21T11:07–11:08Z — verify recipe, lock probe and test runs in the HUN-38 worktree (attributed to HUN-38 via the active_task state file; `memory/event-trace/HUN-38.jsonl`).
