# TASK_REVIEW — HUN-228: Usage page shows each card when its data arrives (NFR-013 page)

> Sibling of `tasks/TASK_GUIDE_HUN-228.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Auto decisions

Mode: auto
1. Extended track — RULE: `AGENTS.md` § Tracks ("a UI component")
2. Each Usage part renders when its own data arrives (per-part loading and error states) — RULE: owner approved the proposal "Page: show each card as its data arrives" ("yes, investigate and fix them [auto]")
3. `since` rounded down to the whole minute in Usage and the rail, so the same question gives the same URL (the 7-day window may start up to 59 s earlier) — SAFER OPTION: no API change; lets HUN-227 share work between the rail and Usage ⚠ decided for you — please review
4. Requirement Fidelity Gate signed `auto (Supervisor)` — RULE: `docs/claude-md/tracks.md` `### Every user stop and its auto rule` (row 13)
5. Spawn model `claude-sonnet-5-5` (C2) — FACT: `PROJECT_SPEC.md:165`
6. Companion-score row marked pass though overall is 75 (below the 80 example line), because the only failing input is lock-file hash hits outside this diff — SAFER OPTION: not blocking the merge gate on an input this change cannot affect; the number and reason are in the row ⚠ decided for you — please review
Checks: tests `cd web && npx vitest run src/UsageView.test.jsx src/Rail.test.jsx && npx vitest run && npm run build` — 109 passed, 651 passed, built; reviews code-review — P0 0 / P1 0 / P2 0 / P3 0; easy-verifier overall 75 (advisory); real run on a store copy (implementer) — first card ~1.5 s (was ~6 s, nothing before the end), KPI strip median 4.05 s (was 6.24 s), full page ~6.2–7.2 s (unchanged, set by the server: HUN-227) (auto-checked, not /verify)
Not done: merge (yours); auto verify: running (T182)


## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `web/src/UsageView.test.jsx` (describe "each part renders when its data arrives (HUN-228, NFR-013)": AC1 sessions held, per-part order, pill only while nothing arrived, AC2 outliers failing, AC3 badges loading/then filled and failed, AC4 late old-period answer dropped (mutation check: removing the guard makes it fail), AC5 `sinceOf` + one `since` per load, All time) and `web/src/Rail.test.jsx` (describe "Rail and Usage ask the same question": same summary and model URLs, `since` seconds 00 / ms 000). Output below. |
| Verification command run | ☑ pass | `cd web && npx vitest run src/UsageView.test.jsx src/Rail.test.jsx && npx vitest run && npm run build` -> `Test Files 2 passed (2) / Tests 109 passed (109)`; `Test Files 21 passed (21) / Tests 651 passed (651)`; `✓ built in 528ms` (run 2026-11-05 after the commit, task state file rewritten before the run). |
| Negative cases hold | ☑ pass | outliers 500 -> exactly one alert "Could not load outlier sessions: … 500", all other parts render; 403 on everything -> one error line per part; previous failed -> "no previous data", no alert; All time -> 7 requests, no `since`, no `until`; old-period answer never shown. |
| verify | ☐ pass / ☐ fail / ☐ N/A | not run by the implementer (Supervisor Stage 5). Real run on a store copy: see Demonstration, pass. |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Stage 4 code-review (Supervisor, 2026-11-05): diff `main...HEAD` = `web/src/UsageView.jsx`, `Rail.jsx`, `app.css` + their tests; direct callers `App.jsx` (renders `UsageView`), `Rail.jsx` (imports `PERIODS`, `sinceOf`). Not reviewed: other views (unchanged). lane: `C2, 5 files, 360 lines, FITS`. Findings: P0 0, P1 0, P2 0, P3 0 after the confidence gate (suppressed at 50: nine per-part `role="status"` lines may announce together for screen readers). Process note: implementer did not read `BRAINSTORMING_LOG_PERF.md` (guide Mandatory Startup 6; not a Permanent-Rule read). Supervisor re-ran: 109/109 and 651/651 passed, `✓ built in 274ms` — pass |
| Full smoke suite still green (no regression) | ☑ pass | `npx vitest run` -> `Test Files 21 passed (21), Tests 651 passed (651)` |
| **Companion score (easy-verifier)** | ☑ pass | advisory, below the 80 line: overall 75, change_risk 100 (kind feature), scope changes, ref `main...HEAD`; the gap is security 30 from `redaction_hits_observed` 124, counted in `uv.lock` / `web/package-lock.json` hashes that this diff does not touch (see Auto decision 6), report: `/home/hungnguyenhuu/.easy-verifier-reports/agent-a72982c50058ec492/evidence-report-changes-20261105T085304-071820Z.html` |
| **UI: Visual regression (diff or verdict pasted)** | ☑ pass | 1280x720, all loaded, BEFORE `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/before-1.png` vs AFTER `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/after-1.png`: same layout (same cards, order, spacing; the top 700 px side by side differ only in live numbers because the store is live and the window moved; full-page height 6771 vs 6786 px from different data, not layout). Sessions still loading: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/after-7-partial.png` (cropped bottom: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/after-7-partial-bottom.png`) shows all cards with "Loading usage by tool" in the pending part's place; the first-card state: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/after-4-firstcard.png`. |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ pass | one CSS rule added (`.part-loading`: flex + `gap: var(--space-2)`), no colour literal; reuses `.card`, `.muted`, `.error`, `.spinner`; page-wide `Loading` pill kept while nothing has arrived. Vitest: each loading line is `role="status"` with text "Loading <part>" (e.g. "Loading sessions"); each error line is `role="alert"` `.error`. |
| **UI: Responsiveness at target viewports** | ☑ pass | 1280 px: `hscroll: 1265 vs 1265` (no horizontal scroll; layout unchanged). 375 px: `hscroll: 375 vs 375`; loading-state screenshot `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/after-375-firstcard.png`. |

**Existing tests changed (AC6)**, in `web/src/UsageView.test.jsx` only, both because they asserted all-at-once behaviour:
1. "shows an error when the API refuses": was `findByRole('alert')` (one page-wide alert); now `findAllByRole('alert')`, more than one, each shows 403 (each part has its own error line).
2. "previous period is [now - 2 days, now - days) …": the `since`/`until` tolerance 5000 ms -> 65000 ms (since is rounded down to the whole minute); the "current `since` equals previous `until`" check is now exact equality.
Not changed: `Loading.test.jsx` and `neon.test.jsx` pass unchanged (the Usage view shows only the page-wide pill while no answer has arrived; a part shows no heading until it has data, so a heading on screen still means the data is there).

---

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: unchanged code (commit 3eca522), `npm run build`, real dashboard on a store copy (`sqlite3 ~/.easy-personal/store.sqlite ".backup <dir>/store.sqlite"`, 415 MB, `EASY_PERSONAL_HOME=<dir> uv run easy-personal dashboard`), headless Chrome 1280x720, Usage page (Last 7 days), polled every 25 ms. Script: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/measure.mjs`. Captured 2026-11-05T08:40:49Z..08:41:12Z, before any implementation commit.

```
== BEFORE run 1   start 2026-11-05T08:40:49.039Z
  response   910 ms  /api/usage/summary            (first answer)
  response  1423 ms  /api/usage/breakdown?by=day&until=…   (previous)
  response  1975 ms  /api/usage/outliers
  response  2590 ms  /api/usage/summary            (rail)
  response  3636 ms  /api/usage/breakdown?by=day
  ... model 3816, tool 4791, project 5300, rail model 5642
  response  5978 ms  /api/usage/sessions            (last)
marks: {"firstKpi":6020,"totals":6020,"timeline":6020}
== BEFORE run 2   marks: {"firstKpi":6244,"totals":6244,"timeline":6244}   (last response 6218 ms)
== BEFORE run 3   marks: {"firstKpi":6793,"totals":6793,"timeline":6793}   (last response 6748 ms)
```

Time to first KPI card = time to Totals = time to Timeline = 6.0 / 6.2 / 6.8 s (median 6.2 s): nothing is on screen until the last request returns, although the summary answered at ~0.9-1.0 s and the day breakdown at 3.6-4.5 s. The rail's `/api/usage/summary?since=…` and `/api/usage/breakdown?by=model&since=…` URLs differ from the Usage view's by milliseconds in `since`.

BEFORE screenshot (all data loaded, identical-layout reference): `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/before-1.png` (also before-2.png, before-3.png).

**AFTER**: same dashboard run on the same store copy (415 MB, ingested again at start), same script and headless Chrome 1280x720, rebuilt web at commit 548bc3a; captured 2026-11-05T08:47Z..08:49Z. Time (ms after navigation) to first card (first Usage heading on screen), to the KPI strip + Totals, and to the Session timeline (= full page):

```
run   firstCard   KPI strip + Totals   Timeline (full page)
 1       -           4622               6737        (first-card probe added from run 4)
 2       -           3311               6520
 3       -           3853               6424
 4     1637          5079               7199
 5     1707          4051               6868
 6     1839          4229               6558
 7     1438          3755               6201        (375 px run: 1627 / 3929 / 6344)
BEFORE (3 runs): first KPI = Totals = Timeline = 6020 / 6244 / 6793 ms
```
Median time to KPI strip: BEFORE 6244 ms, AFTER 4051 ms (7 runs: 3311 3755 3853 4051 4229 4622 5079). Time to the first card on screen: BEFORE 6.0-6.8 s (nothing until the end), AFTER 1.4-1.8 s. Time to full page: unchanged (6.2-7.2 s), set by the server. Each answer shows when it arrives; responses logged per run, e.g. run 2: summary 1135, outliers 1432, day 3284, prev-day 3588, model 3772, tool 4073, project 4404, sessions 6493 ms.
Same URLs: the Usage view's and the rail's `/api/usage/summary?since=…` now carry the same whole-minute `since` (unit test `Rail.test.jsx`).
Screenshots: `/tmp/claude-1000/-home-hungnguyenhuu-workspace-pets-hungnguyen111-easy-personal-mcp/a1aae442-7832-4f99-88fb-167a44a3581f/scratchpad/h228/after-1.png` .. `after-7.png`, partial states `after-*-partial.png`, `after-*-firstcard.png`.

**DELTA**: The owner sees the Usage cards one by one from about 1.5 s (outliers, breakdowns) and the KPI strip at about 4 s instead of a blank page until about 6 s; each pending part shows its own loading line and a failed part its own error line.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
