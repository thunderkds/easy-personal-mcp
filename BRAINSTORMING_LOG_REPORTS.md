# BRAINSTORMING_LOG_REPORTS.md — Saved investigate reports

**Date:** 2026-11-05 · **Tier:** Standard · **Traces:** US-032, FR-076..FR-078, NFR-002 (exception), DDR-0003

## The Problem Space

The `investigate` prompt (FR-075) makes the owner's Claude Code session write a report, and the report lives only in that chat. The owner wants it kept on the dashboard, to reread it and to see whether each warning got better since. Owner choices so far (2026-11-05): report text plus the server's own number snapshot, before / now per rule, a narrow NFR-002 exception, and a section on the Warnings & plan page.

Verified facts from the code:
- `rules.evaluate()` (`src/easy_personal/core/rules.py:670`) returns only fired `hits` and `abstained`. A rule that is measured but does not fire is dropped, with its value. A "40% now" for a rule that stopped firing needs that value.
- `_measure()` (`rules.py:659`) gives `(measured, detail)` for every rule. The value exists; only `evaluate` throws it away.
- MCP tools open the store with `store.opened()` (`core/store.py:96`): WAL plus `busy_timeout`. A write from the MCP process works next to ingest without a new lock.
- Latest migration is `m0009_metadata_abstained.py`, so the new table is `m0010`.
- The leak suite (`tests/leak/`, `MCP_CALLS` and `API_CALLS` in `tests/leak/harness.py`) proves NFR-002 with planted canaries. Every new surface must be registered there.

## Questions for the User

1. Which path (A / B / C)? Recommendation: **A**.

## Alternative Paths

### Option A — One table, JSON snapshot, `evaluate` lists every measured rule
- `m0010`: one table `investigate_reports(id, saved_utc, server_version, workspace, account, summary, fixes_json, snapshot_json)`.
- `rules.evaluate()` gains an opt-in `checked` list: every measured rule's `rule_id`, `measured`, `threshold`, `fired`. The default output stays the same, so FR-073 tests do not move.
- New `core/reports.py`: `validate()` (limits plus secret check), `save()`, `list()`, `get_with_now()`. "Now" = `evaluate(checked=True)` for the saved scope, joined by `rule_id`.
- MCP `save_report`; prompt step 7; `GET /api/reports` and `GET /api/reports/{id}`; page section.
- **Pros:** smallest schema; the snapshot stores exactly what the page shows; "now" reuses the warnings code.
- **Cons:** JSON blobs cannot be queried across reports in SQL. A trend chart later would need to parse them.

### Option B — Normalized tables
- `investigate_reports`, `report_fixes`, `report_rules(report_id, rule_id, measured, threshold, fired, causes_json)`.
- **Pros:** a trend of a rule across reports is one SQL query.
- **Cons:** 3 tables and joins for a feature whose charts are in the cut list. More migration and test surface.

### Option C — Store `evaluate()` output verbatim (50% path)
- No change to `rules.py`. The snapshot = the warnings output at save time (hits plus abstained).
- **Pros:** least code; no change to rule code.
- **Cons:** a rule that did not fire at save time has no number, and "now" has no number for a rule that stopped firing. The page can only say `218% (fired) → clear`, never `→ 40%`. That misses the owner's "before / now per warning" choice, which needs a number.

## Adversarial Review

- **A could fail** if the `checked` option leaks into the default output and changes `warnings` responses. Guard: a test that the default output is byte-identical with and without the change.
- **B could fail** by over-building: the cut list has no charts, so the extra tables carry no current value (Simplicity First).
- **C could fail** the owner's main ask: "trace the improvement" with no number after a fix works.

## 50% Rule Check

Option C is the 50% path. It removes the `rules.py` change (~15 lines) but loses the "now" number. Option A keeps that number for about 15 extra lines, so A is the vital slice. Cut list (not built): delete/edit a report, tick a fix as tried, a CLI command, charts across reports, pagination past the newest 50.

## Recommended Path

**Option A.** Two tasks:
1. Backend (C2, Medium risk: store migration plus the first MCP write tool): m0010, `evaluate(checked=True)`, `core/reports.py`, MCP `save_report`, prompt step 7, two API routes, leak-suite registration plus canary tests.
2. Frontend (C2, Low risk, after 1): Saved reports section on `WarningsPlanView.jsx`, with the open-report before / now view.

Mockup of the section:

```
Saved reports
┌───────────────────────────────────────────────────────────────┐
│ 2026-11-05 08:40 · all workspaces · 1 rule fired              │
│ Token use rose 218% from more sessions, not bigger ones…   ▸  │
├───────────────────────────────────────────────────────────────┤
│ 2026-10-29 …                                               ▸  │
└───────────────────────────────────────────────────────────────┘
  ▾ opened
  Summary   …
  Fixes     1. Start a new session per task…  (session_outliers: 645,301)
  Before / now (now = last 7 days)
    Token use jumped        218% fired   →   40% clear
    Cache reuse low         96.9% clear  →   97.1% clear
    Code index out of date  abstained    →   abstained (easy-context down)
```

## Surgical Scope

- **Touch:** `src/easy_personal/core/rules.py` (opt-in `checked` only), new `src/easy_personal/core/reports.py`, new `src/easy_personal/core/migrations/m0010_investigate_reports.py`, `src/easy_personal/surfaces/mcp.py`, `src/easy_personal/surfaces/api.py`, `web/src/WarningsPlanView.jsx`, `web/src/api.js`, tests, `tests/leak/harness.py`, README/site tool lists (the "new tool breaks skeleton/README/site tests" learning).
- **Must not touch:** rule maths, thresholds or which rules fire (FR-073); ingest and the live tail; existing `warnings` / `improvement_plan` output shape.

## Edge Case Checklist for TASK_GUIDE

- Unknown `workspace` / `account`: `evaluate` raises `ValueError`, so `save_report` refuses and stores nothing.
- Secret check false positives: session ids are UUIDs, and paths contain `/`. The token-like run must not match a UUID or a file path. Test both.
- A rule removed from the config since the save shows "not measured now". A changed threshold shows each side with its own threshold.
- A store older than v10 under an already-running MCP server (today's stale-process case). The prompt must say: if `save_report` is not available, tell the owner to reconnect the server in `/mcp`.
- The model calls `save_report` twice: two reports are stored. Accepted; no dedupe.
- Report text is HTML-escaped on the page (React default; no `dangerouslySetInnerHTML`).
- The list returns the newest 50; each item's summary shows only its first line.
- The leak suite: canaries in `save_report` text are refused when secret-like. Non-secret report text is the one allowed store text (DDR-0003). Fixture message canaries still never reach `/api/reports`.

## Next Actions

1. Owner picks a path.
2. Stage 2: PROJECT_SPEC rows, two Linear tasks, two TASK_GUIDEs.

## User Selection

**Option A** — owner, 2026-11-05: "as you recommend". Tasks HUN-221 (backend), HUN-222 (page, after HUN-221).
