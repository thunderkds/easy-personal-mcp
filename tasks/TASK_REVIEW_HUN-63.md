# TASK_REVIEW — HUN-63: Ingest every confirmed Claude account; account filter

> Sibling of `tasks/TASK_GUIDE_HUN-63.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/core/test_event_accounts.py` (AC1 migration from v4 with rows, FK, index, id/label filter, unknown label, NULL=default, AND with workspace AC8, query plan, 1M-row perf), `tests/sources/test_claude_code_accounts.py` (AC2 confirmed/detected/ignored/missing/late/override, AC3 shared message once, AC5 kill and resume equals clean run), `tests/sources/test_ingest_perf.py` (AC6), `tests/surfaces/test_account_filter.py` (AC4/AC8 CLI + API, 2 accounts x 2 workspaces); AC7 stays covered by `test_source_files_unchanged_after_ingest`; `tests/leak/harness.py` variants registered. Run: 414 passed (sources+core+surfaces+leak), full suite 595 passed, 2 skipped, 4 deselected (perf). |
| Verification command run | ☑ pass | `uv run pytest tests/sources tests/core tests/surfaces tests/leak -q` -> `414 passed, 4 deselected`; `uv run pytest -m perf -q -s` -> `4 passed` with `perf ingest: 215 MB, 50 files, cold 5.2s = 41.2 MB/s; unchanged re-run 0.00s`, `perf m0005 on 1M rows incl. backup: 0.96s; one-time NULL backfill 4.31s; no-op backfill 0.0ms`, `usage_summary 30d over 1M events: [245, 248, 248] ms`; `uv run pytest -q` -> `595 passed, 2 skipped, 4 deselected`. `uv run ruff check .` -> `All checks passed!` |
| Negative cases hold | ☑ pass | unknown account -> `AccountError` (CLI exit 2 listing `Claude default, Claude work`; API 400), message never echoes the input; detected/ignored/missing accounts skipped; missing account keeps rows and offsets; FK refuses unknown account id; leak suite green with account variants. |
| verify | ☑ pass | verify by Supervisor 2026-10-26T09:44:24Z: isolated install, temp agent root with `.claude/projects` and `.claude-work/projects` (both copies of the fixtures) → first ingest reads default only (31 events); after `account confirm .claude-work --label 'Claude work'` ingest tags 31 + 31 (ids 1, 2); usage all 3 messages (shared message ids counted once), --account 'claude default' 3, --account 2 3; unknown → 'valid labels: Claude default, Claude work' exit 2; unchanged re-ingest 0.39 s — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed: m0005, `accounts.default_id/resolve`, `metrics._scope` (AND workspace + account), `claude_code.account_roots/ingest_accounts`, store `save_file`, cli/api params, leak variants, tests. code-review: P0 0 / P1 0 / P2 0 / P3 2 (`tool_signals` keyed by source path: a transcript copied into two accounts doubles suggestion counters — outside ACs, noted for HUN-71; README/RUNBOOK still call `--source` a transcript root). Accepted deviation: no backfill in the migration (migrations must not read the home folder) — NULL = default account at read time, tagged by the first ingest after the default row exists (index `events_account_ts` makes the IS NULL update cheap; 1M rows 4.31 s once). Per-account sums can exceed the total when message ids are shared — documented. security-review (inline): account ref matched against confirmed rows, error lists labels never the input, bound params only, read-only source access; no findings. migration-safety: GO — additive nullable column + index; dry run on .backup copy of owner store (v2→v5 on this branch) rows equal, fk/integrity ok. |
| Full smoke suite still green (no regression) | ☑ pass | `595 passed, 2 skipped, 4 deselected` (the 2 skips are pre-existing). `tests/core/test_store.py` column set updated for `agent_account_id`. |
| easy-verifier score (advisory) | ☑ N/A | N/A — easy-verifier MCP is registered only in the `~/.claude-personal` profile; this Supervisor session runs another profile, tool not loaded |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | Pure backend task; no UI file changed |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | Pure backend task; no UI file changed |
| **UI: Responsiveness at target viewports** | ☑ N/A | Pure backend task; no UI file changed |

### Migration-safety checklist (done by the implementer; no Skill tool available, Reviewer re-runs the skill)

- Migration 5 is additive: nullable `ADD COLUMN ... REFERENCES agent_accounts (id)` (no row rewritten) and `CREATE INDEX IF NOT EXISTS events_account_ts (agent_account_id, ts_utc)`. Idempotent (checks `PRAGMA table_info` first), so the "downgraded `user_version`" test pattern still works.
- Runs in the runner's `BEGIN IMMEDIATE` with the `user_version` bump; the runner writes `store.sqlite.bak-v4` first (restore path = put it back). Test: v4 store with rows keeps every row, adds column + index, leaves the backup.
- Backfill rule (deviation from AC1 wording, see report): the migration does not backfill. A migration must not read the home folder and the default account row may not exist yet, so **NULL = default account**: reads treat NULL as default (`accounts.resolve`), and every `ingest_accounts` run tags NULL rows with the default account id once it exists. Measured on 1M rows: migration with online backup 0.96 s; the one-time NULL backfill `UPDATE` 4.31 s; the no-op re-run 0.0 ms (index).
- Old code on a v5 store: refused by the runner ("newer than this program"); rollback = restore `store.sqlite.bak-v4` (loses events ingested after the upgrade; they are re-read from the transcripts that still exist).
- Large-table: `ALTER TABLE ADD COLUMN` is metadata-only in SQLite; the new index builds on the 1M-row table inside the 0.96 s above (index is created on an all-NULL column).

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-10-26 09:32 UTC, before any HUN-63 implementation commit; temp `EASY_PERSONAL_HOME`, temp `EASY_PERSONAL_AGENT_ROOT` with `.claude` and `.claude-work` (message ids renamed `wmsg_*`), both built from copies of `tests/fixtures/claude_code`; script output, long JSON trimmed to `head`):

```
# date -u
Mon Oct 26 09:32:05 AM UTC 2026
# easy-personal ingest
Using CPython 3.13.2
Creating virtual environment at: .venv
   Building easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-63
      Built easy-personal-mcp @ file:///home/hungnguyenhuu/workspace/pets/hungnguyen111/easy-personal-mcp-HUN-63
Installed 43 packages in 5ms
{
  "context": {
    "status": "no data"
# confirm work account
{
  "config_dir_env": "/tmp/tmp.SWPGJK5dQn/root/.claude-work",
  "folder": "/tmp/tmp.SWPGJK5dQn/root/.claude-work",
  "id": 2,
  "label": "Claude work",
  "provider": "claude",
  "status": "confirmed"
}
# easy-personal ingest (work confirmed)
{
  "context": {
    "status": "no data"
  },
# events by source dir
31
TMP/root/.claude/|31
# usage --account x
usage: easy-personal [-h] [--version]
                     {ping,mcp,ingest,schedule,usage,outliers,workspace,warnings,kit,plan,dashboard,exec,headers,connector,account,doctor} ...
easy-personal: error: unrecognized arguments: --account x
exit=0
```

Result: with the second account confirmed, ingest still reads only the default account (31 events, all under `.claude/`), and `usage --account x` is `unrecognized arguments`.

**AFTER** (2026-10-26 09:41 UTC, same script and fixtures, after the change; JSON trimmed to `head`):

```
# date -u
Mon Oct 26 09:41:44 AM UTC 2026
# easy-personal ingest
{
  "context": {
    "status": "no data"
  },
  "files": 5,
  "kit_trace": {
    "files": 0,
    "new_events": 0
# confirm work account
{
  "config_dir_env": "/tmp/tmp.JT1M1nA228/root/.claude-work",
  "folder": "/tmp/tmp.JT1M1nA228/root/.claude-work",
  "id": 2,
  "label": "Claude work",
  "provider": "claude",
  "status": "confirmed"
}
# easy-personal ingest (work confirmed)
{
  "context": {
    "status": "no data"
  },
# events by source dir
62
TMP/root/.claude-work/|31
TMP/root/.claude/|31
# usage --account x ... and per account (second run of the script)
# usage --account x
easy-personal: unknown account; valid labels: Claude default, Claude work
exit=0
# usage 
  "messages": 6,
  "sessions": 8
# usage --account "Claude default"
  "messages": 3,
  "sessions": 4
# usage --account "Claude work"
  "messages": 3,
  "sessions": 4
```

**DELTA**: Ingest now reads every confirmed Claude account (62 events from two accounts instead of 31 from one), and `usage` / `outliers` (CLI `--account`, API `?account=`) filter by account id or label, alone or with `--workspace`.

**WITNESS**: backend-developer agent, 2026-10-26 09:32 UTC (BEFORE) and 09:41 UTC (AFTER); the Supervisor must confirm from `memory/event-trace/HUN-63.jsonl`.
