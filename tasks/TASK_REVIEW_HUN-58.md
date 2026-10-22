# TASK_REVIEW — HUN-58: Bugfix: ingest crashes on pre-HUN-42 stores

> Sibling of `tasks/TASK_GUIDE_HUN-58.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/sources/test_verifier.py`: `test_connect_rebuilds_pre_status_verifier_scores` (AC a), `test_ingest_on_pre_status_store_succeeds` (AC a, CLI exit 0), `test_verifier_failure_does_not_fail_ingest` + `test_cli_ingest_keeps_other_results_when_verifier_fails` (AC b). Agent run before fix: `4 failed, 424 passed, 2 skipped`; Supervisor re-run after fix: `428 passed, 2 skipped, 1 warning in 26.75s` |
| Verification command run | ☑ pass | `uv run pytest -q` → `428 passed, 2 skipped, 1 warning in 26.75s`; `uv run ruff check src tests` → `All checks passed!` |
| Negative cases hold | ☑ pass | Verifier raising (`_projects` patched to raise `OperationalError` / `ZeroDivisionError`) → `{"status": "failed", "reason": "<TypeName>"}`, no traceback, CLI exit 0, `new_events` kept. Current 7-column store is not dropped (rebuild only when `status` column absent; copy of owner's store via `.backup`: ingest exit 0, columns unchanged). Second ingest after rebuild exit 0 (idempotent). |
| verify | ☑ pass | verify skill run by Supervisor 2026-10-22T10:49:01Z: `uv tool install` into temp dir, `--version` → `0.1.0`, `ping` → `easy-personal core ok`; pre-HUN-42 6-column store + fixture transcripts → `ingest exit=0`, `{"verifier":{"projects":0,"skipped":0,"status":"no data"},"new_events":31,"files":5}`, columns `project_path,status,scope,generated_at_utc,overall_value,overall_abstained,reason_code`; second ingest exit 0; MCP stdio `kit_summary` returns a result — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed: `core/store.py` (`connect`, new `_rebuild_old_verifier_scores`), `sources/verifier.py` (`ingest`/`_ingest`, `verifier_part`), their callers (`surfaces/cli.py` `_ingest`; `store.opened` in `surfaces/mcp.py`, `surfaces/api.py`), `tests/sources/test_verifier.py`. lane_check: `C1, 4 files, 112 lines, FITS`. code-review: P0 0 / P1 1 (process: agent's Startup reads line omits `agents/backend.md`; harness auto-loads it as the agent's system prompt — waived, no code change) / P2 0 / P3 1 (new status word `failed` beside ok/no data/abstained — optional). Suppressed <75: one-time DROP in `connect()` on a read surface could hit `database is locked` if it races an ingest on an old store (conf 50). security-review (built-in skill failed: no `origin/HEAD`; done inline): DROP/PRAGMA use constant identifiers only; failure result carries only `type(exc).__name__`, never the message (NFR-002); no findings. migration-safety: done by hand (agent; Skill tool unavailable to it) and re-checked by Supervisor — drop is of a rebuildable cache (DELETE+re-insert every ingest), idempotent, only fires when `status` is absent, no other table touched, rollback = previous build recreates nothing but works on 7-col table (`CREATE TABLE IF NOT EXISTS`). |
| Full smoke suite still green (no regression) | ☑ pass | `428 passed, 2 skipped` (includes HUN-36 leak suite) |
| easy-verifier score (advisory) | ☑ recorded | `scope=changes`, ref `main`: overall 50 (4 of 13 rated: blast-radius 46, requirement-fidelity 67, security 30, test-strategy 55); 9 abstained (architecture, solution-fit, code-quality, reliability, supply-chain, api, data, operations, infrastructure); change_risk abstained (`all_metrics_abstained`). security 30 driven by `redaction_hits_observed=117` computed from `uv.lock`, `web/package-lock.json`, board/guide files — not from the changed source; false positive for this diff. Raw: session tool-result file (778 KB, not committed). |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Pure backend bugfix; no UI file changed |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Pure backend bugfix; no UI file changed |
| **UI: Responsiveness at target viewports** | ☑ N/A | Pure backend bugfix; no UI file changed |
| Repro loop | ☑ pass | Guide Intake → Trigger loop: before fix (Supervisor 10:41:46Z, main @ 00bb8d5; agent re-ran before any change) `sqlite3.OperationalError: table verifier_scores has 6 columns but 7 values were supplied`; after fix (Supervisor 10:47:37Z) `exit=0`, verifier `no data`, 7 columns |
| Regression test | ☑ pass | `tests/sources/test_verifier.py` (4 tests listed above) |
| Smoke suite | ☑ pass | Installed-tool run (verify row) on old-shape store + MCP stdio `kit_summary` call — pass |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: see the Phase 1 repro loop in `tasks/TASK_GUIDE_HUN-58.md` (Intake → Trigger). Captured by the Supervisor at 2026-10-22T10:41:46Z on `main` @ `00bb8d5`, before any fix commit:

```
    conn.executemany("INSERT INTO verifier_scores VALUES (?, ?, ?, ?, ?, ?, ?)", rows)
    ~~~~~~~~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
sqlite3.OperationalError: table verifier_scores has 6 columns but 7 values were supplied
```

**AFTER**: same Phase 1 repro loop on the worktree build, Supervisor at 2026-10-22T10:47:37Z: `exit=0`, `"verifier": {"projects": 0, "skipped": 0, "status": "no data"}`, `PRAGMA table_info(verifier_scores)` → `project_path,status,scope,generated_at_utc,overall_value,overall_abstained,reason_code`.

**DELTA**: `easy-personal ingest` now succeeds on stores created before HUN-42 (the cache table is rebuilt automatically), and a verifier failure can no longer abort the whole ingest run.

**WITNESS**: Supervisor (not the implementing agent), BEFORE 10:41:46Z, AFTER 10:47:37Z, verify 10:49:01Z on 2026-10-22; Bash calls attributed to HUN-58 via `.claude/hooks/.state/active_task` → `memory/event-trace/HUN-58.jsonl`.
