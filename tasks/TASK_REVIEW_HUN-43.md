# TASK_REVIEW — HUN-43: `ui` source: easy-ui UI checks overall (never `target`) + `kit_summary` part

> Sibling of `tasks/TASK_GUIDE_HUN-43.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass / ☐ fail | `tests/sources/test_ui.py` (18 tests: AC1 only the five fields kept, `target`/`issues`/action labels never stored; AC2 unset/missing folder → `no data`; AC3 env, `config.yaml` `ui: reports_dir:` key, env overrides key, `~` expanded, scheduled-run simulation keeps sessions; dedupe by `id`, >20 MB skipped, bad JSON skipped); leak suite: canaries `ui_target`, `ui_issue`, `ui_action` planted in session files, `test_ui_ingest_actually_ran` guards a vacuous pass (`tests/leak/canaries.py`, `harness.py`, `conftest.py`, `test_leak_suite.py`). Supervisor re-run 2026-10-22T04:18:39Z: `40 passed, 1 warning in 7.60s` |
| Verification command run | ☒ pass / ☐ fail | `uv run pytest tests/sources/test_ui.py tests/leak -q` → `40 passed, 1 warning in 7.60s`; `uv run ruff check src tests` → `All checks passed!` |
| Negative cases hold | ☒ pass / ☐ fail | unset folder → `no data`; missing folder → `no data`; unreadable/oversized file → skipped and counted, ingest continues; second ingest of the same folder → same 203 sessions (dedupe by `id`) |
| verify | ☒ pass / ☐ fail / ☐ N/A | verify recipe 2026-10-22T04:19Z, isolated `uv tool install`, `EASY_PERSONAL_UI_REPORTS` unset, folder set only in `config.yaml` (`ui: reports_dir:`), as the systemd timer would run; read the owner's real easy-ui folder (203 `session-*.json`, read-only): ingest `{'sessions': 203, 'skipped': 0, 'status': 'ok'}`, 2nd ingest same 203; `easy-personal kit` → `ui: {'actions': 4740, 'failed': 93, 'passed': 110, 'other': 0, 'sessions': 203, 'first_utc': '2026-07-01T09:36:58.263000Z', 'last_utc': '2026-10-16T11:22:14.935000Z', 'status': 'ok'}`; `ui_sessions` columns = id, status, started_at_utc, ended_at_utc, action_count. Leak check: 9,321 caller-authored strings (≥8 chars) from `target`/`issues`/`actions` of the real files → 0 in kit output, 0 in `ui_sessions`; 1 match in `events`/`source_files` rows = the account's home-folder name inside fixture transcript paths (not from easy-ui) — feature confirmed working — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☒ pass / ☐ fail | `main...HUN-43`: `sources/ui.py` (new), `core/store.py` (+table), `core/config.py` (+`ui_reports_dir`), `core/kit.py` (+top-level `ui`), `surfaces/cli.py` (ingest step), leak suite; callers: `kit_summary` (MCP/CLI), `cli._ingest` (ingest, dashboard, systemd timer). lane_check `lane: C1, 11 files, 265 lines, EXCEEDS (files 11 > 6; lines 265 > 250)` — mostly leak-suite wiring + tests. code-review: 1 P1 found and fixed in `accf575` (folder from env only → the systemd timer (no `Environment=`) would clear `ui_sessions` every scheduled ingest; now also `config.yaml` `ui: reports_dir:`); P3: `ui_sessions` not pruned by `retention_days`; rows for files easy-ui later deletes are kept (upsert only, intended). Startup reads complete (in full). migration-safety (Supervisor, skill checklist): GO — additive `CREATE TABLE IF NOT EXISTS ui_sessions` via `store.connect`; dry run on a populated store copy: up (events 31, source_files 5, context_index unchanged; +ui_sessions), `main` code reads the upgraded store (sessions 4, messages 3), down `DROP TABLE ui_sessions` lossless (rebuilt from the folder), up again recreates it. security (inline, Medium): read-only folder glob, size cap, decoded object dropped after five values, no caller text stored or printed. |
| Full smoke suite still green (no regression) | ☒ pass / ☐ fail | `uv run pytest -q` → `242 passed, 2 skipped, 1 warning in 21.52s` (2026-10-22T04:18Z) |
| easy-verifier score (advisory) | ☒ recorded / ☐ N/A | 2026-10-22, MCP `score` repo=HUN-43 worktree, scope=changes, ref=`main...HUN-43`: overall **77** (architecture 100, code-quality 100, reliability 100, test-strategy 75, requirement-fidelity 67, blast-radius 65, security 30); abstained: solution-fit, supply-chain, api, data, operations, infrastructure. security = `redaction_hits_observed` 3 (known false-positive class; live leak check above found no caller text stored); blast-radius = `changed_files_in_churn_hotspots_share` 0.42. Raw JSON /home/hungnguyenhuu/.claude-personal/projects/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/75d1f65a-8da1-4f4c-8245-0270482d2af1/tool-results/mcp-easy-verifier-score-1792642786547.txt. Advisory only. |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☒ N/A | Pure backend task: no UI component (Hard-Stop Gate 6 N/A) |

Migration-safety note (checklist done by hand, `skills/migration-safety/SKILL.md`): schema change is one additive `CREATE TABLE IF NOT EXISTS ui_sessions` (5 columns) in `core/store.py` SCHEMA, the project's only schema mechanism (like `context_index`); no ALTER/DROP/RENAME, no NOT NULL added to an existing table, no backfill, no index; old code ignores the table, new code on an old store creates it on connect (expand only, zero downtime). Rollback = `DROP TABLE ui_sessions` (nothing else reads it; data re-derivable from the report folder). Dry run: new tests build the table on a fresh store and the end-to-end run above created it on an empty home; the full suite passes. Verdict: GO.

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: captured 2026-10-22T04:13:44Z, before any implementation commit (empty temp home, `EASY_PERSONAL_HOME=$D/h EASY_PERSONAL_CLAUDE_PROJECTS=$D/p uv run easy-personal kit`):

```
{
  "projects": []
}
```
No `ui` key in `kit_summary`; `ui_sessions` appears nowhere in `core/store.py`.

**AFTER**: 2026-10-22T04:16:14Z, one synthetic session file (target and an action label hold "SECRET"), `EASY_PERSONAL_UI_REPORTS` set, `easy-personal ingest` then `easy-personal kit`:

```
"ui": {"actions": 1, "failed": 0, "first_utc": "2026-10-01T10:00:00.000000Z", "last_utc": "2026-10-01T10:00:00.000000Z", "other": 0, "passed": 1, "sessions": 1, "status": "ok"}
```

**DELTA**: `kit_summary` now reports easy-ui checks overall (counts, actions, first/last time) and never stores the caller-authored `target`.

**WITNESS**: Supervisor — migration dry run 2026-10-22T04:16Z, test re-runs 04:18Z and verify recipe on the real easy-ui folder 04:19Z in the HUN-43 worktree (attributed to HUN-43 via the active_task state file; `memory/event-trace/HUN-43.jsonl`), independent of the implementing agent.
